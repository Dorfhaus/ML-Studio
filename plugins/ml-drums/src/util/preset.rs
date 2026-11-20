use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::engine::kit::{DrumKit, SampleCategory};
use crate::engine::sequencer::PatternBank;
use crate::util::audio_file;

const PRESET_VERSION: u32 = 2;

#[derive(Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub version: u32,
    pub pads: Vec<PresetPad>,
    #[serde(default)]
    pub patterns: Option<PresetPatterns>,
}

#[derive(Serialize, Deserialize)]
pub struct PresetPatterns {
    pub patterns: Vec<crate::engine::sequencer::Pattern>,
    pub active: usize,
}

#[derive(Serialize, Deserialize)]
pub struct PresetPad {
    pub sample_path: Option<String>,
    #[serde(default)]
    pub locked: bool,
    pub name: String,
    pub category: SampleCategory,
    pub volume: f32,
    pub pan: f32,
    pub pitch: f32,
    pub decay: f32,
    #[serde(default)]
    pub start: f32,
    #[serde(default = "default_end")]
    pub end: f32,
}

fn default_end() -> f32 { 1.0 }

/// Returns `~/.local/share/autokit/presets/`, creating it if missing.
pub fn preset_dir() -> PathBuf {
    let base = dirs_next().join("ML Drums").join("presets");
    if !base.exists() {
        let _ = std::fs::create_dir_all(&base);
    }
    base
}

fn dirs_next() -> PathBuf {
    os_paths::user_dir().unwrap_or_else(std::env::temp_dir)
}

/// Sanitize a preset name for use as a filename.
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Save a preset to `{preset_dir}/{name}.json`.
pub fn save_preset(preset: &Preset) -> Result<PathBuf, String> {
    let dir = preset_dir();
    let filename = sanitize_name(&preset.name);
    if filename.is_empty() {
        return Err("Preset name is empty".to_string());
    }
    let path = dir.join(format!("{filename}.json"));
    let json = serde_json::to_string_pretty(preset)
        .map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

/// Load a preset from a JSON file.
pub fn load_preset(path: &Path) -> Result<Preset, String> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&data).map_err(|e| format!("parse {}: {e}", path.display()))
}

/// List all `.json` presets in the preset directory, sorted alphabetically.
pub fn list_presets() -> Vec<(String, PathBuf)> {
    let dir = preset_dir();
    let mut presets = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?")
                    .to_string();
                presets.push((name, path));
            }
        }
    }

    presets.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    presets
}

// --- Pattern persistence (individual patterns) ---

/// Returns `~/.local/share/autokit/patterns/`, creating it if missing.
pub fn pattern_dir() -> PathBuf {
    let base = dirs_next().join("ML Drums").join("patterns");
    if !base.exists() {
        let _ = std::fs::create_dir_all(&base);
    }
    base
}

/// Save a single pattern to `{pattern_dir}/{name}.json`.
pub fn save_pattern(name: &str, pattern: &crate::engine::sequencer::Pattern) -> Result<PathBuf, String> {
    let dir = pattern_dir();
    let filename = sanitize_name(name);
    if filename.is_empty() {
        return Err("Pattern name is empty".to_string());
    }
    let path = dir.join(format!("{filename}.json"));
    let json = serde_json::to_string_pretty(pattern)
        .map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

/// Load a single pattern from a JSON file.
pub fn load_pattern(path: &Path) -> Result<crate::engine::sequencer::Pattern, String> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    let pattern: crate::engine::sequencer::Pattern = serde_json::from_str(&data)
        .map_err(|e| format!("parse {}: {e}", path.display()))?;
    if !pattern.valid() { return Err("Invalid drum pattern".into()); }
    Ok(pattern)
}

/// List all `.json` patterns in the pattern directory, sorted alphabetically.
pub fn list_patterns() -> Vec<(String, PathBuf)> {
    let dir = pattern_dir();
    let mut patterns = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("?")
                    .to_string();
                patterns.push((name, path));
            }
        }
    }

    patterns.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    patterns
}

/// Delete a file (preset or pattern) from disk.
pub fn delete_file(path: &Path) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|e| format!("delete {}: {e}", path.display()))
}

/// Create a `Preset` from the current kit state.
pub fn from_kit(name: &str, kit: &DrumKit, pattern_bank: &crate::engine::sequencer::PatternBank) -> Preset {
    let pads = kit
        .pads
        .iter()
        .map(|p| PresetPad {
            sample_path: p.sample_path.clone(),
            locked: p.locked,
            name: p.name.clone(),
            category: p.category,
            volume: p.volume,
            pan: p.pan,
            pitch: p.pitch,
            decay: p.decay,
            start: p.start,
            end: p.end,
        })
        .collect();

    Preset {
        name: name.to_string(),
        version: PRESET_VERSION,
        pads,
        patterns: Some(PresetPatterns {
            patterns: pattern_bank.patterns.clone(),
            active: pattern_bank.active,
        }),
    }
}

/// Apply a preset to a kit, loading sample audio from disk.
/// Pads with missing or unreadable sample files get `None` sample data.
/// If the preset contains pattern data, it is restored into `pattern_bank`.
pub fn apply_to_kit(preset: &Preset, kit: &mut DrumKit, pattern_bank: &mut PatternBank, sample_rate: f32) {
    apply_with_samples(preset, kit, pattern_bank, sample_rate, &[]);
}

