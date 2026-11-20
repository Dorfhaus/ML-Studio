use crate::{
    app::{Action, App, State, Ui, Win},
    dialog::Then,
    filedlg::{FileDlg, Mode},
    playlist::{self as pl, Cmd, EditTarget, PGeom, PItem, Popup},
    scoring,
};
use os_model::{
    ClipSource, Project,
    timeline::{VisualScene, VisualTemplate},
};
use os_theme::Color;
use os_ui::{Canvas, Font, Rect};
use std::{
    cell::RefCell,
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

pub const VISIBILITY: &str = "visible_visualizer-workspace";
pub const FADES_VISIBILITY: &str = "visible_visuals-fade-handles";
pub const AUTOMATION_VISIBILITY: &str = "visible_visuals-automation-controls";
pub const LANE_VISIBILITY: &str = "visible_playlist-visuals-lane";
const TEMPLATES: [VisualTemplate; 3] = [
    VisualTemplate::Cover,
    VisualTemplate::Spectrum,
    VisualTemplate::Pulse,
];
type Image = (usize, usize, Vec<u32>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    arrangement: usize,
    tick: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Open,
    Add(u32),
    Select(u32),
    Template(VisualTemplate),
    Text(bool),
    Cover,
    ClearCover,
    Delete,
    Duplicate,
    Crossfade,
    ResetFades,
    Parameters,
    Value(Parameter),
    Automate(Parameter),
    Portrait,
    Export,
    Save { vertical: bool, selection: bool },
}

#[derive(Default)]
pub struct Controls {
    selected: Option<Key>,
    pub focus: bool,
    following: bool,
    last_cursor: u32,
    portrait: bool,
    position: f64,
    signal: Signal,
    art: VecDeque<(String, Option<Arc<Image>>)>,
    composite: RefCell<Compositor>,
    pending_art: Option<(String, mpsc::Receiver<Option<Image>>)>,
    last_frame: Option<Instant>,
}

impl Controls {
    pub fn waiting_for_art(&self) -> bool {
        self.pending_art.is_some()
    }
}

pub fn available() -> bool {
    crate::visualizer_video::supported() && crate::workspace_visibility::shown(VISIBILITY)
}

fn label(template: VisualTemplate) -> &'static str {
    match template {
        VisualTemplate::Cover => "Cover art",
        VisualTemplate::Spectrum => "Spectrum",
        VisualTemplate::Pulse => "Pulse",
    }
}

fn colors(template: VisualTemplate) -> (Color, Color, Color) {
    match template {
        VisualTemplate::Cover => (
            Color::rgb(0x191d37),
            Color::rgb(0x080b18),
            Color::rgb(0xffaf80),
        ),
        VisualTemplate::Spectrum => (
            Color::rgb(0x132b4b),
            Color::rgb(0x070b17),
            Color::rgb(0x79eaff),
        ),
        VisualTemplate::Pulse => (
            Color::rgb(0x33142d),
            Color::rgb(0x0d0818),
            Color::rgb(0xff71b4),
        ),
    }
}

fn data(s: &State) -> &[VisualScene] {
    &scoring::data(s).visuals
}

pub fn active(scenes: &[VisualScene], position: f64) -> Option<&VisualScene> {
    scenes
        .get(
            scenes
                .partition_point(|scene| f64::from(scene.tick) <= position)
                .checked_sub(1)?,
        )
        .filter(|scene| position < f64::from(scene.end))
}

fn key(s: &State) -> Option<Key> {
    s.playlist
        .visuals
        .selected
        .filter(|&key| get(s, key).is_some())
        .or_else(|| {
            data(s).first().map(|scene| Key {
                arrangement: s.project.arrangement_index(),
                tick: scene.tick,
            })
        })
}

fn get(s: &State, key: Key) -> Option<&VisualScene> {
    (key.arrangement == s.project.arrangement_index())
        .then(|| data(s).iter().find(|scene| scene.tick == key.tick))
        .flatten()
}

fn change(s: &mut State, key: Key, name: &'static str, edit: impl FnOnce(&mut VisualScene)) {
    if get(s, key).is_none() {
        return;
    }
    scoring::update(s, name, |timeline| {
        if let Some(scene) = timeline
            .visuals
            .iter_mut()
            .find(|scene| scene.tick == key.tick)
        {
            edit(scene);
        }
    });
}

pub fn text(s: &State, key: Key, artist: bool) -> String {
    get(s, key).map_or_else(String::new, |scene| {
        if artist {
            scene.artist.clone()
        } else {
            scene.title.clone()
        }
    })
}

pub fn edit_text(s: &mut State, key: Key, artist: bool, text: &str) {
    let text: String = text.chars().take(100).collect();
    change(s, key, "Edit visual scene text", |scene| {
        if artist {
            scene.artist = text
        } else {
            scene.title = text
        }
    });
}

