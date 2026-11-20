// SPDX-License-Identifier: GPL-3.0-or-later

//! Per-note pitch editing over the Piano Roll's own timeline and key geometry.

use crate::{
    app::{Action, App, State, Ui},
    history::{self, Snap},
    piano::{self, Geom},
    playlist::{Cmd, Mark, PItem, Popup},
};
use os_model::{
    Note,
    mpe::{Expression, Point},
};
use os_ui::{Font, Rect};

pub(crate) const VISIBILITY: &str = "visible_piano-pitch-draw";
const GRID: f32 = 1024.;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Brush {
    #[default]
    Draw,
    Points,
    Vibrato,
}

impl Brush {
    fn name(self) -> &'static str {
        match self {
            Self::Draw => "Draw",
            Self::Points => "Points",
            Self::Vibrato => "Vibrato",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    Toggle,
    Brush(Brush),
    Range(u8),
    Snap,
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    Command(Command),
    BrushMenu(Rect),
    RangeMenu(Rect),
    Note(usize),
    Point(usize, usize),
}

#[derive(Default)]
pub struct UiState {
    pub open: bool,
    brush: Brush,
    range: u8,
    snap: bool,
    gesture: Option<Gesture>,
}

struct Gesture {
    pattern: u16,
    index: usize,
    note: Note,
    geom: Geom,
    before: Expression,
    draft: Expression,
    start: (f32, f32),
    stroke: Vec<Point>,
    point: Option<usize>,
}

pub fn shown(s: &State) -> bool {
    s.pitch_draw.open && crate::workspace_visibility::shown(VISIBILITY)
}

fn range(s: &State) -> u8 {
    if s.pitch_draw.range == 0 {
        24
    } else {
        s.pitch_draw.range
    }
}

fn editable(n: &Note) -> bool {
    !n.slide() && n.length != 0
}

pub fn handle(app: &mut App, hit: Hit) {
    let menu = match hit {
        Hit::Command(command) => {
            command_run(app, command);
            return;
        }
        Hit::BrushMenu(rect) => {
            let choices = [Brush::Draw, Brush::Points, Brush::Vibrato];
            let entries = choices
                .into_iter()
                .map(|b| {
                    PItem::new(b.name(), Cmd::PitchDraw(Command::Brush(b)))
                        .mark(Mark::Radio(app.state.pitch_draw.brush == b))
                })
                .collect();
            (rect, entries)
        }
        Hit::RangeMenu(rect) => {
            let entries = [2, 12, 24, 48, 96]
                .into_iter()
                .map(|n| {
                    PItem::new(format!("±{n} semitones"), Cmd::PitchDraw(Command::Range(n)))
                        .mark(Mark::Radio(range(&app.state) == n))
                })
                .collect();
            (rect, entries)
        }
        Hit::Note(_) | Hit::Point(..) => return,
    };
    let (rect, items) = menu;
    app.state.playlist.popup = Some(Popup {
        x: rect.x,
        y: rect.bottom(),
        items,
        sub: None,
        scroll: [0; 2],
        cascade: Vec::new(),
    });
    app.damage_piano();
}

pub fn command_run(app: &mut App, c: Command) {
    finish(app, false);
    let s = &mut app.state;
    match c {
        Command::Toggle => s.pitch_draw.open = !s.pitch_draw.open,
        Command::Brush(b) => s.pitch_draw.brush = b,
        Command::Range(r) => s.pitch_draw.range = r,
        Command::Snap => s.pitch_draw.snap = !s.pitch_draw.snap,
        Command::Clear => {
            let edits = s
                .piano
                .selection(s.pattern, piano::channel(s))
                .into_iter()
                .filter_map(|i| {
                    let mut expression = s
                        .project
                        .mpe
                        .get(piano::notes(s).get(i)?.expression)?
                        .clone();
                    if expression.lanes[0].is_empty() {
                        return None;
                    }
                    expression.lanes[0].clear();
                    Some((i, expression))
                })
                .collect::<Vec<_>>();
            if !edits.is_empty() {
                let snap = Snap::pattern(s, s.pattern);
                history::record(s, "Clear note pitch", snap);
                for (i, expression) in edits {
                    write_expression(s, i, expression);
                }
            }
        }
    }
    app.damage_piano();
}

fn write_expression(s: &mut State, index: usize, expression: Expression) {
    let id = if expression.lanes.iter().all(Vec::is_empty) {
        0
    } else {
        s.project.mpe.add(expression)
    };
    piano::pattern_mut(s).notes[index].expression = id;
    s.piano.dirty = true;
}

pub fn paint_toolbar(ui: &mut Ui, s: &State, rect: Rect) {
    let t = ui.t;
    ui.c.fill_rect(rect, t.sunken);
    ui.c.line(
        rect.x,
        rect.bottom() - 0.5,
        rect.right(),
        rect.bottom() - 0.5,
        1.,
        t.toolbar_edge,
    );
    let mut x = rect.x + 6.;
    let mut button = |ui: &mut Ui, width, name: &str, hit, hint: &str, on, menu| {
        let r = Rect::new(x, rect.y + 3., width, rect.h - 6.);
        x = r.right() + 5.;
        let hover = ui.hit(r, Action::PitchDraw(hit), hint.to_owned());
        ui.c.fill_rrect(
            r,
            3.,
            if hover {
                t.button_hi
            } else if on {
                t.brand_lit(os_ui::Color::rgb(0x3d9edc)).with_alpha(150)
            } else {
                t.button
            },
        );
        let label = if menu {
            Rect::new(r.x, r.y, r.w - 10., r.h)
        } else {
            r
        };
        ui.c.text_center(Font::new(11), label, t.text, name);
        if menu {
            os_ui::kit::triangle(
                ui.c,
                r.right() - 8.,
                r.cy(),
                2.5,
                os_ui::kit::Dir::Down,
                t.text_dim,
            );
        }
    };
    let y = rect.y + 3.;
    let h = rect.h - 6.;
    let tools_end = if rect.w >= 460. {
        for brush in [Brush::Draw, Brush::Points, Brush::Vibrato] {
            button(
                ui,
                54.,
                brush.name(),
                Hit::Command(Command::Brush(brush)),
                "Switch pitch editing tool",
                s.pitch_draw.brush == brush,
                false,
            );
        }
        rect.x + 6. + 3. * 59.
    } else {
        let tool = Rect::new(rect.x + 6., y, 84., h);
        button(
            ui,
            tool.w,
            s.pitch_draw.brush.name(),
            Hit::BrushMenu(tool),
            "Choose freehand drawing, movable points, or vibrato",
            false,
            true,
        );
        tool.right() + 5.
    };
    let bend = Rect::new(tools_end, y, 83., h);
    // The bend menu opens under its chip, to the right of the drawing tools.
    button(
        ui,
        bend.w,
        &format!("Bend ±{}", range(s)),
        Hit::RangeMenu(bend),
        "Limit how far a note can bend in Pitch Draw",
        false,
        true,
    );
    let snap_label = if s.pitch_draw.snap {
        "Snap grid"
    } else {
        "Snap free"
    };
    button(
        ui,
        68.,
        snap_label,
        Hit::Command(Command::Snap),
        "Snap to the Piano Roll time grid and semitones. Hold Alt for free placement.",
        s.pitch_draw.snap,
        false,
    );
    button(
        ui,
        68.,
        "Clear bend",
        Hit::Command(Command::Clear),
        "Clear pitch curves on the selected notes",
        false,
        false,
    );

    let sel = s.piano.selection(s.pattern, piano::channel(s));
    if x + 205. < rect.right()
        && let [index] = sel.as_slice()
        && let Some(note) = piano::notes(s).get(*index)
    {
        let beat = f32::from(s.project.ppq.max(1));
        let label = format!(
            "{}     Velocity {}     Start {:.2}     Length {:.2} beats",
            crate::views::piano::key_name(i32::from(note.key)),
            note.velocity,
            note.position as f32 / beat + 1.,
            note.length as f32 / beat
        );
        ui.c.line(x, y + 2., x, y + h - 2., 1., t.toolbar_edge);
        ui.c.text_clip(
            Font::new(11),
            x + 9.,
            y + 3.,
            rect.right() - x - 14.,
            t.text_dim,
            &label,
        );
    }
}

fn note_x(g: &Geom, n: &Note, at: f32) -> f32 {
    g.x_of(n.position as f32 + n.length as f32 * at)
}

fn key_y(g: &Geom, key: f32) -> f32 {
    let i = g.keys.partition_point(|k| *k as f32 > key);
    let hi = i.saturating_sub(1).min(g.keys.len() - 2);
    let a = g.keys[hi] as f32;
    let b = g.keys[hi + 1] as f32;
    let row = hi as f32 + (a - key) / (a - b);
    g.grid.y + (row + 0.5) * g.row - g.scroll_y
}

fn key_at(g: &Geom, y: f32) -> f32 {
    let index = (y - g.grid.y + g.scroll_y) / g.row - 0.5;
    let i = (index.floor() as usize).min(g.keys.len() - 2);
    let a = g.keys[i] as f32;
    a + (g.keys[i + 1] as f32 - a) * (index - i as f32)
}

fn cast(g: &Geom, n: &Note, p: Point) -> (f32, f32) {
    (note_x(g, n, p.at), key_y(g, f32::from(n.key) + p.value))
}

fn control(at: f32, value: f32) -> Point {
    Point {
        at,
        value,
        smooth: true,
    }
}

pub fn paint_note(
    ui: &mut Ui,
    s: &State,
    g: &Geom,
    index: usize,
    n: &Note,
    selected: bool,
) -> bool {
    if !editable(n) || g.keys.len() < 2 || !g.keys.contains(&i32::from(n.key)) {
        return false;
    }
    let Some(body) = g.note_rect(n) else {
        return true;
    };
    let clip = Rect::new(body.x, g.grid.y, body.w, g.grid.h).intersect(&g.grid);
    if clip.is_empty() {
        return true;
    }
    let preview = s
        .pitch_draw
        .gesture
        .as_ref()
        .filter(|e| e.pattern == s.pattern && e.index == index);
    let curve = preview
        .map(|e| &e.draft)
        .or_else(|| s.project.mpe.get(n.expression));
    let start = n.position as f32;
    let len = n.length as f32;
    let from = ((g.tick_at(clip.x) - start) / len).clamp(0., 1.);
    let to = ((g.tick_at(clip.right()) - start) / len).clamp(from, 1.);
    let value = |at| curve.map_or(0., |e| e.value(0, at));
    let lane = curve.map(|e| e.lanes[0].as_slice()).unwrap_or(&[]);
    let first = lane.partition_point(|p| p.at <= from);
    let last = lane.partition_point(|p| p.at < to);
    let mut path = Vec::with_capacity(last.saturating_sub(first) * 2 + 2);
    path.push(cast(g, n, control(from, value(from))));
    for i in first..last {
        let p = lane[i];
        if i > 0 && !lane[i - 1].smooth {
            path.push(cast(g, n, control(p.at, lane[i - 1].value)));
        }
        path.push(cast(g, n, p));
    }
    // Render the vertical side of a stepped segment ending at the clip boundary.
    let end_value = value(to);
    if let Some(b) = lane.get(last).filter(|b| b.at == to)
        && let Some(a) = last
            .checked_sub(1)
            .and_then(|i| lane.get(i))
            .filter(|a| !a.smooth)
    {
        path.push(cast(g, n, control(b.at, a.value)));
    }
    path.push(cast(g, n, control(to, end_value)));

    let ribbon_width = (g.row * 0.5).clamp(4.5, 16.);
    // Keep the original grab area even where the pitch curve is thin.
    let hit_half = (g.row * 0.43).clamp(3.5, 14.);
    let base_y = key_y(g, f32::from(n.key));
    let color = if n.muted() {
        ui.t.text_dim
    } else {
        ui.t.brand_lit(os_ui::Color::rgb(0x3d9edc))
    };
    let rim = color.mix(
        os_ui::Color::rgb(0xe5f5ff),
        if selected { 0.62 } else { 0.27 },
    );
    let body_color = color.shade(if selected { 0.05 } else { -0.12 });
    let point_color = os_ui::Color::rgb(0xf4cd78);
    ui.c.push_clip(clip);
    if !lane.is_empty() {
        ui.c.line(
            clip.x,
            base_y,
            clip.right(),
            base_y,
            1.,
            color.with_alpha(85),
        );
    }
    ui.c.rounded_polyline(&path, ribbon_width + 1., rim);
    ui.c.rounded_polyline(&path, ribbon_width - 1., body_color);
    if ribbon_width >= 9. && clip.w >= 44. && from <= 0.0001 {
        let label = crate::views::piano::key_name(i32::from(n.key));
        let r = Rect::new(path[0].0 + 7., path[0].1 - 5., (clip.w - 14.).min(34.), 12.);
        ui.c.text_center(Font::new(9), r, ui.t.sunken, &label);
    }
    for w in path.windows(2) {
        let [(x1, y1), (x2, y2)] = [w[0], w[1]];
        // Keep the hit area close to a diagonal even at high horizontal zoom.
        let steps = ((y2 - y1).abs() / (hit_half * 2.)).ceil().clamp(1., 32.) as usize;
        for i in 0..steps {
            let a = i as f32 / steps as f32;
            let b = (i + 1) as f32 / steps as f32;
            let (y0, y_end) = (y1 + (y2 - y1) * a, y1 + (y2 - y1) * b);
            let r = Rect::span(
                x1 + (x2 - x1) * a - 2.,
                x1 + (x2 - x1) * b + 2.,
                y0.min(y_end) - hit_half - 2.,
                y0.max(y_end) + hit_half + 2.,
            )
            .intersect(&clip);
            if !r.is_empty() {
                ui.hit(
                    r,
                    Action::PitchDraw(Hit::Note(index)),
                    "Drag on this note to edit its pitch",
                );
            }
        }
    }
    if selected {
        let mut previous = f32::NEG_INFINITY;
        for (i, &p) in lane
            .iter()
            .enumerate()
            .skip(first.saturating_sub(1))
            .take(last + 1 - first.saturating_sub(1))
        {
            let (x, y) = cast(g, n, p);
            let r = Rect::new(x - 5., y - 5., 10., 10.).intersect(&clip);
            if !r.is_empty() && (x - previous >= 7. || p.at >= 0.9999) {
                previous = x;
                ui.c.fill_circle(x, y, 3., point_color);
                ui.c.stroke_circle(x, y, 3., 1., body_color.shade(-0.35));
                ui.hit(
                    r,
                    Action::PitchDraw(Hit::Point(index, i)),
                    format!("{:+.2} st · right-click to remove", p.value),
                );
            }
        }
    }
    ui.c.pop_clip();
    for (at, x) in [(from, clip.x), (to, clip.right())] {
        if (at <= 0.0001 || at >= 0.9999) && g.grid.x + 3. < x && x < g.grid.right() - 3. {
            let y = key_y(g, f32::from(n.key) + value(at));
            let radius = (hit_half * 0.55).clamp(3.5, 6.);
            ui.c.fill_circle(x, y, radius + 1., color);
            ui.c.fill_circle(x, y, radius - 0.5, os_ui::Color::rgb(0xc5eeff));
        }
    }
    true
}

fn pointer(s: &State, e: &Gesture, (x, y): (f32, f32), alt: bool) -> (f32, f32) {
    let mut tick = e.geom.tick_at(x);
    let mut value = key_at(&e.geom, y) - f32::from(e.note.key);
    if s.pitch_draw.snap && !alt {
        tick = crate::piano_snap::place(s, &e.geom, tick, false) as f32;
        value = value.round();
    }
    let start = e.note.position as f32;
    let len = e.note.length as f32;
    if let Some((a, b)) = e.geom.source_window {
        tick = tick.clamp(a, b);
    }
    let at = ((tick - start) / len).clamp(0., 1.);
    let limit = f32::from(range(s));
    ((at * GRID).round() / GRID, value.clamp(-limit, limit))
}

pub fn begin(app: &mut App, hit: Hit, right: bool) {
    let (index, point) = match hit {
        Hit::Note(i) => (i, None),
        Hit::Point(i, p) => (i, Some(p)),
        _ => return,
    };
    finish(app, false);
    let s = &mut app.state;
    let Some(&note) = piano::notes(s)
        .get(index)
        .filter(|n| editable(n) && Some(n.channel) == piano::channel(s))
    else {
        return;
    };
    let geom = Geom::new(s, crate::views::piano::grid_rect(s));
    if geom.keys.len() < 2 {
        return;
    }
    if !s
        .piano
        .selection(s.pattern, Some(note.channel))
        .contains(&index)
    {
        s.piano.select_notes(s.pattern, note.channel, vec![index]);
    }
    let before = s
        .project
        .mpe
        .get(note.expression)
        .cloned()
        .unwrap_or_default();
    let mut event = Gesture {
        pattern: s.pattern,
        index,
        note,
        geom,
        draft: before.clone(),
        before,
        start: (0., 0.),
        stroke: Vec::new(),
        point,
    };
    event.start = pointer(s, &event, app.mouse, app.alt);
    if right {
        if let Some(p) = point.and_then(|i| event.draft.lanes[0].get(i)) {
            let at = p.at;
            event.draft.remove(0, at);
            s.pitch_draw.gesture = Some(event);
            finish(app, true);
        }
        app.damage_piano();
        return;
    }
    s.pitch_draw.gesture = Some(event);
    if point.is_none() && app.state.pitch_draw.brush == Brush::Points {
        motion(app);
    } else {
        app.damage_piano();
    }
}

fn splice(e: &mut Gesture, points: &[Point], blend: f32) {
    let (Some(first), Some(last)) = (points.first(), points.last()) else {
        return;
    };
    let left = (first.at - blend).max(0.);
    let right = (last.at + blend).min(1.);
    let mut result = Vec::with_capacity(e.before.lanes[0].len() + points.len() + 2);
    result.extend(e.before.lanes[0].iter().copied().filter(|p| p.at < left));
    if left < first.at {
        result.push(control(left, e.before.value(0, left)));
    }
    result.extend_from_slice(points);
    if right > last.at {
        let original = &e.before.lanes[0];
        let smooth = original
            .partition_point(|p| p.at <= right)
            .checked_sub(1)
            .and_then(|i| original.get(i))
            .is_none_or(|p| p.smooth);
        result.push(Point {
            at: right,
            value: e.before.value(0, right),
            smooth,
        });
    }
    result.extend(e.before.lanes[0].iter().copied().filter(|p| p.at > right));
    e.draft.lanes[0] = result;
}

pub fn motion(app: &mut App) -> bool {
    let s = &mut app.state;
    let Some(mut e) = s.pitch_draw.gesture.take() else {
        return false;
    };
    let (at, value) = pointer(s, &e, app.mouse, app.alt);
    let limit = f32::from(range(s));
    if let Some(i) = e.point {
        e.draft = e.before.clone();
        if i < e.draft.lanes[0].len() {
            let old = e.draft.lanes[0].remove(i);
            e.draft.set(0, at, value, old.smooth);
        }
    } else {
        match s.pitch_draw.brush {
            Brush::Draw => {
                let point = control(at, value);
                if e.stroke.is_empty() {
                    e.stroke.push(control(e.start.0, e.start.1));
                }
                if let Some(prior) = e.stroke.last().copied() {
                    let a = prior.at.min(at);
                    let b = prior.at.max(at);
                    e.stroke.retain(|p| p.at < a || p.at > b);
                    if prior.at != at {
                        e.stroke.push(prior);
                    }
                }
                e.stroke.push(point);
                let mut points = e.stroke.clone();
                points.sort_unstable_by(|a, b| a.at.total_cmp(&b.at));
                splice(&mut e, &points, 0.06);
            }
            Brush::Points => {
                e.draft = e.before.clone();
                if e.draft.lanes[0].is_empty() {
                    e.draft.set(0, 0., 0., true);
                    e.draft.set(0, 1., 0., true);
                }
                e.draft.set(0, at, value, true);
            }
            Brush::Vibrato => {
                let a = at.min(e.start.0);
                let b = at.max(e.start.0);
                e.draft = e.before.clone();
                if b - a > 2. / GRID {
                    let depth = (value - e.start.1).abs();
                    let points = (0..=128)
                        .map(|i| {
                            let t = i as f32 / 128.;
                            let at = a + (b - a) * t;
                            let edge = (8. * t.min(1. - t)).min(1.);
                            let value = e.before.value(0, at)
                                + depth * edge * (4. * std::f32::consts::TAU * t).sin();
                            control(at, value.clamp(-limit, limit))
                        })
                        .collect::<Vec<_>>();
                    splice(&mut e, &points, 1. / GRID);
                }
            }
        }
    }
    s.pitch_draw.gesture = Some(e);
    app.damage_piano();
    true
}

pub fn finish(app: &mut App, commit: bool) -> bool {
    let s = &mut app.state;
    let Some(event) = s.pitch_draw.gesture.take() else {
        return false;
    };
    if commit
        && event.before != event.draft
        && s.pattern == event.pattern
        && piano::notes(s).get(event.index) == Some(&event.note)
    {
        let snap = Snap::pattern(s, s.pattern);
        history::record(s, "Draw note pitch", snap);
        write_expression(s, event.index, event.draft);
    }
    app.damage_piano();
    true
}
