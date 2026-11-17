use crate::{impulse::Impulse, specs::CONVOLVER};
use lemon_small_fx::{
    Shared,
    editor::{Extension, FxEditor},
    params::FxParams,
};
use nih_plug::prelude::*;
use os_ui::{Canvas, Rect};
use std::{
    collections::BTreeMap,
    ffi::c_void,
    num::NonZeroU32,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
unsafe extern "C" {
    fn ml_conv_new(ir: *const f32, n: usize) -> *mut c_void;
    fn ml_conv_process(p: *mut c_void, input: *const f32, out: *mut f32, n: usize);
    fn ml_conv_clear(p: *mut c_void);
    fn ml_conv_free(p: *mut c_void);
}
pub struct Native(*mut c_void);
unsafe impl Send for Native {}
impl Native {
    pub fn new(ir: &[f32]) -> Result<Self, String> {
        let p = unsafe { ml_conv_new(ir.as_ptr(), ir.len()) };
        if p.is_null() {
            Err("Could not prepare impulse response".into())
        } else {
            Ok(Self(p))
        }
    }
    pub fn process(&mut self, input: &[f32], out: &mut [f32]) {
        unsafe { ml_conv_process(self.0, input.as_ptr(), out.as_mut_ptr(), input.len()) }
    }
    pub fn clear(&mut self) {
        unsafe { ml_conv_clear(self.0) }
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        unsafe { ml_conv_free(self.0) }
    }
}
pub(crate) struct Prepared {
    native: [Native; 2],
    pub revision: u64,
    rate: u32,
    present: bool,
}
impl Prepared {
    fn new(ir: Option<&Impulse>, rate: u32, revision: u64) -> Result<Self, String> {
        let data = ir
            .map(|ir| ir.at_rate(rate))
            .unwrap_or_else(|| [vec![1.], vec![1.]]);
        Ok(Self {
            native: [Native::new(&data[0])?, Native::new(&data[1])?],
            revision,
            rate,
            present: ir.is_some(),
        })
    }
    fn clear(&mut self) {
        for n in &mut self.native {
            n.clear();
        }
    }
}
struct Asset {
    ir: Option<Impulse>,
    saved: String,
    revision: u64,
}
pub(crate) struct Exchange {
    asset: Mutex<Asset>,
    revision: AtomicU64,
    rate: AtomicU32,
    pending: AtomicPtr<Prepared>,
    retired: AtomicPtr<Prepared>,
    stop: AtomicBool,
    status: Mutex<String>,
    edits: Mutex<Vec<String>>,
}
impl Exchange {
    fn new() -> Self {
        Self {
            asset: Mutex::new(Asset {
                ir: None,
                saved: "null".into(),
                revision: 1,
            }),
            revision: AtomicU64::new(1),
            rate: AtomicU32::new(0),
            pending: AtomicPtr::new(std::ptr::null_mut()),
            retired: AtomicPtr::new(std::ptr::null_mut()),
            stop: AtomicBool::new(false),
            status: Mutex::new("Load a WAV impulse or try Small room. No IR: dry signal.".into()),
            edits: Mutex::new(Vec::new()),
        }
    }
    pub(crate) fn replace(&self, ir: Option<Impulse>, undo: bool) {
        let saved = ir.as_ref().map_or_else(|| "null".into(), Impulse::save);
        let mut a = self.asset.lock().unwrap();
        if a.saved == saved {
            return;
        }
        if undo {
            let patch = serde_json::json!({"fields":{"impulse":a.saved}}).to_string();
            let mut edits = self.edits.lock().unwrap();
            // Bound pending GUI transactions as well as the host's undo history.
            const BUDGET: usize = 128 * 1024 * 1024;
            if patch.len() <= BUDGET {
                while !edits.is_empty()
                    && (edits.len() >= 128
                        || edits.iter().map(String::len).sum::<usize>() + patch.len() > BUDGET)
                {
                    edits.remove(0);
                }
                edits.push(patch);
            }
        }
        a.ir = ir;
        a.saved = saved;
        a.revision += 1;
        self.revision.store(a.revision, Ordering::Release);
        *self.status.lock().unwrap() = "Preparing impulse response…".into();
    }
    pub(crate) fn load(&self, p: &Path) {
        match Impulse::import(p) {
            Ok(ir) => self.replace(Some(ir), true),
            Err(e) => *self.status.lock().unwrap() = e,
        }
    }
    fn build(&self, rate: u32) -> Result<Prepared, String> {
        let (ir, revision) = {
            let a = self.asset.lock().unwrap();
            (a.ir.clone(), a.revision)
        };
        Prepared::new(ir.as_ref(), rate, revision)
    }
    fn reclaim(ptr: &AtomicPtr<Prepared>) {
        let p = ptr.swap(std::ptr::null_mut(), Ordering::AcqRel);
        if !p.is_null() {
            unsafe {
                drop(Box::from_raw(p));
            }
        }
    }
}
impl Drop for Exchange {
    fn drop(&mut self) {
        Self::reclaim(&self.pending);
        Self::reclaim(&self.retired);
    }
}
pub(crate) struct ConvParams {
    pub fx: Arc<FxParams>,
    pub(crate) shared: Arc<Exchange>,
}
unsafe impl Params for ConvParams {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        self.fx.param_map()
    }
    fn serialize_fields(&self) -> BTreeMap<String, String> {
        let mut f = self.fx.serialize_fields();
        f.insert(
            "impulse".into(),
            self.shared.asset.lock().unwrap().saved.clone(),
        );
        f
    }
    fn deserialize_fields(&self, f: &BTreeMap<String, String>) {
        self.fx.deserialize_fields(f);
        if let Some(s) = f.get("impulse") {
            let ir = if s == "null" {
                Ok(None)
            } else {
                Impulse::load(s).map(Some)
            };
            match ir {
                Ok(ir) => self.shared.replace(ir, false),
                Err(e) => *self.shared.status.lock().unwrap() = format!("IR state rejected: {e}"),
            }
        }
    }
}
pub(crate) struct Panel {
    pub(crate) shared: Arc<Exchange>,
}
impl Extension for Panel {
    fn take_state_edits(&self) -> Vec<String> {
        std::mem::take(&mut *self.shared.edits.lock().unwrap())
    }
    fn accepts_files(&self) -> bool {
        true
    }
    fn file_dropped(&self, p: &Path) {
        self.shared.load(p)
    }
    fn paint(&self, c: &mut Canvas, _: &dyn lemon_small_fx::view::Model) {
        if !os_plugkit::visibility::stock_dsp_controls() {
            return;
        }
        crate::button(c, Rect::new(16., 170., 120., 30.), "Load WAV…");
        crate::button(c, Rect::new(146., 170., 120., 30.), "Small room");
        crate::button(c, Rect::new(276., 170., 100., 30.), "Clear IR");
        let a = self.shared.asset.lock().unwrap();
        let name =
            a.ir.as_ref()
                .map_or("No impulse loaded", |ir| ir.name.as_str());
        crate::text(c, 16., 224., &name.chars().take(75).collect::<String>());
        if let Some(ir) = &a.ir {
            crate::text(
                c,
                16.,
                245.,
                &format!(
                    "{:.2} s • {} Hz • stereo • saved inside project",
                    ir.left.len() as f64 / ir.rate as f64,
                    ir.rate
                ),
            );
        }
        drop(a);
        crate::text(
            c,
            16.,
            274.,
            &self
                .shared
                .status
                .lock()
                .unwrap()
                .chars()
                .take(86)
                .collect::<String>(),
        );
        crate::text(
            c,
            16.,
            296.,
            "Drop mono/stereo WAV here • up to 10 seconds • PCM or float32",
        );
    }
    fn click(&self, x: f32, y: f32, _: Option<&Arc<dyn GuiContext>>) -> bool {
        if !os_plugkit::visibility::stock_dsp_controls() {
            return false;
        }
        if Rect::new(16., 170., 120., 30.).contains(x, y) {
            if let Some(p) = rfd::FileDialog::new()
                .set_title("Load impulse response (maximum 10 seconds)")
                .add_filter("WAV", &["wav"])
                .pick_file()
            {
                self.shared.load(&p);
            }
            return true;
        }
        if Rect::new(146., 170., 120., 30.).contains(x, y) {
            self.shared.replace(Some(Impulse::room()), true);
            return true;
        }
        if Rect::new(276., 170., 100., 30.).contains(x, y) {
            self.shared.replace(None, true);
            return true;
        }
        false
    }
}
pub struct Convolver {
    pub(crate) params: Arc<ConvParams>,
    worker: Option<JoinHandle<()>>,
    active: Option<Box<Prepared>>,
    old: Option<Box<Prepared>>,
    fade: usize,
    fade_len: usize,
    delay: [Vec<f32>; 2],
    pos: usize,
    rate: f32,
    smoothed: [f32; 5],
    hp: [f32; 2],
    lp: [f32; 2],
}
impl Default for Convolver {
    fn default() -> Self {
        let shared = Arc::new(Exchange::new());
        let params = Arc::new(ConvParams {
            fx: Arc::new(FxParams::new(&CONVOLVER)),
            shared: shared.clone(),
        });
        let worker = thread::spawn(move || {
            let mut done = (0, 0);
            while !shared.stop.load(Ordering::Acquire) {
                Exchange::reclaim(&shared.retired);
                let rate = shared.rate.load(Ordering::Acquire);
                let rev = shared.revision.load(Ordering::Acquire);
                if rate != 0 && (rate, rev) != done {
                    match shared.build(rate) {
                        Ok(p) => {
                            if shared.rate.load(Ordering::Acquire) == rate
                                && shared.revision.load(Ordering::Acquire) == p.revision
                            {
                                let old = shared
                                    .pending
                                    .swap(Box::into_raw(Box::new(p)), Ordering::AcqRel);
                                if !old.is_null() {
                                    unsafe {
                                        drop(Box::from_raw(old));
                                    }
                                }
                                *shared.status.lock().unwrap() =
                                    "Ready. IR changes crossfade during playback.".into();
                            }
                        }
                        Err(e) => *shared.status.lock().unwrap() = e,
                    }
                    done = (rate, rev);
                }
                thread::sleep(Duration::from_millis(20));
            }
        });
        Self {
            params,
            worker: Some(worker),
            active: None,
            old: None,
            fade: 0,
            fade_len: 1,
            delay: [Vec::new(), Vec::new()],
            pos: 0,
            rate: 48000.,
            smoothed: [0.; 5],
            hp: [0.; 2],
            lp: [0.; 2],
        }
    }
}
impl Drop for Convolver {
    fn drop(&mut self) {
        self.params.shared.stop.store(true, Ordering::Release);
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}
impl Convolver {
    fn targets(&self) -> [f32; 5] {
        let p = &self.params.fx;
        [
            p.v(0) as f32 / 1000.,
            p.v(1) as f32 * self.rate / 10000.,
            p.v(2) as f32,
            p.v(3) as f32,
            10f32.powf(p.v(4) as f32 / 200.),
        ]
    }
    pub(crate) fn prepare(&mut self, rate: u32) -> bool {
        self.params.shared.rate.store(0, Ordering::Release);
        match self.params.shared.build(rate) {
            Ok(p) => self.active = Some(Box::new(p)),
            Err(_) => return false,
        };
        self.rate = rate as f32;
        self.fade_len = (rate as usize / 50).max(1);
        self.delay = [
            vec![0.; rate as usize / 4 + 2],
            vec![0.; rate as usize / 4 + 2],
        ];
        self.old = None;
        self.reset();
        self.params.shared.rate.store(rate, Ordering::Release);
        true
    }
    pub(crate) fn audio(&mut self, l: &mut [f32], r: &mut [f32]) {
        let shared = &self.params.shared;
        if self.old.is_none() && shared.retired.load(Ordering::Acquire).is_null() {
            let p = shared.pending.swap(std::ptr::null_mut(), Ordering::AcqRel);
            if !p.is_null() {
                let incoming = unsafe { Box::from_raw(p) };
                if incoming.rate == self.rate as u32
                    && incoming.revision == shared.revision.load(Ordering::Acquire)
                    && self
                        .active
                        .as_ref()
                        .is_none_or(|a| a.revision != incoming.revision || a.rate != incoming.rate)
                {
                    self.old = self.active.replace(incoming);
                    self.fade = 0;
                } else {
                    shared
                        .retired
                        .store(Box::into_raw(incoming), Ordering::Release);
                }
            }
        }
        let targets = self.targets();
        let smooth = 1. - (-1. / (0.01 * self.rate)).exp();
        let mut input = [[0.; 256]; 2];
        let mut wet = [[0.; 256]; 2];
        let mut prior = [[0.; 256]; 2];
        for start in (0..l.len()).step_by(256) {
            let n = (l.len() - start).min(256);
            for i in 0..n {
                input[0][i] = if l[start + i].is_finite() {
                    l[start + i]
                } else {
                    0.
                };
                input[1][i] = if r[start + i].is_finite() {
                    r[start + i]
                } else {
                    0.
                };
            }
            if let Some(a) = &mut self.active {
                for ch in 0..2 {
                    a.native[ch].process(&input[ch][..n], &mut wet[ch][..n]);
                }
            }
            if let Some(old) = &mut self.old {
                for ch in 0..2 {
                    old.native[ch].process(&input[ch][..n], &mut prior[ch][..n]);
                }
            }
            for i in 0..n {
                for k in 0..5 {
                    self.smoothed[k] += smooth * (targets[k] - self.smoothed[k]);
                }
                let alpha = if self.old.is_some() {
                    (self.fade as f32 / self.fade_len as f32).min(1.)
                } else {
                    1.
                };
                let len = self.delay[0].len();
                let read = (self.pos as f32 - self.smoothed[1]).rem_euclid(len as f32);
                let idx = read.floor() as usize;
                let frac = read - read.floor();
                let low = self.smoothed[2].min(self.rate * 0.45);
                let high = self.smoothed[3].min(self.rate * 0.45);
                let hc = 1. - (-std::f32::consts::TAU * low / self.rate).exp();
                let lc = 1. - (-std::f32::consts::TAU * high / self.rate).exp();
                let present = self.active.as_ref().is_some_and(|p| p.present);
                let was = self.old.as_ref().map_or(present, |p| p.present);
                let amount = self.smoothed[0]
                    * ((was as u8 as f32) * (1. - alpha) + (present as u8 as f32) * alpha);
                for ch in 0..2 {
                    let v = wet[ch][i] * alpha
                        + if self.old.is_some() {
                            prior[ch][i] * (1. - alpha)
                        } else {
                            0.
                        };
                    self.delay[ch][self.pos] = v;
                    let mut w =
                        self.delay[ch][idx] * (1. - frac) + self.delay[ch][(idx + 1) % len] * frac;
                    self.hp[ch] += hc * (w - self.hp[ch]);
                    if low > 0.5 {
                        w -= self.hp[ch];
                    }
                    self.lp[ch] += lc * (w - self.lp[ch]);
                    if high < 19999. && high < self.rate * 0.449 {
                        w = self.lp[ch];
                    }
                    let out = (input[ch][i] * (1. - amount) + w * amount) * self.smoothed[4];
                    if ch == 0 {
                        l[start + i] = out;
                    } else {
                        r[start + i] = out;
                    }
                }
                self.pos = (self.pos + 1) % len;
                self.fade = self.fade.saturating_add(1);
            }
        }
        if self.fade >= self.fade_len {
            if let Some(old) = self.old.take() {
                // Only the worker destroys replaced engines; retired was empty before the swap.
                shared.retired.store(Box::into_raw(old), Ordering::Release);
            }
        }
    }
}
impl Plugin for Convolver {
    const NAME: &'static str = "ML Convolver";
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
            &CONVOLVER,
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
        if !(8000.0..=192000.0).contains(&c.sample_rate) {
            return false;
        }
        ctx.set_latency_samples(0);
        self.prepare(c.sample_rate.round() as u32)
    }
    fn reset(&mut self) {
        if let Some(a) = &mut self.active {
            a.clear();
        }
        if let Some(a) = &mut self.old {
            a.clear();
        }
        self.delay.iter_mut().for_each(|d| d.fill(0.));
        self.pos = 0;
        self.hp = [0.; 2];
        self.lp = [0.; 2];
        self.smoothed = self.targets();
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
impl ClapPlugin for Convolver {
    const CLAP_ID: &'static str = "com.mlstudio.convolver";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Stereo impulse-response convolution");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Reverb,
        ClapFeature::Stereo,
    ];
}
impl Vst3Plugin for Convolver {
    const VST3_CLASS_ID: [u8; 16] = *b"MLConvolver00001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Reverb];
}