fn add(s: &mut State, tick: u32, source: Option<VisualScene>) {
    let key = Key {
        arrangement: s.project.arrangement_index(),
        tick,
    };
    s.playlist.visuals.selected = Some(key);
    if get(s, key).is_some() {
        return;
    }
    let duration = s
        .project
        .playlist_loop
        .filter(|r| r.0.min(r.1) == tick && r.0 != r.1)
        .map(|r| r.0.abs_diff(r.1))
        .unwrap_or_else(|| {
            source
                .as_ref()
                .map_or(s.project.bar_of(4, 4).saturating_mul(4), |scene| {
                    scene.end - scene.tick
                })
        });
    let copy = source.is_some();
    let mut scene = source.unwrap_or_else(|| VisualScene {
        tick,
        end: tick,
        template: VisualTemplate::Cover,
        title: crate::backup::project_name(s.path.as_deref()),
        artist: String::new(),
        cover: None,
        automation_id: 0,
        style: Default::default(),
    });
    scene.tick = tick;
    scene.automation_id = 0;
    scene.end = tick.saturating_add(duration);
    scoring::update(
        s,
        if copy {
            "Duplicate visual scene"
        } else {
            "Add visual scene"
        },
        |timeline| timeline.visuals.push(scene),
    );
    scoring::show_visuals();
}

pub fn run(app: &mut App, command: Command) {
    if !available() {
        return;
    }
    let s = &mut app.state;
    s.playlist.popup = None;
    match command {
        Command::Open => {
            if s.shown(Win::Visualizer)
                && !s.native_detached.contains(&Win::Visualizer)
                && s.in_front(Win::Visualizer)
            {
                s.hide(Win::Visualizer);
            } else {
                s.raise(Win::Visualizer);
                s.playlist.visuals.focus = true;
            }
        }
        Command::Add(tick) => {
            add(s, tick, None);
            s.playlist.visuals.following = false;
            s.raise(Win::Visualizer);
        }
        Command::Select(tick) => {
            s.playlist.visuals.following = false;
            s.playlist.visuals.selected = Some(Key {
                arrangement: s.project.arrangement_index(),
                tick,
            });
            s.raise(Win::Visualizer);
            s.playlist.visuals.focus = true;
        }
        Command::Portrait => s.playlist.visuals.portrait = !s.playlist.visuals.portrait,
        Command::Save {
            vertical,
            selection,
        } => {
            crate::visualizer_video::choose(app, vertical, selection);
            return;
        }
        Command::Export => {
            if !data(s).is_empty() {
                crate::visualizer_video::workspace_options(app);
            }
            return;
        }
        command => {
            let Some(key) = key(s) else { return };
            match command {
                Command::Template(template) => change(s, key, "Change visual template", |scene| {
                    scene.template = template
                }),
                Command::Text(artist) => {
                    let at = (app.mouse.0.min(s.screen.0 - 244.).max(0.), app.mouse.1);
                    pl::start_edit(s, EditTarget::VisualText(key, artist), at);
                }
                Command::Cover => {
                    let dlg = FileDlg::new(
                        Mode::Open,
                        "Choose cover art",
                        "visualizer cover art",
                        Then::VisualCover(key),
                    )
                    .filter("Cover art", &["png", "jpg", "jpeg"])
                    .start(s.path.as_deref().and_then(Path::parent));
                    app.file_dialog(dlg);
                }
                Command::ClearCover => {
                    change(s, key, "Remove cover art", |scene| scene.cover = None)
                }
                Command::Crossfade => change(s, key, "Change visual transition", |scene| {
                    scene.style.crossfade = !scene.style.crossfade;
                }),
                Command::ResetFades => change(s, key, "Reset visual fades", |scene| {
                    scene.style.fade_in = 0;
                    scene.style.fade_out = 0;
                    scene.style.crossfade = false;
                }),
                Command::Parameters => {
                    if crate::workspace_visibility::shown(AUTOMATION_VISIBILITY) {
                        let (x, y) = app.mouse;
                        s.playlist.popup = Some(Popup {
                            x,
                            y,
                            items: parameter_items(),
                            sub: None,
                            scroll: [0; 2],
                            cascade: Vec::new(),
                        });
                    }
                }
                Command::Value(param) => {
                    pl::start_edit(s, EditTarget::VisualValue(key, param), app.mouse)
                }
                Command::Automate(param) => {
                    let result = automate(s, key, param);
                    if let Err(error) = result {
                        app.say(error);
                    }
                }
                Command::Delete => scoring::update(s, "Delete visual scene", |timeline| {
                    timeline.visuals.retain(|scene| scene.tick != key.tick)
                }),
                Command::Duplicate => {
                    let source = get(s, key).unwrap().clone();
                    let mut next = source.end;
                    for scene in data(s) {
                        if scene.tick <= next && next < scene.end {
                            next = scene.end;
                        }
                    }
                    add(s, next, Some(source));
                }
                _ => unreachable!(),
            }
        }
    }
    app.invalidate();
}

pub fn read_cover(path: &Path) -> Option<Image> {
    if std::fs::metadata(path).ok()?.len() > 32 * 1024 * 1024 {
        return None;
    }
    crate::wallpaper::read(path)
}

pub fn cover_done(app: &mut App, key: Key, path: PathBuf) {
    if !path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ["png", "jpg", "jpeg"]
                .iter()
                .any(|kind| ext.eq_ignore_ascii_case(kind))
        })
    {
        app.say("Choose a PNG or JPEG image.".into());
        return;
    }
    change(&mut app.state, key, "Set visual cover art", |scene| {
        scene.cover = Some(path.to_string_lossy().into_owned())
    });
    app.invalidate();
}

