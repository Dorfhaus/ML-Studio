// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Lemon Reverb: Lemon Studio's plate reverb (a recreation of FL's Fruity Reeverb 2) as a VST3 / CLAP plugin
//! (nih-plug). The sound is `os_engine::effects::reverb`.
//!
// spec: plugins/lemon-reverb.md, plugins/fruity-reeverb-2.md, plugins/plugin-formats.md
#![allow(semicolon_in_expressions_from_non_local_macros)]

pub mod editor;
pub mod params;
pub mod room;
pub mod view;

use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use nih_plug::prelude::*;
use os_engine::effects::reverb::{Reverb, Settings};

use params::ReverbParams;

/// What the audio thread and the editor share.
pub struct Shared {
    pub rate: AtomicU32,
    levels: [AtomicU32; 3],
    /// Set while an editor is open.
    pub editor_open: AtomicBool,
}

impl Default for Shared {
    fn default() -> Self {
        Shared { levels: Default::default(), rate: AtomicU32::new(44100), editor_open: AtomicBool::new(false) }
    }
}

pub struct LemonReverb {
    params: Arc<ReverbParams>,
    shared: Arc<Shared>,
    dsp: Reverb,
    raw: Vec<u8>,
    raw_seen: u64,
}

impl Default for LemonReverb {
    fn default() -> Self {
        LemonReverb {
            params: Arc::new(ReverbParams::default()),
            shared: Arc::new(Shared::default()),
            dsp: Reverb::new(44100, Settings::default(), 120.0),
            raw: params::default_state(),
            raw_seen: u64::MAX,
        }
    }
}

impl LemonReverb {
    /// The saved state's copy (never blocks: keeps the last one while the editor writes).
    fn read_raw(&mut self) {
        let v = self.params.raw_version.load(Ordering::Acquire);
        if v == self.raw_seen {
            return;
        }
        if let Ok(b) = self.params.raw.try_read() {
            self.raw.clone_from(&b);
            self.raw_seen = v;
        }
    }

    fn settings(&mut self) -> Settings {
        self.read_raw();
        self.params.settings(&self.raw)
    }
}

impl Plugin for LemonReverb {
    const NAME: &'static str = "ML Reverb";
    const VENDOR: &'static str = "ML Studio";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] =
        &[AudioIOLayout { main_input_channels: NonZeroU32::new(2), main_output_channels: NonZeroU32::new(2), ..AudioIOLayout::const_default() }];
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        Some(Box::new(editor::ReverbEditor::new(self.params.clone(), self.shared.clone())))
    }

    fn initialize(&mut self, _layout: &AudioIOLayout, config: &BufferConfig, _ctx: &mut impl InitContext<Self>) -> bool {
        self.shared.rate.store(config.sample_rate as u32, Ordering::Relaxed);
        self.raw_seen = u64::MAX;
        let s = self.settings();
        self.dsp = Reverb::new(config.sample_rate.round() as u32, s, 120.0);
        true
    }

    fn reset(&mut self) {
        self.dsp.clear();
        for level in &self.shared.levels { level.store(0, Ordering::Relaxed); }
    }

    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, ctx: &mut impl ProcessContext<Self>) -> ProcessStatus {
        let s = self.settings();
        self.dsp.set(s);
        self.dsp.duck_mode=self.params.live.value(&self.params.duck) as u8;
        self.dsp.duck_amount = self.params.live.value(&self.params.duck_amount) as f32 / 100.;
        if let Some(bpm) = ctx.transport().tempo {
            self.dsp.set_tempo(bpm);
        }
        let out = buffer.as_slice();
        if out.len() < 2 {
            return ProcessStatus::Normal;
        }
        let (left, right) = out.split_at_mut(1);
        if self.shared.editor_open.load(Ordering::Relaxed) {
            let levels = self.dsp.process_metered(left[0],right[0]);
            for (meter,value) in self.shared.levels.iter().zip(levels) {
                meter.fetch_max(value.to_bits(), Ordering::Relaxed);
            }
        } else {
            self.dsp.process(left[0], right[0]);
        }
        // The tail rings on after the input stops.
        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for LemonReverb {
    const CLAP_ID: &'static str = "org.openstudio.lemon-reverb";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("ML Studio's plate reverb");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Reverb, ClapFeature::Stereo];
}

impl Vst3Plugin for LemonReverb {
    const VST3_CLASS_ID: [u8; 16] = *b"LemonStudioRevrb";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Reverb];
}

