//! Playlist chord, tempo and picture controls.
use crate::{
    app::{Action, App, State, Ui, Win},
    playlist::{self as pl, Cmd, EditTarget, Mark, PGeom, PItem, Popup},
};
use os_model::{
    ClipSource, TimeMarker,
    timeline::{Chord, TempoMap, TempoPoint, Timeline, Video},
};
use os_ui::{Font, Rect};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

mod chords;
pub(crate) mod video;

pub const TEMPO: &str = "visible_playlist-tempo-track";
pub const CHORDS: &str = "visible_playlist-chord-track";
pub const VIDEO: &str = "visible_playlist-video-track";
pub const TRACKS_MENU: &str = "visible_playlist-timeline-tracks-menu";
pub const MARKERS: &str = "visible_playlist-marker-timebase";

const LANES: [Lane; 4] = [Lane::Tempo, Lane::Chords, Lane::Video, Lane::Visuals];
static EMPTY: Timeline = Timeline {
    tempo: Vec::new(),
    chords: Vec::new(),
    videos: Vec::new(),
    visuals: Vec::new(),
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    Tempo,
    Chords,
    Video,
    Visuals,
}

impl Lane {
    fn key(self) -> &'static str {
        match self {
            Self::Tempo => TEMPO,
            Self::Chords => CHORDS,
            Self::Video => VIDEO,
            Self::Visuals => crate::visual_scenes::LANE_VISIBILITY,
        }
    }

    fn preference(self) -> &'static str {
        match self {
            Self::Tempo => "show_tempo_lane",
            Self::Chords => "show_chord_lane",
            Self::Video => "show_video_lane",
            Self::Visuals => "show_visuals_lane",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Tempo => "Tempo track",
            Self::Chords => "Chord chart",
            Self::Video => "Video track",
            Self::Visuals => "Visuals",
        }
    }

    fn height(self) -> f32 {
        match self {
            Self::Tempo => 58.0,
            Self::Chords => 25.0,
            Self::Video => 30.0,
            Self::Visuals => 31.0,
        }
    }

    fn has_data(self, timeline: &Timeline) -> bool {
        match self {
            Self::Tempo => !timeline.tempo.is_empty(),
            Self::Chords => !timeline.chords.is_empty(),
            Self::Video => !timeline.videos.is_empty(),
            Self::Visuals => !timeline.visuals.is_empty(),
        }
    }

    fn shown(self, s: &State) -> bool {
        self.available()
            && crate::settings_file::flag(self.preference(), self.has_data(data(s)))
    }

    fn available(self) -> bool {
        crate::workspace_visibility::shown(self.key()) && (self != Self::Visuals || crate::visual_scenes::available())
    }

    fn show(self, on: bool) {
        crate::settings_file::set_flag(self.preference(), on);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Show(Lane, bool),
    Extract(usize),
    EditTempo(u32),
    MoveTempo(u32),
    Ramp(u32, bool),
    DeleteTempo(u32),
    ClearTempo,
    Warp(bool),
    RulerTime(bool),
    EditChord(u32),
    DeleteChord(u32),
    MarkerBase(usize, bool),
    EditMarkerSeconds(usize),
    ImportVideo,
    ImportVideoAt(f64),
    RelinkVideo(usize),
    SplitVideo(usize, f64),
    MoveVideo(usize),
    RemoveVideo(usize),
    PreviewVideo,
}