pub fn drop_cover(app: &mut App, path: &Path) -> bool {
    if !app.state.shown(Win::Visualizer)
        || app
            .top_hit(app.mouse.0, app.mouse.1)
            .and_then(|hit| hit.win)
            != Some(Win::Visualizer)
    {
        return false;
    }
    let Some(key) = key(&app.state) else {
        return false;
    };
    if !path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ["png", "jpg", "jpeg"]
                .iter()
                .any(|kind| ext.eq_ignore_ascii_case(kind))
        })
    {
        return false;
    }
    cover_done(app, key, path.to_owned());
    true
}

pub fn paint_lane(ui: &mut Ui, s: &State, g: &PGeom, r: Rect, scenes: &[VisualScene]) {
    for scene in scenes {
        let x = g.x_of(scene.tick as f32).max(r.x);
        let right = g.x_of(scene.end as f32).min(r.right());
        if right - x < 2. {
            continue;
        }
        let body = Rect::new(x + 1., r.y + 3., right - x - 2., r.h - 6.);
        let (_, bg, accent) = colors(scene.template);
        let selected = key(s).is_some_and(|key| key.tick == scene.tick);
        ui.c.fill_rrect(body, 4., bg.mix(accent, if selected { 0.34 } else { 0.18 }));
        ui.c.fill_rrect(Rect::new(body.x, body.y + 3., 3., body.h - 6.), 1., accent);
        if body.w > 12. {
            let text = if scene.title.is_empty() {
                label(scene.template)
            } else {
                &scene.title
            };
            ui.c.text_clip(
                Font::new(12),
                body.x + 8.,
                body.y + 9.,
                body.w - 10.,
                Color::rgb(0xf5f6fa),
                text,
            );
        }
        if crate::workspace_visibility::shown(FADES_VISIBILITY) {
            let [head, tail] = fade_handles(g, scene, r.y);
            for handle in [head, tail] {
                ui.c.fill_rrect(handle, 2., accent);
            }
            if scene.style.fade_in > 0 {
                let start = g.x_of(scene.tick as f32);
                ui.c.line(start, body.bottom() - 2., head.cx(), head.cy(), 1.2, accent);
            }
            if scene.style.fade_out > 0 {
                let end = g.x_of(scene.end as f32);
                ui.c.line(tail.cx(), tail.cy(), end, body.bottom() - 2., 1.2, accent);
            }
        }
    }
}

fn fade_handles(g: &PGeom, scene: &VisualScene, top: f32) -> [Rect; 2] {
    let start = g.x_of(scene.tick as f32);
    let end = g.x_of(scene.end as f32).max(start);
    let inset = (end - start).min(12.) * 0.25;
    let head = g
        .x_of((scene.tick + scene.style.fade_in) as f32)
        .clamp(start + inset, end - inset);
    let tail = g
        .x_of((scene.end - scene.style.fade_out) as f32)
        .clamp(start + inset, end - inset);
    [
        Rect::new(head - 3., top + 4., 6., 5.),
        Rect::new(tail - 3., top + 9., 6., 5.),
    ]
}

#[derive(Clone, Copy, Debug)]
pub enum Edge {
    Move,
    Start,
    End,
    FadeIn,
    FadeOut,
}

pub fn lane_press(app: &mut App, g: &PGeom, tick: u32, lane_top: f32, header: bool, right: bool) {
    let (x, y) = app.mouse;
    let s = &mut app.state;
    let at = f64::from(g.tick_at(x));
    let hit = (!header)
        .then(|| {
            active(data(s), at).or_else(|| {
                data(s).iter().rev().find(|scene| {
                    let edge = g.x_of(scene.end as f32);
                    x >= edge && x - edge < 4.
                })
            })
        })
        .flatten()
        .cloned();
    if header || right {
        let item = |label, command| PItem::new(label, Cmd::Visual(command));
        let mut items = if let Some(scene) = hit {
            s.playlist.visuals.selected = Some(Key {
                arrangement: s.project.arrangement_index(),
                tick: scene.tick,
            });
            let mut entries = vec![
                item("Edit scene", Command::Select(scene.tick)),
                item("Duplicate scene", Command::Duplicate),
                item("Delete scene", Command::Delete),
            ];
            if crate::workspace_visibility::shown(FADES_VISIBILITY) {
                entries.push(
                    item("Crossfade from previous scene", Command::Crossfade)
                        .mark(pl::Mark::Check(scene.style.crossfade)),
                );
                entries.push(item("Reset fades", Command::ResetFades));
            }
            if crate::workspace_visibility::shown(AUTOMATION_VISIBILITY) {
                entries.push(PItem::sub("Scene controls", parameter_items()));
            }
            entries
        } else {
            vec![
                item("Add visual scene here", Command::Add(tick)),
                item("Open ML Visualizer", Command::Open),
            ]
        };
        items.push(PItem::new(
            "Hide Visuals lane",
            Cmd::Scoring(scoring::Command::Show(scoring::Lane::Visuals, false)),
        ));
        s.playlist.popup = Some(Popup {
            x,
            y,
            items,
            sub: None,
            scroll: [0; 2],
            cascade: Vec::new(),
        });
    } else if let Some(scene) = hit {
        let handles = fade_handles(g, &scene, lane_top);
        let fades = crate::workspace_visibility::shown(FADES_VISIBILITY);
        let edge = if fades && handles[0].contains(x, y) {
            Edge::FadeIn
        } else if fades && handles[1].contains(x, y) {
            Edge::FadeOut
        } else if (g.x_of(scene.tick as f32) - x).abs() < 6. {
            Edge::Start
        } else if (g.x_of(scene.end as f32) - x).abs() < 6. {
            Edge::End
        } else {
            Edge::Move
        };
        s.playlist.visuals.selected = Some(Key {
            arrangement: s.project.arrangement_index(),
            tick: scene.tick,
        });
        let now = Instant::now();
        let double = s.playlist.scoring.last_click.is_some_and(|(lane, t, at)| {
            lane == scoring::Lane::Visuals
                && t == scene.tick
                && now.duration_since(at) < Duration::from_millis(420)
        });
        s.playlist.scoring.last_click =
            (!double).then_some((scoring::Lane::Visuals, scene.tick, now));
        s.playlist.visuals.following = false;
        if double && !matches!(edge, Edge::FadeIn | Edge::FadeOut) {
            s.raise(Win::Visualizer);
            s.playlist.visuals.focus = true;
        } else {
            app.drag = Some(crate::app::Drag::Scoring(scoring::Gesture::Visual {
                start: scene.tick,
                x,
                edge,
            }));
        }
    } else {
        s.playlist.pos = tick;
        s.song_mode = true;
        if app.transport.playing() {
            app.play_song_from(tick);
        }
    }
    app.invalidate();
}

