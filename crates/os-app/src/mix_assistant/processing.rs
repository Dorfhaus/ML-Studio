// SPDX-License-Identifier: GPL-3.0-or-later
//! Auditionable stock processing. Instances are private until the single Apply transaction.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Fx {
    Eq,
    Compressor,
    Press,
    Transient,
    Tape,
    Clipper,
    Distortion,
    Filter,
    Stereo,
    Matrix,
    Chorus,
    Reverb,
    Hall,
    Delay,
    Balance,
    Tesla,
}
impl Fx {
    pub(super) const ALL: [Self; 16] = [
        Self::Eq,
        Self::Compressor,
        Self::Press,
        Self::Transient,
        Self::Tape,
        Self::Clipper,
        Self::Distortion,
        Self::Filter,
        Self::Stereo,
        Self::Matrix,
        Self::Chorus,
        Self::Reverb,
        Self::Hall,
        Self::Delay,
        Self::Balance,
        Self::Tesla,
    ];
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Eq => "ML EQ",
            Self::Compressor => "ML Limiter",
            Self::Press => "ML Press",
            Self::Transient => "ML Transient Processor",
            Self::Tape => "ML Tape",
            Self::Clipper => "ML Soft Clipper",
            Self::Distortion => "ML Fast Distortion",
            Self::Filter => "ML Pulp Filter",
            Self::Stereo => "ML Stereo Enhancer",
            Self::Matrix => "ML Stereo Shaper",
            Self::Chorus => "ML Chorus",
            Self::Reverb => "ML Reverb",
            Self::Hall => "ML Hall",
            Self::Delay => "ML Delay",
            Self::Balance => "ML Balance",
            Self::Tesla => "ML Tesla",
        }
    }
    pub(super) fn purpose(self) -> &'static str {
        match self {
            Self::Eq => "Measured low-end / brightness correction",
            Self::Compressor => "Level control with role-specific attack and release",
            Self::Press => "Gentle multiband control; alternative to compression",
            Self::Transient => "Soften excessive drum attack; preserve the body",
            Self::Tape => "Subtle warmth, no added flutter",
            Self::Clipper => "Round high peaks; can change drum tone",
            Self::Distortion => "Optional parallel harmonics / grit",
            Self::Filter => "Optional darkening; changes the source's tone",
            Self::Stereo => "Narrow low-end sources or gently widen pads",
            Self::Matrix => "Mono matrix for low end; otherwise slight narrowing",
            Self::Chorus => "Optional movement for sustained instruments",
            Self::Reverb => "Short room, filtered low end",
            Self::Hall => "Longer space for pads and backing vocals",
            Self::Delay => "Quiet tempo-related echo with bounded feedback",
            Self::Balance => "Optional 1 dB input trim (separate from balancing)",
            Self::Tesla => "Audio editor / recorder: use manually, never auto-record",
        }
    }
}

