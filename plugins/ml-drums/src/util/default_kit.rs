//! Factory pads are embedded in the instrument and recalled by stable sample IDs.
//! Sounds were rendered with slammer by the Autokit author. See resources/default_samples/README.md.

use std::sync::Arc;

use crate::engine::kit::{DrumKit, SampleCategory, NUM_PADS};
use crate::ui::state::{SharedState, WaveformSummary};
use crate::util::audio_file;

/// Number of waveform display points per pad (mirrors plugin.rs).
const WAVEFORM_POINTS: usize = 200;

/// One entry in the default kit — which pad it lands on, display name,
/// category, and the raw WAV bytes embedded in the binary.
struct DefaultPad {
    name: &'static str,
    category: SampleCategory,
    bytes: &'static [u8],
}

/// Pad-index → sample mapping. 8 hand-picked samples for the 8 pads so the
/// user hears a balanced kit the moment the plugin opens.
const DEFAULT_PADS: [DefaultPad; NUM_PADS] = [
    DefaultPad {
        name: "bd_909.wav",
        category: SampleCategory::Kick,
        bytes: include_bytes!("../../resources/default_samples/bd_909.wav"),
    },
    DefaultPad {
        name: "sd_1.wav",
        category: SampleCategory::Snare,
        bytes: include_bytes!("../../resources/default_samples/sd_1.wav"),
    },
    DefaultPad {
        name: "hh_1.wav",
        category: SampleCategory::Hihat,
        bytes: include_bytes!("../../resources/default_samples/hh_1.wav"),
    },
    DefaultPad {
        name: "hh_2.wav",
        category: SampleCategory::Hihat,
        bytes: include_bytes!("../../resources/default_samples/hh_2.wav"),
    },
    DefaultPad {
        name: "808.wav",
        category: SampleCategory::Bass,
        bytes: include_bytes!("../../resources/default_samples/808.wav"),
    },
    DefaultPad {
        name: "tom.wav",
        category: SampleCategory::Tom,
        bytes: include_bytes!("../../resources/default_samples/tom.wav"),
    },
    DefaultPad {
        name: "hitom.wav",
        category: SampleCategory::Tom,
        bytes: include_bytes!("../../resources/default_samples/hitom.wav"),
    },
    DefaultPad {
        name: "bd_psy.wav",
        category: SampleCategory::Kick,
        bytes: include_bytes!("../../resources/default_samples/bd_psy.wav"),
    },
];

pub fn sample(id: &str, rate: f32) -> Option<Arc<Vec<f32>>> {
    let name = id.strip_prefix("factory://")?;
    let entry = DEFAULT_PADS.iter().find(|entry| entry.name == name)?;
    audio_file::load_wav_mono_from_bytes(entry.bytes, name, rate).ok().map(Arc::new)
}

/// Load every bundled sample and assign it to the matching pad.
/// Non-fatal on decode error — logs and skips the offender so the plugin
/// still starts.
pub fn apply_to_kit(shared: &mut SharedState, sample_rate: f32) {
    let kit: &mut DrumKit = &mut shared.kit;
    let mut loaded = 0usize;

    for (pad_index, entry) in DEFAULT_PADS.iter().enumerate() {
        if kit.pads[pad_index].locked {
            continue;
        }
        match audio_file::load_wav_mono_from_bytes(entry.bytes, entry.name, sample_rate) {
            Ok(samples) => {
                let data = Arc::new(samples);
                let pad = &mut kit.pads[pad_index];
                pad.sample = Some(Arc::clone(&data));
                // No filesystem path — bundled samples live only in memory.
                pad.sample_path = Some(format!("factory://{}", entry.name));
                pad.name = entry.name.to_string();
                pad.category = entry.category;
                loaded += 1;
            }
            Err(e) => {
                tracing::warn!(name = entry.name, error = %e, "default kit: decode failed");
            }
        }
    }

    // Recompute waveform summaries for the assigned pads.
    for pad_index in 0..NUM_PADS {
        if shared.kit.pads[pad_index].sample.is_some() && shared.waveforms[pad_index].is_none() {
            let samples = Arc::clone(shared.kit.pads[pad_index].sample.as_ref().unwrap());
            shared.waveforms[pad_index] =
                Some(WaveformSummary::from_samples(&samples, WAVEFORM_POINTS));
        }
    }

    tracing::info!(loaded, total = DEFAULT_PADS.len(), "default sample kit applied");
}