pub fn apply_with_samples(preset: &Preset, kit: &mut DrumKit, pattern_bank: &mut PatternBank, sample_rate: f32, embedded: &[Option<Arc<Vec<f32>>>]) {
    for (i, pp) in preset.pads.iter().enumerate() {
        if i >= kit.pads.len() {
            break;
        }
        let pad = &mut kit.pads[i];
        pad.locked = pp.locked;
        pad.name = pp.name.clone();
        pad.category = pp.category;
        pad.volume = pp.volume;
        pad.pan = pp.pan;
        pad.pitch = pp.pitch;
        pad.decay = pp.decay;
        pad.start = pp.start;
        pad.end = pp.end;

        if let Some(Some(audio)) = embedded.get(i) {
            pad.sample = Some(audio.clone());
            pad.sample_path = pp.sample_path.clone();
            continue;
        }
        match &pp.sample_path {
            Some(path) if path.starts_with("factory://") => {
                pad.sample = crate::util::default_kit::sample(path, sample_rate);
                pad.sample_path = Some(path.clone());
            }
            Some(path) if !path.is_empty() => {
                // Defensive: short-circuit when the parent directory is obviously
                // missing. Avoids open() on dead paths, and (more importantly)
                // limits filesystem probing on stale network/FUSE mounts where
                // open() can hang for many seconds. This MUST NOT run on the
                // audio thread — see `restore_to_fresh` below.
                if !sample_path_likely_present(path) {
                    tracing::warn!("Preset pad {i}: missing sample (parent dir absent): {path}");
                    pad.sample = None;
                    pad.sample_path = Some(path.clone());
                    continue;
                }
                match audio_file::load_wav_mono(path, sample_rate) {
                    Ok(samples) => {
                        pad.sample = Some(Arc::new(samples));
                        pad.sample_path = Some(path.clone());
                    }
                    Err(e) => {
                        tracing::warn!("Preset pad {i}: could not load {path}: {e}");
                        pad.sample = None;
                        pad.sample_path = Some(path.clone());
                    }
                }
            }
            _ => {
                pad.sample = None;
                pad.sample_path = None;
            }
        }
    }

    if let Some(ref pat_data) = preset.patterns {
        for (i, pat) in pat_data.patterns.iter().enumerate() {
            if i < pattern_bank.patterns.len() {
                if pat.valid() {
                    pattern_bank.patterns[i] = pat.clone();
                }
            }
        }
        pattern_bank.active = pat_data.active.min(pattern_bank.patterns.len().saturating_sub(1));
    }
}

/// Cheap check: does this sample path's parent directory exist?
/// Used to skip dead paths without doing a full file open. This still does
/// a `stat()` syscall on the parent, which can hang on a truly broken mount,
/// but is much cheaper than `open()` + symphonia probe and avoids triggering
/// per-file automounts.
fn sample_path_likely_present(path: &str) -> bool {
    let p = Path::new(path);
    match p.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.is_dir(),
        // No parent component (e.g. bare filename, or root) — let the open() try.
        _ => true,
    }
}

#[cfg(test)]
mod restore_tests {
    use super::*;

    fn pad_with_path(name: &str, path: &str) -> PresetPad {
        PresetPad {
            sample_path: Some(path.to_string()),
            locked: false,
            name: name.to_string(),
            category: SampleCategory::Kick,
            volume: 1.0,
            pan: 0.0,
            pitch: 0.0,
            decay: 1.0,
            start: 0.0,
            end: 1.0,
        }
    }

    #[test]
    fn restore_to_fresh_collects_missing_paths_without_panicking() {
        // Two pads referencing files in a directory that does not exist on
        // any host. This is the host-freeze scenario from the field report:
        // a project saved on machine A is loaded on machine B where the
        // sample tree was never present.
        let preset = Preset {
            name: "missing-samples".to_string(),
            version: PRESET_VERSION,
            pads: vec![
                pad_with_path("kick", "/nonexistent-dir-xyz/kick.wav"),
                pad_with_path("snare", "/nonexistent-dir-xyz/snare.wav"),
            ],
            patterns: None,
        };

        let mut kit = DrumKit::new();
        apply_to_kit(&preset, &mut kit, &mut PatternBank::new(), 44100.0);

        // Pads come back with metadata intact but no audio data.
        assert_eq!(kit.pads[0].name, "kick");
        assert_eq!(kit.pads[1].name, "snare");
        assert!(kit.pads[0].sample.is_none());
        assert!(kit.pads[1].sample.is_none());
        // Original paths preserved so the user can see what's broken.
        assert_eq!(
            kit.pads[0].sample_path.as_deref(),
            Some("/nonexistent-dir-xyz/kick.wav")
        );
    }

    #[test]
    fn sample_path_likely_present_rejects_missing_parent() {
        assert!(!sample_path_likely_present("/definitely/not/here/x.wav"));
    }

    #[test]
    fn sample_path_likely_present_accepts_real_parent() {
        // Use the platform temp dir so this works on Linux, macOS, and Windows.
        let mut probe = std::env::temp_dir();
        probe.push("autokit-nonexistent-probe.wav");
        assert!(sample_path_likely_present(probe.to_str().unwrap()));
    }
}
