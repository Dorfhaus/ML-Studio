use crate::*;
use ml_space_core::{convolver::Native as Convolution, impulse::Impulse};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ffi::{CStr, c_void},
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
const LATENCY: usize = 256;
const MAX_MODEL: usize = 8 * 1024 * 1024;
unsafe extern "C" {
    fn ml_amp_new(
        data: *const u8,
        size: usize,
        rate: f64,
        error: *mut i8,
        capacity: usize,
    ) -> *mut c_void;
    fn ml_amp_process(p: *mut c_void, input: *const f32, out: *mut f32, n: i32) -> bool;
    fn ml_amp_clear(p: *mut c_void);
    fn ml_amp_free(p: *mut c_void);
}
struct Native(*mut c_void);
unsafe impl Send for Native {}
impl Native {
    fn new(data: &str, rate: u32) -> Result<Self, String> {
        let mut error = [0i8; 1024];
        let p = unsafe {
            ml_amp_new(
                data.as_ptr(),
                data.len(),
                rate as f64,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        if p.is_null() {
            Err(unsafe { CStr::from_ptr(error.as_ptr()) }
                .to_string_lossy()
                .into_owned())
        } else {
            Ok(Self(p))
        }
    }
    fn clear(&mut self) {
        unsafe { ml_amp_clear(self.0) }
    }
    fn process(&mut self, input: &[f32], out: &mut [f32]) -> bool {
        unsafe { ml_amp_process(self.0, input.as_ptr(), out.as_mut_ptr(), input.len() as i32) }
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        unsafe { ml_amp_free(self.0) }
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct Asset {
    pub name: String,
    pub model: String,
    pub cab: String,
    #[serde(default)]
    pub cab_name: String,
}
impl Asset {
    fn validate(&self) -> Result<(), String> {
        if self.name.len() > 256
            || self.cab_name.len() > 256
            || self.model.len() > MAX_MODEL
            || self.cab.len() > 4 * 1024 * 1024
        {
            return Err("Amp asset exceeds the supported size".into());
        }
        if !self.model.is_empty() {
            let j: serde_json::Value =
                serde_json::from_str(&self.model).map_err(|_| "Invalid NAM JSON")?;
            let w = j
                .get("weights")
                .and_then(|v| v.as_array())
                .ok_or("NAM model has no weights")?;
            if w.is_empty()
                || w.len() > 1_000_000
                || w.iter()
                    .any(|v| v.as_f64().is_none_or(|n| !n.is_finite() || n.abs() > 1e6))
            {
                return Err("Invalid or oversized NAM weights".into());
            }
        }
        if !self.cab.is_empty() {
            let c = Impulse::load(&self.cab)?;
            if c.left.len() > c.rate as usize / 2 {
                return Err("Cabinet IR must be 500 ms or shorter".into());
            }
        }
        Ok(())
    }
    fn load_model(&mut self, path: &Path) -> Result<(), String> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take((MAX_MODEL + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        self.model = String::from_utf8(bytes).map_err(|_| "NAM must be UTF-8 JSON")?;
        self.name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .chars()
            .take(100)
            .collect();
        self.validate()
    }
    fn load_cab(&mut self, path: &Path) -> Result<(), String> {
        let ir = Impulse::import(path)?;
        self.cab_name = ir.name.clone();
        self.cab = ir.save();
        self.validate()
    }
}
pub(crate) struct Prepared {
    native: Option<[Native; 2]>,
    cab: Option<[Convolution; 2]>,
    pad: [[f32; 2]; LATENCY],
    at: usize,
    rate: u32,
    revision: u64,
}
impl Prepared {
    fn new(a: &Asset, rate: u32, revision: u64) -> Result<Self, String> {
        a.validate()?;
        let native = if a.model.is_empty() {
            None
        } else {
            Some([Native::new(&a.model, rate)?, Native::new(&a.model, rate)?])
        };
        let cab = if a.cab.is_empty() {
            None
        } else {
            let data = Impulse::load(&a.cab)?.at_rate(rate);
            Some([Convolution::new(&data[0])?, Convolution::new(&data[1])?])
        };
        Ok(Self {
            native,
            cab,
            pad: [[0.; 2]; LATENCY],
            at: 0,
            rate,
            revision,
        })
    }
    fn clear(&mut self) {
        if let Some(n) = &mut self.native {
            for x in n {
                x.clear();
            }
        }
        if let Some(c) = &mut self.cab {
            for x in c {
                x.clear();
            }
        }
        self.pad.fill([0.; 2]);
        self.at = 0;
    }
    fn process(
        &mut self,
        input: &[[f32; 256]; 2],
        out: &mut [[f32; 256]; 2],
        n: usize,
        cabmix: &[f32; 256],
    ) -> bool {
        let mut ok = true;
        if let Some(m) = &mut self.native {
            for ch in 0..2 {
                ok &= m[ch].process(&input[ch][..n], &mut out[ch][..n]);
            }
        } else {
            for i in 0..n {
                out[0][i] = self.pad[self.at][0];
                out[1][i] = self.pad[self.at][1];
                self.pad[self.at] = [input[0][i], input[1][i]];
                self.at = (self.at + 1) % LATENCY;
            }
        }
        if let Some(c) = &mut self.cab {
            let mut filtered = [0.; 256];
            for ch in 0..2 {
                c[ch].process(&out[ch][..n], &mut filtered[..n]);
                for i in 0..n {
                    out[ch][i] += (filtered[i] - out[ch][i]) * cabmix[i];
                }
            }
        }
        ok && out
            .iter()
            .all(|channel| channel[..n].iter().all(|v| v.is_finite() && v.abs() <= 16.))
    }
}
enum Request {
    Model(PathBuf),
    Cab(PathBuf),
    ClearModel,
    ClearCab,
}
pub(crate) struct Exchange {
    asset: Mutex<Asset>,
    request: Mutex<Option<(u64, Request)>>,
    revision: AtomicU64,
    rate: AtomicU32,
    pending: AtomicPtr<Prepared>,
    retired: AtomicPtr<Prepared>,
    stop: AtomicBool,
    status: Mutex<String>,
    edits: Mutex<Vec<String>>,
    fault: AtomicBool,
}
impl Exchange {
    fn new() -> Self {
        Self {
            asset: Mutex::new(Asset::default()),
            request: Mutex::new(None),
            revision: AtomicU64::new(1),
            rate: AtomicU32::new(0),
            pending: AtomicPtr::new(std::ptr::null_mut()),
            retired: AtomicPtr::new(std::ptr::null_mut()),
            stop: AtomicBool::new(false),
            status: Mutex::new(
                "Load a .nam amp model. No model loaded: audio passes through with compensation."
                    .into(),
            ),
            edits: Mutex::new(Vec::new()),
            fault: AtomicBool::new(false),
        }
    }
    fn request(&self, r: Request) {
        let mut request = self.request.lock().unwrap();
        let id = self.revision.fetch_add(1, Ordering::AcqRel) + 1;
        *request = Some((id, r));
        *self.status.lock().unwrap() = "Loading and preparing…".into();
    }
    pub(crate) fn replace(&self, a: Asset) {
        let mut req = self.request.lock().unwrap();
        *req = None;
        *self.asset.lock().unwrap() = a;
        self.revision.fetch_add(1, Ordering::AcqRel);
        *self.status.lock().unwrap() = "Preparing saved amp settings…".into();
    }
    fn reclaim(p: &AtomicPtr<Prepared>) {
        let ptr = p.swap(std::ptr::null_mut(), Ordering::AcqRel);
        if !ptr.is_null() {
            unsafe {
                drop(Box::from_raw(ptr));
            }
        }
    }
    fn publish(&self, p: Prepared) {
        let old = self
            .pending
            .swap(Box::into_raw(Box::new(p)), Ordering::AcqRel);
        if !old.is_null() {
            unsafe {
                drop(Box::from_raw(old));
            }
        }
        self.fault.store(false, Ordering::Release);
    }
}
impl Drop for Exchange {
    fn drop(&mut self) {
        Self::reclaim(&self.pending);
        Self::reclaim(&self.retired);
    }
}
pub(crate) struct AmpParams {
    pub fx: Arc<FxParams>,
    pub shared: Arc<Exchange>,
}
unsafe impl Params for AmpParams {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        self.fx.param_map()
    }
    fn serialize_fields(&self) -> BTreeMap<String, String> {
        let mut f = self.fx.serialize_fields();
        f.insert(
            "amp_assets".into(),
            serde_json::to_string(&*self.shared.asset.lock().unwrap()).unwrap(),
        );
        f
    }
    fn deserialize_fields(&self, f: &BTreeMap<String, String>) {
        self.fx.deserialize_fields(f);
        if let Some(s) = f.get("amp_assets") {
            let parsed = if s.len() <= 16 * 1024 * 1024 {
                serde_json::from_str::<Asset>(s)
                    .map_err(|e| e.to_string())
                    .and_then(|a| a.validate().map(|_| a))
            } else {
                Err("Saved amp assets exceed size limit".into())
            };
            match parsed {
                Ok(a) => self.shared.replace(a),
                Err(e) => *self.shared.status.lock().unwrap() = e,
            }
        }
    }
}
pub(crate) struct Panel {
    pub shared: Arc<Exchange>,
}
impl Extension for Panel {
    fn take_state_edits(&self) -> Vec<String> {
        std::mem::take(&mut *self.shared.edits.lock().unwrap())
    }
    fn accepts_files(&self) -> bool {
        true
    }
    fn file_dropped(&self, p: &Path) {
        if p.extension().is_some_and(|s| s.eq_ignore_ascii_case("wav")) {
            self.shared.request(Request::Cab(p.to_owned()));
        } else {
            self.shared.request(Request::Model(p.to_owned()));
        }
    }
    fn paint(&self, c: &mut Canvas, _: &dyn lemon_small_fx::view::Model) {
        if !os_plugkit::visibility::stock_dsp_controls() {
            return;
        }
        for (i, label) in ["Load .nam…", "Clear amp", "Load cabinet…", "Clear cabinet"]
            .iter()
            .enumerate()
        {
            button(c, Rect::new(16. + i as f32 * 160., 170., 150., 30.), label);
        }
        let a = self.shared.asset.lock().unwrap();
        text(
            c,
            16.,
            218.,
            &format!("Amp: {}", if a.model.is_empty() { "None" } else { &a.name }),
        );
        let cab = if a.cab.is_empty() {
            "None".into()
        } else {
            a.cab_name.clone()
        };
        text(
            c,
            16.,
            242.,
            &format!("Cabinet: {}", cab.chars().take(85).collect::<String>()),
        );
        drop(a);
        let status = if self.shared.fault.load(Ordering::Acquire) {
            "Model output was invalid or excessive; using dry audio. Check the model and input level.".into()
        } else {
            self.shared.status.lock().unwrap().clone()
        };
        text(c, 16., 270., &status.chars().take(95).collect::<String>());
        text(
            c,
            16.,
            293.,
            &status.chars().skip(95).take(95).collect::<String>(),
        );
        text(
            c,
            16.,
            324.,
            "Drop .nam or cabinet WAV here • assets saved in project • cabinet IR maximum 500 ms",
        );
        text(
            c,
            16.,
            348.,
            "Mono models run independently in stereo • rate conversion included • fixed 256-sample latency",
        );
    }
    fn click(&self, x: f32, y: f32, _: Option<&Arc<dyn GuiContext>>) -> bool {
        if !os_plugkit::visibility::stock_dsp_controls() {
            return false;
        }
        for i in 0..4 {
            if Rect::new(16. + i as f32 * 160., 170., 150., 30.).contains(x, y) {
                match i {
                    0 => {
                        if let Some(p) = rfd::FileDialog::new()
                            .add_filter("NAM amp model", &["nam"])
                            .pick_file()
                        {
                            self.shared.request(Request::Model(p));
                        }
                    }
                    1 => self.shared.request(Request::ClearModel),
                    2 => {
                        if let Some(p) = rfd::FileDialog::new()
                            .add_filter("Cabinet impulse response", &["wav"])
                            .pick_file()
                        {
                            self.shared.request(Request::Cab(p));
                        }
                    }
                    _ => self.shared.request(Request::ClearCab),
                }
                return true;
            }
        }
        false
    }
}
pub struct Amp {
    pub(crate) params: Arc<AmpParams>,
    worker: Option<JoinHandle<()>>,
    active: Option<Box<Prepared>>,
    old: Option<Box<Prepared>>,
    fade: usize,
    fade_len: usize,
    rate: f32,
    dry: [[f32; 2]; LATENCY],
    at: usize,
    smooth: [f32; 4],
}
impl Default for Amp {
    fn default() -> Self {
        let shared = Arc::new(Exchange::new());
        let params = Arc::new(AmpParams {
            fx: Arc::new(FxParams::new(&specs::AMP)),
            shared: shared.clone(),
        });
        let worker = thread::spawn(move || {
            let mut done = (0, 0);
            while !shared.stop.load(Ordering::Acquire) {
                Exchange::reclaim(&shared.retired);
                let rate = shared.rate.load(Ordering::Acquire);
                let rev = shared.revision.load(Ordering::Acquire);
                if rate != 0 && (rate, rev) != done {
                    let request = shared.request.lock().unwrap().take();
                    let mut a = shared.asset.lock().unwrap().clone();
                    let user = request.is_some();
                    let result = (|| -> Result<Prepared, String> {
                        if let Some((id, r)) = request {
                            if id != rev {
                                return Err("Load superseded".into());
                            }
                            match r {
                                Request::Model(p) => a.load_model(&p)?,
                                Request::Cab(p) => a.load_cab(&p)?,
                                Request::ClearModel => {
                                    a.model.clear();
                                    a.name.clear();
                                }
                                Request::ClearCab => {
                                    a.cab.clear();
                                    a.cab_name.clear();
                                }
                            }
                        }
                        Prepared::new(&a, rate, rev)
                    })();
                    // Serialize acceptance against GUI requests/state loads; a failed load keeps the previous assets.
                    let _guard = shared.request.lock().unwrap();
                    if shared.rate.load(Ordering::Acquire) == rate
                        && shared.revision.load(Ordering::Acquire) == rev
                    {
                        match result {
                            Ok(p) => {
                                if user {
                                    let mut old = shared.asset.lock().unwrap();
                                    let patch=serde_json::json!({"fields":{"amp_assets":serde_json::to_string(&*old).unwrap()}}).to_string();
                                    let mut edits = shared.edits.lock().unwrap();
                                    while !edits.is_empty()
                                        && (edits.len() >= 32
                                            || edits.iter().map(String::len).sum::<usize>()
                                                + patch.len()
                                                > 128 * 1024 * 1024)
                                    {
                                        edits.remove(0);
                                    }
                                    edits.push(patch);
                                    *old = a;
                                }
                                shared.publish(p);
                                *shared.status.lock().unwrap() =
                                    "Ready. Amp and cabinet changes crossfade during playback."
                                        .into();
                            }
                            Err(e) => {
                                if user {
                                    let prior = shared.asset.lock().unwrap().clone();
                                    if let Ok(p) = Prepared::new(&prior, rate, rev) {
                                        shared.publish(p);
                                    }
                                }
                                *shared.status.lock().unwrap() = e
                            }
                        }
                    }
                    done = (rate, rev);
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
        Self {
            params,
            worker: Some(worker),
            active: None,
            old: None,
            fade: 0,
            fade_len: 1,
            rate: 48000.,
            dry: [[0.; 2]; LATENCY],
            at: 0,
            smooth: [1., 1., 1., 1.],
        }
    }
}
impl Drop for Amp {
    fn drop(&mut self) {
        self.params.shared.stop.store(true, Ordering::Release);
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}
impl Amp {
    fn targets(&self) -> [f32; 4] {
        let p = &self.params.fx;
        [
            10f32.powf(p.v(0) as f32 / 200.),
            10f32.powf(p.v(1) as f32 / 200.),
            p.v(2) as f32 / 1000.,
            p.v(3) as f32,
        ]
    }
    pub(crate) fn prepare(&mut self, rate: u32) -> bool {
        if !(8000..=192000).contains(&rate) {
            return false;
        }
        let s = &self.params.shared;
        s.rate.store(0, Ordering::Release);
        let a = s.asset.lock().unwrap().clone();
        let rev = s.revision.load(Ordering::Acquire);
        let p = match Prepared::new(&a, rate, rev) {
            Ok(p) => p,
            Err(e) => {
                *s.status.lock().unwrap() = e;
                Prepared::new(&Asset::default(), rate, rev).unwrap()
            }
        };
        self.active = Some(Box::new(p));
        self.old = None;
        Exchange::reclaim(&s.pending);
        Exchange::reclaim(&s.retired);
        self.rate = rate as f32;
        self.fade_len = rate as usize / 50;
        self.reset();
        self.params.shared.rate.store(rate, Ordering::Release);
        true
    }
    pub(crate) fn audio(&mut self, l: &mut [f32], r: &mut [f32]) {
        let s = &self.params.shared;
        if self.old.is_none() && s.retired.load(Ordering::Acquire).is_null() {
            let ptr = s.pending.swap(std::ptr::null_mut(), Ordering::AcqRel);
            if !ptr.is_null() {
                let incoming = unsafe { Box::from_raw(ptr) };
                if incoming.rate == self.rate as u32
                    && incoming.revision == s.revision.load(Ordering::Acquire)
                    && self
                        .active
                        .as_ref()
                        .is_none_or(|a| a.revision != incoming.revision)
                {
                    self.old = self.active.replace(incoming);
                    self.fade = 0;
                } else {
                    s.retired.store(Box::into_raw(incoming), Ordering::Release);
                }
            }
        }
        let target = self.targets();
        let coeff = 1. - (-1. / (0.01 * self.rate)).exp();
        for start in (0..l.len()).step_by(256) {
            let n = (l.len() - start).min(256);
            let mut input = [[0.; 256]; 2];
            let mut dry = [[0.; 256]; 2];
            let mut cab = [0.; 256];
            let mut mix = [0.; 256];
            let mut gain = [0.; 256];
            for i in 0..n {
                for k in 0..4 {
                    self.smooth[k] += coeff * (target[k] - self.smooth[k]);
                }
                let x = [l[start + i], r[start + i]].map(|v| if v.is_finite() { v } else { 0. });
                for ch in 0..2 {
                    input[ch][i] = x[ch] * self.smooth[0];
                    dry[ch][i] = self.dry[self.at][ch];
                }
                self.dry[self.at] = x;
                self.at = (self.at + 1) % LATENCY;
                cab[i] = self.smooth[3];
                mix[i] = self.smooth[2];
                gain[i] = self.smooth[1];
            }
            let mut wet = [[0.; 256]; 2];
            let mut prior = [[0.; 256]; 2];
            let mut ok = true;
            if let Some(a) = &mut self.active {
                ok &= a.process(&input, &mut wet, n, &cab);
            }
            if let Some(a) = &mut self.old {
                ok &= a.process(&input, &mut prior, n, &cab);
            }
            if !ok {
                s.fault.store(true, Ordering::Release);
            }
            let fault = s.fault.load(Ordering::Acquire);
            for i in 0..n {
                let alpha = if self.old.is_some() {
                    (self.fade.saturating_sub(LATENCY) as f32 / self.fade_len as f32).min(1.)
                } else {
                    1.
                };
                let mut out = [0.; 2];
                for ch in 0..2 {
                    let w = wet[ch][i] * alpha + prior[ch][i] * (1. - alpha);
                    out[ch] = if fault {
                        dry[ch][i] * gain[i]
                    } else {
                        (dry[ch][i] + (w - dry[ch][i]) * mix[i]) * gain[i]
                    };
                }
                l[start + i] = out[0];
                r[start + i] = out[1];
                self.fade = self.fade.saturating_add(1);
            }
        }
        if self.fade >= LATENCY + self.fade_len {
            if let Some(old) = self.old.take() {
                s.retired.store(Box::into_raw(old), Ordering::Release);
            }
        }
    }
}
impl Plugin for Amp {
    const NAME: &'static str = "ML Amp";
    const VENDOR: &'static str = "ML Studio";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    type SysExMessage = ();
    type BackgroundTask = ();
    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    fn editor(&mut self, _: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let mut s = Shared::default();
        s.extension = Some(Arc::new(Panel {
            shared: self.params.shared.clone(),
        }));
        Some(Box::new(FxEditor::new(
            &specs::AMP,
            self.params.fx.clone(),
            Arc::new(s),
        )))
    }
    fn initialize(
        &mut self,
        _: &AudioIOLayout,
        c: &BufferConfig,
        ctx: &mut impl InitContext<Self>,
    ) -> bool {
        ctx.set_latency_samples(LATENCY as u32);
        self.prepare(c.sample_rate as u32)
    }
    fn reset(&mut self) {
        if let Some(a) = &mut self.active {
            a.clear();
        }
        if let Some(a) = &mut self.old {
            a.clear();
        }
        self.dry.fill([0.; 2]);
        self.at = 0;
        self.smooth = self.targets();
        self.params.shared.fault.store(false, Ordering::Release);
    }
    fn process(
        &mut self,
        b: &mut Buffer,
        _: &mut AuxiliaryBuffers,
        _: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let c = b.as_slice();
        if c.len() >= 2 {
            let (l, r) = c.split_at_mut(1);
            self.audio(l[0], r[0]);
        }
        ProcessStatus::KeepAlive
    }
}
impl ClapPlugin for Amp {
    const CLAP_ID: &'static str = "com.mlstudio.amp";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("NAM amp models and cabinet convolution");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}
impl Vst3Plugin for Amp {
    const VST3_CLASS_ID: [u8; 16] = *b"MLAmpModeler0001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_nam_architectures_and_rejected_loads() {
        let mut models=vec![("Synthetic LSTM".to_owned(),serde_json::json!({"version":"0.5.4","architecture":"LSTM","config":{"num_layers":1,"input_size":1,"hidden_size":1},"weights":vec![0.1f32;16],"sample_rate":48000}).to_string())];
        if let Ok(root) = std::env::var("ML_NAM_TEST_MODELS") {
            for file in ["lstm.nam", "wavenet.nam", "wavenet_a1_standard.nam"] {
                models.push((
                    file.to_owned(),
                    std::fs::read_to_string(Path::new(&root).join(file)).unwrap(),
                ));
            }
        }
        for (file, model) in models {
            let mut n = Native::new(&model, 48000).unwrap_or_else(|e| panic!("{file}: {e}"));
            let input: [f32; 256] = std::array::from_fn(|i| (i as f32 * 0.1).sin() * 0.1);
            let mut out = [0.; 256];
            n.clear();
            let mut first = Vec::new();
            for count in [256, 13, 1, 137, 256] {
                assert!(n.process(&input[..count], &mut out[..count]));
                assert!(out[..count].iter().all(|v| v.is_finite()));
                first.extend_from_slice(&out[..count]);
            }
            n.clear();
            let mut at = 0;
            for count in [256, 13, 1, 137, 256] {
                assert!(n.process(&input[..count], &mut out[..count]));
                assert!(
                    out[..count]
                        .iter()
                        .zip(&first[at..at + count])
                        .all(|(a, b)| (a - b).abs() < 1e-6),
                    "reset {file}"
                );
                at += count;
            }
            let mut bad: serde_json::Value = serde_json::from_str(&model).unwrap();
            bad["weights"] = serde_json::json!([0.]);
            assert!(
                Native::new(&bad.to_string(), 48000).is_err(),
                "truncated {file}"
            );
        }
        let conv=serde_json::json!({"version":"0.5.4","architecture":"ConvNet","config":{"channels":1,"dilations":[1],"batchnorm":false,"activation":"Tanh"},"weights":[0.,1.,0.,1.,0.],"sample_rate":48000}).to_string();
        let mut n = Native::new(&conv, 48000).unwrap();
        let mut out = [0.; 256];
        for size in [256, 1, 137, 256] {
            assert!(n.process(&[0.1; 256][..size], &mut out[..size]));
        }
        n.clear();
        assert!(n.process(&[0.; 256], &mut out));
        assert!(out.iter().all(|v| v.abs() < 1e-6));
    }
    #[test]
    fn rate_conversion_rejects_out_of_band_aliases() {
        let a = crate::tests::linear(32000, 1.);
        let mut n = Native::new(&a.model, 48000).unwrap();
        let mut out = [0.; 256];
        let mut energy = 0.;
        let mut count = 0;
        for block in 0..100 {
            let input = std::array::from_fn::<_, 256, _>(|i| {
                (((block * 256 + i) as f64 * std::f64::consts::TAU * 20000. / 48000.).sin() * 0.1)
                    as f32
            });
            assert!(n.process(&input, &mut out));
            if block > 10 {
                energy += out.iter().map(|v| (*v as f64).powi(2)).sum::<f64>();
                count += 256;
            }
        }
        assert!(
            (energy / count as f64).sqrt() < 0.001,
            "downsampled out-of-band tone aliases into output"
        );
    }
    #[test]
    fn worker_load_clear_undo_and_invalid_file_preserves_accepted_assets() {
        let root = std::env::var("ML_TOOLS_RENDER_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        let good = root.join("batch-five-synthetic.nam");
        let bad = root.join("batch-five-invalid.nam");
        let asset = crate::tests::linear(48000, 0.5);
        std::fs::write(&good, &asset.model).unwrap();
        std::fs::write(&bad, "broken JSON").unwrap();
        let mut a = Amp::default();
        a.prepare(48000);
        let wait = |a: &mut Amp| {
            let rev = a.params.shared.revision.load(Ordering::Acquire);
            let deadline = std::time::Instant::now() + Duration::from_secs(20);
            loop {
                let mut l = [0.; 256];
                let mut r = l;
                a.audio(&mut l, &mut r);
                if a.active.as_ref().is_some_and(|p| p.revision == rev) && a.old.is_none() {
                    break;
                }
                assert!(std::time::Instant::now() < deadline, "worker timeout");
                thread::sleep(Duration::from_millis(10));
            }
        };
        a.params.shared.request(Request::Model(good.clone()));
        wait(&mut a);
        assert!(!a.params.shared.asset.lock().unwrap().model.is_empty());
        let saved = a.params.serialize_fields();
        let patches = Panel {
            shared: a.params.shared.clone(),
        }
        .take_state_edits();
        assert_eq!(patches.len(), 1);
        a.params.shared.request(Request::Model(bad.clone()));
        wait(&mut a);
        assert_eq!(a.params.serialize_fields(), saved);
        assert!(
            Panel {
                shared: a.params.shared.clone()
            }
            .take_state_edits()
            .is_empty()
        );
        a.params.shared.request(Request::ClearModel);
        wait(&mut a);
        assert!(a.params.shared.asset.lock().unwrap().model.is_empty());
        a.params.deserialize_fields(&saved);
        wait(&mut a);
        assert_eq!(a.params.serialize_fields(), saved);
        std::fs::remove_file(good).unwrap();
        std::fs::remove_file(bad).unwrap();
    }
}