#[derive(Clone, Debug)]
pub enum Gesture {
    Tempo(TempoPoint, (f32, f32)),
    Warp {
        tick: u32,
        map: Arc<TempoMap>,
        points: Arc<Vec<TempoPoint>>,
    },
    Chord(u32),
    Visual { start: u32, x: f32, edge: crate::visual_scenes::Edge },
    Video {
        index: usize,
        edge: VideoEdge,
        origin: f32,
        seek: u32,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum VideoEdge {
    Body,
    Left,
    Right,
}

type Extraction = Result<Vec<(u32, u32, Vec<Chord>)>, String>;

pub struct Analysis {
    previous: Vec<Chord>,
    map: TempoMap,
    arrangement: usize,
    clips: Vec<os_model::Clip>,
    rx: mpsc::Receiver<Extraction>,
}

#[derive(Default)]
pub struct Controls {
    pub warp: bool,
    pub last_click: Option<(Lane, u32, Instant)>,
    pub preview: Option<Timeline>,
    pub warp_target: Option<f32>,
    pub warp_failed: bool,
    visual_preview_tick: Option<u32>,
    pub video: video::Player,
    analysis: Option<Analysis>,
}

pub fn data(s: &State) -> &Timeline {
    s.project.arrangement().map_or(&EMPTY, |a| &a.timeline)
}

pub fn marker_x(s: &State, marker: &TimeMarker) -> f64 {
    marker.linear_us.map_or(f64::from(marker.position), |us| {
        let preview = s
            .playlist
            .scoring
            .preview
            .as_ref()
            .unwrap_or_else(|| data(s));
        TempoMap::new(s.project.ppq, s.project.bar().1, s.tempo, &preview.tempo)
            .tick(us as f64 / 1_000_000.0)
    })
}

pub fn height(s: &State) -> f32 {
    LANES
        .into_iter()
        .filter(|lane| lane.shown(s))
        .map(Lane::height)
        .sum()
}

pub fn tracks_menu_available() -> bool {
    LANES
        .into_iter()
        .any(Lane::available)
}

fn timeline_items(s: &State) -> Vec<PItem> {
    let mut items = LANES
        .into_iter()
        .filter(|lane| lane.available())
        .map(|lane| {
            PItem::new(
                lane.label(),
                Cmd::Scoring(Command::Show(lane, !lane.shown(s))),
            )
            .mark(Mark::Check(lane.shown(s)))
        })
        .collect::<Vec<_>>();
    if crate::workspace_visibility::shown(VIDEO) {
        items.push(PItem::new(
            "Import video...",
            Cmd::Scoring(Command::ImportVideo),
        ));
    }
    items
}

pub fn view_menu(s: &State) -> Option<PItem> {
    let items = timeline_items(s);
    (!items.is_empty()).then(|| PItem::sub("Timeline tracks", items))
}

pub fn open_tracks_menu(s: &mut State, x: f32, y: f32) {
    s.playlist.popup = Some(Popup {
        x,
        y,
        items: timeline_items(s),
        sub: None,
        scroll: [0; 2],
        cascade: Vec::new(),
    });
}

fn apply(s: &mut State, name: &'static str, timeline: Timeline) {
    if &timeline == data(s) {
        return;
    }
    pl::arr_mut(s);
    let index = s.project.arrangement_index();
    crate::history::record(
        s,
        name,
        crate::history::Snap::Scoring(index, data(s).clone()),
    );
    pl::arr_mut(s).timeline = timeline;
    s.piano.dirty = true;
}

pub fn show_visuals() { Lane::Visuals.show(true); }

pub(crate) fn update(s: &mut State, name: &'static str, change: impl FnOnce(&mut Timeline)) {
    let mut timeline = data(s).clone();
    change(&mut timeline);
    timeline.normalize();
    apply(s, name, timeline);
}

pub fn time_ruler(s: &State) -> bool {
    crate::settings_file::flag("playlist_ruler_seconds", false)
        && (Lane::Tempo.shown(s) || Lane::Video.shown(s))
}

pub fn timeline_end(s: &State) -> f64 {
    let timeline = data(s);
    let last = timeline
        .tempo
        .last()
        .map_or(0.0, |p| f64::from(p.tick))
        .max(timeline.chords.last().map_or(0.0, |p| f64::from(p.tick)))
        .max(timeline.visuals.last().map_or(0.0, |p| f64::from(p.end)));
    let linear = pl::markers(s)
        .iter()
        .filter_map(|m| m.linear_us)
        .max()
        .unwrap_or(0) as f64
        / 1_000_000.0;
    let media = timeline
        .videos
        .iter()
        .map(Video::end)
        .fold(linear, f64::max);
    if media > 0.0 {
        last.max(s.project.tempo_map(s.tempo).tick(media))
    } else {
        last
    }
}

pub fn paint_seconds(ui: &mut Ui, s: &State, g: &PGeom, ruler: Rect) {
    let map = s.project.tempo_map(s.tempo);
    let from = map.seconds(f64::from(g.tick_at(ruler.x).max(0.0)));
    let to = map.seconds(f64::from(g.tick_at(ruler.right()).max(0.0)));
    let step = [
        0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 300.0, 900.0, 3600.0,
    ]
    .into_iter()
    .find(|step| *step >= (to - from) * 74.0 / f64::from(ruler.w))
    .unwrap_or(7200.0);
    for i in ((from / step).floor() as u64)
        ..=((to / step).ceil() as u64).min((from / step).floor() as u64 + 200)
    {
        let seconds = i as f64 * step;
        let x = g.x64(map.tick(seconds)) as f32;
        let millis = (seconds * 1000.0).round() as u64;
        let label = format!(
            "{:02}:{:02}.{:03}",
            millis / 60_000,
            millis / 1000 % 60,
            millis % 1000
        );
        ui.c.text(
            Font::new(12),
            x + 2.0,
            ruler.bottom() - 16.0,
            ui.t.ruler_text,
            &label,
        );
    }
}

pub fn paint(ui: &mut Ui, s: &State, g: &PGeom, top: f32, header: Rect) {
    if height(s) == 0.0 {
        return;
    }
    let timeline = s
        .playlist
        .scoring
        .preview
        .as_ref()
        .unwrap_or_else(|| data(s));
    let map = TempoMap::new(s.project.ppq, s.project.bar().1, s.tempo, &timeline.tempo);
    let mut y = top;
    for lane in LANES.into_iter().filter(|l| l.shown(s)) {
        let r = Rect::new(g.grid.x, y, g.grid.w, lane.height());
        let h = Rect::new(header.x, y, header.w, r.h);
        y += r.h;
        ui.c.fill_rect(h, ui.t.track_header);
        ui.c.fill_rect(r, ui.t.ruler.shade(0.06));
        ui.c.text_clip(
            Font::new(12).bold(),
            h.x + 6.0,
            h.y + 4.0,
            h.w - 10.0,
            ui.t.text,
            lane.label(),
        );
        ui.hit(
            h,
            Action::ScoringLane(lane, true),
            "Click for track options",
        );
        ui.hit(
            r,
            Action::ScoringLane(lane, false),
            match lane {
                Lane::Tempo => {
                    "Double-click to enter tempo; drag a point; right-click for Time Warp"
                }
                Lane::Chords => "Double-click to add or edit a chord; drag a chord to move it",
                Lane::Video => "Drag a video to move; drag its edges to trim; use Slice or right-click to split",
                Lane::Visuals => "Right-click to add a scene; drag its center or edges; double-click to edit",
            },
        );
        ui.c.push_clip(r);
        match lane {
            Lane::Visuals => crate::visual_scenes::paint_lane(ui, s, g, r, &timeline.visuals),
            Lane::Tempo => {
                let left = f64::from(g.tick_at(r.x).max(0.0));
                let right = f64::from(g.tick_at(r.right()).max(0.0));
                let mut last_x = r.x;
                let mut last_y = tempo_y(r, map.bpm(left));
                for point in timeline
                    .tempo
                    .iter()
                    .filter(|p| f64::from(p.tick) >= left && f64::from(p.tick) <= right)
                {
                    let x = g.x_of(point.tick as f32);
                    let before = tempo_y(r, map.bpm((f64::from(point.tick) - 1e-5).max(0.0)));
                    let py = tempo_y(r, point.bpm);
                    ui.c.line(last_x, last_y, x, before, 1.5, ui.t.accent);
                    ui.c.line(x, before, x, py, 1.0, ui.t.accent);
                    ui.c.fill_circle(x, py, 3.5, ui.t.accent);
                    ui.c.text_clip(
                        Font::new(11),
                        x + 5.0,
                        (py - 15.0).max(r.y + 1.0),
                        56.0,
                        ui.t.text,
                        &format!("{:.1}", point.bpm),
                    );
                    (last_x, last_y) = (x, py);
                }
                ui.c.line(
                    last_x,
                    last_y,
                    r.right(),
                    tempo_y(r, map.bpm(right)),
                    1.5,
                    ui.t.accent,
                );
                if let Some(x) = s.playlist.scoring.warp_target {
                    ui.c.line(
                        x,
                        r.y,
                        x,
                        r.bottom(),
                        2.0,
                        if s.playlist.scoring.warp_failed {
                            os_ui::Color::rgb(0xd5746a)
                        } else {
                            ui.t.playhead
                        },
                    );
                }
                if s.playlist.scoring.warp {
                    ui.c.text_clip(
                        Font::new(11),
                        h.x + 6.0,
                        h.y + 24.0,
                        h.w - 10.0,
                        ui.t.accent,
                        "Time Warp on",
                    );
                }
            }
            Lane::Chords => {
                if timeline.chords.is_empty() {
                    ui.c.text_clip(
                        Font::new(12),
                        r.x + 6.0,
                        r.y + 5.0,
                        r.w - 12.0,
                        ui.t.text_dim,
                        "Double-click to add a chord, or choose Extract Chords from a clip.",
                    );
                }
                for (i, chord) in timeline.chords.iter().enumerate() {
                    let x = g.x_of(chord.tick as f32);
                    let end = timeline
                        .chords
                        .get(i + 1)
                        .map_or(r.right(), |next| g.x_of(next.tick as f32));
                    let cell =
                        Rect::new(x, r.y + 2.0, (end - x - 1.0).max(0.0), r.h - 4.0).intersect(&r);
                    if cell.w < 3.0 {
                        continue;
                    }
                    ui.c.fill_rrect(cell, 3.0, ui.t.accent.with_alpha(65));
                    let root_len =
                        chord
                            .symbol
                            .char_indices()
                            .nth(1)
                            .map_or(chord.symbol.len(), |(i, c)| {
                                if matches!(c, '#' | 'b' | '♯' | '♭') {
                                    i + c.len_utf8()
                                } else {
                                    i
                                }
                            });
                    let symbol = if cell.w < 42.0 {
                        &chord.symbol[..root_len]
                    } else {
                        &chord.symbol
                    };
                    ui.c.text_clip(
                        Font::new(13).bold(),
                        cell.x + 5.0,
                        cell.y + 2.0,
                        cell.w - 8.0,
                        ui.t.text,
                        symbol,
                    );
                }
            }
            Lane::Video => {
                for v in &timeline.videos {
                    let x = g.x64(map.tick(v.start)) as f32;
                    let end = g.x64(map.tick(v.end())) as f32;
                    let cell = Rect::new(x, r.y + 3.0, (end - x).max(1.0), r.h - 6.0);
                    if cell.intersect(&r).w <= 0.0 {
                        continue;
                    }
                    ui.c.fill_rrect(cell, 3.0, ui.t.accent.with_alpha(65));
                    ui.c.stroke_rrect(cell, 3.0, 1.0, ui.t.accent.with_alpha(160));
                    if cell.w >= 14.0 {
                        for edge in [cell.x + 4.0, cell.right() - 4.0] {
                            ui.c.line(
                                edge,
                                cell.y + 7.0,
                                edge,
                                cell.bottom() - 7.0,
                                1.0,
                                ui.t.accent,
                            );
                        }
                    }
                    let cell = cell.intersect(&r);
                    let name = std::path::Path::new(&v.path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Video");
                    ui.c.text_clip(
                        Font::new(12),
                        cell.x + 8.0,
                        cell.y + 3.0,
                        (cell.w - 16.0).max(0.0),
                        ui.t.text,
                        name,
                    );
                }
                if timeline.videos.is_empty() {
                    ui.c.text_clip(
                        Font::new(12),
                        r.x + 6.0,
                        r.y + 7.0,
                        r.w - 12.0,
                        ui.t.text_dim,
                        "Right-click to import a reference video.",
                    );
                }
            }
        }
        ui.c.pop_clip();
        ui.c.fill_rect(
            Rect::new(h.x, y - 1.0, r.right() - h.x, 1.0),
            ui.t.text_dim.with_alpha(120),
        );
    }
    video::paint(ui, s, g.grid);
}

fn tempo_y(r: Rect, bpm: f64) -> f32 {
    r.bottom()
        - 5.0
        - ((bpm / 10.0).ln() / (522.0_f64 / 10.0).ln()) as f32 * (Lane::Tempo.height() - 10.0)
}

fn envelope_at(s: &State, g: &PGeom, lane: Lane, x: f32) -> Option<u32> {
    match lane {
        Lane::Tempo => data(s)
            .tempo
            .iter()
            .find(|p| (x - g.x_of(p.tick as f32)).abs() <= 6.0)
            .map(|p| p.tick),
        Lane::Chords => data(s)
            .chords
            .iter()
            .enumerate()
            .rev()
            .find(|(i, c)| {
                let start = g.x_of(c.tick as f32);
                let end = data(s)
                    .chords
                    .get(i + 1)
                    .map_or(g.grid.right(), |next| g.x_of(next.tick as f32));
                x >= start && x < end.min(start + 78.0)
            })
            .map(|(_, c)| c.tick),
        Lane::Video | Lane::Visuals => None,
    }
}

fn video_hit(s: &State, g: &PGeom, x: f32) -> Option<(usize, f64, VideoEdge)> {
    let map = s.project.tempo_map(s.tempo);
    let seconds = map.seconds(f64::from(g.tick_at(x).max(0.0)));
    data(s)
        .videos
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, video)| {
            let left = g.x64(map.tick(video.start)) as f32;
            let right = g.x64(map.tick(video.end())) as f32;
            if x < left || x > right {
                return None;
            }
            let grip = ((right - left) / 3.0).min(6.0);
            let edge = if x <= left + grip {
                VideoEdge::Left
            } else if x >= right - grip {
                VideoEdge::Right
            } else {
                VideoEdge::Body
            };
            Some((i, seconds, edge))
        })
}

pub fn video_trim_cursor(app: &App) -> bool {
    app.pl_grid().is_some_and(|r| {
        let g = PGeom::new(&app.state, r);
        app.state.playlist.tool != pl::PlTool::Slice
            && matches!(
                video_hit(&app.state, &g, app.mouse.0),
                Some((_, _, VideoEdge::Left | VideoEdge::Right))
            )
    })
}

fn seek(app: &mut App, tick: u32) {
    app.state.playlist.pos = tick;
    app.state.song_mode = true;
    if app.transport.playing() {
        app.play_song_from(tick);
    }
}

pub fn press(app: &mut App, lane: Lane, header: bool, right: bool) {
    let Some(grid) = app.pl_grid() else {
        return;
    };
    let (x, y) = app.mouse;
    let g = PGeom::new(&app.state, grid);
    let s = &mut app.state;
    let tick = if header {
        s.playlist.pos
    } else {
        pl::snap_round(s, &g, g.tick_at(x), app.alt)
    };
    if lane == Lane::Visuals {
        let top = grid.y - Lane::Visuals.height();
        return crate::visual_scenes::lane_press(app, &g, tick, top, header, right);
    }
    let hit = (!header).then(|| envelope_at(s, &g, lane, x)).flatten();
    let video_hit = (lane == Lane::Video && !header)
        .then(|| video_hit(s, &g, x))
        .flatten();
    let command = |label, cmd| PItem::new(label, Cmd::Scoring(cmd));
    if right || header {
        let mut items = match lane {
            Lane::Visuals => unreachable!(),
            Lane::Tempo => {
                let mut items = vec![
                    command(
                        "Add or edit tempo...",
                        Command::EditTempo(hit.unwrap_or(tick)),
                    ),
                    command("Time Warp", Command::Warp(!s.playlist.scoring.warp))
                        .mark(Mark::Check(s.playlist.scoring.warp)),
                    command("Ruler in elapsed time", Command::RulerTime(!time_ruler(s)))
                        .mark(Mark::Check(time_ruler(s))),
                ];
                if let Some(p) = data(s).tempo.iter().find(|p| Some(p.tick) == hit) {
                    if p.tick != 0 {
                        items.push(command(
                            "Move tempo point (ticks)...",
                            Command::MoveTempo(p.tick),
                        ));
                    }
                    items.push(
                        command("Ramp to next point", Command::Ramp(p.tick, !p.ramp))
                            .mark(Mark::Check(p.ramp)),
                    );
                    items.push(command("Delete tempo point", Command::DeleteTempo(p.tick)));
                }
                if !data(s).tempo.is_empty() {
                    items.push(command("Clear tempo track", Command::ClearTempo));
                }
                items
            }
            Lane::Chords => {
                let mut items = vec![command(
                    "Add or edit chord...",
                    Command::EditChord(hit.unwrap_or(tick)),
                )];
                if let Some(t) = hit {
                    items.push(command("Delete chord", Command::DeleteChord(t)));
                }
                items
            }
            Lane::Video => {
                let mut items = vec![
                    command(
                        "Import video...",
                        if header {
                            Command::ImportVideo
                        } else {
                            let time = s
                                .project
                                .tempo_map(s.tempo)
                                .seconds(f64::from(g.tick_at(x).max(0.0)));
                            Command::ImportVideoAt(time)
                        },
                    ),
                    command("Ruler in elapsed time", Command::RulerTime(!time_ruler(s)))
                        .mark(Mark::Check(time_ruler(s))),
                ];
                if !data(s).videos.is_empty() {
                    items.push(
                        command("Video preview", Command::PreviewVideo)
                            .mark(Mark::Check(s.playlist.scoring.video.open)),
                    );
                }
                if let Some((i, seconds, _)) = video_hit {
                    items.extend([
                        command("Split here", Command::SplitVideo(i, seconds)),
                        command("Set video start time...", Command::MoveVideo(i)),
                        command("Relink video file...", Command::RelinkVideo(i)),
                        command("Remove video clip", Command::RemoveVideo(i)),
                    ]);
                }
                items
            }
        };
        items.push(command("Hide track", Command::Show(lane, false)));
        s.playlist.popup = Some(Popup {
            x,
            y,
            items,
            sub: None,
            scroll: [0; 2],
            cascade: Vec::new(),
        });
    } else if lane == Lane::Video {
        let seek_tick = g.tick_at(x).round().max(0.0) as u32;
        if let Some((index, seconds, edge)) = video_hit {
            if s.playlist.tool == pl::PlTool::Slice {
                video::split(s, index, seconds);
            } else {
                app.drag = Some(crate::app::Drag::Scoring(Gesture::Video {
                    index,
                    edge,
                    origin: x,
                    seek: seek_tick,
                }));
            }
        } else {
            seek(app, seek_tick);
        }
    } else {
        let origin = hit.unwrap_or(tick);
        let now = Instant::now();
        let double = s.playlist.scoring.last_click.is_some_and(|(l, t, when)| {
            l == lane && t == origin && now.duration_since(when) < Duration::from_millis(430)
        });
        s.playlist.scoring.last_click = (!double).then_some((lane, origin, now));
        if lane == Lane::Tempo && s.playlist.scoring.warp {
            begin_warp(app, origin);
        } else if double {
            pl::start_edit(
                s,
                if lane == Lane::Tempo {
                    EditTarget::TimelineTempo(origin)
                } else {
                    EditTarget::TimelineChord(origin)
                },
                (x, y),
            );
        } else if lane == Lane::Tempo {
            if let Some(point) = data(s).tempo.iter().find(|p| Some(p.tick) == hit).copied() {
                app.drag = Some(crate::app::Drag::Scoring(Gesture::Tempo(point, (x, y))));
            }
        } else {
            if lane == Lane::Chords && hit.is_some() {
                app.drag = Some(crate::app::Drag::Scoring(Gesture::Chord(origin)));
            }
            seek(app, origin);
        }
    }
    app.invalidate();
}

pub fn ruler_warp(app: &mut App, grid: Rect) -> bool {
    if !app.state.playlist.scoring.warp || !Lane::Tempo.shown(&app.state) {
        return false;
    }
    let g = PGeom::new(&app.state, grid);
    let tick = pl::snap_round(&app.state, &g, g.tick_at(app.mouse.0), app.alt);
    begin_warp(app, tick);
    true
}

fn begin_warp(app: &mut App, tick: u32) {
    if tick == 0 {
        app.state.message = Some("Choose a beat after the song start to warp.".into());
    } else if app.transport.playing() || app.audio_rec.is_some() {
        app.state.message = Some("Stop playback and recording before using Time Warp.".into());
    } else {
        let s = &app.state;
        app.drag = Some(crate::app::Drag::Scoring(Gesture::Warp {
            tick,
            map: Arc::new(s.project.tempo_map(s.tempo)),
            points: Arc::new(data(s).tempo.clone()),
        }));
    }
    app.invalidate();
}

pub fn drag(app: &mut App, gesture: Gesture) {
    let Some(grid) = app.pl_grid() else {
        return;
    };
    let g = PGeom::new(&app.state, grid);
    let s = &mut app.state;
    let mut timeline = data(s).clone();
    let x = app.mouse.0;
    let tick = pl::snap_round(s, &g, g.tick_at(x), app.alt);
    match gesture {
        Gesture::Visual { start, x: origin, edge } => {
            let free = app.alt || matches!(edge, crate::visual_scenes::Edge::FadeIn | crate::visual_scenes::Edge::FadeOut);
            let delta = pl::snap_delta(s, &g, g.tick_at(x) - g.tick_at(origin), free);
            s.playlist.scoring.visual_preview_tick = crate::visual_scenes::drag(&mut timeline.visuals, start, delta, edge);
        }
        Gesture::Tempo(point, (old_x, old_y)) => {
            if (x - old_x).hypot(app.mouse.1 - old_y) < 3.0 {
                s.playlist.scoring.preview = None;
                app.invalidate();
                return;
            }
            let bpm = (point.bpm * (f64::from(old_y - app.mouse.1) * (52.2_f64).ln() / 48.0).exp())
                .clamp(10.0, 522.0);
            let to = if point.tick == 0 || (x - old_x).abs() < 4.0 {
                point.tick
            } else {
                tick.max(1)
            };
            if to != point.tick && timeline.tempo.iter().any(|p| p.tick == to) {
                s.playlist.scoring.preview = None;
                app.invalidate();
                return;
            }
            if let Some(p) = timeline.tempo.iter_mut().find(|p| p.tick == point.tick) {
                p.bpm = bpm;
                p.tick = to;
            }
        }
        Gesture::Chord(from) => {
            if from == tick {
                s.playlist.scoring.preview = None;
                app.invalidate();
                return;
            }
            if timeline.chords.iter().any(|p| p.tick == tick) {
                s.playlist.scoring.preview = None;
                app.invalidate();
                return;
            }
            if let Some(p) = timeline.chords.iter_mut().find(|p| p.tick == from) {
                p.tick = tick;
            }
        }
        Gesture::Video {
            index,
            edge,
            origin,
            ..
        } => {
            if (x - origin).abs() < 3.0 {
                s.playlist.scoring.preview = None;
                app.invalidate();
                return;
            }
            let map = s.project.tempo_map(s.tempo);
            let seconds =
                map.seconds(f64::from(g.tick_at(x))) - map.seconds(f64::from(g.tick_at(origin)));
            if !video::adjust(&mut timeline.videos, index, edge, seconds) {
                s.playlist.scoring.preview = None;
                app.invalidate();
                return;
            }
        }
        Gesture::Warp { tick, map, points } => {
            let mut seconds = map.seconds(f64::from(g.tick_at(x).max(0.0)));
            if !app.alt {
                if let Some(us) = pl::markers(s)
                    .iter()
                    .filter_map(|m| m.linear_us)
                    .find(|us| {
                        (g.x64(map.tick(*us as f64 / 1_000_000.0)) - f64::from(x)).abs() < 8.0
                    })
                {
                    seconds = us as f64 / 1_000_000.0;
                } else if let Some(v) = timeline
                    .videos
                    .iter()
                    .find(|v| v.source_time(seconds).is_some())
                {
                    seconds =
                        v.start + video::snap(seconds - v.start + v.source_in, v.fps) - v.source_in;
                }
            }
            s.playlist.scoring.warp_target = Some(g.x64(map.tick(seconds)) as f32);
            if (seconds - map.seconds(f64::from(tick))).abs() < 0.00001 {
                s.playlist.scoring.preview = None;
                s.playlist.scoring.warp_failed = false;
                app.invalidate();
                return;
            }
            if let Some(next) = map.warp(&points, tick, seconds) {
                timeline.tempo = next;
            } else {
                s.playlist.scoring.preview = None;
                s.playlist.scoring.warp_failed = true;
                app.invalidate();
                return;
            }
        }
    }
    timeline.normalize();
    s.playlist.scoring.warp_failed = false;
    s.playlist.scoring.preview = Some(timeline);
    app.invalidate();
}

pub fn finish(app: &mut App, commit: bool) -> bool {
    if !matches!(app.drag, Some(crate::app::Drag::Scoring(_))) {
        return false;
    }
    let label = match app.drag.take() {
        Some(crate::app::Drag::Scoring(Gesture::Warp { .. })) => "Time Warp",
        Some(crate::app::Drag::Scoring(Gesture::Chord(_))) => "Move chord",
        Some(crate::app::Drag::Scoring(Gesture::Visual { edge: crate::visual_scenes::Edge::FadeIn | crate::visual_scenes::Edge::FadeOut, .. })) => "Edit visual fade",
        Some(crate::app::Drag::Scoring(Gesture::Visual { .. })) => "Move or trim visual scene",
        Some(crate::app::Drag::Scoring(Gesture::Video {
            edge,
            seek: tick,
            origin,
            ..
        })) => {
            if commit && (app.mouse.0 - origin).abs() < 3.0 {
                seek(app, tick);
            }
            match edge {
                VideoEdge::Body => "Move video clip",
                VideoEdge::Left | VideoEdge::Right => "Trim video clip",
            }
        }
        _ => "Change tempo point",
    };
    app.state.playlist.scoring.warp_target = None;
    if std::mem::take(&mut app.state.playlist.scoring.warp_failed) && commit {
        app.state.message = Some(
            "That cue would put tempo outside 10 to 522 BPM or before the previous anchor.".into(),
        );
    }
    let visual_tick = app.state.playlist.scoring.visual_preview_tick.take();
    if let Some(timeline) = app.state.playlist.scoring.preview.take().filter(|_| commit) {
        apply(&mut app.state, label, timeline);
        if let Some(tick) = visual_tick { crate::visual_scenes::moved(&mut app.state, tick); }
    }
    app.invalidate();
    true
}

pub fn run(app: &mut App, cmd: Command) -> Option<String> {
    let s = &mut app.state;
    s.playlist.popup = None;
    let at = app.mouse;
    match cmd {
        Command::Show(lane, visible) => {
            lane.show(visible);
            if lane == Lane::Tempo && !visible {
                s.playlist.scoring.warp = false;
            }
        }
        Command::RulerTime(enabled) => {
            crate::settings_file::set_flag("playlist_ruler_seconds", enabled)
        }
        Command::Warp(on) => {
            s.playlist.scoring.warp = on;
            if on {
                return Some(
                    "Drag a beat in the Tempo track to a time-locked marker or video frame.".into(),
                );
            }
        }
        Command::EditTempo(t) => pl::start_edit(s, EditTarget::TimelineTempo(t), at),
        Command::MoveTempo(t) => pl::start_edit(s, EditTarget::TimelineTempoPosition(t), at),
        Command::Ramp(t, ramp) => update(s, "Tempo curve", |map| {
            if let Some(p) = map.tempo.iter_mut().find(|p| p.tick == t) {
                p.ramp = ramp;
                p.ramp_end = None;
            }
        }),
        Command::DeleteTempo(t) => update(s, "Delete tempo point", |map| {
            map.tempo.retain(|p| p.tick != t)
        }),
        Command::ClearTempo => update(s, "Clear tempo track", |map| map.tempo.clear()),
        Command::EditChord(t) => pl::start_edit(s, EditTarget::TimelineChord(t), at),
        Command::DeleteChord(t) => {
            update(s, "Delete chord", |map| map.chords.retain(|p| p.tick != t))
        }
        Command::MarkerBase(i, linear) => {
            let map = s.project.tempo_map(s.tempo);
            let m = pl::markers(s).get(i)?;
            if m.kind != TimeMarker::PLAIN || m.linear_us.is_some() == linear {
                return None;
            }
            let tick = m.song_tick(&map);
            let us = (map.seconds(f64::from(tick)) * 1_000_000.0).round() as u64;
            pl::snapshot_as(s, "Marker timebase");
            let m = &mut pl::arr_mut(s).markers[i];
            m.position = tick;
            m.linear_us = if linear { Some(us) } else { None };
            pl::sort_markers(s);
            s.piano.dirty = true;
        }
        Command::EditMarkerSeconds(i) => {
            pl::start_edit(s, EditTarget::TimelineMarkerSeconds(i), at)
        }
        Command::Extract(i) => return chords::start(app, i),
        Command::ImportVideo | Command::ImportVideoAt(_) | Command::RelinkVideo(_) => {
            use crate::filedlg::{FileDlg, Mode};
            let (title, then) = match cmd {
                Command::RelinkVideo(i) => {
                    let clip = data(s).videos.get(i)?;
                    (
                        "Relink reference video",
                        crate::dialog::Then::RelinkVideo(clip.clone()),
                    )
                }
                Command::ImportVideoAt(seconds) => (
                    "Import reference video",
                    crate::dialog::Then::ReferenceVideo(seconds),
                ),
                _ => {
                    let seconds = s
                        .project
                        .tempo_map(s.tempo)
                        .seconds(f64::from(s.playlist.pos));
                    (
                        "Import reference video",
                        crate::dialog::Then::ReferenceVideo(seconds),
                    )
                }
            };
            app.file_dialog(
                FileDlg::new(Mode::Open, title, "video", then).filter("Video", video::EXTENSIONS),
            );
        }
        Command::MoveVideo(i) => pl::start_edit(s, EditTarget::TimelineVideoStart(i), at),
        Command::SplitVideo(i, seconds) => video::split(s, i, seconds),
        Command::RemoveVideo(i) => update(s, "Remove video clip", |map| {
            if i < map.videos.len() {
                map.videos.remove(i);
            }
        }),
        Command::PreviewVideo => s.playlist.scoring.video.open = !s.playlist.scoring.video.open,
    }
    app.invalidate();
    None
}

pub fn edit_text(s: &State, target: EditTarget) -> String {
    match target {
        EditTarget::TimelineTempo(t) => {
            format!("{:.3}", s.project.tempo_map(s.tempo).bpm(f64::from(t)))
        }
        EditTarget::TimelineChord(t) => data(s)
            .chords
            .iter()
            .find(|p| p.tick == t)
            .map(|p| p.symbol.clone())
            .unwrap_or_default(),
        EditTarget::TimelineTempoPosition(t) => t.to_string(),
        EditTarget::TimelineMarkerSeconds(i) => pl::markers(s)
            .get(i)
            .and_then(|m| m.linear_us)
            .map(|us| format!("{:.6}", us as f64 / 1_000_000.0))
            .unwrap_or_default(),
        EditTarget::TimelineVideoStart(i) => data(s)
            .videos
            .get(i)
            .map(|v| format!("{:.6}", v.start))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

pub fn edit(s: &mut State, target: EditTarget, text: &str) {
    match target {
        EditTarget::TimelineTempo(tick) => {
            let Ok(bpm) = text.parse::<f64>() else {
                s.message = Some("Enter a tempo in BPM.".into());
                return;
            };
            if !(10.0..=522.0).contains(&bpm) {
                s.message = Some("Enter a tempo from 10 to 522 BPM.".into());
                return;
            }
            update(s, "Set tempo point", |map| {
                if let Some(p) = map.tempo.iter_mut().find(|p| p.tick == tick) {
                    p.bpm = bpm;
                } else {
                    map.tempo.push(TempoPoint {
                        tick,
                        bpm,
                        ramp: false,
                        ramp_end: None,
                    });
                }
            });
        }
        EditTarget::TimelineChord(tick) => {
            if text.is_empty() {
                return;
            }
            update(s, "Edit chord", |map| {
                map.chords.retain(|p| p.tick != tick);
                map.chords.push(Chord {
                    tick,
                    symbol: text.chars().take(40).collect(),
                });
            });
        }
        EditTarget::TimelineTempoPosition(tick) => {
            let Ok(to) = text.parse::<u32>() else {
                s.message = Some("Enter a timeline position in ticks.".into());
                return;
            };
            update(s, "Move timeline point", |map| {
                if matches!(target, EditTarget::TimelineTempoPosition(_))
                    && tick != 0
                    && !map.tempo.iter().any(|p| p.tick == to)
                {
                    if let Some(p) = map.tempo.iter_mut().find(|p| p.tick == tick) {
                        p.tick = to;
                    }
                }
            });
        }
        EditTarget::TimelineMarkerSeconds(i) => {
            let Ok(seconds) = text.parse::<f64>() else {
                s.message = Some("Enter the cue time in seconds.".into());
                return;
            };
            if !(0.0..=86400.0).contains(&seconds) {
                return;
            }
            if !pl::markers(s).get(i).is_some_and(|m| m.linear_us.is_some()) {
                return;
            }
            let tick = s.project.tempo_map(s.tempo).tick(seconds).round() as u32;
            pl::snapshot_as(s, "Set cue time");
            let marker = &mut pl::arr_mut(s).markers[i];
            marker.linear_us = Some((seconds * 1_000_000.0).round() as u64);
            marker.position = tick;
            pl::sort_markers(s);
            s.piano.dirty = true;
        }
        EditTarget::TimelineVideoStart(i) => {
            let Ok(start) = text.parse::<f64>() else {
                s.message = Some("Enter the video start in seconds.".into());
                return;
            };
            if !(0.0..=86400.0).contains(&start) {
                return;
            }
            let mut timeline = data(s).clone();
            if let Some(v) = timeline.videos.get(i) {
                let delta = start - v.start;
                if video::adjust(&mut timeline.videos, i, VideoEdge::Body, delta) {
                    timeline.normalize();
                    apply(s, "Set video start", timeline);
                } else {
                    s.message = Some("Choose an open position for this video clip.".into());
                }
            }
        }
        _ => {}
    }
}

pub fn tick(app: &mut App) {
    chords::poll(app);
    video::poll(app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use os_model::Clip;

    #[test]
    fn scoring_workflow_preserves_picture_and_linear_cue_on_warp_and_reload() {
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state.tempo = 120.0;
        app.state.project.tempo = Some(120.0);
        let s = &mut app.state;
        pl::arr_mut(s).clips = vec![Clip::new(0, 768, 0, ClipSource::Pattern(1))];
        pl::add_marker(s, TimeMarker::new(576, "Cut"));
        pl::add_marker(s, TimeMarker::new(384, "Music cue"));
        let cue = pl::markers(s).iter().position(|m| m.name == "Cut").unwrap();
        run(&mut app, Command::MarkerBase(cue, true));
        let s = &mut app.state;
        edit(s, EditTarget::TimelineChord(384), "Dm7");
        update(s, "Import video", |map| {
            map.videos.push(Video {
                path: "E:/film.mp4".into(),
                start: 3.0,
                duration: 10.0,
                fps: 24.0,
                source_in: 0.0,
                source_duration: Some(10.0),
            });
        });
        let before = s.project.tempo_map(s.tempo);
        let warped = before.warp(&[], 384, 3.0).unwrap();
        update(s, "Time Warp", |timeline| timeline.tempo = warped);
        let map = s.project.tempo_map(s.tempo);
        let linear = pl::markers(s).iter().find(|m| m.name == "Cut").unwrap();
        assert_eq!(linear.linear_us, Some(3_000_000));
        assert!((marker_x(s, linear) - 384.0).abs() < 1e-7);
        assert!((map.seconds(384.0) - 3.0).abs() < 1e-9);
        assert_eq!(data(s).chords[0].tick, 384);
        assert_eq!(data(s).videos[0].start, 3.0);

        let saved = os_flp::project::create(&s.project)
            .unwrap()
            .to_bytes()
            .unwrap();
        let loaded = os_flp::project::load(&os_flp::FlpFile::parse(&saved).unwrap()).unwrap();
        assert_eq!(loaded.arrangement().unwrap().timeline, *data(s));
        assert_eq!(
            loaded
                .arrangement()
                .unwrap()
                .markers
                .iter()
                .find(|m| m.name == "Cut")
                .unwrap()
                .linear_us,
            Some(3_000_000)
        );
        assert!(crate::history::undo(s).is_some());
        assert!(data(s).tempo.is_empty());
        assert!(crate::history::redo(s).is_some());
        assert!((s.project.tempo_map(s.tempo).seconds(384.0) - 3.0).abs() < 1e-9);
        s.project.set_ppq(192);
        assert!((s.project.tempo_map(s.tempo).seconds(768.0) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn video_edits_round_trip_with_source_offsets_and_undo() {
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        let s = &mut app.state;
        let legacy: Timeline = serde_json::from_str(
            r#"{"video":{"path":"E:/film.mp4","start":3.0,"duration":10.0,"fps":24.0}}"#,
        )
        .unwrap();
        update(s, "Import old video", |timeline| *timeline = legacy);
        video::split(s, 0, 7.0);
        assert_eq!(data(s).videos[0].duration, 4.0);
        assert_eq!(data(s).videos[1].source_time(8.0), Some(5.0));

        let mut timeline = data(s).clone();
        assert!(video::adjust(&mut timeline.videos, 1, VideoEdge::Body, 3.0));
        assert!(video::adjust(
            &mut timeline.videos,
            1,
            VideoEdge::Left,
            -2.0
        ));
        assert!(video::adjust(
            &mut timeline.videos,
            1,
            VideoEdge::Right,
            -2.0
        ));
        assert_eq!(timeline.videos[1].start, 8.0);
        assert_eq!(timeline.videos[1].source_in, 2.0);
        assert_eq!(timeline.videos[1].duration, 6.0);
        assert_eq!(timeline.videos[1].source_time(9.0), Some(3.0));
        assert_eq!(timeline.videos[1].source_time(14.0), None);
        assert!(!video::adjust(
            &mut timeline.videos,
            1,
            VideoEdge::Body,
            -2.0
        ));
        apply(s, "Edit video clip", timeline);

        let saved = os_flp::project::create(&s.project)
            .unwrap()
            .to_bytes()
            .unwrap();
        let loaded = os_flp::project::load(&os_flp::FlpFile::parse(&saved).unwrap()).unwrap();
        assert_eq!(loaded.arrangement().unwrap().timeline, *data(s));
        assert!(crate::history::undo(s).is_some());
        assert_eq!(data(s).videos[1].start, 7.0);
        assert!(crate::history::undo(s).is_some());
        assert_eq!(data(s).videos.len(), 1);
        assert!(crate::history::redo(s).is_some());
        assert_eq!(data(s).videos.len(), 2);
    }

    #[test]
    fn scoring_lane_clicks_and_extraction_use_playlist_actions() {
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state.z = vec![Win::Playlist];
        app.state.playlist_bar_w = 192.0;
        let s = &mut app.state;
        pl::arr_mut(s).clips = vec![os_model::Clip::new(0, 768, 0, ClipSource::Pattern(1))];
        for lane in [Lane::Tempo, Lane::Chords, Lane::Video] {
            run(&mut app, Command::Show(lane, true));
        }
        let mut canvas = os_ui::Canvas::new(1600, 1000, 1.0).unwrap();
        app.paint(&mut canvas, None);
        let tracks = app
            .hits
            .iter()
            .find(|h| h.action == Action::PlTimelineTracks)
            .unwrap()
            .rect;
        let plus = app
            .hits
            .iter()
            .find(|h| h.action == Action::PlAddInstrument)
            .unwrap()
            .rect;
        assert_eq!(tracks.intersect(&plus).w, 0.0);
        app.mouse_move(tracks.cx(), tracks.cy());
        app.mouse_down();
        app.mouse_up();
        assert!(
            app.state
                .playlist
                .popup
                .as_ref()
                .unwrap()
                .items
                .iter()
                .any(|item| { item.cmd == Cmd::Scoring(Command::Show(Lane::Tempo, false)) })
        );
        app.state.playlist.popup = None;
        app.paint(&mut canvas, None);
        let r = app
            .hits
            .iter()
            .find(|h| h.action == Action::ScoringLane(Lane::Chords, false))
            .unwrap()
            .rect;
        for _ in 0..2 {
            app.mouse_move(r.x + 20.0, r.cy());
            app.mouse_down();
            app.mouse_up();
            app.paint(&mut canvas, None);
        }
        assert!(matches!(
            app.state.playlist.edit.as_ref().map(|e| e.target),
            Some(EditTarget::TimelineChord(_))
        ));
        app.state.playlist.edit.as_mut().unwrap().text = "Gm7".into();
        pl::commit_edit(&mut app.state);
        assert_eq!(data(&app.state).chords[0].symbol, "Gm7");
        let items = pl::clip_menu(&app.state, 0, 100.0, 100.0).items;
        assert!(
            items
                .iter()
                .any(|i| i.cmd == Cmd::Scoring(Command::Extract(0)))
        );
    }
}
