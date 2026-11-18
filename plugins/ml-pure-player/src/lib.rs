// SPDX-License-Identifier: GPL-3.0-or-later
#![allow(semicolon_in_expressions_from_non_local_macros)]
mod library;
mod native;
mod spec;
mod ui;
use lemon_small_fx::{editor::FxEditor, params::FxParams};
use nih_plug::prelude::Params as NihParams;
use nih_plug::prelude::*;
use std::{
    collections::BTreeMap,
    num::NonZeroU32,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct Asset {
    path: String,
}
impl Asset {
    fn resolved(&self) -> PathBuf {
        if self.path.is_empty() {
            library::folder().join(library::FILE)
        } else {
            self.path.clone().into()
        }
    }
}
pub(crate) struct Shared {
    asset: Mutex<Asset>,
    revision: AtomicU64,
    rate: AtomicU32,
    pending: AtomicPtr<native::Synth>,
    retired: AtomicPtr<native::Synth>,
    presets: Mutex<Vec<native::Preset>>,
    status: Mutex<String>,
    stop: AtomicBool,
    prompt: AtomicBool,
    download: AtomicBool,
    downloading: AtomicBool,
    cancel: AtomicBool,
    progress: AtomicU64,
    preview: AtomicU32,
}
impl Default for Shared {
    fn default() -> Self {
        Self {
            asset: Mutex::default(),
            revision: AtomicU64::new(1),
            rate: AtomicU32::new(48000),
            pending: AtomicPtr::default(),
            retired: AtomicPtr::default(),
            presets: Mutex::default(),
            status: Mutex::new("Download the sound library, or load your own SF2.".into()),
            stop: AtomicBool::new(false),
            prompt: AtomicBool::new(true),
            download: AtomicBool::new(false),
            downloading: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            progress: AtomicU64::new(0),
            preview: AtomicU32::new(0),
        }
    }
}
impl Shared {
    fn message(&self, message: impl Into<String>) {
        *self.status.lock().unwrap() = message.into();
    }
    fn asset(&self) -> Asset {
        self.asset.lock().unwrap().clone()
    }
    fn replace(&self, asset: Asset) {
        *self.asset.lock().unwrap() = asset;
        self.revision.fetch_add(1, Ordering::Release);
    }
    fn reclaim(pointer: &AtomicPtr<native::Synth>) {
        let p = pointer.swap(std::ptr::null_mut(), Ordering::AcqRel);
        if !p.is_null() {
            unsafe {
                drop(Box::from_raw(p));
            }
        }
    }
    fn prepared(&self, synth: &native::Synth) {
        *self.presets.lock().unwrap() = synth.presets.clone();
        self.prompt.store(false, Ordering::Release);
        self.message(format!(
            "Ready / {} instruments and kits",
            synth.presets.len()
        ));
    }
}
impl Drop for Shared {
    fn drop(&mut self) {
        Self::reclaim(&self.pending);
        Self::reclaim(&self.retired);
    }
}
pub(crate) struct Params {
    fx: Arc<FxParams>,
    shared: Arc<Shared>,
}
unsafe impl nih_plug::prelude::Params for Params {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        self.fx.param_map()
    }
    fn serialize_fields(&self) -> BTreeMap<String, String> {
        let mut fields = self.fx.serialize_fields();
        fields.insert(
            "pure_soundfont".into(),
            serde_json::to_string(&self.shared.asset()).unwrap(),
        );
        fields
    }
    fn deserialize_fields(&self, fields: &BTreeMap<String, String>) {
        self.fx.deserialize_fields(fields);
        if let Some(value) = fields.get("pure_soundfont").filter(|v| v.len() < 16384) {
            if let Ok(asset) = serde_json::from_str::<Asset>(value) {
                if asset.path.len() <= 4096 {
                    if self.shared.asset().path != asset.path {
                        self.shared.replace(asset);
                    }
                }
            }
        }
    }
}
pub struct PurePlayer {
    params: Arc<Params>,
    active: *mut native::Synth,
    worker: Option<JoinHandle<()>>,
    preview_seen: u32,
    preview_left: usize,
    preview_key: u8,
}
unsafe impl Send for PurePlayer {}
impl Default for PurePlayer {
    fn default() -> Self {
        let shared = Arc::new(Shared::default());
        Self {
            params: Arc::new(Params {
                fx: Arc::new(FxParams::new(&spec::SPEC)),
                shared,
            }),
            active: std::ptr::null_mut(),
            worker: None,
            preview_seen: 0,
            preview_left: 0,
            preview_key: 60,
        }
    }
}
impl PurePlayer {
    fn stop_worker(&mut self) {
        let s = &self.params.shared;
        s.stop.store(true, Ordering::Release);
        s.cancel.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
    fn take_pending(&mut self) {
        let s = &self.params.shared;
        if !s.retired.load(Ordering::Acquire).is_null() {
            return;
        }
        let next = s.pending.swap(std::ptr::null_mut(), Ordering::AcqRel);
        if !next.is_null() {
            if unsafe { (*next).revision } != s.revision.load(Ordering::Acquire) {
                s.retired.store(next, Ordering::Release);
                return;
            }
            let old = std::mem::replace(&mut self.active, next);
            s.retired.store(old, Ordering::Release);
            self.preview_left = 0;
        }
    }
    fn start_worker(&mut self, seen: u64, had_asset: bool) {
        let s = self.params.shared.clone();
        s.stop.store(false, Ordering::Release);
        s.cancel.store(false, Ordering::Release);
        self.worker = Some(thread::spawn(move || {
            let mut revision = seen;
            let mut available = had_asset;
            while !s.stop.load(Ordering::Acquire) {
                Shared::reclaim(&s.retired);
                if s.download.swap(false, Ordering::AcqRel) {
                    s.downloading.store(true, Ordering::Release);
                    s.cancel.store(false, Ordering::Release);
                    s.message("Downloading GeneralUser GS from its official GitHub repository...");
                    match library::download(&library::folder(), &s.progress, || {
                        s.cancel.load(Ordering::Acquire) || s.stop.load(Ordering::Acquire)
                    }) {
                        Ok(_) => {
                            // Installation never overwrites a custom file chosen during the transfer.
                            if s.asset().path.is_empty() {
                                s.revision.fetch_add(1, Ordering::Release);
                            }
                            s.prompt.store(false, Ordering::Release);
                        }
                        Err(e) => s.message(e),
                    }
                    s.downloading.store(false, Ordering::Release);
                }
                let wanted = s.revision.load(Ordering::Acquire);
                let asset = s.asset();
                let newly_installed =
                    !available && asset.path.is_empty() && library::installed(&asset.resolved());
                if wanted != revision || newly_installed {
                    revision = wanted;
                    s.message("Loading SoundFont...");
                    match native::Synth::new(&asset.resolved(), s.rate.load(Ordering::Acquire)) {
                        Ok(mut synth) => {
                            synth.revision = wanted;
                            if s.revision.load(Ordering::Acquire) == wanted
                                && !s.stop.load(Ordering::Acquire)
                            {
                                s.prepared(&synth);
                                let p = Box::into_raw(Box::new(synth));
                                let old = s.pending.swap(p, Ordering::AcqRel);
                                if !old.is_null() {
                                    unsafe {
                                        drop(Box::from_raw(old));
                                    }
                                }
                                available = true;
                            }
                        }
                        Err(e) => {
                            s.message(e);
                            available = true;
                        }
                    }
                }
                thread::sleep(Duration::from_millis(20));
            }
            Shared::reclaim(&s.retired);
        }));
    }
    fn render(&mut self, l: &mut [f32], r: &mut [f32]) {
        if self.active.is_null() {
            l.fill(0.);
            r.fill(0.);
            return;
        }
        unsafe {
            (*self.active).render(l, r);
        }
        if self.preview_left > 0 {
            if l.len() >= self.preview_left {
                unsafe {
                    (*self.active).note(self.preview_key, 0., false);
                };
                self.preview_left = 0;
            } else {
                self.preview_left -= l.len();
            }
        }
    }
}
impl Drop for PurePlayer {
    fn drop(&mut self) {
        // Closing an instrument must not wait for a network timeout on the GUI thread.
        // The worker owns Shared, observes cancellation, and releases it when its bounded I/O returns.
        self.params.shared.stop.store(true, Ordering::Release);
        self.params.shared.cancel.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            if worker.is_finished() {
                let _ = worker.join();
            }
        }
        if !self.active.is_null() {
            unsafe {
                drop(Box::from_raw(self.active));
            }
            self.active = std::ptr::null_mut();
        }
    }
}
impl Plugin for PurePlayer {
    const NAME: &'static str = "ML Pure Player";
    const VENDOR: &'static str = "ML Studio";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    type SysExMessage = ();
    type BackgroundTask = ();
    fn params(&self) -> Arc<dyn nih_plug::prelude::Params> {
        self.params.clone()
    }
    fn editor(&mut self, _: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let mut shared = lemon_small_fx::Shared::default();
        shared.extension = Some(Arc::new(ui::Panel::new(self.params.clone())));
        Some(Box::new(FxEditor::new(
            &spec::SPEC,
            self.params.fx.clone(),
            Arc::new(shared),
        )))
    }
    fn initialize(
        &mut self,
        _: &AudioIOLayout,
        config: &BufferConfig,
        ctx: &mut impl InitContext<Self>,
    ) -> bool {
        if !(8000. ..=192000.).contains(&config.sample_rate) {
            return false;
        }
        self.stop_worker();
        Shared::reclaim(&self.params.shared.pending);
        Shared::reclaim(&self.params.shared.retired);
        if !self.active.is_null() {
            unsafe {
                drop(Box::from_raw(self.active));
            }
            self.active = std::ptr::null_mut();
        }
        let s = &self.params.shared;
        let rate = config.sample_rate.round() as u32;
        s.rate.store(rate, Ordering::Release);
        let asset = s.asset();
        match native::Synth::new(&asset.resolved(), rate) {
            Ok(mut synth) => {
                synth.revision = s.revision.load(Ordering::Acquire);
                s.prepared(&synth);
                self.active = Box::into_raw(Box::new(synth));
            }
            Err(e) => {
                s.message(e);
                s.prompt.store(asset.path.is_empty(), Ordering::Release);
            }
        }
        let seen = s.revision.load(Ordering::Acquire);
        let present = !self.active.is_null();
        self.start_worker(seen, present);
        ctx.set_latency_samples(0);
        true
    }
    fn reset(&mut self) {
        self.preview_left = 0;
        if !self.active.is_null() {
            unsafe {
                (*self.active).reset();
            }
        }
    }
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _: &mut AuxiliaryBuffers,
        ctx: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.take_pending();
        if !self.active.is_null() {
            let p = &self.params.fx;
            unsafe {
                (*self.active).select(p.v(0), p.v(1));
                (*self.active).controls(
                    0.2 * p.v(2) as f32 / 100.,
                    p.v(3) as f64 / 100.,
                    p.v(4) as f64 / 30.,
                );
            }
            let preview = self.params.shared.preview.load(Ordering::Acquire);
            if preview != self.preview_seen {
                self.preview_seen = preview;
                self.preview_key = if p.v(0) == 128 { 36 } else { 60 };
                self.preview_left = self.params.shared.rate.load(Ordering::Relaxed) as usize / 3;
                unsafe {
                    (*self.active).note(self.preview_key, 0.75, true);
                }
            }
        }
        let frames = buffer.samples();
        let channels = buffer.as_slice();
        if channels.len() < 2 {
            return ProcessStatus::Normal;
        }
        let (left, right) = channels.split_at_mut(1);
        let (l, r) = (&mut left[0][..frames], &mut right[0][..frames]);
        let mut at = 0;
        while let Some(event) = ctx.next_event() {
            let timing = (event.timing() as usize).min(frames).max(at);
            self.render(&mut l[at..timing], &mut r[at..timing]);
            at = timing;
            if !self.active.is_null() {
                let synth = unsafe { &mut *self.active };
                match event {
                    NoteEvent::NoteOn { note, velocity, .. } => synth.note(note, velocity, true),
                    NoteEvent::NoteOff { note, .. } => synth.note(note, 0., false),
                    NoteEvent::MidiPitchBend { value, .. } => synth.bend(value),
                    NoteEvent::MidiCC { cc, value, .. } => synth.cc(cc, value),
                    NoteEvent::MidiChannelPressure { pressure, .. } => synth.pressure(pressure),
                    _ => {}
                }
            }
        }
        self.render(&mut l[at..], &mut r[at..]);
        ProcessStatus::KeepAlive
    }
}
impl ClapPlugin for PurePlayer {
    const CLAP_ID: &'static str = "com.mlstudio.pureplayer";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("ML Studio SoundFont instrument powered by FluidSynth");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Sampler,
        ClapFeature::Stereo,
    ];
}
impl Vst3Plugin for PurePlayer {
    const VST3_CLASS_ID: [u8; 16] = *b"MLPurePlayer0001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Sampler];
}
#[cfg(not(feature = "linked"))]
nih_export_clap!(PurePlayer);
#[cfg(not(feature = "linked"))]
nih_export_vst3!(PurePlayer);
os_plugkit::linked_clap_entry!(PurePlayer);
#[cfg(test)]
mod tests;