// Built as a plugin module: the C entry points hosts look for. Linked into Lemon Studio (`linked` feature): only the
// symbol-free table, so several Lemon plugins fit in one program (spec: plugins/plugin-formats.md).
#[cfg(not(feature = "linked"))]
nih_export_clap!(LemonReverb);
#[cfg(not(feature = "linked"))]
nih_export_vst3!(LemonReverb);
os_plugkit::linked_clap_entry!(LemonReverb);

#[cfg(test)]
mod tests {
    use super::*;
    use os_engine::effects::reverb::{DEFAULTS, WET};

    #[test]
    fn amount_scales_wet_attenuation_and_dry_signal_never_lights_the_return() {
        let render = |mode,depth,dry_only| {
            let mut settings=Settings::default();
            if dry_only {settings.knobs[WET]=0;settings.knobs[os_engine::effects::reverb::EARLY]=0;}
            let mut dsp=Reverb::new(32000,settings.clone(),120.);
            dsp.duck_mode=mode;dsp.duck_amount=depth;
            let mut left:Vec<_>=(0..16000).map(|i|(i as f32*0.12).sin()*0.6).collect();
            let mut right=left.clone();
            let m=dsp.process_metered(&mut left,&mut right);
            let mut normal=Reverb::new(32000,settings,120.);
            normal.duck_mode=mode;normal.duck_amount=depth;
            let mut l:Vec<_>=(0..16000).map(|i|(i as f32*0.12).sin()*0.6).collect();
            let mut r=l.clone();
            normal.process(&mut l,&mut r);
            assert_eq!(l,left);
            m
        };
        let bypass=render(0,1.,false);
        assert_eq!(render(2,0.,false),bypass);
        let half=render(2,0.5,false);
        let full=render(2,1.,false);
        assert!((half[2]*2.-full[2]).abs()<1e-5);
        assert!(bypass[0]>half[0] && half[0]>full[0]);
        assert!(full[2]>render(1,1.,false)[2]);
        assert_eq!(render(2,1.,true)[..2], [0.;2]);
    }

    #[test]
    fn hard_duck_silences_the_return_under_quiet_input_and_releases_the_tail() {
        for rate in [44100, 48000, 96000] {
            let mut settings = Settings::default();
            settings.knobs[os_engine::effects::reverb::DRY] = 0;
            let mut reverb = Reverb::new(rate, settings, 120.);
            reverb.duck_mode = 2;
            reverb.duck_amount = 1.;
            let mut left: Vec<_> = (0..rate / 4)
                .map(|i| 0.003 * (i as f32 * 220. * std::f32::consts::TAU / rate as f32).sin())
                .collect();
            let mut right = vec![0.; left.len()];
            assert_eq!(reverb.process_metered(&mut left, &mut right)[2], 1.);
            for wet in [&left, &right] {
                assert!(wet[rate as usize / 100..].iter().all(|&sample| sample == 0.));
            }
            left.fill(0.); right.fill(0.);
            let tail = reverb.process_metered(&mut left, &mut right);
            assert!(tail[0].max(tail[1]) > 1e-6);
        }
    }

    #[test]
    fn editor_edits_reach_the_sound_and_stored_bytes_play_as_stored() {
        let mut r = LemonReverb::default();
        assert_eq!(r.settings(), Settings::default());
        r.params.live.set(r.params.p[WET].as_ptr(), 0.0);
        assert_eq!(r.settings().knobs[WET], 0);
        // A state whose decay is below the knob's range keeps it while the knob is not moved.
        let mut fl = params::default_state();
        params::set_field(&mut fl, 6, 0);
        let p = ReverbParams::default();
        p.set_raw(fl.clone());
        let mut r = LemonReverb { params: Arc::new(p), ..LemonReverb::default() };
        // The parameter is at its default, the stored field differs: the parameter wins (it was edited).
        assert_eq!(r.settings().knobs[5], DEFAULTS[5]);
    }
}