pub fn drag(next: &mut [VisualScene], start: u32, delta: i64, edge: Edge) -> Option<u32> {
    let index = next.iter().position(|scene| scene.tick == start)?;
    let lower = next.get(index.wrapping_sub(1)).map_or(0, |scene| scene.end);
    let upper = next.get(index + 1).map_or(u32::MAX, |scene| scene.tick);
    let scene = &mut next[index];
    let shifted = |tick: u32| (i64::from(tick) + delta).clamp(0, i64::from(u32::MAX)) as u32;
    let length = scene.end - scene.tick;
    match edge {
        Edge::FadeIn => scene.style.fade_in = shifted(scene.style.fade_in).min(length),
        Edge::FadeOut => {
            scene.style.fade_out =
                (i64::from(scene.style.fade_out) - delta).clamp(0, i64::from(length)) as u32
        }
        Edge::Start => scene.tick = shifted(scene.tick).clamp(lower, scene.end - 1),
        Edge::End => scene.end = shifted(scene.end).clamp(scene.tick + 1, upper),
        Edge::Move => {
            let len = scene.end - scene.tick;
            scene.tick = shifted(scene.tick).clamp(lower, upper - len);
            scene.end = scene.tick + len;
        }
    }
    Some(scene.tick)
}

pub fn moved(s: &mut State, tick: u32) {
    s.playlist.visuals.selected = Some(Key {
        arrangement: s.project.arrangement_index(),
        tick,
    });
}

#[derive(Clone, Copy, Default)]
pub struct Signal {
    rms: f32,
    bands: [f32; 32],
}

impl Signal {
    pub fn from_audio(samples: &[f32]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }
        let mut signal = Self::default();
        let mut real = [0.; 1024];
        let mut imaginary = [0.; 1024];
        let mut total = 0.;
        let n = samples.len().min(real.len());
        for (i, &v) in samples[samples.len() - n..].iter().enumerate() {
            let v = if v.is_finite() { v } else { 0. };
            total += v * v;
            let phase = std::f32::consts::TAU * i as f32 / n as f32;
            real[i] = v * (0.5 - 0.5 * phase.cos());
        }
        crate::monitor::fft_in_place(&mut real, &mut imaginary);
        for i in 1..real.len() / 2 {
            let band = ((i as f32).ln() * (32. / 512_f32.ln())) as usize;
            let strength = real[i].hypot(imaginary[i]) * (4. / n as f32);
            signal.bands[band] = signal.bands[band].max(strength.sqrt().min(1.));
        }
        signal.rms = (total / samples.len() as f32).sqrt().clamp(0., 1.);
        signal
    }
}

pub fn tick(app: &mut App) {
    if !app.state.shown(Win::Visualizer) {
        return;
    }
    if !available() {
        app.state.hide(Win::Visualizer);
        app.invalidate();
        return;
    }
    let now = Instant::now();
    let s = &mut app.state;
    if s.playlist
        .visuals
        .last_frame
        .is_some_and(|last| now.duration_since(last) < Duration::from_millis(50))
    {
        return;
    }
    let playing = app.transport.playing() && app.transport.song();
    let position = if playing {
        app.transport.tick()
    } else {
        f64::from(s.playlist.pos)
    };
    let following = if playing || s.playlist.visuals.last_cursor != s.playlist.pos {
        true
    } else {
        s.playlist.visuals.following
    };
    let selection = (!following)
        .then(|| key(s).and_then(|key| get(s, key)))
        .flatten();
    let position = selection.map_or(position, |scene| {
        if position >= f64::from(scene.tick) && position < f64::from(scene.end) {
            position
        } else {
            f64::from(scene.tick)
        }
    });
    let mut paths = layers(&s.project, data(s), selection, position)
        .map(|layer| layer.and_then(|layer| layer.scene.cover.clone()));
    if following && paths[1].is_none() {
        let scenes = data(s);
        let next = scenes.partition_point(|scene| f64::from(scene.tick) <= position);
        paths[1] = scenes.get(next).and_then(|scene| scene.cover.clone());
    }
    let controls = &mut s.playlist.visuals;
    let moving = playing || controls.following != following || controls.position != position;
    controls.following = following;
    controls.last_cursor = s.playlist.pos;
    controls.position = position;
    controls.last_frame = Some(now);
    if playing {
        let mut samples = [0.; 1024];
        app.transport.tap(&mut samples);
        controls.signal = Signal::from_audio(&samples);
    } else {
        controls.signal = Signal::default();
    }
    let mut updated = false;
    if let Some((pending, rx)) = &controls.pending_art {
        if let Ok(image) = rx.try_recv() {
            if controls.art.len() == 3 {
                controls.art.pop_front();
            }
            controls
                .art
                .push_back((pending.clone(), image.map(Arc::new)));
            controls.pending_art = None;
            updated = true;
        }
    }
    if controls.pending_art.is_none()
        && let Some(path) = paths
            .into_iter()
            .flatten()
            .find(|path| !controls.art.iter().any(|(p, _)| p == path))
    {
        let name = path.clone();
        let (tx, rx) = mpsc::channel();
        controls.pending_art = Some((path, rx));
        std::thread::spawn(move || {
            let _ = tx.send(read_cover(Path::new(&name)));
        });
    }
    if moving || updated {
        app.damage_win(Win::Visualizer);
    }
}

