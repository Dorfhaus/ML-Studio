// SPDX-License-Identifier: GPL-3.0-or-later

use crate::app::{Action, App, State, Win};
use crate::playlist::{self as pl, PGeom, PickKind};
use os_model::{ClipSource, Rgb};
use os_ui::Rect;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

pub const COLOR_VISIBILITY: &str = "visible_playlist-picker-colors";
pub const FILTER_VISIBILITY: &str = "visible_playlist-picker-filter";
pub const LOOP_VISIBILITY: &str = "visible_playlist-picker-loop";
pub const RESIZE_VISIBILITY: &str = "visible_playlist-picker-resize";
const LOOP_KEY: &str = "playlist_picker_loop";
const WIDTH_KEY: &str = "playlist_picker_width";
const DEFAULT_WIDTH: f32 = 132.0;
pub(crate) const PREVIEW_LIMIT: usize = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Usage {
    #[default]
    All,
    Used,
    Unused,
}

impl Usage {
    pub const OPTIONS: [(Self, &'static str); 3] = [(Self::All, "All"), (Self::Used, "Used"), (Self::Unused, "Unused")];
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Destination {
    Row(ClipSource, bool, Rect),
    End,
    Playlist { track: u16, tick: u32 },
}

#[derive(Default)]
pub struct Picker {
    pub usage: [Usage; 3],
    pub selected: Vec<ClipSource>,
    anchor: Option<ClipSource>,
    pub destination: Option<Destination>,
    pub width: Option<f32>,
    previews: Vec<(ClipSource, Arc<AtomicBool>)>,
}

impl Drop for Picker {
    fn drop(&mut self) {
        self.stop_previews();
    }
}

impl Picker {
    pub fn playing(&self, source: ClipSource) -> bool {
        self.previews.iter().any(|(item, active)| *item == source && active.load(Ordering::Acquire))
    }

    pub fn has_previews(&self) -> bool {
        !self.previews.is_empty()
    }

    pub fn stop_previews(&mut self) {
        for (_, active) in self.previews.drain(..) {
            active.store(false, Ordering::Release);
        }
    }

    pub fn prune_previews(&mut self) -> bool {
        let before = self.previews.len();
        self.previews.retain(|(_, active)| active.load(Ordering::Acquire));
        before != self.previews.len()
    }
}

pub fn looping() -> bool {
    crate::settings_file::flag(LOOP_KEY, false)
}

pub fn toggle_loop(app: &App) {
    let on = !looping();
    crate::settings_file::set_flag(LOOP_KEY, on);
    app.transport.set_picker_loop(on);
}

pub fn width(s: &State, area: Rect) -> f32 {
    if !s.playlist.picker_visible || area.w < 430.0 {
        return 0.0;
    }
    let saved = s.playlist.picker_ui.width.or_else(|| crate::settings_file::get(WIDTH_KEY)?.parse().ok());
    clamp_width(saved.filter(|w| w.is_finite()).unwrap_or(DEFAULT_WIDTH), area.w)
}

pub fn clamp_width(width: f32, area_width: f32) -> f32 {
    width.clamp(112.0, (area_width - 295.0).clamp(112.0, 360.0))
}

pub fn save_width(s: &State) {
    if let Some(width) = s.playlist.picker_ui.width {
        crate::settings_file::write(WIDTH_KEY, &width.round().to_string());
    }
}

pub struct Rows {
    pub items: Vec<ClipSource>,
    track_colors: HashMap<ClipSource, Rgb>,
}

pub fn rows(s: &State) -> Rows {
    let kind = s.playlist.picker;
    let mut items = ordered(s, kind);
    let mut used = HashSet::new();
    let mut track_colors = HashMap::new();
    if let Some(arrangement) = s.project.arrangement() {
        for clip in arrangement.clips.iter().chain(arrangement.tracks.iter().filter_map(|t| t.frozen.as_deref()).flatten()) {
            if used.insert(clip.source) {
                if let Some(color) = pl::track_info(s, clip.track).and_then(|track| track.color) {
                    track_colors.insert(clip.source, color);
                }
            }
        }
    }
    if crate::workspace_visibility::shown(FILTER_VISIBILITY) {
        match s.playlist.picker_ui.usage[kind as usize] {
            Usage::All => {}
            Usage::Used => items.retain(|item| used.contains(item)),
            Usage::Unused => items.retain(|item| !used.contains(item)),
        }
    }
    Rows { items, track_colors }
}

impl Rows {
    pub fn color(&self, s: &State, source: ClipSource) -> Option<Rgb> {
        pl::clip_tint(s, source)
            .or_else(|| self.track_colors.get(&source).copied())
            .or_else(|| pl::source_color(s, source))
    }
}

fn ordered(s: &State, kind: PickKind) -> Vec<ClipSource> {
    let items = crate::playlist_tools::sources(s, kind);
    let mut remaining: HashSet<_> = items.iter().copied().collect();
    let mut result = Vec::with_capacity(items.len());
    result.extend(s.project.clip_picker_order.iter().copied().filter(|item| remaining.remove(item)));
    result.extend(items.into_iter().filter(|item| remaining.remove(item)));
    result
}

pub fn press(app: &mut App, source: ClipSource) {
    let s = &mut app.state;
    if app.shift {
        let items = rows(s).items;
        let range = s.playlist.picker_ui.anchor.and_then(|anchor| {
            Some((items.iter().position(|&item| item == anchor)?, items.iter().position(|&item| item == source)?))
        });
        if !app.ctrl {
            s.playlist.picker_ui.selected.clear();
        }
        if let Some((a, b)) = range {
            for &item in &items[a.min(b)..=a.max(b)] {
                if !s.playlist.picker_ui.selected.contains(&item) {
                    s.playlist.picker_ui.selected.push(item);
                }
            }
        } else {
            s.playlist.picker_ui.selected = vec![source];
            s.playlist.picker_ui.anchor = Some(source);
        }
    } else if app.ctrl {
        let picker = &mut s.playlist.picker_ui;
        if let Some(index) = picker.selected.iter().position(|&item| item == source) {
            picker.selected.remove(index);
        } else {
            picker.selected.push(source);
        }
        picker.anchor = Some(source);
    } else if !s.playlist.picker_ui.selected.contains(&source) {
        s.playlist.picker_ui.selected = vec![source];
        s.playlist.picker_ui.anchor = Some(source);
    }
    s.playlist.source = Some(source);
    s.playlist.audio_template = None;
    if let ClipSource::Pattern(number) = source {
        s.pattern = number;
    }
    app.invalidate();
}

pub fn destination(app: &App, grid: Option<Rect>) -> Option<Destination> {
    let (x, y) = app.mouse;
    let hit = app.top_hit(x, y)?;
    if hit.win != Some(Win::Playlist) {
        return None;
    }
    match hit.action {
        Action::PlPick(source) | Action::PlPickPreview(source) => {
            let row = app.hits.iter().rev().find(|hit| hit.action == Action::PlPick(source))?.rect;
            return Some(Destination::Row(source, y >= row.cy(), row));
        }
        Action::PlPickList => return Some(Destination::End),
        _ => {}
    }
    let grid = grid.filter(|grid| grid.contains(x, y))?;
    let geom = PGeom::new(&app.state, grid);
    let track = geom.track_at(y);
    let tick = pl::snap_place(&app.state, &geom, geom.tick_at(x), app.alt);
    Some(Destination::Playlist { track, tick })
}

pub(crate) fn selected(s: &State, fallback: ClipSource) -> Vec<ClipSource> {
    let picked = &s.playlist.picker_ui.selected;
    if !picked.contains(&fallback) {
        return vec![fallback];
    }
    let kind = s.playlist.picker;
    // Use the visible ordering for this tab, then include picks from other tabs.
    let mut items = rows(s).items.into_iter().filter(|item| picked.contains(item)).collect::<Vec<_>>();
    for other in [PickKind::Pattern, PickKind::Audio] {
        if other != kind {
            items.extend(ordered(s, other).into_iter().filter(|item| picked.contains(item)));
        }
    }
    items
}

pub(crate) fn use_selection_for_delete(s: &State, source: ClipSource, popup: &mut pl::Popup) {
    let picked = &s.playlist.picker_ui.selected;
    if picked.len() < 2 || !picked.contains(&source) {
        return;
    }
    if let Some(item) = popup.items.iter_mut().find(|item| matches!(item.cmd,
        pl::Cmd::Pattern(crate::patterns::Cmd::Delete) | pl::Cmd::Pick(pl::PickCmd::Delete, _)))
    {
        item.cmd = pl::Cmd::DeletePickerSelection(source);
    }
}

pub(crate) fn ask_delete_selection(app: &mut App, source: ClipSource) {
    use crate::dialog::{Answer, Kind, Msg, Then};
    use crate::messages::{self, DELETE_CHANNEL, DELETE_PATTERN};

    let picked = &app.state.playlist.picker_ui.selected;
    if !picked.contains(&source) {
        return;
    }
    let sources = picked.clone();
    let patterns = sources.iter().filter(|item| matches!(item, ClipSource::Pattern(_))).count();
    let channels = sources.len() - patterns;
    if (patterns > 0 && messages::on(&DELETE_PATTERN)) || (channels > 0 && messages::on(&DELETE_CHANNEL)) {
        let msg = format!("Delete {} selected clip sources and their Playlist placements?", sources.len());
        let mut prompt = Msg::new(Kind::Question, "Delete selected clips", vec![(msg, false)],
            &[Answer::Yes, Answer::No], Then::DeletePickerSelection(sources));
        prompt.focus = 1;
        app.show_msg(prompt);
    } else {
        delete_selection(app, &sources);
    }
}

pub(crate) fn delete_selection(app: &mut App, sources: &[ClipSource]) {
    let s = &mut app.state;
    let mut patterns: Vec<_> = sources.iter().filter_map(|src| match src {
        ClipSource::Pattern(n) => Some(*n),
        ClipSource::Channel(_) => None,
    }).collect();
    patterns.sort_unstable();
    patterns.dedup();
    let rows: Vec<_> = s.project.channels.iter().enumerate().filter_map(|(row, channel)| {
        (matches!(channel.kind, os_model::ChannelKind::AudioClip | os_model::ChannelKind::Automation)
            && sources.contains(&ClipSource::Channel(channel.index))).then_some(row)
    }).collect();
    if !rows.is_empty() && rows.len() == s.project.channels.len() {
        s.message = Some("A project keeps at least one channel".into());
        return;
    }
    if patterns.is_empty() && rows.is_empty() {
        return;
    }
    let count = patterns.len() + rows.len();
    let previous = s.history.begin_sequence();
    if !patterns.is_empty() {
        crate::patterns::delete_picker_patterns(s, &patterns);
    }
    if !rows.is_empty() {
        crate::rack::delete_rows(app, &rows);
    }
    let s = &mut app.state;
    s.history.end_sequence_named(previous, "Delete selected clips");
    s.playlist.picker_ui.selected.clear();
    s.playlist.picker_ui.anchor = None;
    s.playlist.picker_ui.stop_previews();
    s.playlist.source = None;
    s.playlist.audio_template = None;
    s.message = Some(format!("Deleted {count} clip sources"));
    app.invalidate();
}

pub fn drop(app: &mut App, source: ClipSource, destination: Option<Destination>) {
    let placement_edge = if matches!(destination, Some(Destination::Playlist { .. })) {
        app.playlist_placement_edge(app.mouse.0)
    } else {
        None
    };
    let s = &mut app.state;
    if !crate::playlist_tools::sources(s, s.playlist.picker).contains(&source) {
        return;
    }
    let items = selected(s, source);
    match destination {
        Some(Destination::Playlist { track, tick }) => {
            let before = pl::clips(s).len();
            match pl::place_picker_sources(s, &items, track, tick) {
                Ok(()) if pl::clips(s).len() > before => {
                    s.playlist.placement_edge = s.playlist.placement_edge.or(placement_edge);
                }
                Ok(()) => {}
                Err(reason) => s.message = Some(reason.into()),
            }
        }
        Some(Destination::Row(target, after, _)) => reorder(s, &items, Some((target, after))),
        Some(Destination::End) => reorder(s, &items, None),
        None => {}
    }
}

fn reorder(s: &mut State, items: &[ClipSource], target: Option<(ClipSource, bool)>) {
    let kind = s.playlist.picker;
    if kind == PickKind::Automation || target.is_some_and(|(target, _)| items.contains(&target)) {
        return;
    }
    let previous = ordered(s, kind);
    let moving: Vec<_> = previous.iter().copied().filter(|item| items.contains(item)).collect();
    let mut next: Vec<_> = previous.iter().copied().filter(|item| !items.contains(item)).collect();
    let index = target.and_then(|(target, after)| next.iter().position(|&item| item == target).map(|i| i + usize::from(after)))
        .unwrap_or(next.len());
    next.splice(index..index, moving);
    if next != previous {
        let before = crate::history::Snap::ClipPickerOrder(s.project.clip_picker_order.clone());
        crate::history::record(s, "Reorder clip picker", before);
        s.project.clip_picker_order.retain(|item| !previous.contains(item));
        s.project.clip_picker_order.extend(next);
    }
}

pub fn toggle_preview(app: &mut App, source: ClipSource) {
    let pattern = matches!(source, ClipSource::Pattern(_));
    let picker = &mut app.state.playlist.picker_ui;
    picker.prune_previews();
    if let Some(index) = picker.previews.iter().position(|(item, _)| *item == source) {
        picker.previews.remove(index).1.store(false, Ordering::Release);
        return;
    }
    if pattern && app.transport.playing() {
        app.state.message = Some("Stop playback to preview a pattern in the Clip Picker.".into());
        return;
    }
    if pattern {
        picker.previews.retain(|(item, active)| {
            if matches!(item, ClipSource::Pattern(_)) {
                active.store(false, Ordering::Release);
                false
            } else {
                true
            }
        });
    }
    if picker.previews.len() >= PREVIEW_LIMIT {
        app.state.message = Some("Stop a clip preview before starting another (8 active previews).".into());
        return;
    }
    crate::browser_preview::dismiss(app);
    let project = preview_project(&app.state, source);
    let active = Arc::new(AtomicBool::new(true));
    app.transport.preview_picker(project, active.clone(), pattern, looping());
    app.state.playlist.picker_ui.previews.push((source, active));
}

pub(crate) fn preview_project(state: &State, source: ClipSource) -> os_model::Project {
    let mut project = crate::rack::play_project(state);
    let clip = os_model::Clip::new(0, pl::source_len(state, source).max(1), 0, source);
    let channels = project.clip_channels(&[clip.clone()]);
    // The engine schedules audio clips and notes by rack slot. Keep those slots
    // intact, while skipping sample loads for channels this preview never plays.
    for channel in &mut project.channels {
        if !channels.contains(&channel.index) {
            channel.sample_path = None;
        }
    }
    project.patterns.retain(|pattern| source == ClipSource::Pattern(pattern.number));
    project.arrangements = vec![os_model::Arrangement { clips: vec![clip], ..Default::default() }];
    project.current_arrangement = None;
    project
}

#[cfg(test)]
pub fn mark_preview(s: &mut State, source: ClipSource) {
    s.playlist.picker_ui.previews.push((source, Arc::new(AtomicBool::new(true))));
}


#[cfg(test)]
mod tests {
    use super::*;
    use os_engine::{Engine, PlayMode, Sample, SampleLibrary};

    #[test]
    fn picker_preview_renders_audio_from_later_rack_slots() {
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        let audio = 3;
        let sampler = 4;
        let key = "clip-picker-preview-test.wav";
        app.state.project.channels[audio].kind = os_model::ChannelKind::AudioClip;
        for slot in [0, audio, sampler] {
            app.state.project.channels[slot].sample_path = Some(key.into());
        }
        app.state.project.patterns[0].notes = vec![os_model::Note {
            length: 96,
            ..os_model::Note::step(sampler as u16, 0)
        }];
        let signal: Vec<f32> = (0..48000).map(|i| 0.25 * (i as f32 * 0.0576).sin()).collect();
        let mut library = SampleLibrary::new(None);
        library.insert_buffer(key.into(), Arc::new(Sample {
            rate: 48000,
            left: signal.clone(),
            right: signal,
        }));
        for source in [ClipSource::Channel(audio as u16), ClipSource::Pattern(1)] {
            let preview = preview_project(&app.state, source);
            assert_eq!(preview.channels.len(), app.state.project.channels.len());
            assert!(preview.channels[0].sample_path.is_none());
            let mut engine = Engine::with_instruments(&preview, &mut library, 48000, PlayMode::Song, &[]);
            engine.set_looping(true);
            let mut output = vec![0.0; 8192];
            engine.process(&mut output);
            assert!(output.iter().any(|sample| sample.abs() > 0.005), "{source:?} rendered silence");
        }
    }
}
