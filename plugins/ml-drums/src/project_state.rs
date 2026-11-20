// SPDX-License-Identifier: GPL-3.0-or-later
//! The CLAP state callback snapshots each instance directly, including its custom oneshots.
use crate::engine::{kit::{DrumKit, NUM_PADS}, sequencer::PatternBank};
use crate::ui::state::SharedState;
use crate::util::{audio_file, default_kit, preset::{self, Preset}};
use base64::{engine::general_purpose::STANDARD, Engine};
use nih_plug::params::persist::PersistentField;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct SavedKit {
    kit: Preset,
    sample_rate: u32,
    audio: Vec<Option<String>>,
}

impl SavedKit {
    fn capture(live: &Mutex<SharedState>) -> Self {
        // Release the performance lock before encoding the custom sample buffers.
        let (kit, rate, samples) = {
            let live = live.lock();
            let samples: Vec<_> = live.kit.pads.iter().map(|p| {
                if p.sample_path.as_deref().is_some_and(|s| s.starts_with("factory://")) {
                    None
                } else { p.sample.clone() }
            }).collect();
            (preset::from_kit("ML Drums", &live.kit, &live.pattern_bank), live.sample_rate, samples)
        };
        let audio = samples.into_iter().map(|data| data.map(|pcm| {
            let bytes: Vec<_> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
            STANDARD.encode(bytes)
        })).collect();
        Self { kit, sample_rate: rate as u32, audio }
    }

    fn install(self, live: &Mutex<SharedState>, rate: f32) {
        let samples: Vec<_> = self.audio.into_iter().take(NUM_PADS).map(|field| {
            let bytes = STANDARD.decode(field?).ok()?;
            if bytes.len() % 4 != 0 || !(1..=768000).contains(&self.sample_rate) { return None; }
            let pcm = bytes.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect();
            audio_file::resample_if_needed(pcm, self.sample_rate, rate, "project pad").ok().map(Arc::new)
        }).collect();
        let mut kit = DrumKit::new();
        let mut bank = PatternBank::new();
        preset::apply_with_samples(&self.kit, &mut kit, &mut bank, rate, &samples);
        let mut live = live.lock();
        live.kit = kit;
        live.pattern_bank = bank;
        live.sample_rate = rate;
        live.update_all_waveforms(200);
    }
}

pub struct ProjectState {
    live: Arc<Mutex<SharedState>>,
    pending: Mutex<Option<SavedKit>>,
}

impl ProjectState {
    pub fn new(live: Arc<Mutex<SharedState>>) -> Self {
        Self { live, pending: Mutex::new(None) }
    }

    pub fn activate(&self, rate: f32) {
        let old_rate = self.live.lock().sample_rate;
        let saved = self.pending.lock().take();
        if let Some(saved) = saved {
            saved.install(&self.live, rate);
        } else if old_rate == 0.0 {
            let mut live = self.live.lock();
            default_kit::apply_to_kit(&mut live, rate);
            live.sample_rate = rate;
        } else if rate != old_rate {
            SavedKit::capture(&self.live).install(&self.live, rate);
        }
        if old_rate != 0.0 && old_rate != rate {
            let mut live = self.live.lock();
            live.library = None;
            live.bg_rx = None;
            live.scan_status = crate::ui::state::ScanStatus::NeedsSetup { suggested_path: None };
        }
    }
}

impl<'a> PersistentField<'a, SavedKit> for ProjectState {
    fn set(&self, value: SavedKit) { *self.pending.lock() = Some(value); }

    fn map<F: Fn(&SavedKit) -> R, R>(&self, f: F) -> R {
        if let Some(saved) = self.pending.lock().as_ref() { return f(saved); }
        f(&SavedKit::capture(&self.live))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ml_drums_custom_pad_moves_with_project_at_new_rate() {
        let live = Arc::new(Mutex::new(SharedState::new()));
        let field = ProjectState::new(live.clone());
        field.activate(48000.0);
        {
            let mut state = live.lock();
            let pad = &mut state.kit.pads[1];
            pad.sample_path = Some("/no-longer-present/snare.wav".into());
            pad.sample = Some(Arc::new(vec![0.25; 480]));
            pad.locked = true;
            pad.pan = -0.5;
            state.pattern_bank.patterns[3].lanes[1].steps[5].enabled = true;
            state.pattern_bank.active = 3;
        }
        let bytes = field.map(|s| serde_json::to_vec(s).unwrap());
        let next = Arc::new(Mutex::new(SharedState::new()));
        let restored = ProjectState::new(next.clone());
        restored.set(serde_json::from_slice(&bytes).unwrap());
        restored.activate(96000.0);
        let state = next.lock();
        let pad = &state.kit.pads[1];
        assert_eq!(pad.pan, -0.5);
        assert!(pad.locked && state.pattern_bank.active_pattern().lanes[1].steps[5].enabled);
        let pcm = pad.sample.as_ref().unwrap();
        assert_eq!(pcm.len(), 960);
        assert!((pcm[300] - 0.25).abs() < 0.005);
        assert!(state.kit.pads[0].sample.as_ref().unwrap().iter().any(|v| v.abs() > 0.01));
    }
}
