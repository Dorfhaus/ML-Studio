// SPDX-License-Identifier: GPL-3.0-or-later
//! Musical starting points, not genre detection or fixed mastering targets.
//! Research and the limits of these authored values: docs/development/mix-assistant-genres.md.
use super::Role;

const SETTING: &str = "mix_assistant_genre";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Genre {
    #[default]
    Balanced,
    Pop,
    HipHop,
    Rnb,
    Rock,
    Metal,
    Electronic,
    Country,
    Acoustic,
    Jazz,
    Classical,
}

impl Genre {
    pub const ALL: [Self; 11] = [
        Self::Balanced,
        Self::Pop,
        Self::HipHop,
        Self::Rnb,
        Self::Rock,
        Self::Metal,
        Self::Electronic,
        Self::Country,
        Self::Acoustic,
        Self::Jazz,
        Self::Classical,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Balanced => "Balanced / no genre",
            Self::Pop => "Pop",
            Self::HipHop => "Hip-hop / Rap",
            Self::Rnb => "R&B / Soul",
            Self::Rock => "Rock / Indie",
            Self::Metal => "Metal",
            Self::Electronic => "Electronic / Dance",
            Self::Country => "Country",
            Self::Acoustic => "Acoustic / Folk",
            Self::Jazz => "Jazz",
            Self::Classical => "Classical / Orchestral",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Balanced => "Measured correction with moderate dynamics control",
            Self::Pop => "Steady, forward vocals; firm bass and controlled dynamics",
            Self::HipHop => "Present rap vocals; strong kick and bass; tight optional space",
            Self::Rnb => "Smooth vocal control; warm bass; softer optional ambience",
            Self::Rock => "Keep drum attack; support guitars and an audible lead vocal",
            Self::Metal => "Firm vocal and bass control; keep drum attack in dense mixes",
            Self::Electronic => "Prioritize kick and bass; preserve punch; controlled envelopes",
            Self::Country => "Keep the lyric clear; moderate control and natural instruments",
            Self::Acoustic => "Gentle control and EQ; preserve performance dynamics",
            Self::Jazz => "Light corrective processing; retain the ensemble's dynamics",
            Self::Classical => "Minimal correction; preserve orchestral dynamic contrasts",
        }
    }

    pub fn load() -> Self {
        let saved = crate::prefs::text(SETTING);
        Self::ALL
            .into_iter()
            .find(|g| Some(g.label()) == saved.as_deref())
            .unwrap_or_default()
    }

    pub fn save(self) {
        crate::prefs::set_text(SETTING, self.label());
    }

    /// Threshold for upper active-window RMS variation, in dB. Existing
    /// processors and explicit per-effect choices are checked separately.
    pub fn dynamics_gate(self, role: Role) -> f64 {
        let source = match self {
            Self::Pop | Self::HipHop | Self::Rnb => 1.8,
            Self::Metal | Self::Electronic => 2.0,
            Self::Rock => 2.5,
            Self::Balanced | Self::Country => 3.0,
            Self::Acoustic => 4.5,
            Self::Jazz => 6.0,
            Self::Classical => 8.0,
        };
        source + if role.is_bus() { 2.0 } else { 0.0 }
    }

    /// Approximate rendered gain reduction sought on louder moments at 100%.
    /// These are conservative product choices, not published genre standards.
    pub fn reduction(self, role: Role) -> f64 {
        if role.is_bus() {
            return match self {
                Self::Pop | Self::Rock | Self::Metal | Self::Electronic => 1.5,
                Self::HipHop | Self::Rnb | Self::Country => 1.2,
                Self::Balanced => 1.0,
                Self::Acoustic => 0.8,
                Self::Jazz | Self::Classical => 0.5,
            };
        }
        let vocal = matches!(role, Role::LeadVocal | Role::BackingVocal);
        let rhythm = matches!(role, Role::Bass | Role::Kick | Role::Drums);
        match self {
            Self::Pop | Self::HipHop if vocal => 3.5,
            Self::Metal if vocal || rhythm => 3.0,
            Self::Rnb if vocal || role == Role::Bass => 2.8,
            Self::Rock | Self::Electronic if rhythm => 2.8,
            Self::Country if vocal => 2.5,
            Self::Acoustic => 1.3,
            Self::Jazz => 0.8,
            Self::Classical => 0.5,
            _ => 2.0,
        }
    }

    pub fn ratio_scale(self) -> f64 {
        match self {
            Self::Pop | Self::HipHop | Self::Metal => 1.5,
            Self::Rock | Self::Electronic => 1.3,
            Self::Rnb => 1.2,
            Self::Balanced | Self::Country => 1.0,
            Self::Acoustic => 0.7,
            Self::Jazz => 0.5,
            Self::Classical => 0.35,
        }
    }

    pub fn preserve_punch(self) -> bool {
        matches!(
            self,
            Self::HipHop | Self::Rock | Self::Metal | Self::Electronic
        )
    }

    pub fn eq_scale(self) -> f64 {
        match self {
            Self::Acoustic => 0.75,
            Self::Jazz => 0.6,
            Self::Classical => 0.4,
            _ => 1.0,
        }
    }

    /// Only used when the user enables space or explicitly picks a space effect.
    pub fn space_scale(self) -> f64 {
        match self {
            Self::HipHop | Self::Metal => 0.65,
            Self::Country | Self::Acoustic => 0.8,
            Self::Jazz | Self::Classical => 0.6,
            Self::Rnb => 1.15,
            _ => 1.0,
        }
    }

    pub fn balance_offset(self, role: Role) -> f64 {
        match (self, role) {
            (Self::HipHop | Self::Electronic, Role::Bass | Role::Kick) => 1.5,
            (Self::Rnb, Role::Bass) => 1.0,
            (Self::Pop | Self::Country | Self::Rnb | Self::HipHop, Role::LeadVocal) => 0.75,
            (Self::Rock | Self::Metal, Role::Drums | Role::Music) => 0.75,
            (Self::Acoustic, Role::Music) => 0.75,
            _ => 0.0,
        }
    }

    pub fn balance_scale(self) -> f64 {
        match self {
            Self::Acoustic => 0.7,
            Self::Jazz => 0.5,
            Self::Classical => 0.3,
            _ => 1.0,
        }
    }
}