fn fit(r: Rect, aspect: f32) -> Rect {
    let w = r.w.min(r.h * aspect).max(0.);
    let h = w / aspect;
    Rect::new(r.cx() - w / 2., r.cy() - h / 2., w, h)
}

pub fn draw(
    canvas: &mut Canvas,
    r: Rect,
    scene: Option<&VisualScene>,
    art: Option<&Image>,
    beat: f64,
    signal: Signal,
    zoom: f32,
    intensity: f32,
) {
    let Some(scene) = scene else {
        canvas.fill_rect(r, Color::rgb(0x070910));
        return;
    };
    let outer = r;
    let (top, bottom, accent) = colors(scene.template);
    let r = Rect::new(
        r.cx() - r.w * zoom * 0.5,
        r.cy() - r.h * zoom * 0.5,
        r.w * zoom,
        r.h * zoom,
    );
    let portrait = r.h > r.w;
    let k = (r.w / if portrait { 540. } else { 850. }).max(0.1);
    let phase = beat.rem_euclid(1.) as f32;
    let energy = (signal.rms * 3.2).min(1.);
    let pulse = (1. - phase).powi(4) * (0.26 + energy * 0.74) * intensity;
    canvas.push_clip(outer);
    if zoom < 1. {
        canvas.fill_rect(outer, bottom);
    }
    let center_y = r.y + r.h * if portrait { 0.38 } else { 0.43 };
    canvas.fill_radial(r, top.mix(accent, 0.11 + pulse * 0.13), bottom);
    match scene.template {
        VisualTemplate::Spectrum => {
            let w = r.w * 0.73;
            let step = w / signal.bands.len() as f32;
            for (i, value) in signal
                .bands
                .into_iter()
                .map(|value| value * intensity)
                .enumerate()
            {
                let height = (0.035 + 0.34 * (value * 2.).min(1.)) * r.h;
                let x = r.cx() - w / 2. + step * i as f32;
                canvas.fill_rrect(
                    Rect::new(x, center_y - height / 2., step * 0.66, height),
                    3. * k,
                    top.mix(accent, 0.45 + value.min(0.5)),
                );
            }
        }
        VisualTemplate::Pulse | VisualTemplate::Cover => {
            let base = r.h.min(r.w) * if portrait { 0.30 } else { 0.25 };
            for i in 0..4 {
                let radius = base * (0.56 + i as f32 * 0.25 + pulse * 0.22);
                canvas.stroke_circle(
                    r.cx(),
                    center_y,
                    radius,
                    (k * (2.5 - i as f32 * 0.42)).max(0.7),
                    accent.with_alpha((135. + pulse * 75.) as u8 / (i + 1)),
                );
            }
        }
    }
    if let Some((w, h, pixels)) = art {
        let size = r.h.min(r.w)
            * if scene.template == VisualTemplate::Cover {
                0.45
            } else {
                0.22
            };
        let art_rect = fit(
            Rect::new(r.cx() - size / 2., center_y - size / 2., size, size),
            *w as f32 / *h as f32,
        );
        canvas.image(art_rect, *w, *h, pixels);
    }
    let title = Rect::new(r.x + r.w * 0.06, r.y + r.h * 0.71, r.w * 0.88, r.h * 0.11);
    canvas.push_clip(title);
    canvas.text_center(
        Font::new((k * 36.).round().max(1.) as u16).bold(),
        title,
        Color::rgb(0xf7f7fc),
        &scene.title,
    );
    canvas.pop_clip();
    let artist = Rect::new(title.x, title.bottom(), title.w, r.h * 0.065);
    canvas.push_clip(artist);
    canvas.text_center(
        Font::new((k * 20.).round().max(1.) as u16),
        artist,
        accent,
        &scene.artist,
    );
    canvas.pop_clip();
    canvas.pop_clip();
}

fn button(ui: &mut Ui, r: Rect, label: &'static str, command: Command, selected: bool) {
    let hover = ui.hit(r, Action::Visual(command), label);
    ui.c.fill_rrect(
        r,
        5.,
        if selected {
            ui.t.accent.shade(-0.48)
        } else if hover {
            ui.t.button_hi
        } else {
            ui.t.sunken
        },
    );
    ui.c.text_center(Font::new(13), r, ui.t.text, label);
}

