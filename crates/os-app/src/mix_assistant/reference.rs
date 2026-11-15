// SPDX-License-Identifier: GPL-3.0-or-later
//! Optional mix reference data. A loaded file supplies measurements, never DSP or code.
use super::{Level, Project, Role, Track, quality};
use serde::Deserialize;
use std::{collections::BTreeMap, io::Read, path::Path};

pub(crate) const FILE: &str = "ML Studio Mix References.mlmixref";
const HEADER: &[u8] = b"MLMIXREF/1\n";
const MAX_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(super) enum Kind {
    AcousticGuitar,
    ElectricGuitar,
    DistortedGuitar,
    Bass,
    DeepBass,
    SubBass,
    Kick,
    Snare,
    Toms,
    Cymbals,
    Drums,
    Piano,
    ElectricPiano,
    SynthLead,
    SynthPad,
    HighStrings,
    LowStrings,
    HighBrass,
    LowBrass,
    Vocals,
    Dialogue,
    Instrument,
    InstrumentBus,
    DrumBus,
    VocalBus,
    BassBus,
    MixBus,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Self::AcousticGuitar => "Acoustic guitar",
            Self::ElectricGuitar => "Clean electric guitar",
            Self::DistortedGuitar => "Distorted guitar",
            Self::Bass => "Bass",
            Self::DeepBass => "Deep bass",
            Self::SubBass => "Sub bass",
            Self::Kick => "Kick",
            Self::Snare => "Snare",
            Self::Toms => "Toms",
            Self::Cymbals => "Cymbals",
            Self::Drums => "Drums",
            Self::Piano => "Piano",
            Self::ElectricPiano => "Electric keys",
            Self::SynthLead => "Synth lead",
            Self::SynthPad => "Synth pad",
            Self::HighStrings => "High strings",
            Self::LowStrings => "Low strings",
            Self::HighBrass => "High brass",
            Self::LowBrass => "Low brass",
            Self::Vocals => "Vocals",
            Self::Dialogue => "Dialogue",
            Self::Instrument => "Instrument",
            Self::InstrumentBus => "Instrument bus",
            Self::DrumBus => "Drum bus",
            Self::VocalBus => "Vocal bus",
            Self::BassBus => "Bass bus",
            Self::MixBus => "Mix bus",
        }
    }

    fn named(role: Role, name: &str) -> Option<Self> {
        let lower = name.to_lowercase();
        let words: Vec<_> = lower.split(|c: char| !c.is_alphanumeric()).collect();
        let has = |choices: &[&str]| choices.iter().any(|word| words.contains(word));
        match role {
            Role::Bass if has(&["808", "sub", "subbass"]) => Some(Self::SubBass),
            Role::Bass if has(&["deep", "synth"]) => Some(Self::DeepBass),
            Role::Drums if has(&["snare", "clap"]) => Some(Self::Snare),
            Role::Drums if has(&["tom", "toms"]) => Some(Self::Toms),
            Role::Drums if has(&["hat", "hats", "hihat", "cymbal", "cymbals", "ride", "crash"]) => {
                Some(Self::Cymbals)
            }
            Role::LeadVocal if has(&["speech", "dialogue", "spoken"]) => Some(Self::Dialogue),
            Role::Music | Role::Lead => {
                if has(&["guitar", "guitars", "git", "gtr"]) {
                    return Some(
                        if has(&["distorted", "distortion", "dist", "crunch", "highgain"]) {
                            Self::DistortedGuitar
                        } else if has(&["acoustic", "ac", "nylon", "steel"]) {
                            Self::AcousticGuitar
                        } else {
                            Self::ElectricGuitar
                        },
                    );
                }
                if has(&["rhodes", "wurlitzer", "wurli", "ep", "organ", "e-piano"])
                    || has(&["electric"]) && has(&["piano", "keys"])
                {
                    Some(Self::ElectricPiano)
                } else if has(&["piano", "grand", "keys"]) {
                    Some(Self::Piano)
                } else if has(&["cello", "contrabass"]) || has(&["low"]) && has(&["strings"]) {
                    Some(Self::LowStrings)
                } else if has(&["strings", "violin", "viola", "violins"]) {
                    Some(Self::HighStrings)
                } else if has(&["tuba", "trombone", "euphonium"])
                    || has(&["low"]) && has(&["brass"])
                {
                    Some(Self::LowBrass)
                } else if has(&["brass", "trumpet", "flugelhorn"]) {
                    Some(Self::HighBrass)
                } else if has(&["pad", "pads"]) {
                    Some(Self::SynthPad)
                } else if has(&["synth", "synths", "synthlead"]) {
                    Some(Self::SynthLead)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn fallback(role: Role) -> Option<Self> {
        Some(match role {
            Role::Bass => Self::Bass,
            Role::Kick => Self::Kick,
            Role::Drums => Self::Drums,
            Role::LeadVocal | Role::BackingVocal => Self::Vocals,
            Role::Pad => Self::SynthPad,
            Role::Music | Role::Lead => Self::Instrument,
            Role::InstrumentBus => Self::InstrumentBus,
            Role::VocalBus => Self::VocalBus,
            Role::BassBus => Self::BassBus,
            Role::DrumBus => Self::DrumBus,
            Role::MixBus | Role::Bus => Self::MixBus,
            Role::Unknown | Role::Return | Role::General => return None,
        })
    }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Dynamics {
    pub ratio: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tonal {
    centre: [f64; quality::HZ.len()],
    upper: [f64; quality::HZ.len()],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Profile {
    tonal: Option<Tonal>,
    pub dynamics: Option<Dynamics>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Pack {
    profiles: BTreeMap<Kind, Profile>,
}

#[derive(Default)]
pub(super) enum Library {
    #[default]
    Absent,
    Invalid,
    Loaded(Pack),
}

impl Library {
    pub fn load() -> Self {
        let beside_app = std::env::current_exe()
            .ok()
            .map(|path| path.with_file_name(FILE));
        let profile = os_paths::user_dir().map(|p| p.join(FILE));
        for path in beside_app.into_iter().chain(profile) {
            match Pack::read(&path) {
                Ok(Some(pack)) => return Self::Loaded(pack),
                Ok(None) => (),
                Err(()) => return Self::Invalid,
            }
        }
        Self::Absent
    }

    pub fn status(&self) -> &'static str {
        match self {
            Self::Absent => "Built-in profiles",
            Self::Invalid => "Reference pack unreadable · Using built-in profiles",
            Self::Loaded(_) => "Reference profiles loaded",
        }
    }

    pub fn count(&self) -> usize {
        match self {
            Self::Loaded(pack) => pack.profiles.len(),
            _ => 0,
        }
    }

    pub fn get(&self, track: &Track, project: &Project) -> Option<(Kind, &Profile)> {
        let Self::Loaded(pack) = self else {
            return None;
        };
        let role = track.role;
        let kind = Kind::named(role, &track.name)
            .or_else(|| {
                let mut suggestions = project
                    .channels
                    .iter()
                    .filter(|ch| ch.enabled && ch.mixer_track == track.id)
                    .filter_map(|ch| {
                        Kind::named(role, ch.name.as_deref().unwrap_or_default()).or_else(|| {
                            let name = ch.sample_path.as_deref()?.rsplit(['/', '\\']).next()?;
                            Kind::named(role, name)
                        })
                    });
                let first = suggestions.next()?;
                suggestions.all(|kind| kind == first).then_some(first)
            })
            .or_else(|| Kind::fallback(role))?;
        pack.profiles.get(&kind).map(|profile| (kind, profile))
    }
}

impl Pack {
    fn read(path: &Path) -> Result<Option<Self>, ()> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(()),
        };
        let mut data = Vec::new();
        file.take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut data)
            .map_err(|_| ())?;
        Self::parse(&data).map(Some)
    }

    pub(super) fn parse(data: &[u8]) -> Result<Self, ()> {
        if data.len() > MAX_BYTES {
            return Err(());
        }
        let pack: Self =
            serde_json::from_slice(data.strip_prefix(HEADER).ok_or(())?).map_err(|_| ())?;
        if pack.profiles.is_empty()
            || !pack.profiles.values().all(|p| {
                (p.tonal.is_some() || p.dynamics.is_some())
                    && p.tonal.as_ref().is_none_or(|t| {
                        t.centre
                            .into_iter()
                            .zip(t.upper)
                            .all(|(c, u)| (-90.0..=0.0).contains(&c) && (c..=6.0).contains(&u))
                    })
                    && p.dynamics.is_none_or(|d| {
                        (1.0..=4.0).contains(&d.ratio)
                            && (1.0..=80.0).contains(&d.attack_ms)
                            && (40.0..=300.0).contains(&d.release_ms)
                    })
            })
        {
            return Err(());
        }
        Ok(pack)
    }
}

impl Profile {
    /// Compare relative band energy on active windows. A cut requires sustained
    /// excess over the upper reference bound and a second audible band. This
    /// avoids treating a single fundamental or a short drum hit as a tonal fault.
    pub(super) fn excess(
        &self,
        level: &Level,
        skip: [bool; quality::HZ.len()],
    ) -> Option<(usize, f64)> {
        let tonal = self.tonal.as_ref()?;
        let gate = (level.rms(0.5) - 14.).max(-60.) as f32;
        let frames: Vec<_> = level.frames.iter().filter(|f| f.rms > gate).collect();
        if frames.len() < 25 {
            return None;
        }
        let observed = std::array::from_fn::<_, { quality::HZ.len() }, _>(|band| {
            quality::quantile(frames.iter().map(|f| f64::from(f.bands[band] - f.rms)), 0.5)
        });
        if observed.iter().filter(|&&db| db > -15.).count() < 2 {
            return None;
        }
        observed
            .into_iter()
            .zip(tonal.upper)
            .enumerate()
            .filter_map(|(band, (measured, upper))| {
                let excess = measured - upper;
                (!skip[band] && measured > -16. && tonal.centre[band] > -35. && excess > 2.5)
                    .then_some((band, excess))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
    }
}

pub(super) fn label(kind: Kind) -> &'static str {
    kind.label()
}

pub(crate) fn startup_profile_count() -> usize {
    Library::load().count()
}
