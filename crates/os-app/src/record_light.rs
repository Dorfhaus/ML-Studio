// SPDX-License-Identifier: GPL-3.0-or-later
//! Record buttons for Playlist track headers and Traditional Mixer inserts.
use os_theme::{Color, Palette, appearance::{RecordLight, RecordLightStyle}};
use os_ui::{Canvas, Font, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Playlist,
    TraditionalMixer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lamp {
    Idle,
    Armed,
    Recording,
}

impl Lamp {
    pub const ALL: [Self; 3] = [Self::Idle, Self::Armed, Self::Recording];

    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Armed => "Armed",
            Self::Recording => "Recording",
        }
    }
}

impl Surface {
    pub fn appearance(self, t: &Palette) -> RecordLight {
        match self {
            Self::Playlist => t.appearance.playlist_record_light,
            Self::TraditionalMixer => t.appearance.trad_record_light.unwrap_or(RecordLight {
                style: RecordLightStyle::ConsoleLed,
                ..Default::default()
            }),
        }
    }

    fn colors(self, t: &Palette) -> [Color; 4] {
        match self {
            Self::Playlist => [t.playlist_record_idle, t.playlist_record_armed, t.playlist_record_live, t.playlist_record_rim],
            Self::TraditionalMixer => [t.trad_record_idle, t.trad_record_armed, t.trad_record_armed, t.trad_record_rim],
        }
    }

    pub fn states(self) -> &'static [Lamp] {
        match self {
            Self::Playlist => &Lamp::ALL,
            Self::TraditionalMixer => &[Lamp::Idle, Lamp::Armed],
        }
    }
}

pub fn playlist_width(t: &Palette, available: f32) -> f32 {
    if !t.style.vintage_2005 && available >= 38.
        && Surface::Playlist.appearance(t).style == RecordLightStyle::RecBadge {
        38.
    } else {
        22.
    }
}

pub fn paint(c: &mut Canvas, t: &Palette, r: Rect, surface: Surface, lamp: Lamp, hovered: bool) {
    let look = surface.appearance(t);
    if surface == Surface::TraditionalMixer && (t.style.vintage_2005 || look.style == RecordLightStyle::Original) {
        let color = if lamp == Lamp::Idle { t.text_dim } else { t.h(0xff6868) };
        c.fill_circle(r.cx(), r.cy(), 4., color);
        return;
    }
    let [idle, armed, recording, rim] = surface.colors(t);
    let color = match lamp { Lamp::Idle => idle, Lamp::Armed => armed, Lamp::Recording => recording };
    // The Mixer button shows disk arm. Brighten its armed appearance without changing the Playlist's live state.
    let lamp = if surface == Surface::TraditionalMixer && lamp == Lamp::Armed { Lamp::Recording } else { lamp };
    let glow = look.glow.unwrap_or(0.6);
    let w = r.w;
    let radius = (look.size.unwrap_or(9.) / 2.).min((r.h - 2.) / 2.);
    if radius <= 0. || w < 14. { return; }
    c.push_clip(r);
    let edge = if hovered { t.glyph } else { rim };
    match look.style {
        RecordLightStyle::ConsoleLed | RecordLightStyle::Original => {
            let frame = (radius + 2.).min((r.h - 1.) / 2.);
            c.fill_circle(r.cx(), r.cy(), frame, t.sunken);
            c.stroke_circle(r.cx(), r.cy(), frame, 1., edge);
            lens(c, r.cx(), r.cy(), radius.min(frame - 0.5), color, lamp, glow);
        }
        style => {
            let badge = style == RecordLightStyle::RecBadge && w >= 34.;
            let compact_badge = style == RecordLightStyle::RecBadge && surface == Surface::TraditionalMixer && !badge;
            let width = if badge || compact_badge { w - 2. } else { (radius * 2. + 10.).min(w - 2.) };
            let height = if compact_badge { (radius * 2. + 4.).min(r.h - 1.) } else { r.h - 1. };
            let face = Rect::new(r.cx() - width / 2., r.cy() - height / 2., width, height);
            let fill = match lamp {
                Lamp::Idle => t.sunken,
                Lamp::Armed => t.sunken.mix(color, 0.24),
                Lamp::Recording => color,
            };
            let edge = if hovered || lamp == Lamp::Idle { edge } else { color.shade(0.2) };
            if lamp == Lamp::Recording && glow > 0. {
                c.fill_rrect(face.outset(1.), 3., color.with_alpha((glow * 105.) as u8));
            }
            c.fill_rrect(face, 3., fill);
            c.stroke_rrect(face, 3., 1., edge);
            let x = if badge { face.x + 7. } else { face.cx() };
            let radius = if badge { (radius * 0.65).clamp(2., 4.) } else { (radius - 1.).max(2.) };
            if !compact_badge && lamp == Lamp::Recording {
                c.fill_circle(x, face.cy(), radius, os_ui::kit::lemon::ink_on(fill));
            } else if !compact_badge {
                c.stroke_circle(x, face.cy(), radius, 1., color);
            }
            if badge || compact_badge {
                let label = if badge { Rect::new(face.x + 11., face.y, face.w - 14., face.h) } else { face };
                let ink = if lamp == Lamp::Idle { t.text_dim } else { os_ui::kit::lemon::ink_on(fill) };
                c.text_center(Font::new(if badge { 9 } else { 8 }).bold(), label, ink, "REC");
            }
        }
    }
    c.pop_clip();
}

fn lens(c: &mut Canvas, x: f32, y: f32, radius: f32, color: Color, lamp: Lamp, glow: f32) {
    match lamp {
        Lamp::Idle => {
            c.fill_circle(x, y, radius, color.shade(-0.25));
            c.stroke_circle(x, y, radius, 1., color);
        }
        Lamp::Armed => {
            c.fill_circle(x, y, radius, color.shade(-0.72));
            c.stroke_circle(x, y, radius, 1.5, color);
        }
        Lamp::Recording => {
            if glow > 0. {
                c.fill_circle(x, y, radius + 1.6, color.with_alpha((glow * 135.) as u8));
            }
            c.fill_circle(x, y, radius, color);
            c.fill_circle(x - radius * 0.28, y - radius * 0.3, radius * 0.24, color.shade(0.42));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_light_styles_draw_distinct_states_at_channel_size() {
        let mut palette = crate::app::App::new(None).theme.colors;
        palette.playlist_record_live = Color::rgb(0xff1938);
        for style in [RecordLightStyle::ConsoleLed, RecordLightStyle::BacklitSquare, RecordLightStyle::RecBadge] {
            palette.appearance.playlist_record_light.style = style;
            let drawn = Lamp::ALL.map(|lamp| {
                let mut canvas = Canvas::new(64, 40, 1.).unwrap();
                paint(&mut canvas, &palette, Rect::new(8., 7., 38., 16.), Surface::Playlist, lamp, false);
                canvas.pixmap.data().to_vec()
            });
            assert_ne!(drawn[0], drawn[1], "{style:?} idle and armed");
            assert_ne!(drawn[1], drawn[2], "{style:?} armed and recording");
            assert!(drawn[2].chunks_exact(4).any(|pixel| pixel == [255, 25, 56, 255]), "{style:?} custom recording color");
        }
    }

}