pub fn paint(ui: &mut Ui, s: &State, frame: Rect) {
    let win = Win::Visualizer;
    let r = crate::views::window_frame(ui, win, frame, "ML Visualizer", 14., true);
    crate::views::resize_zones(ui, s, win, frame);
    if r.h < 30. {
        return;
    }
    ui.c.fill_rect(r, ui.t.window_title);
    let controls = &s.playlist.visuals;
    let panel = Rect::new(r.x + 12., r.y + 10., 170., r.h - 20.);
    let preview = Rect::new(
        panel.right() + 14.,
        panel.y,
        r.right() - panel.right() - 26.,
        panel.h - 43.,
    );
    ui.c.fill_rrect(preview, 6., ui.t.sunken);
    let screen = fit(
        preview.inset(4.),
        if controls.portrait {
            9. / 16.
        } else {
            16. / 9.
        },
    );
    let selected = key(s).and_then(|key| get(s, key));
    let layers = layers(
        &s.project,
        data(s),
        if controls.following { None } else { selected },
        controls.position,
    );
    let art = layers.map(|layer| {
        layer.and_then(|layer| {
            let path = layer.scene.cover.as_ref()?;
            controls
                .art
                .iter()
                .find(|(cached, _)| cached == path)
                .map(|(_, image)| image)
        })
    });
    controls.composite.borrow_mut().paint(
        ui.c,
        screen,
        layers,
        art.map(|art| art.and_then(|image| image.as_deref())),
        controls.position / f64::from(s.project.ppq.max(1)),
        controls.signal,
    );
    if layers[0].is_none() {
        ui.c.text_center(
            Font::new(15),
            screen,
            ui.t.text_dim,
            if data(s).is_empty() {
                "Add a scene to start"
            } else {
                "No scene at the playhead"
            },
        );
    }
    if art.iter().any(|entry| entry.is_some_and(Option::is_none)) {
        ui.c.text_center(
            Font::new(13),
            Rect::new(screen.x, screen.y + 5., screen.w, 22.),
            ui.t.text,
            "Cover art is missing or unreadable",
        );
    }
    button(
        ui,
        Rect::new(panel.x, panel.y, panel.w, 29.),
        "Add scene at playhead",
        Command::Add(s.playlist.pos),
        false,
    );
    if let Some(scene) = selected {
        for (i, template) in TEMPLATES.into_iter().enumerate() {
            let card = Rect::new(panel.x, panel.y + 39. + i as f32 * 34., panel.w, 29.);
            let active = scene.template == template;
            let hover = ui.hit(
                card,
                Action::Visual(Command::Template(template)),
                label(template),
            );
            ui.c.fill_rrect(
                card,
                5.,
                if active {
                    ui.t.accent.shade(-0.48)
                } else if hover {
                    ui.t.button_hi
                } else {
                    ui.t.sunken
                },
            );
            let thumb = Rect::new(card.x + 3., card.y + 3., 41., 23.);
            let example = VisualScene {
                template,
                title: String::new(),
                artist: String::new(),
                cover: None,
                automation_id: 0,
                style: Default::default(),
                tick: 0,
                end: 1,
            };
            draw(
                ui.c,
                thumb,
                Some(&example),
                None,
                controls.position / f64::from(s.project.ppq.max(1)),
                controls.signal,
                1.,
                1.,
            );
            ui.c.text_clip(
                Font::new(13),
                thumb.right() + 8.,
                card.y + 6.,
                card.right() - thumb.right() - 14.,
                ui.t.text,
                label(template),
            );
        }
        for (i, (label, command)) in [
            ("Edit title", Command::Text(false)),
            ("Edit artist", Command::Text(true)),
            ("Choose cover art…", Command::Cover),
            ("Remove cover art", Command::ClearCover),
        ]
        .into_iter()
        .enumerate()
        {
            button(
                ui,
                Rect::new(panel.x, panel.y + 151. + i as f32 * 29., panel.w, 25.),
                label,
                command,
                false,
            );
        }
        if crate::workspace_visibility::shown(AUTOMATION_VISIBILITY) {
            button(
                ui,
                Rect::new(panel.x, panel.y + 269., panel.w, 26.),
                "Scene controls…",
                Command::Parameters,
                false,
            );
        }
        let row = Rect::new(preview.right() - 150., preview.bottom() + 8., 150., 28.);
        button(ui, row, "Export video…", Command::Export, false);
    }
    button(
        ui,
        Rect::new(preview.x, preview.bottom() + 8., 126., 28.),
        if controls.portrait {
            "Vertical preview"
        } else {
            "Landscape preview"
        },
        Command::Portrait,
        controls.portrait,
    );
}
/// Bound memory to the most recently rendered image. Scene clips play in order.
#[derive(Default)]
pub struct Artwork(VecDeque<(String, Arc<Image>)>);

