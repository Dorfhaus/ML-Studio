// SPDX-License-Identifier: GPL-3.0-or-later
//! Optional appearance extensions in a Lemon Studio .flstheme. Legacy themes inherit their skin.

use crate::{Color, Palette, Style};

pub const COLOR_PREFIX: &str = "LemonStudioColor.";
pub const STYLE_PREFIX: &str = "LemonStudioStyle.";

pub const NOTE_GROUPS: [&str; 16] = [
    "note_group_01",
    "note_group_02",
    "note_group_03",
    "note_group_04",
    "note_group_05",
    "note_group_06",
    "note_group_07",
    "note_group_08",
    "note_group_09",
    "note_group_10",
    "note_group_11",
    "note_group_12",
    "note_group_13",
    "note_group_14",
    "note_group_15",
    "note_group_16",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    Flat,
    Beveled,
    Recessed,
}

impl Finish {
    pub const CHOICES: [&str; 4] = ["Inherit", "Flat", "Beveled", "Recessed"];

    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "flat" => Some(Self::Flat),
            "beveled" => Some(Self::Beveled),
            "recessed" => Some(Self::Recessed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indicator {
    Round,
    Square,
    Bar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Knob {
    Dial,
    Ring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lettering {
    Bold,
    Italic,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mixer {
    Original,
    CleanModern,
    Analog,
    Theme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeakMeterFill {
    Gradient,
    Solid,
    Segments,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TraditionalMeter {
    pub fill: Option<PeakMeterFill>,
    pub width: Option<f32>,
    pub gap: Option<f32>,
    pub gradient: Option<f32>,
    pub edge: Option<f32>,
    pub radius: Option<f32>,
    pub ticks: Option<bool>,
    pub hold: Option<bool>,
    pub hold_width: Option<f32>,
    pub cap: Option<bool>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RecordLightStyle {
    #[default]
    ConsoleLed,
    BacklitSquare,
    RecBadge,
    Original,
}

impl RecordLightStyle {
    pub const CHOICES: [&'static str; 3] = ["Console LED", "Backlit square", "REC badge"];
    pub const MIXER_CHOICES: [&'static str; 4] = ["Original", "Console LED", "Backlit square", "REC badge"];

    fn parse(value: &str) -> Option<Self> {
        match value {
            "Console LED" => Some(Self::ConsoleLed),
            "Backlit square" => Some(Self::BacklitSquare),
            "REC badge" => Some(Self::RecBadge),
            "Original" => Some(Self::Original),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Original => Self::MIXER_CHOICES[0],
            _ => Self::CHOICES[self as usize],
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RecordLight {
    pub style: RecordLightStyle,
    pub size: Option<f32>,
    pub glow: Option<f32>,
}

/// A theme can keep the surface's original treatment or choose a flat mix with its assigned color.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AssignedColor(Option<f32>);

impl AssignedColor {
    pub fn fill(self, base: Color, assigned: Option<Color>) -> Option<Color> {
        Some(base.mix(assigned?, self.0?))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AssignedColors {
    pub traditional_mixer: AssignedColor,
    pub full_mixer: AssignedColor,
    pub playlist_tracks: AssignedColor,
    pub playlist_clips: AssignedColor,
    pub rack_channels: AssignedColor,
}

/// Resolved rendering options. Each stays absent until a theme opts in.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Appearance {
    pub assigned_colors: AssignedColors,
    /// Insert-slot controls are independent of shared dials and the Full Mixer finish.
    pub playlist_halo_visible: Option<bool>,
    pub playlist_halo_strength: Option<f32>,
    pub playlist_record_light: RecordLight,
    pub trad_record_light: Option<RecordLight>,
    pub traditional_meter: TraditionalMeter,
    pub insert_knob: Option<Knob>,
    pub insert_full_knob: bool,
    pub insert_traditional_knob: bool,
    pub insert_custom: bool,
    pub insert_decorations_custom: bool,
    pub insert_face_custom: bool,
    pub insert_face_finish: Option<Finish>,
    pub insert_face_size: Option<f32>,
    pub insert_face_edge_width: Option<f32>,
    pub insert_pointer_width: Option<f32>,
    pub insert_pointer_length: Option<f32>,
    pub insert_track: Option<bool>,
    pub insert_ring_radius: Option<f32>,
    pub insert_rim_radius: Option<f32>,
    pub insert_ring: Option<bool>,
    pub insert_rim: Option<bool>,
    pub insert_ring_width: Option<f32>,
    pub insert_rim_width: Option<f32>,
    pub knob_track_custom: bool,
    pub knob_rim_custom: bool,
    pub knob_ring_visible: Option<bool>,
    pub knob_rim_visible: Option<bool>,
    pub knob_track_visible: Option<bool>,
    pub knob_ring_width: Option<f32>,
    pub knob_pointer_width: Option<f32>,
    /// Painter-only: render a cap without its surrounding decorations.
    pub knob_body_only: bool,
    pub buttons: Option<Finish>,
    pub panels: Option<Finish>,
    pub playlist_buttons: Option<Finish>,
    pub piano_buttons: Option<Finish>,
    pub rack_steps: Option<Finish>,
    pub mixer_buttons: Option<Finish>,
    pub indicator: Option<Indicator>,
    pub knob: Option<Knob>,
    pub mixer: Option<Mixer>,
    pub title_text: Option<Lettering>,
    pub icon_pack: u8,
    pub icon_mixing: bool,
    pub icon_color: Option<Color>,
    pub icon_variants: [[u8; 16]; 5],
    pub icon_colors: [[Option<Color>; 16]; 5],
    pub cursor_color: Option<Color>,
    pub cursor_size: Option<f32>,
    pub cursor_family: Option<u8>,
    /// Stable cursor roles: 0 inherits, otherwise family + 1 (including Default).
    pub cursor_variants: [[u8; 16]; 3],
    pub ui_font: u8,
    pub bevel: Option<f32>,
    pub border: Option<f32>,
    pub button_radius: Option<f32>,
    pub small_radius: Option<f32>,
    pub panel_radius: Option<f32>,
    pub playlist_lines: Option<f32>,
    pub piano_lines: Option<f32>,
    pub themed_buttons: bool,
    pub chrome_overrides: u64,
    pub note_groups: u16,
}

pub const CUSTOM_ICON_PACKS: u8 = 27;
pub const VINTAGE_ICON_PACK: u8 = 27;
pub const VINTAGE_CURSOR_PACK: u8 = 28;

impl Appearance {
    pub(crate) fn parse(get: impl Fn(&str) -> Option<String>) -> Self {
        let option = |key: &str| get(key).unwrap_or_default();
        let num = |key: &str, max: f32| {
            get(key)?
                .parse::<f32>()
                .ok()
                .filter(|n| n.is_finite())
                .map(|n| n.clamp(0.0, max))
        };
        let visible = |key| match option(key).as_str() {
            "Show" => Some(true), "Hide" => Some(false), _ => None,
        };
        let assigned_color = |key| AssignedColor(num(key, 100.0).map(|value| value / 100.0));
        let insert_decorations_custom = [
            "insert-track", "insert-ring", "insert-rim", "insert-ring-width", "insert-rim-width",
            "insert-ring-radius", "insert-rim-radius",
        ].iter().any(|key| option(key).as_str() != "Inherit" && get(key).is_some());
        let insert_face_finish = Finish::parse(&option("insert-face-finish"));
        let insert_face_size = num("insert-face-size", 18.0).map(|n| n.max(6.0));
        let insert_face_edge_width = num("insert-face-edge-width", 2.0);
        let insert_pointer_width = num("insert-pointer-width", 3.0);
        let insert_pointer_length = num("insert-pointer-length", 100.0);
        let insert_face_custom = insert_face_finish.is_some() || insert_face_edge_width.is_some()
            || insert_pointer_width.is_some() || insert_pointer_length.is_some();
        let record_light = |style, size, glow, default| RecordLight {
            style: RecordLightStyle::parse(&option(style)).unwrap_or(default),
            size: num(size, 13.0).map(|n| n.max(6.0)),
            glow: num(glow, 100.0).map(|n| n / 100.0),
        };
        Self {
            assigned_colors: AssignedColors {
                traditional_mixer: assigned_color("assigned-color-traditional"),
                full_mixer: assigned_color("assigned-color-full-mixer"),
                playlist_tracks: assigned_color("assigned-color-playlist-tracks"),
                playlist_clips: assigned_color("assigned-color-playlist-clips"),
                rack_channels: assigned_color("assigned-color-rack-channels"),
            },
            playlist_halo_visible: visible("playlist-halo-visible"),
            playlist_halo_strength: num("playlist-halo-strength", 200.0).map(|n| n / 100.0),
            playlist_record_light: record_light("playlist-record-light", "playlist-record-size", "playlist-record-glow", RecordLightStyle::ConsoleLed),
            trad_record_light: RecordLightStyle::parse(&option("trad-record-light")).map(|style| RecordLight {
                style,
                size: num("trad-record-size", 13.0).map(|n| n.max(6.0)),
                glow: num("trad-record-glow", 100.0).map(|n| n / 100.0),
            }),
            traditional_meter: TraditionalMeter {
                fill: match option("trad-meter-fill").as_str() {
                    "Gradient" => Some(PeakMeterFill::Gradient),
                    "Solid" => Some(PeakMeterFill::Solid),
                    "Segments" => Some(PeakMeterFill::Segments),
                    _ => None,
                },
                width: num("trad-meter-width", 100.0).map(|value| (value / 100.0).max(0.2)),
                gap: num("trad-meter-gap", 60.0).map(|value| value / 100.0),
                gradient: num("trad-meter-gradient", 100.0).map(|value| value / 100.0),
                edge: num("trad-meter-edge", 100.0).map(|value| value / 100.0),
                radius: num("trad-meter-radius", 4.0),
                ticks: visible("trad-meter-ticks"),
                hold: visible("trad-meter-hold"),
                hold_width: num("trad-meter-hold-width", 3.0),
                cap: visible("trad-meter-cap"),
            },
            insert_knob: match option("insert-knob").as_str() {
                "Dial" => Some(Knob::Dial), "Ring" => Some(Knob::Ring), _ => None,
            },
            insert_full_knob: option("insert-knob") == "Full Mixer",
            insert_traditional_knob: option("insert-knob") == "Traditional",
            insert_custom: matches!(option("insert-knob").as_str(), "Traditional" | "Full Mixer" | "Dial" | "Ring")
                || insert_decorations_custom || insert_face_custom || insert_face_size.is_some(),
            insert_decorations_custom,
            insert_face_custom,
            insert_face_finish,
            insert_face_size,
            insert_face_edge_width,
            insert_pointer_width,
            insert_pointer_length,
            insert_track: visible("insert-track"),
            insert_ring_radius: num("insert-ring-radius", 12.0).map(|n| n.max(4.0)),
            insert_rim_radius: num("insert-rim-radius", 13.0).map(|n| n.max(4.0)),
            insert_ring: visible("insert-ring"),
            insert_rim: visible("insert-rim"),
            insert_ring_width: num("insert-ring-width", 2.0),
            insert_rim_width: num("insert-rim-width", 2.0),
            knob_track_custom: false,
            knob_rim_custom: false,
            knob_ring_visible: visible("knob-ring-visible"),
            knob_rim_visible: visible("knob-rim-visible"),
            knob_track_visible: visible("knob-track-visible"),
            knob_ring_width: num("knob-ring-width", 3.0),
            knob_pointer_width: num("knob-pointer-width", 3.0),
            knob_body_only: false,
            buttons: Finish::parse(&option("buttons")),
            panels: Finish::parse(&option("panels")),
            playlist_buttons: Finish::parse(&option("playlist-buttons")),
            piano_buttons: Finish::parse(&option("piano-buttons")),
            rack_steps: Finish::parse(&option("rack-steps")),
            mixer_buttons: Finish::parse(&option("mixer-buttons")),
            indicator: match option("indicator").as_str() {
                "Round" => Some(Indicator::Round),
                "Square" => Some(Indicator::Square),
                "Bar" => Some(Indicator::Bar),
                _ => None,
            },
            knob: match option("knob").as_str() {
                "Dial" => Some(Knob::Dial),
                "Ring" => Some(Knob::Ring),
                _ => None,
            },
            mixer: match option("mixer").as_str() {
                "Original" => Some(Mixer::Original),
                "Clean Modern" => Some(Mixer::CleanModern),
                "Analog" => Some(Mixer::Analog),
                "Theme colors" => Some(Mixer::Theme),
                _ => None,
            },
            // The previous numeric setting described weight and tint recipes. Only the
            // explicit icon-family setting opts a theme into the authored artwork.
            icon_pack: option("icon-family").parse::<u8>().ok().filter(|id| *id <= CUSTOM_ICON_PACKS).unwrap_or(0),
            icon_mixing: option("icon-mixing") == "1",
            icon_color: Color::parse(&option("icon-color")),
            // IDs are append-only positions in the semantic icon catalogue.
            icon_variants: std::array::from_fn(|row| std::array::from_fn(|col| {
                option(&format!("icon-{}-family", row * 16 + col)).parse::<u8>()
                    .ok().filter(|id| *id <= CUSTOM_ICON_PACKS + 1).unwrap_or(0)
            })),
            icon_colors: std::array::from_fn(|row| std::array::from_fn(|col| {
                Color::parse(&option(&format!("icon-{}-color", row * 16 + col)))
            })),
            cursor_color: Color::parse(&option("cursor-color")),
            cursor_size: num("cursor-size", 200.0).map(|n| n.clamp(50.0, 200.0)),
            cursor_family: option("cursor-family").parse::<u8>().ok().filter(|id| *id <= VINTAGE_CURSOR_PACK),
            cursor_variants: std::array::from_fn(|row| std::array::from_fn(|col| {
                option(&format!("cursor-{}-family", row * 16 + col)).parse::<u8>()
                    .ok().filter(|id| *id <= VINTAGE_CURSOR_PACK + 1).unwrap_or(0)
            })),
            ui_font: num("ui-font", 4.0).unwrap_or(0.0) as u8,
            title_text: match option("title-text").as_str() {
                "Bold" => Some(Lettering::Bold),
                "Italic" => Some(Lettering::Italic),
                "Light" => Some(Lettering::Light),
                _ => None,
            },
            bevel: num("bevel", 40.0).map(|n| n / 100.0),
            border: num("border", 3.0),
            button_radius: num("button-radius", 12.0),
            small_radius: num("small-radius", 8.0),
            panel_radius: num("panel-radius", 20.0),
            playlist_lines: num("playlist-lines", 200.0).map(|n| n / 100.0),
            piano_lines: num("piano-lines", 200.0).map(|n| n / 100.0),
            themed_buttons: false,
            chrome_overrides: 0,
            note_groups: 0,
        }
    }

    pub(crate) fn style(self, base: Style) -> Style {
        Style {
            radius: self.button_radius.unwrap_or(base.radius),
            radius_small: self.small_radius.unwrap_or(base.radius_small),
            radius_panel: self.panel_radius.unwrap_or(base.radius_panel),
            ..base
        }
    }

    pub(crate) fn apply(mut self, palette: &mut Palette, colors: &[(&'static str, Color)]) {
        let overridden = |key: &str| colors.iter().find(|(name, _)| *name == key).map(|(_, c)| *c);
        let white = overridden("key_white");
        let black = overridden("key_black");
        let black_left = overridden("key_black_left");
        if let Some(white) = white {
            palette.playlist_key_white = white.mix(palette.sunken, 0.12);
        }
        if black.is_some() || black_left.is_some() {
            palette.playlist_key_black = black_left.unwrap_or(palette.key_black_left)
                .mix(black.unwrap_or(palette.key_black), 0.25);
        }
        if let Some(black_left) = black_left {
            palette.playlist_key_frame = black_left.mix(palette.sunken, 0.35);
        }
        if white.is_some() || black.is_some() {
            palette.playlist_key_separator = white.unwrap_or(palette.key_white)
                .mix(black.unwrap_or(palette.key_black), 0.38);
        }
        if let Some(accent) = overridden("accent") {
            palette.playlist_key_white_active = accent;
            palette.playlist_key_black_active = accent;
            palette.playlist_activity_halo = accent;
            palette.playlist_waveform_active = accent;
        }
        if let Some(text) = overridden("text_dim") {
            palette.playlist_waveform = text;
        }
        for &(name, color) in colors {
            if let Some(chrome) = crate::chrome::Chrome::for_key(name) {
                self.chrome_overrides |= chrome.bit();
            }
            if let Some(slot) = palette.get_mut(name) {
                *slot = color;
            }
            if let Some(group) = NOTE_GROUPS.iter().position(|&key| key == name) {
                self.note_groups |= 1 << group;
            }
            if matches!(name, "indicator_on" | "indicator_off") {
                self.indicator.get_or_insert(Indicator::Round);
            }
            self.knob_track_custom |= name == "knob_track";
            self.knob_rim_custom |= name == "knob_rim";
            if name.starts_with("knob_") {
                self.knob.get_or_insert(Knob::Ring);
            }
            if name.starts_with("insert_knob_") {
                self.insert_custom = true;
                if matches!(name, "insert_knob_face" | "insert_knob_face_edge" | "insert_knob_pointer") {
                    self.insert_face_custom = true;
                } else {
                    self.insert_decorations_custom = true;
                    if name == "insert_knob_track" && self.insert_track.is_none() {
                        self.insert_track = Some(true);
                    }
                }
            }
            // Existing exported themes with mixer overrides also need the palette
            // renderer. An explicitly chosen authored finish still takes precedence.
            if (name.starts_with("mixer_") && crate::chrome::Chrome::for_key(name).is_none())
                || name.starts_with("slot") || name == "fader_handle"
            {
                self.mixer.get_or_insert(Mixer::Theme);
            }
            self.themed_buttons |= matches!(
                name,
                "button"
                    | "button_hi"
                    | "button_pressed"
                    | "button_disabled"
                    | "button_border"
                    | "on_top"
                    | "on_bottom"
            );
        }
        if self.knob_ring_visible.is_some() || self.knob_rim_visible.is_some()
            || self.knob_track_visible.is_some() || self.knob_ring_width.is_some()
            || self.knob_pointer_width.is_some() {
            self.knob.get_or_insert(Knob::Ring);
        }
        palette.style = self.style(palette.style);
        palette.pl_lines = self.playlist_lines.unwrap_or(palette.pl_lines);
        palette.pr_lines = self.piano_lines.unwrap_or(palette.pr_lines);
        palette.appearance = self;
    }
}

#[cfg(test)]
mod icon_family_tests {
    use super::*;

    #[test]
    fn icon_families_require_explicit_selection() {
        let parse = |old: Option<&str>, family: Option<&str>| Appearance::parse(|key| {
            match key {
                "icon-pack" => old,
                "icon-family" => family,
                _ => None,
            }.map(str::to_owned)
        }).icon_pack;
        assert_eq!(parse(None, None), 0);
        assert_eq!(parse(Some("7"), None), 0);
        assert_eq!(parse(Some("2"), Some("0")), 0);
        assert_eq!(parse(Some("2"), Some("13")), 13);
        assert_eq!(parse(None, Some("28")), 0);
        assert_eq!(parse(None, Some("1.2")), 0);
    }
}