#[derive(Clone)]
pub(super) struct Recipe {
    pub fx: Fx,
    pub reason: String,
    pub values: Vec<(&'static str, f64)>,
    pub blob: Option<Vec<u8>>,
    pub mix: f64,
    pub target_reduction: Option<f64>,
}
#[derive(Clone)]
pub(super) struct Addition {
    pub track: u16,
    pub slot: u8,
    pub saved: os_model::SlotPlugin,
    pub recipe: Recipe,
    pub plugin: os_host::SharedPlugin,
}

#[cfg(test)]
pub(super) fn recipe(fx: Fx, track: &Track, project: &Project, strength: u8) -> Recipe {
    recipe_for(fx, track, project, strength, Genre::Balanced, None)
}

fn recipe_for(
    fx: Fx,
    track: &Track,
    project: &Project,
    strength: u8,
    genre: Genre,
    timing: Option<reference::Dynamics>,
) -> Recipe {
    let s = f64::from(strength) / 100.;
    let role = track.role;
    let low = matches!(role, Role::Bass | Role::Kick | Role::BassBus);
    let drum = matches!(role, Role::Kick | Role::Drums | Role::DrumBus);
    let bus = role.is_bus();
    let vocal = matches!(role, Role::LeadVocal | Role::BackingVocal | Role::VocalBus);
    let input = track.level.rms(0.85)
        - db(fader_gain(
            project
                .mixer
                .iter()
                .find(|m| m.index == track.id)
                .and_then(|m| m.volume)
                .unwrap_or(0.8),
        ));
    let mut r = Recipe {
        fx,
        reason: fx.purpose().into(),
        values: Vec::new(),
        blob: None,
        mix: 1.,
        target_reduction: None,
    };
    match fx {
        Fx::Eq => {
            let cutoff = match role {
                Role::VocalBus => 65.,
                Role::InstrumentBus => 42.,
                Role::MixBus => 22.,
                _ if low => 25.,
                _ if vocal => 65.,
                _ if bus => 30.,
                _ => 45.,
            };
            let cut = match role {
                Role::MixBus => 1.,
                _ if bus => 1.8,
                _ => 2.5,
            } * s
                * genre.eq_scale();
            // Do not remove a fundamental merely because an instrument has bass.
            if track.level.sub_fraction > if low || bus { 0.16 } else { 0.08 } {
                r.values.extend([
                    ("Band 1 type", 3.),
                    ("Band 1 freq", os_engine::effects::eq::freq_pos(cutoff)),
                    ("Band 1 width", 0.55),
                ]);
            }
            if !low && track.level.low_fraction > 0.60 {
                r.values.extend([
                    ("Band 2 freq", os_engine::effects::eq::freq_pos(220.)),
                    ("Band 2 level", -cut.min(1.2)),
                ]);
            }
            if track.level.high_fraction > 0.34 {
                r.values.extend([
                    ("Band 7 freq", os_engine::effects::eq::freq_pos(6500.)),
                    ("Band 7 level", -cut.min(2. * s)),
                ]);
            }
            if r.values.is_empty() {
                r.values.extend([
                    ("Band 1 freq", os_engine::effects::eq::freq_pos(100.)),
                    ("Band 1 level", -0.8 * s * genre.eq_scale()),
                ]);
            }
            r.reason = format!(
                "Measured broad correction ≤{:.1} dB; low {:.0}%, high {:.0}%",
                cut,
                track.level.low_fraction * 100.,
                track.level.high_fraction * 100.
            );
        }
        Fx::Compressor => {
            use lemon_limiter::params::{DEFS, parse_text};
            use os_engine::effects::dynamics::RANGES;
            let variability = track.level.dynamics();
            let mut ratio = 1.
                + if bus {
                    0.65 * s
                } else {
                    (0.8 + variability * 0.10).clamp(0.8, 2.) * s * genre.ratio_scale()
                };
            let target = genre.reduction(role) * s;
            r.target_reduction = Some(target);
            let burst = track.level.burst_ms();
            let mut attack = if drum || bus {
                if genre.preserve_punch() { 30. } else { 25. }
            } else {
                (burst / 15.).clamp(3., 12.)
            };
            let mut release = (burst * 1.2).clamp(if low { 120. } else { 80. }, 240.);
            if let Some(timing) = timing {
                // Blend timing and ratio with the measured recipe. Do not bring
                // across fixed thresholds, makeup, gates or output gain.
                let ratio_hint = 1. + (timing.ratio - 1.) * s * genre.ratio_scale();
                ratio = (0.65 * ratio + 0.35 * ratio_hint).clamp(1.05, if bus { 2.0 } else { 4.0 });
                attack = (0.65 * attack + 0.35 * timing.attack_ms)
                    .clamp(2., if vocal && !bus { 15. } else { 50. });
                if drum && genre.preserve_punch() {
                    attack = attack.max(22.);
                }
                release = (0.65 * release + 0.35 * timing.release_ms)
                    .clamp(if low { 100. } else { 65. }, 260.);
            }
            let threshold = (input - target / (1. - 1. / ratio)).clamp(-54., -3.);
            for (k, value) in [
                (0, 0.),
                (1, 12.),
                (2, 12.),
                (3, 0.),
                (8, threshold),
                (9, ratio),
                (10, 0.),
                (11, attack),
                (12, release),
                (13, 1.),
                (14, if vocal { 5. } else { 10. }),
                (15, 0.),
                (16, -100.),
            ] {
                let v = if k == 10 {
                    0
                } else {
                    parse_text(DEFS[k].text, &value.to_string(), RANGES[k].0, RANGES[k].1).unwrap()
                };
                r.values.push((DEFS[k].name, f64::from(v)));
            }
            r.reason = format!(
                "{}: target ~{target:.1} dB control; {ratio:.1}:1, {attack:.0}/{release:.0} ms; no makeup",
                genre.label()
            );
        }
        Fx::Press => {
            use os_engine::effects::maximus::state::*;
            let mut state = State::new_default();
            // Preserve unity below threshold; no maximising pre-gain.
            for b in 0..4 {
                state.params[band(b, B_MODE)] = if b == MASTER { MODE_COMP_OFF } else { MODE_ON };
                let threshold = gain((input + 2.).clamp(-42., -8.));
                let scale = |x: f64| (1. + 2. * x).ln() / 3_f64.ln() / 2.;
                let t = scale(threshold);
                let output = threshold * (1. / threshold).powf(1. / (1. + 0.5 * s));
                state.maps[b * 2].points = vec![
                    MapPoint::new(0., 0.),
                    MapPoint::new(t, t),
                    MapPoint::new(0.5 - t, scale(output)),
                    MapPoint::new(0.5, scale(output * 1.5)),
                ];
            }
            r.blob = Some(press::plugin_state_from_fl(&state.to_bytes()));
        }
        Fx::Transient => {
            r.values
                .extend([("Attack", 80. - 10. * s), ("Release", 80.), ("Drive", 0.)]);
            r.mix = 0.7;
        }
        Fx::Tape => {
            r.values.extend([
                ("Input", 500.),
                ("Soften", 180.),
                ("Head bump", if low { 0. } else { 80. }),
                ("Flutter", 0.),
                ("Mix", 250. * s),
            ]);
        }
        Fx::Clipper => {
            let threshold = gain((input + track.level.crest - 2. * s).clamp(-18., -1.));
            r.values
                .push(("Threshold", (threshold * 128.).round().clamp(1., 127.)));
        }
        Fx::Distortion => {
            r.values
                .extend([("Pre amp", 128.), ("Mix", 18.), ("Post gain", 128.)]);
            r.mix = 0.3 * s;
        }
        Fx::Filter => {
            use os_engine::effects::love_philter::state::*;
            let mut state = State::new_default();
            state.filters[0].enabled = true;
            state.filters[0].kind = VANILLA_LP;
            state.filters[0].controls[C_CUT] = 58000;
            r.blob = Some(pulp_filter::plugin_state_from_fl(&state.to_bytes()));
            r.mix = 0.2 * s;
        }
        Fx::Stereo => {
            r.values.push((
                "Stereo separation",
                if low {
                    64. * s
                } else if role == Role::Pad {
                    -10. * s
                } else {
                    12. * s
                },
            ));
        }
        Fx::Matrix => {
            let side = if low { 0.5 } else { 0.1 * s };
            r.values.extend([
                ("Right into left", side * 12800.),
                ("Left", (1. - side) * 12800.),
                ("Right", (1. - side) * 12800.),
                ("Left into right", side * 12800.),
            ]);
        }
        Fx::Chorus => {
            r.values.push(("Wet only", 0.));
            r.mix = 0.15 * s;
        }
        Fx::Reverb => {
            r.values.extend([
                ("Low cut", 200.),
                ("Predelay", if vocal { 25. } else { 10. }),
                ("Decay time", if drum { 6. } else { 12. }),
                ("Dry level", 128.),
                ("Early reflection level", 8. * s * genre.space_scale()),
                ("Wet level", 10. * s * genre.space_scale()),
            ]);
        }
        Fx::Hall => {
            r.values.extend([
                ("Decay", if role == Role::Pad { 220. } else { 140. }),
                ("Low cut", 220.),
                ("Pre-delay", 250.),
                ("Mix", 100. * s * genre.space_scale()),
            ]);
        }
        Fx::Delay => {
            // Eighth-note delay follows the song tempo, including tempo automation.
            r.values.extend([
                ("Tempo sync", 1.),
                ("Time", 96.),
                ("Delay type", 1.),
                ("Feedback level", 960.),
                ("Output wet", 480. * s * genre.space_scale()),
                ("Output dry", 6000.),
            ]);
        }
        Fx::Balance => {
            let v = (1..=320)
                .min_by(|&a, &b| {
                    let f =
                        |x| (db(f64::from(os_engine::effects::balance::gains(0, x)[0])) + s).abs();
                    f(a).total_cmp(&f(b))
                })
                .unwrap();
            r.values.push(("Volume", f64::from(v)));
        }
        Fx::Tesla => {}
    }
    r
}

pub(super) fn choices(form: &Form, project: &Project, track: &Track) -> Vec<Recipe> {
    if !form.processing || !processing_eligible(track, &form.selected, form.selection_only) {
        return Vec::new();
    }
    let Some(m) = project.mixer.iter().find(|m| m.index == track.id) else {
        return Vec::new();
    };
    if !m.slots_enabled && m.slots.iter().any(Option::is_some) {
        return Vec::new();
    }
    let existing: Vec<_> = m
        .slots
        .iter()
        .flatten()
        .map(crate::project_plugins::slot_title)
        .collect();
    let has = |fx: Fx| existing.iter().any(|name| name == fx.name());
    let low = matches!(track.role, Role::Bass | Role::Kick | Role::BassBus);
    let known = !matches!(track.role, Role::Unknown | Role::General | Role::Return);
    let profile = form.references.get(track, project);
    let sub = track.level.sub_fraction
        > if low || track.role.is_bus() {
            0.16
        } else {
            0.08
        };
    let mud = !low && track.level.low_fraction > 0.60;
    let bright = track.level.high_fraction > 0.34;
    let overlap = focus_for(form).and_then(|f| quality::masking(track, f));
    let harsh = harsh_band(track);
    let reference_cut = (!has(Fx::Eq))
        .then(|| {
            let (role, profile) = profile?;
            let skip = std::array::from_fn(|band| {
                (band == 0 && (low || sub))
                    || (band == 1 && mud)
                    || (band >= 4 && (bright || harsh.is_some()))
                    || overlap.is_some_and(|(focus_band, _)| focus_band == band)
            });
            let (band, excess) = profile.excess(&track.level, skip)?;
            Some((role, band, excess))
        })
        .flatten();
    let dynamics_in_path = path_dynamics(project, form, track.id);
    // Multiband processing is available for review, but requires an explicit choice.
    let use_press = false;
    let use_transient = track.role == Role::Drums
        && !form.genre.preserve_punch()
        && track.level.crest > 17.
        && !has(Fx::Compressor)
        && !has(Fx::Press)
        && !dynamics_in_path
        && m.strip[1] == 0;
    Fx::ALL
        .into_iter()
        .filter(|&fx| fx != Fx::Tesla && !has(fx))
        .filter(|&fx| {
            let automatic = match fx {
                Fx::Eq => {
                    known
                        && (sub
                            || mud
                            || bright
                            || overlap.is_some()
                            || harsh.is_some()
                            || reference_cut.is_some())
                }
                Fx::Press => use_press,
                Fx::Transient => use_transient,
                Fx::Compressor => {
                    known
                        && !use_press
                        && !use_transient
                        && !has(Fx::Transient)
                        && track.level.dynamics() > form.genre.dynamics_gate(track.role)
                        && !dynamics_in_path
                        && !has(Fx::Press)
                        && m.strip[1] == 0
                }
                Fx::Stereo => low && track.level.side_fraction > 0.08 && !has(Fx::Matrix),
                Fx::Tape => {
                    form.character && known && !matches!(track.role, Role::Pad | Role::MixBus)
                }
                Fx::Hall => {
                    form.space
                        && matches!(track.role, Role::Pad | Role::BackingVocal)
                        && !has(Fx::Reverb)
                }
                Fx::Reverb => {
                    form.space
                        && matches!(track.role, Role::LeadVocal | Role::Music | Role::VocalBus)
                        && !has(Fx::Hall)
                }
                Fx::Chorus => form.character && track.role == Role::Pad,
                _ => false,
            };
            track.fx_choices.get(&fx).copied().unwrap_or(automatic)
        })
        .map(|fx| {
            let timing = profile.and_then(|(_, profile)| profile.dynamics);
            let mut r = recipe_for(fx, track, project, form.strength, form.genre, timing);
            if fx == Fx::Compressor && timing.is_some() {
                if let Some((kind, _)) = profile {
                    r.reason
                        .push_str(&format!(" · {} timing reference", reference::label(kind)));
                }
            }
            if fx == Fx::Eq {
                if track.fx_choices.get(&Fx::Eq) != Some(&true)
                    && r.values.len() == 2
                    && r.values[0].0 == "Band 1 freq"
                {
                    // The gentle manual starting curve is not evidence for an
                    // automatic bass cut when only masking or brightness triggered EQ.
                    r.values.clear();
                    r.reason = "Context correction".into();
                }
                if let Some((kind, band, excess)) = reference_cut {
                    let limit = if track.role.is_bus() { 0.9 } else { 1.6 };
                    let cut = ((excess - 2.5) * 0.35_f64).min(limit) * f64::from(form.strength)
                        / 100.
                        * form.genre.eq_scale();
                    if cut >= 0.15 {
                        let hz = quality::HZ[band];
                        r.values.extend([
                            ("Band 3 freq", os_engine::effects::eq::freq_pos(hz)),
                            ("Band 3 width", 0.47),
                            ("Band 3 level", -cut),
                        ]);
                        r.reason.push_str(&format!(
                            " · {} reference: cut {cut:.1} dB at {hz:.0} Hz",
                            reference::label(kind)
                        ));
                    }
                }
                if let Some((band, _)) = overlap {
                    let cut = 1.5 * f64::from(form.strength) / 100. * form.genre.eq_scale();
                    r.values.extend([
                        (
                            "Band 4 freq",
                            os_engine::effects::eq::freq_pos(quality::HZ[band]),
                        ),
                        ("Band 4 width", 0.3),
                        ("Band 4 level", -cut),
                    ]);
                    r.reason.push_str(&format!(
                        "; {cut:.1} dB at {:.0} Hz for overlapping focus",
                        quality::HZ[band]
                    ));
                }
                if let Some(threshold) = harsh {
                    let fader = db(fader_gain(m.volume.unwrap_or(0.8)));
                    r.values.extend([
                        ("Band 6 freq", os_engine::effects::eq::freq_pos(4000.)),
                        ("Band 6 width", 0.3),
                        (
                            "Band 6 Range",
                            -2. * f64::from(form.strength) / 100. * form.genre.eq_scale(),
                        ),
                        ("Band 6 Threshold", (threshold - fader).clamp(-65., -6.)),
                        ("Band 6 Attack", 5.),
                        ("Band 6 Release", 100.),
                    ]);
                    r.reason
                        .push_str("; dynamic 4 kHz cut on intermittent brightness");
                }
            }
            r
        })
        .filter(|recipe| !recipe.values.is_empty() || recipe.blob.is_some())
        .collect()
}

/// Explain an absent proposal using the same controls and measurements as choices.
pub(super) fn skipped_reason(form: &Form, project: &Project, track: &Track, fx: Fx) -> String {
    if !form.processing {
        return "Levels only: stock processing is off".into();
    }
    if !processing_eligible(track, &form.selected, form.selection_only) {
        return "Track excluded or protected".into();
    }
    let Some(m) = project.mixer.iter().find(|m| m.index == track.id) else {
        return "Mixer insert unavailable".into();
    };
    if !m.slots_enabled && m.slots.iter().any(Option::is_some) {
        return "Existing effect chain is bypassed; preserved".into();
    }
    if m.slots
        .iter()
        .flatten()
        .any(|s| crate::project_plugins::slot_title(s) == fx.name())
    {
        return "Existing instance preserved, including its bypass state".into();
    }
    if track.fx_choices.get(&fx) == Some(&false) {
        return "Excluded in your effect choices".into();
    }
    if matches!(track.role, Role::Unknown | Role::General | Role::Return) {
        return "No automatic recipe for this role; choose a specific role or effect".into();
    }
    match fx {
        Fx::Compressor => {
            if path_dynamics(project, form, track.id) {
                return "Dynamics already on this track or its bus route; automatic stacking skipped".into();
            }
            let measured = track.level.dynamics();
            let gate = form.genre.dynamics_gate(track.role);
            if measured <= gate {
                format!(
                    "RMS variation {measured:.1} dB; {} asks for >{gate:.1} dB. Click to include.",
                    form.genre.label()
                )
            } else {
                "Another dynamics processor takes priority; click to choose compression".into()
            }
        }
        Fx::Eq => {
            "No measured tonal/masking correction needed; click for a gentle manual EQ".into()
        }
        Fx::Tape | Fx::Chorus if !form.character => {
            "Optional colour: enable Add warmth or click this effect".into()
        }
        Fx::Reverb | Fx::Hall if !form.space => {
            "Optional ambience: enable Add space or click this effect".into()
        }
        Fx::Tesla => "Manual audio editor; never inserted or recorded automatically".into(),
        _ => "Not suggested for these measurements and role; click to include".into(),
    }
}

// nih-plug exposes floats in 0..1, and integers in 0..(max-min), NOT their
// original range. Missing this offset turns a positive compressor ratio into
// expansion and a mono stereo-matrix recipe into inverted crossfeeds.
pub(super) fn clap_value(fx: Fx, name: &str, value: f64) -> f64 {
    use lemon_small_fx as small;
    let settings: Option<&[small::Setting]> = match fx {
        Fx::Tape => Some(small::tape::SPEC.settings),
        Fx::Hall => Some(small::hall::SPEC.settings),
        Fx::Clipper => Some(small::soft_clipper::SPEC.settings),
        Fx::Distortion => Some(small::fast_distortion::SPEC.settings),
        Fx::Stereo => Some(small::stereo_enhancer::SPEC.settings),
        Fx::Matrix => Some(small::stereo_shaper::SPEC.settings),
        Fx::Transient => Some(small::transient_processor::SPEC.settings),
        Fx::Chorus => Some(small::chorus::SPEC.settings),
        Fx::Balance => Some(small::balance::SPEC.settings),
        _ => None,
    };
    if let Some(setting) = settings.and_then(|s| s.iter().find(|s| s.name == name)) {
        return value - f64::from(setting.range.0);
    }
    let range = match fx {
        Fx::Compressor => lemon_limiter::params::DEFS
            .iter()
            .position(|d| d.name == name)
            .map(|i| os_engine::effects::dynamics::RANGES[i]),
        Fx::Reverb => lemon_reverb::params::DEFS
            .iter()
            .position(|d| d.name == name)
            .map(|i| os_engine::effects::reverb::RANGES[i]),
        Fx::Delay => lemon_delay::params::DEFS
            .iter()
            .position(|d| d.name == name)
            .map(|i| os_engine::effects::delay::RANGES[i]),
        _ => None,
    };
    if let Some((min, _)) = range {
        return value - f64::from(min);
    }
    if fx == Fx::Eq && name.ends_with(" level") {
        (value + 18.) / 36.
    } else if fx == Fx::Eq && name.ends_with(" Range") {
        (value + 18.) / 36.
    } else if fx == Fx::Eq && name.ends_with(" Threshold") {
        (value + 72.) / 72.
    } else if fx == Fx::Eq && name.ends_with(" Attack") {
        (value - 0.5) / 199.5
    } else if fx == Fx::Eq && name.ends_with(" Release") {
        (value - 10.) / 1490.
    } else if fx == Fx::Eq && name.ends_with(" order") {
        value + 3.
    } else {
        value
    }
}

fn focus_for(form: &Form) -> Option<&Track> {
    let tracks = &form.analysis.as_ref()?.tracks;
    form.focus
        .and_then(|id| tracks.iter().find(|t| t.id == id))
        .or_else(|| {
            let mut vocals = tracks.iter().filter(|t| t.role == Role::LeadVocal);
            let first = vocals.next()?;
            vocals.next().is_none().then_some(first)
        })
}

fn harsh_band(track: &Track) -> Option<f64> {
    if !matches!(
        track.role,
        Role::LeadVocal | Role::BackingVocal | Role::Music
    ) {
        return None;
    }
    let active: Vec<_> = track.level.frames.iter().filter(|f| f.rms > -60.).collect();
    if active.len() < 50 {
        return None;
    }
    let median = quality::quantile(active.iter().map(|f| f64::from(f.bands[4] - f.rms)), 0.5);
    let high = quality::quantile(active.iter().map(|f| f64::from(f.bands[4] - f.rms)), 0.95);
    (high > -5. && high - median > 6.)
        .then(|| quality::quantile(active.iter().map(|f| f64::from(f.bands[4])), 0.85))
}

fn has_dynamics(m: &MixerTrack) -> bool {
    m.strip[1] != 0
        || m.slots_enabled
            && m.slots.iter().enumerate().any(|(i, s)| {
                m.slot_on[i]
                    && s.as_ref().is_some_and(|s| {
                        let name = crate::project_plugins::slot_title(s).to_lowercase();
                        ["compress", "limit", "ml press", "master assistant"]
                            .iter()
                            .any(|word| name.contains(word))
                    })
            })
}

fn reaches(form: &Form, start: u16, end: u16) -> bool {
    let mut queue = vec![start];
    let mut seen = BTreeSet::new();
    while let Some(id) = queue.pop() {
        if id == end {
            return true;
        }
        if seen.insert(id) {
            if let Some(t) = form.rows.iter().find(|t| t.id == id) {
                queue.extend(&t.downstream);
            }
        }
    }
    false
}

fn path_dynamics(project: &Project, form: &Form, id: u16) -> bool {
    project.mixer.iter().any(|m| {
        m.index != 0
            && has_dynamics(m)
            && (reaches(form, id, m.index) || reaches(form, m.index, id))
    })
}

pub(super) fn existing_expansion(project: &Project) -> Option<String> {
    for track in &project.mixer {
        if !track.slots_enabled {
            continue;
        }
        for (index, slot) in track.slots.iter().enumerate() {
            let Some(slot) = slot.as_ref().filter(|s| {
                track.slot_on[index] && crate::project_plugins::slot_title(s) == "ML Limiter"
            }) else {
                continue;
            };
            let Some(wrapped) = crate::project_plugins::wrapped_slot(slot) else {
                continue;
            };
            let Some(state) = lemon_limiter::params::fl_from_plugin_state(&wrapped.state) else {
                continue;
            };
            if lemon_limiter::params::field(&state, 10) < 0 {
                return Some(format!(
                    "Track {} has ML Limiter expansion. Check Comp ratio, or undo the earlier assistant pass before mixing again.",
                    track.index
                ));
            }
        }
    }
    None
}

pub(super) fn prepare(
    app: &mut App,
    plans: Vec<(u16, Vec<Recipe>)>,
    project: &Project,
    rate: u32,
) -> Result<Vec<Addition>, String> {
    let mut additions = Vec::new();
    for (id, recipes) in plans {
        let m = project
            .mixer
            .iter()
            .find(|m| m.index == id)
            .ok_or("Track disappeared")?;
        // Append after user processors. Dedicated EQ slot 10 stays reserved.
        let last = m
            .slots
            .iter()
            .enumerate()
            .filter(|(i, s)| *i != 10 && s.is_some())
            .map(|(i, _)| i + 1)
            .max()
            .unwrap_or(0);
        let free: Vec<_> = (last..64)
            .filter(|&i| i != 10 && m.slots[i].is_none() && !slot_controlled(project, id, i as u8))
            .collect();
        if recipes.len() > free.len() {
            return Err(format!(
                "Track {id} needs {} free trailing slots. Remove a proposed effect or free a slot.",
                recipes.len()
            ));
        }
        for (recipe, &slot) in recipes.into_iter().zip(&free) {
            let a = app
                .plugins
                .effects
                .iter()
                .find(|a| {
                    a.desc.name == recipe.fx.name() && crate::project_plugins::is_builtin(&a.desc)
                })
                .ok_or_else(|| format!("{} is unavailable", recipe.fx.name()))?;
            let plugin = a.lib.clone().create(&a.desc.id)?;
            let saved = {
                let mut p = plugin.lock().map_err(|_| "Effect is busy")?;
                if let Some(blob) = &recipe.blob {
                    if !p.load_state(blob) {
                        return Err(format!("{} rejected its settings", recipe.fx.name()));
                    }
                }
                for &(name, value) in &recipe.values {
                    let value = clap_value(recipe.fx, name, value);
                    let param = (0..p.param_count())
                        .filter_map(|i| p.param_info(i))
                        .find(|info| info.name == name)
                        .ok_or_else(|| format!("{} has no {name} control", recipe.fx.name()))?;
                    if !value.is_finite() || value < param.min || value > param.max {
                        return Err(format!(
                            "{}: {name} is outside its range ({value})",
                            recipe.fx.name()
                        ));
                    }
                    if !p.set_param(param.id, (value - param.min) / (param.max - param.min)) {
                        return Err(format!("Cannot set {} {name}", recipe.fx.name()));
                    }
                }
                p.idle();
                let state = p.save_state().ok_or("Could not save effect settings")?;
                // Loading the exact saved bytes makes the audition and applied project identical.
                if !p.load_state(&state) || !p.activate(rate) {
                    return Err(format!("{} could not start", recipe.fx.name()));
                }
                p.set_offline(true);
                os_model::SlotPlugin {
                    name: os_flp::wrapper::WRAPPER.into(),
                    state: os_flp::wrapper::encode(
                        None,
                        &crate::project_plugins::plugin_ref(&a.desc, state),
                    ),
                }
            };
            additions.push(Addition {
                track: id,
                slot: slot as u8,
                saved,
                recipe,
                plugin,
            });
        }
    }
    Ok(additions)
}

fn slot_controlled(project: &Project, id: u16, slot: u8) -> bool {
    project
        .automations
        .iter()
        .flat_map(|a| &a.targets)
        .chain(project.init_controls.iter().map(|c| &c.target))
        .chain(project.peak_links.iter().map(|c| &c.target))
        .any(|&t| match decode(t) {
            Target::SlotMix(t, s) | Target::SlotOn(t, s) => t == id && s == slot,
            Target::SlotParam(t, s, _) => t == id && s == slot,
            _ => false,
        })
}

pub(super) fn with_processing(project: &Project, additions: &[Addition]) -> Project {
    let mut result = project.clone();
    for a in additions {
        if let Some(m) = result.mixer.iter_mut().find(|m| m.index == a.track) {
            if m.slots.iter().all(Option::is_none) {
                m.slots_enabled = true;
            }
            m.slots[a.slot as usize] = Some(a.saved.clone());
            m.slot_on[a.slot as usize] = true;
            m.slot_mix[a.slot as usize] = a.recipe.mix;
        }
    }
    result
}

/// One bounded correction using the rendered chain, including the actual stock
/// compressor's detector, smoothing, integer controls and preceding processors.
pub(super) fn calibrate(
    app: &mut App,
    additions: &mut [Addition],
    before: &BTreeMap<u16, Level>,
    after: &BTreeMap<u16, Level>,
    project: &Project,
    rate: u32,
) -> Result<bool, String> {
    let mut changed = false;
    for a in additions
        .iter_mut()
        .filter(|a| a.recipe.fx == Fx::Compressor)
    {
        let (Some(b), Some(p)) = (before.get(&a.track), after.get(&a.track)) else {
            continue;
        };
        let c = quality::change(b, p);
        // Keep the chosen genre AND strength when calibrating. A fixed target
        // here used to undo a gentler initial recipe during the render retry.
        let Some(target) = a.recipe.target_reduction else {
            continue;
        };
        if (c.reduction - target).abs() <= 0.8 {
            continue;
        }
        let mut r = a.recipe.clone();
        let Some((_, raw)) = r.values.iter_mut().find(|(n, _)| *n == "Comp threshold") else {
            continue;
        };
        let old = db(f64::from(os_engine::effects::dynamics::level(
            *raw as i32,
            7.,
        )));
        // Relax when over-compressing; a small deeper threshold is allowed only
        // when measured phrase dynamics warrant it. No automatic makeup gain.
        let gate = form(app)
            .and_then(|f| {
                f.rows
                    .iter()
                    .find(|t| t.id == a.track)
                    .map(|t| f.genre.dynamics_gate(t.role))
            })
            .unwrap_or(3.);
        let shift =
            ((c.reduction - target) * 2.).clamp(if b.dynamics() > gate { -4. } else { 0. }, 6.);
        if shift.abs() < 0.5 {
            continue;
        }
        let next = (old + shift).clamp(-54., -3.);
        *raw = f64::from(
            lemon_limiter::params::parse_text(
                lemon_limiter::params::DEFS[8].text,
                &next.to_string(),
                0,
                1000,
            )
            .unwrap(),
        );
        r.reason.push_str(&format!(
            " · Measured {:.1} dB, adjusted threshold to {next:.1} dB",
            c.reduction
        ));
        let mut fresh = prepare(app, vec![(a.track, vec![r])], project, rate)?.remove(0);
        fresh.slot = a.slot;
        *a = fresh;
        changed = true;
    }
    Ok(changed)
}
