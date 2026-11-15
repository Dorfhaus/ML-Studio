// SPDX-License-Identifier: GPL-3.0-or-later
//! The three state indicators shared by Playlist track headers and Channel Rack rows.
use crate::app::{Action, State, Ui};
use os_theme::{Color, Palette, appearance::{RecordLight, RecordLightStyle}};
use os_ui::{Canvas, Font, Rect};

pub const RACK_VISIBILITY: &str = "visible_rack-record-lights";
pub const RACK_COLUMN: f32 = 42.;

#[derive(Clone, Copy, Debug)]
pub enum Surface {
    Playlist,
    Rack,
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
            Self::Rack => t.appearance.rack_record_light,
        }
    }

    fn colors(self, t: &Palette) -> [Color; 4] {
        match self {
            Self::Playlist => [t.playlist_record_idle, t.playlist_record_armed, t.playlist_record_live, t.playlist_record_rim],
            Self::Rack => [t.rack_record_idle, t.rack_record_armed, t.rack_record_live, t.rack_record_rim],
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
    let [idle, armed, recording, rim] = surface.colors(t);
    let color = match lamp { Lamp::Idle => idle, Lamp::Armed => armed, Lamp::Recording => recording };
    let glow = look.glow.unwrap_or(0.6);
    let w = r.w;
    let radius = (look.size.unwrap_or(9.) / 2.).min((r.h - 2.) / 2.);
    if radius <= 0. || w < 14. { return; }
    c.push_clip(r);
    let edge = if hovered { t.glyph } else { rim };
    match look.style {
        RecordLightStyle::ConsoleLed => {
            let frame = (radius + 2.).min((r.h - 1.) / 2.);
            c.fill_circle(r.cx(), r.cy(), frame, t.sunken);
            c.stroke_circle(r.cx(), r.cy(), frame, 1., edge);
            lens(c, r.cx(), r.cy(), radius.min(frame - 0.5), color, lamp, glow);
        }
        style => {
            let badge = style == RecordLightStyle::RecBadge && w >= 34.;
            let width = if badge { w - 2. } else { (radius * 2. + 10.).min(w - 2.) };
            let face = Rect::new(r.cx() - width / 2., r.y + 0.5, width, r.h - 1.);
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
            if lamp == Lamp::Recording {
                c.fill_circle(x, face.cy(), radius, os_ui::kit::lemon::ink_on(fill));
            } else {
                c.stroke_circle(x, face.cy(), radius, 1., color);
            }
            if badge {
                let label = Rect::new(face.x + 11., face.y, face.w - 14., face.h);
                let ink = if lamp == Lamp::Idle { t.text_dim } else { os_ui::kit::lemon::ink_on(fill) };
                c.text_center(Font::new(9).bold(), label, ink, "REC");
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

pub fn rack_column() -> f32 {
    if crate::workspace_visibility::shown(RACK_VISIBILITY) { RACK_COLUMN } else { 0. }
}

pub fn rack_lamp(s: &State, row: usize) -> Option<Lamp> {
    let ch = s.project.channels.get(row)?;
    use os_model::ChannelKind::{Layer, Plugin, Sampler};
    if !matches!(ch.kind, Plugin | Sampler | Layer) { return None; }
    let armed = s.selected_channel == row && s.rec.armed && crate::record::Opt::Notes.on();
    Some(match (armed, s.rec.rack_note_record_channel == Some(ch.index)) {
        (true, true) => Lamp::Recording,
        (true, false) => Lamp::Armed,
        _ => Lamp::Idle,
    })
}

pub fn rack_indicator(ui: &mut Ui, s: &State, row: usize, rect: Rect) {
    let Some(lamp) = rack_lamp(s, row) else { return; };
    let hint = match lamp {
        Lamp::Idle => "Select a channel and arm Notes with the transport Record button",
        Lamp::Armed => "Note recording armed for this channel",
        Lamp::Recording => "Recording notes on this channel",
    };
    ui.hit(rect, Action::Swallow, hint);
    paint(ui.c, ui.t, rect, Surface::Rack, lamp, false);
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

    #[test]
    fn record_lights_follow_note_capture_and_leave_room_for_rack_controls() {
        use crate::record::Opt;
        let mut app = crate::app::App::new(None);
        app.state.project = crate::app::test_project();
        app.state.song_mode = false;
        let row = app.state.project.channels.iter().position(|ch| {
            matches!(ch.kind, os_model::ChannelKind::Plugin | os_model::ChannelKind::Sampler)
        }).unwrap();
        app.state.selected_channel = row;
        assert_eq!(rack_lamp(&app.state, row), Some(Lamp::Idle));
        crate::mixer_recording::flag(Opt::Notes, true);
        app.state.rec.armed = true;
        assert_eq!(rack_lamp(&app.state, row), Some(Lamp::Armed));
        app.transport.pretend_at(48., false);
        app.record_tick();
        assert_eq!(rack_lamp(&app.state, row), Some(Lamp::Recording));
        crate::mixer_recording::flag(Opt::Notes, false);
        app.record_tick();
        assert_eq!(rack_lamp(&app.state, row), Some(Lamp::Idle));
        app.state.vintage_2005 = false;
        let visible = crate::views::rack::cols(&app.state);
        assert!(visible.step_x >= visible.name_x + visible.name_w + 189.);
        crate::settings_file::set_flag(RACK_VISIBILITY, false);
        let hidden = crate::views::rack::cols(&app.state);
        assert_eq!(visible.step_x - hidden.step_x, RACK_COLUMN);
        assert_eq!(visible.name_w, hidden.name_w);
    }
}