impl Artwork {
    pub fn for_scene(&mut self, scene: Option<&VisualScene>) -> Result<Option<Arc<Image>>, String> {
        let Some(path) = scene.and_then(|scene| scene.cover.as_ref()) else {
            return Ok(None);
        };
        if let Some((_, image)) = self.0.iter().find(|(previous, _)| previous == path) {
            return Ok(Some(image.clone()));
        }
        let image = Arc::new(
            read_cover(Path::new(path))
                .ok_or_else(|| format!("Could not read cover art: {path}"))?,
        );
        if self.0.len() == 2 {
            self.0.pop_front();
        }
        self.0.push_back((path.clone(), image.clone()));
        Ok(Some(image))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parameter {
    Opacity,
    Zoom,
    Intensity,
}

impl Parameter {
    const ALL: [Self; 3] = [Self::Opacity, Self::Zoom, Self::Intensity];

    fn index(self) -> usize {
        match self {
            Self::Opacity => 0,
            Self::Zoom => 1,
            Self::Intensity => 2,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Opacity => "Opacity",
            Self::Zoom => "Zoom",
            Self::Intensity => "Effect intensity",
        }
    }

    fn tag(self, id: u16) -> u32 {
        ((0x5a10 + self.index() as u32) << 16) | u32::from(id)
    }

    fn parse(tag: u32) -> Option<(Self, u16)> {
        let index = (tag >> 16).checked_sub(0x5a10)? as usize;
        let id = tag as u16;
        (id != 0).then_some((*Self::ALL.get(index)?, id))
    }

    fn display(self, value: f32) -> f32 {
        match self {
            Self::Zoom => 50. + value * 100.,
            _ => value * 100.,
        }
    }
    fn normalize(self, value: f32) -> f32 {
        match self {
            Self::Zoom => (value - 50.) / 100.,
            _ => value / 100.,
        }
    }
}

fn parameter_items() -> Vec<PItem> {
    Parameter::ALL
        .into_iter()
        .map(|param| {
            PItem::sub(
                param.label(),
                vec![
                    PItem::new("Set value…", Cmd::Visual(Command::Value(param))),
                    PItem::new(
                        "Create automation clip",
                        Cmd::Visual(Command::Automate(param)),
                    ),
                ],
            )
        })
        .collect()
}

pub fn value_text(s: &State, key: Key, param: Parameter) -> String {
    get(s, key).map_or_else(String::new, |scene| {
        format!("{:.1}%", param.display(scene.style.values[param.index()]))
    })
}

pub fn edit_value(s: &mut State, key: Key, param: Parameter, text: &str) {
    let value = text
        .trim_end_matches('%')
        .trim()
        .replace(',', ".")
        .parse::<f32>()
        .ok()
        .map(|value| param.normalize(value))
        .filter(|value| (0. ..=1.).contains(value));
    if let Some(value) = value {
        change(s, key, "Set visual scene control", |scene| {
            scene.style.values[param.index()] = value
        });
    } else {
        s.message = Some(
            if param == Parameter::Zoom {
                "Enter a zoom from 50% to 150%."
            } else {
                "Enter a value from 0% to 100%."
            }
            .into(),
        );
    }
}

fn automate(s: &mut State, key: Key, param: Parameter) -> Result<(), String> {
    let Some(scene) = get(s, key) else {
        return Ok(());
    };
    let (start, end, id, value) = (
        scene.tick,
        scene.end,
        scene.automation_id,
        scene.style.values[param.index()],
    );
    let label = if scene.title.is_empty() {
        label(scene.template)
    } else {
        &scene.title
    };
    let name = format!("{label} - {}", param.label());
    let row = pl::recording_row(s, start, end)
        .ok_or("Free an unlocked Playlist track for the visual automation clip.")?;
    let id = if id == 0 {
        let mut used = vec![false; u16::MAX as usize + 1];
        for scene in s
            .project
            .arrangements
            .iter()
            .flat_map(|a| &a.timeline.visuals)
        {
            used[usize::from(scene.automation_id)] = true;
        }
        for (_, id) in s
            .project
            .automations
            .iter()
            .flat_map(|a| &a.targets)
            .filter_map(|&tag| Parameter::parse(tag))
        {
            used[usize::from(id)] = true;
        }
        (1..=u16::MAX)
            .find(|&id| !used[usize::from(id)])
            .ok_or("This project has no more visual automation IDs.")?
    } else {
        id
    };
    let snapshot = (get(s, key).unwrap().automation_id == 0)
        .then(|| crate::history::Snap::Scoring(key.arrangement, scoring::data(s).clone()));
    pl::create_visual_automation(
        s,
        param.tag(id),
        &name,
        f64::from(value),
        row,
        (start, end),
        snapshot,
    );
    if let Some(scene) = pl::arr_mut(s)
        .timeline
        .visuals
        .iter_mut()
        .find(|scene| scene.tick == key.tick)
    {
        scene.automation_id = id;
    }
    s.raise(Win::Playlist);
    Ok(())
}

fn render_values(project: &Project, scene: &VisualScene, tick: f64) -> [f32; 3] {
    let mut values = scene.style.values;
    let mut priority = [0; 3];
    let Some(arr) = project.arrangement().filter(|_| scene.automation_id != 0) else {
        return values;
    };
    for clip in &arr.clips {
        if clip.muted
            || tick < f64::from(clip.position)
            || tick >= f64::from(clip.end())
            || arr
                .tracks
                .get(usize::from(clip.track))
                .is_some_and(|track| !track.enabled)
        {
            continue;
        }
        let ClipSource::Channel(ch) = clip.source else {
            continue;
        };
        let Some(automation) = project.automation(ch) else {
            continue;
        };
        let beat = clip.window.map_or(0., |w| w.0)
            + (tick - f64::from(clip.position)) / f64::from(project.ppq.max(1));
        for (param, id) in automation
            .targets
            .iter()
            .filter_map(|&tag| Parameter::parse(tag))
        {
            let index = param.index();
            if id == scene.automation_id
                && clip.position >= priority[index]
                && let Some(value) = automation.value_at(beat).filter(|value| value.is_finite())
            {
                values[index] = value.clamp(0., 1.) as f32;
                priority[index] = clip.position;
            }
        }
    }
    values
}

fn fades(scene: &VisualScene, tick: f64, blend_from_previous: bool) -> f32 {
    let ramp = |left: f64, length: u32| {
        if length == 0 {
            1.
        } else {
            (left / f64::from(length)).clamp(0., 1.) as f32
        }
    };
    let fade_in = if blend_from_previous {
        1.
    } else {
        ramp(tick - f64::from(scene.tick), scene.style.fade_in)
    };
    fade_in * ramp(f64::from(scene.end) - tick, scene.style.fade_out)
}

fn connected(prior: Option<&VisualScene>, next: &VisualScene) -> bool {
    next.style.crossfade
        && next.style.fade_in > 0
        && prior.is_some_and(|prior| prior.end == next.tick)
}

#[derive(Clone, Copy)]
pub(crate) struct Layer<'a> {
    pub scene: &'a VisualScene,
    opacity: f32,
    zoom: f32,
    intensity: f32,
}

#[cfg(test)]
impl Layer<'_> {
    pub(crate) fn opacity_for_test(self) -> f32 {
        self.opacity
    }
    pub(crate) fn controls_for_test(self) -> (f32, f32) {
        (self.zoom, self.intensity)
    }
}

pub(crate) fn layers<'a>(
    project: &Project,
    scenes: &'a [VisualScene],
    selected: Option<&'a VisualScene>,
    tick: f64,
) -> [Option<Layer<'a>>; 2] {
    let layer = |scene, blend, t, weight| {
        let [opacity, zoom, intensity] = render_values(project, scene, t);
        Layer {
            scene,
            opacity: opacity * blend * weight,
            zoom: 0.5 + zoom,
            intensity,
        }
    };
    if let Some(scene) = selected {
        return [Some(layer(scene, 1., tick, 1.)), None];
    }
    let Some(index) = scenes
        .partition_point(|scene| f64::from(scene.tick) <= tick)
        .checked_sub(1)
    else {
        return [None, None];
    };
    let scene = &scenes[index];
    if tick >= f64::from(scene.end) {
        return [None, None];
    }
    let prior = index.checked_sub(1).and_then(|index| scenes.get(index));
    let fade = fades(scene, tick, connected(prior, scene));
    if let Some(next) = scenes
        .get(index + 1)
        .filter(|next| connected(Some(scene), next))
    {
        let len = next.style.fade_in.min(scene.end - scene.tick);
        let from = f64::from(scene.end - len);
        if tick >= from {
            let amount = ((tick - from) / f64::from(len)).clamp(0., 1.) as f32;
            let entry = f64::from(next.tick);
            return [
                Some(layer(scene, fade, tick, 1. - amount)),
                Some(layer(next, 1., entry, amount)),
            ];
        }
    }
    [Some(layer(scene, fade, tick, 1.)), None]
}
#[derive(Default)]
pub(crate) struct Compositor {
    buffer: Option<Canvas>,
}

impl Compositor {
    pub fn paint(
        &mut self,
        canvas: &mut Canvas,
        r: Rect,
        layers: [Option<Layer<'_>>; 2],
        artwork: [Option<&Image>; 2],
        beat: f64,
        signal: Signal,
    ) {
        if let [Some(layer), None] = layers
            && layer.opacity >= 1.
        {
            draw(
                canvas,
                r,
                Some(layer.scene),
                artwork[0],
                beat,
                signal,
                layer.zoom,
                layer.intensity,
            );
            return;
        }
        canvas.fill_rect(r, Color::rgb(0x000000));
        if layers
            .into_iter()
            .flatten()
            .all(|layer| layer.opacity <= 0.)
        {
            return;
        }
        let scale = canvas.scale;
        let origin = ((r.x * scale).round() as i32, (r.y * scale).round() as i32);
        let width = ((r.right() * scale).round() as i32 - origin.0).max(1) as u32;
        let height = ((r.bottom() * scale).round() as i32 - origin.1).max(1) as u32;
        if self.buffer.is_none() {
            self.buffer = Canvas::new(width, height, scale).ok();
        }
        let Some(buffer) = &mut self.buffer else {
            return;
        };
        if buffer.pixmap.width() != width
            || buffer.pixmap.height() != height
            || buffer.scale != scale
        {
            buffer.resize(width, height, scale);
        }
        let local = Rect::new(0., 0., width as f32 / scale, height as f32 / scale);
        for (layer, art) in layers.into_iter().zip(artwork) {
            let Some(layer) = layer.filter(|layer| layer.opacity > 0.) else {
                continue;
            };
            draw(
                buffer,
                local,
                Some(layer.scene),
                art,
                beat,
                signal,
                layer.zoom,
                layer.intensity,
            );
            canvas.add_opaque_pixmap(origin, &buffer.pixmap, layer.opacity);
        }
    }
}
