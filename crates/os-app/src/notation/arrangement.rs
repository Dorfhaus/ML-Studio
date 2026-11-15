// SPDX-License-Identifier: GPL-3.0-or-later

//! Playlist score projection. A score hit identifies the source note and the
//! particular Playlist occurrence used to display it.

use super::{
    ARRANGEMENT_VISIBILITY, BARS, Command, MAX_NOTES, PartNote, Request, engrave, page_bars,
};
use crate::{
    app::{Action, App, State, Ui, Win},
    piano, playlist as pl,
};
use os_model::{Clip, ClipSource};
use os_ui::{Color, Font, Rect};
use std::collections::{BTreeMap, BTreeSet};

mod compose;

const STAFF_COUNT: usize = 4;

pub(super) struct Editor {
    follow: bool,
    tracks: bool,
    staff_page: usize,
    playhead: Option<u32>,
    arrangement: usize,
    anchor: Option<(usize, Clip)>,
    selection: Vec<compose::Choice>,
    entry: bool,
    pending: Option<compose::Plan>,
}

impl Editor {
    fn new(arrangement: usize) -> Self {
        Self {
            follow: true,
            tracks: false,
            staff_page: 0,
            playhead: None,
            arrangement,
            anchor: None,
            selection: Vec::new(),
            entry: false,
            pending: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Staff {
    pub label: String,
    pub bass: bool,
    pub channel: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Origin {
    clip: usize,
    pattern: u16,
    index: usize,
    channel: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Sources {
    pub staves: Vec<Staff>,
    total: usize,
    first: usize,
    arrangement: usize,
    clips: BTreeMap<usize, Clip>,
    notes: Vec<Origin>,
}

impl Sources {
    pub fn selected(&self, s: &State) -> Vec<usize> {
        let selected = compose::choices(s, self);
        self.notes
            .iter()
            .enumerate()
            .filter_map(|(hit, src)| {
                selected
                    .iter()
                    .any(|choice| choice.matches(src))
                    .then_some(hit)
            })
            .collect()
    }

    pub fn status(&self) -> String {
        if self.total == 0 {
            return "   •   No instruments in scope".into();
        }
        format!(
            "   •   Instruments {}–{} of {}",
            self.first + 1,
            self.first + self.staves.len(),
            self.total
        )
    }
}

fn regions(s: &State) -> Vec<(u32, u32, (u16, u16))> {
    s.project.marker_bars(pl::markers(s))
}

fn page_at(regions: &[(u32, u32, (u16, u16))], tick: u32) -> usize {
    let mut bars = 0;
    for (i, &(start, len, _)) in regions.iter().enumerate() {
        if let Some(&next) = regions
            .get(i + 1)
            .map(|m| &m.0)
            .filter(|&&next| tick >= next)
        {
            bars += (next - start).div_ceil(len.max(1)) as usize;
        } else {
            bars += (tick.saturating_sub(start) / len.max(1)) as usize;
            break;
        }
    }
    (bars / BARS).min(16_383)
}

pub(super) fn tick(app: &mut App) {
    if app.state.notation.song.is_none() {
        return;
    }
    if !crate::workspace_visibility::shown(ARRANGEMENT_VISIBILITY) {
        command(app, Command::Mode);
        return;
    }
    let arrangement = app.state.project.arrangement_index();
    let editor = app.state.notation.song.as_mut().unwrap();
    if editor.arrangement != arrangement {
        editor.arrangement = arrangement;
        editor.anchor = None;
        editor.selection.clear();
        editor.staff_page = 0;
        app.state.notation.page = 0;
        app.state.notation.checked = None;
    }
    let tick = (app.transport.song()
        && !matches!(app.transport.state(), crate::transport::State::Stopped))
    .then(|| app.transport.tick().max(0.) as u32);
    if editor.playhead.map(|t| t / 2) == tick.map(|t| t / 2) {
        return;
    }
    editor.playhead = tick;
    if editor.follow {
        if let Some(tick) = tick {
            let page = page_at(&regions(&app.state), tick);
            if page != app.state.notation.page {
                app.state.notation.page = page;
                app.state.notation.checked = None;
            }
        }
    }
    if !app.state.native_detached.contains(&Win::Score) {
        app.damage_win(Win::Score);
    }
}

pub(super) fn snapshot(s: &State, editor: &Editor) -> Option<Request> {
    project(s, editor, page_bars(&regions(s), s.notation.page), false)
}

pub(super) fn whole_score(s: &State) -> Result<Request, String> {
    let editor = s
        .notation
        .song
        .as_ref()
        .ok_or("Open Arrangement mode to export its score.")?;
    let regions = regions(s);
    let plays = os_engine::engine::arrangement_pattern_plays(&s.project);
    let clips = pl::clips(s);
    let end = plays
        .iter()
        .filter(|p| !editor.tracks || s.playlist.header_selection.contains(&clips[p.clip].track))
        .map(|p| {
            let tail = s.project.pattern(p.pattern).map_or(0, |pattern| {
                pattern
                    .notes
                    .iter()
                    .filter(|note| {
                        !note.muted() && u64::from(note.position) < p.from.saturating_add(p.len)
                    })
                    .map(|note| p.song_tick(u64::from(note.position) + u64::from(note.length)))
                    .max()
                    .unwrap_or(0)
            });
            if s.project.play_truncated {
                p.end
            } else {
                p.end.max(tail)
            }
        })
        .max()
        .unwrap_or(0);
    if end == 0 {
        return Err("There are no playable Playlist patterns to export.".into());
    }
    let last = page_at(
        &regions,
        end.saturating_sub(1).min(u64::from(u32::MAX)) as u32,
    );
    if last >= 256 {
        return Err("Score export supports up to 1,024 measures.".into());
    }
    let mut bars: Vec<_> = (0..=last)
        .flat_map(|page| page_bars(&regions, page))
        .collect();
    bars.retain(|b| u64::from(b.start) < end);
    let req = project(s, editor, bars, true).ok_or("There is no score to export.")?;
    if req.notes.is_empty() {
        return Err("There are no notes in the selected Playlist rows.".into());
    }
    if req.notes.len() > 200_000 {
        return Err("Score export supports up to 200,000 notes.".into());
    }
    Ok(req)
}

fn project(s: &State, editor: &Editor, bars: Vec<super::Measure>, full: bool) -> Option<Request> {
    let (start, end) = (u64::from(bars.first()?.start), u64::from(bars.last()?.end));
    let ppq = u32::from(s.project.ppq.max(4));
    let grid = super::GRIDS[s.notation.resolution];
    let clips = pl::clips(s);
    let plays = os_engine::engine::arrangement_pattern_plays(&s.project);
    let in_scope =
        |i: usize| !editor.tracks || s.playlist.header_selection.contains(&clips[i].track);
    let patterns: BTreeSet<_> = plays
        .iter()
        .filter(|p| in_scope(p.clip))
        .map(|p| p.pattern)
        .collect();
    // Instrument order and clefs come from the selection across the song so paging does not shuffle the score.
    let mut pitch = BTreeMap::<u16, (u64, u64)>::new();
    for pat in s
        .project
        .patterns
        .iter()
        .filter(|pat| patterns.contains(&pat.number))
    {
        for n in pat.notes.iter().filter(|n| !n.muted()) {
            let (sum, count) = pitch.entry(n.channel).or_default();
            *sum += u64::from(n.key);
            *count += 1;
        }
    }
    if let Some(a) = s.project.arrangement() {
        for &(channel, number, row, _) in &s.project.instrument_lanes {
            if a.number == number
                && (!editor.tracks || s.playlist.header_selection.contains(&row))
                && pl::track_on(s, row)
            {
                pitch.entry(channel).or_default();
            }
        }
    }
    let available = s
        .project
        .channels
        .iter()
        .filter(|ch| pitch.contains_key(&ch.index))
        .collect::<Vec<_>>();
    let total = available.len();
    let first = if full {
        0
    } else {
        editor.staff_page.min(total.saturating_sub(1) / STAFF_COUNT) * STAFF_COUNT
    };
    let group = available
        .into_iter()
        .skip(first)
        .take(if full { total } else { STAFF_COUNT })
        .collect::<Vec<_>>();
    let staves = group
        .iter()
        .map(|ch| {
            let (sum, count) = pitch[&ch.index];
            Staff {
                label: ch
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("Instrument {}", ch.index + 1)),
                bass: sum < 60 * count,
                channel: ch.index,
            }
        })
        .collect();
    let mut source = Sources {
        staves,
        total,
        first,
        arrangement: s.project.arrangement_index(),
        clips: BTreeMap::new(),
        notes: Vec::new(),
    };
    let mut notes = Vec::new();
    let mut plays = plays
        .into_iter()
        .filter(|p| in_scope(p.clip))
        .collect::<Vec<_>>();
    plays.sort_by_key(|p| (clips[p.clip].position, p.clip));
    'clips: for play in plays {
        if u64::from(clips[play.clip].position) >= end {
            break;
        }
        if s.project.play_truncated && play.end <= start {
            continue;
        }
        let Some(pattern) = s.project.pattern(play.pattern) else {
            continue;
        };
        let window_end = play.from.saturating_add(play.len);
        for (i, &note) in pattern.notes.iter().enumerate().filter(|(_, n)| !n.muted()) {
            let Some(staff) = group.iter().position(|ch| ch.index == note.channel) else {
                continue;
            };
            let source_start = u64::from(note.position);
            if source_start >= window_end {
                continue;
            }
            let length = if note.length == 0 {
                (ppq / 4).max(1)
            } else {
                note.length
            };
            let source_end = source_start + u64::from(length);
            if source_start < play.from
                && !(s.project.play_truncated && note.length > 0 && source_end > play.from)
            {
                continue;
            }
            let first_tick = play.song_tick(source_start.max(play.from));
            if first_tick >= play.starts_until || first_tick >= end {
                continue;
            }
            let last_tick = play.song_tick(source_end).max(first_tick + 1);
            let last_tick = if s.project.play_truncated {
                last_tick.min(play.end)
            } else {
                last_tick
            };
            if last_tick <= start || last_tick <= first_tick {
                continue;
            }
            source.notes.push(Origin {
                clip: play.clip,
                pattern: play.pattern,
                index: i,
                channel: note.channel,
            });
            source
                .clips
                .entry(play.clip)
                .or_insert_with(|| clips[play.clip].clone());
            notes.push(PartNote {
                index: notes.len(),
                staff,
                note: os_model::Note {
                    position: first_tick as u32,
                    ..note
                },
                length: (last_tick - first_tick).min(u64::from(u32::MAX - first_tick as u32))
                    as u32,
            });
            if notes.len() > if full { 200_000 } else { MAX_NOTES } {
                break 'clips;
            }
        }
    }
    Some(Request {
        pattern: 0,
        channel: 0,
        ppq,
        grid,
        bars,
        notes,
        song: Some(source),
        staff_notation: s
            .project
            .arrangement()
            .map_or_else(Vec::new, |a| a.staff_notation.clone()),
        tempo: s
            .project
            .arrangement()
            .map_or_else(Vec::new, |a| a.timeline.tempo.clone()),
    })
}

pub(super) fn single_part(req: &Request, i: usize) -> Request {
    let mut part = req.clone();
    let source = part.song.as_mut().unwrap();
    source.staves = vec![source.staves[i].clone()];
    part.notes.retain_mut(|n| {
        if n.staff != i {
            return false;
        }
        n.staff = 0;
        true
    });
    part
}

pub(super) fn locked(s: &State, clip: &Clip) -> bool {
    pl::track_info(s, clip.track).is_some_and(|track| track.locked || track.frozen.is_some())
}

pub(super) fn command(app: &mut App, cmd: Command) -> bool {
    if cmd == Command::Mode {
        let s = &mut app.state;
        if s.notation.song.is_none() && !crate::workspace_visibility::shown(ARRANGEMENT_VISIBILITY)
        {
            return true;
        }
        s.notation.song = if s.notation.song.is_some() {
            None
        } else {
            Some(Editor::new(s.project.arrangement_index()))
        };
        s.notation.page = 0;
        s.notation.drag = None;
        s.notation.checked = None;
        app.invalidate();
        return true;
    }
    if app.state.notation.song.is_none() {
        return false;
    }
    let s = &mut app.state;
    let editor = s.notation.song.as_mut().unwrap();
    match cmd {
        Command::Page(delta) => {
            editor.follow = false;
            s.notation.page = s
                .notation
                .page
                .saturating_add_signed(delta as isize)
                .min(16_383);
        }
        Command::Instruments(delta) => {
            let max = s
                .notation
                .latest
                .as_ref()
                .and_then(|r| r.song.as_ref())
                .map_or(0, |src| src.total.saturating_sub(1) / STAFF_COUNT);
            editor.staff_page = editor
                .staff_page
                .min(max)
                .saturating_add_signed(delta as isize)
                .min(max);
        }
        Command::Follow => {
            editor.follow = !editor.follow;
            if editor.follow {
                if let Some(tick) = editor.playhead {
                    s.notation.page = page_at(&regions(s), tick);
                }
            }
        }
        Command::Tracks => {
            editor.tracks = !editor.tracks;
            editor.staff_page = 0;
            editor.anchor = None;
        }
        Command::Quantize => {
            s.notation.resolution = (s.notation.resolution + 1) % super::GRIDS.len()
        }
        Command::Blank => {
            s.piano.clear_selection();
            editor.anchor = None;
            editor.selection.clear();
        }
        Command::Add | Command::Entry
            if crate::workspace_visibility::shown(super::ENTRY_VISIBILITY) =>
        {
            editor.entry = !editor.entry
        }
        _ => return edit(app, cmd),
    }
    app.state.notation.checked = None;
    app.invalidate();
    true
}

fn edit(app: &mut App, cmd: Command) -> bool {
    let s = &mut app.state;
    let Some(req) = super::snapshot(s) else {
        return true;
    };
    if s.notation.frame.is_none() || s.notation.started.as_ref() != Some(&req) {
        return true;
    }
    let Some(src) = req.song.as_ref() else {
        return true;
    };
    if let Command::Note(hit) = cmd {
        if let Some(note) = src.notes.get(hit) {
            compose::select(app, src, note, hit);
        }
        return true;
    }
    if let Command::Place(staff, number, hit, lines) = cmd {
        compose::place(app, &req, staff, number, hit, lines);
        return true;
    }
    if cmd != Command::MakeUnique {
        compose::edit(app, &req, cmd);
        return true;
    }
    let Some(channel) = piano::channel(s) else {
        return true;
    };
    let selected = s.piano.selection(s.pattern, Some(channel));
    let Some((clip_index, clip)) = s
        .notation
        .song
        .as_ref()
        .and_then(|editor| editor.anchor.as_ref())
        .filter(|(i, clip)| {
            src.clips.get(i) == Some(clip)
                && src.notes.iter().any(|n| {
                    n.clip == *i
                        && n.pattern == s.pattern
                        && n.channel == channel
                        && selected.contains(&n.index)
                })
        })
    else {
        s.message = Some("Click a note in the Playlist occurrence you want to make unique.".into());
        app.invalidate();
        return true;
    };
    let clip_index = *clip_index;
    if locked(s, clip) {
        s.message = Some("Unlock or unfreeze this Playlist track to edit its score.".into());
        app.invalidate();
        return true;
    }
    if s.project
        .patterns
        .iter()
        .any(|p| p.number >= crate::patterns::MAX)
    {
        s.message = Some("No spare pattern number is available.".into());
    } else {
        let message = pl::run(app, pl::Cmd::ClipUnique(clip_index));
        let s = &mut app.state;
        s.piano.select(s.pattern, channel, selected);
        let editor = s.notation.song.as_mut().unwrap();
        editor.anchor = None;
        editor.selection.clear();
        s.notation.checked = None;
        s.message = message;
    }
    app.invalidate();
    true
}

pub(crate) use compose::confirm_edit;
pub(super) use compose::{choices as note_choices, submit as submit_notes};
pub(super) use compose::{entry_active, entry_hits, selection_count};

pub(super) fn anchored(s: &State, req: &Request, hit: usize) -> bool {
    let Some((clip, original)) = s
        .notation
        .song
        .as_ref()
        .and_then(|editor| editor.anchor.as_ref())
    else {
        return false;
    };
    req.song.as_ref().is_some_and(|source| {
        source.notes.get(hit).is_some_and(|note| note.clip == *clip)
            && source.clips.get(clip) == Some(original)
    })
}

pub(super) fn controls(ui: &mut Ui, s: &State, client: Rect) {
    if !crate::workspace_visibility::shown(ARRANGEMENT_VISIBILITY) {
        return;
    }
    let mut x = client.right() - 10.;
    let mut button = |label: &str, w: f32, command, on, hint: &'static str| {
        x -= w;
        let r = Rect::new(x, client.y + 4., w, 23.);
        let hot = ui.hit(r, Action::Score(command), hint);
        ui.c.fill_rrect(
            r,
            4.,
            if on {
                ui.t.pill
            } else if hot {
                ui.t.button_hi
            } else {
                ui.t.toolbar
            },
        );
        ui.c.text_center(
            Font::new(13),
            r,
            if on { ui.t.pill_text } else { ui.t.text },
            label,
        );
        x -= 4.;
    };
    let song = s.notation.song.as_ref();
    button(
        if song.is_some() {
            "Arrangement"
        } else {
            "Pattern"
        },
        100.,
        Command::Mode,
        song.is_some(),
        "Switch between the active pattern and the Playlist arrangement",
    );
    if let Some(song) = song {
        button(
            "Follow",
            60.,
            Command::Follow,
            song.follow,
            "Follow the song playhead across pages",
        );
        button(
            if song.tracks {
                "Selected rows"
            } else {
                "All rows"
            },
            100.,
            Command::Tracks,
            song.tracks,
            "Limit the score to selected Playlist track headers",
        );
        button(
            "Parts ›",
            52.,
            Command::Instruments(1),
            false,
            "Next four instruments",
        );
        button(
            "‹ Parts",
            52.,
            Command::Instruments(-1),
            false,
            "Previous four instruments",
        );
    }
}

pub(super) fn paint_playhead(
    ui: &mut Ui,
    s: &State,
    req: &Request,
    frame: &engrave::Frame,
    sheet: Rect,
    scale: f32,
) {
    if req.song.is_none() {
        return;
    }
    let Some(tick) = s.notation.song.as_ref().and_then(|mode| mode.playhead) else {
        return;
    };
    for (&(number, bounds), bar) in frame.bars.iter().filter_map(|entry| {
        req.bars
            .iter()
            .find(|b| b.number == entry.0)
            .map(|bar| (entry, bar))
    }) {
        let _ = number;
        if tick >= bar.start && tick < bar.end {
            let phase = (tick - bar.start) as f32 / (bar.end - bar.start) as f32;
            ui.c.fill_rect(
                Rect::new(
                    sheet.x + (bounds.x + bounds.w * phase) * scale,
                    sheet.y + bounds.y * scale,
                    2.,
                    bounds.h * scale,
                ),
                Color::rgb(0x3794e4).with_alpha(178),
            );
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use os_model::{Note, Pattern};

    fn fixture() -> App {
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state.project.play_truncated = true;
        app.state.project.channels[0].name = Some("Low & <keys>".into());
        app.state.project.patterns = vec![Pattern {
            number: 1,
            length: Some(384),
            notes: vec![
                Note {
                    key: 50,
                    length: 96,
                    ..Note::step(0, 0)
                },
                Note {
                    key: 69,
                    length: 60,
                    ..Note::step(1, 96)
                },
            ],
            ..Pattern::default()
        }];
        let mut slice = Clip::new(768, 96, 1, ClipSource::Pattern(1));
        slice.window = Some((48., 240.));
        slice.pattern_speed = 2.;
        let mut muted = Clip::new(1152, 384, 0, ClipSource::Pattern(1));
        muted.muted = true;
        pl::arr_mut(&mut app.state).clips =
            vec![Clip::new(0, 384, 0, ClipSource::Pattern(1)), slice, muted];
        command(&mut app, Command::Mode);
        app
    }

    #[test]
    fn arrangement_score_maps_trim_stretch_mute_selection_and_unique() {
        let mut app = fixture();
        let req = super::super::snapshot(&app.state).unwrap();
        let song = req.song.as_ref().unwrap();
        assert_eq!(song.staves.len(), 2);
        assert_eq!(
            req.notes
                .iter()
                .map(|n| (n.staff, n.note.position, n.length))
                .collect::<Vec<_>>(),
            [(0, 0, 96), (1, 96, 60), (0, 768, 24), (1, 792, 30)]
        );
        let mei = engrave::test_markup(&req);
        assert!(mei.contains("Low &amp; &lt;keys&gt;"));
        assert!(mei.contains("score-measure-4"));
        if engrave::available() {
            let frame = engrave::render(&req).unwrap();
            assert!(frame.notes.iter().any(|n| n.0 == 3));
            assert_eq!(frame.bars.len(), 4);
            app.state.notation.frame = Some(frame);
        } else {
            app.state.notation.frame = Some(engrave::Frame {
                w: 1,
                h: 1,
                pixels: vec![0; 4],
                notes: Vec::new(),
                bars: Vec::new(),
                staves: Vec::new(),
            });
        }
        app.state.notation.started = Some(req.clone());
        command(&mut app, Command::Note(3));
        assert_eq!(piano::channel(&app.state), Some(1));
        assert_eq!(song.selected(&app.state), [1, 3]);
        command(&mut app, Command::Pitch(2));
        assert_eq!(piano::notes(&app.state)[1].key, 69);
        confirm_edit(&mut app, true);
        assert_eq!(
            piano::notes(&app.state)[1].key,
            71,
            "only one shared source note changes"
        );
        command(&mut app, Command::Pitch(2));
        assert_eq!(
            piano::notes(&app.state)[1].key,
            71,
            "stale score hits cannot change the updated source"
        );
        let updated = super::super::snapshot(&app.state).unwrap();
        assert_eq!(updated.notes[1].note.key, 71);
        assert_eq!(updated.notes[3].note.key, 71);
        app.state.notation.started = Some(updated);
        command(&mut app, Command::Note(3));
        command(&mut app, Command::MakeUnique);
        let now = pl::clips(&app.state);
        assert_eq!(now[0].source, ClipSource::Pattern(1));
        let ClipSource::Pattern(unique) = now[1].source else {
            panic!("pattern")
        };
        assert_ne!(unique, 1);
        assert_eq!(
            (now[1].position, now[1].window, now[1].pattern_speed),
            (768, Some((48., 240.)), 2.)
        );
        let req = super::super::snapshot(&app.state).unwrap();
        app.state.notation.started = Some(req);
        command(&mut app, Command::Note(3));
        command(&mut app, Command::Pitch(-1));
        assert_eq!(app.state.project.pattern(1).unwrap().notes[1].key, 71);
        assert_eq!(app.state.project.pattern(unique).unwrap().notes[1].key, 70);
        piano::undo(&mut app.state);
        assert_eq!(app.state.project.pattern(unique).unwrap().notes[1].key, 71);
        piano::undo(&mut app.state);
        assert_eq!(pl::clips(&app.state)[1].source, ClipSource::Pattern(1));
        assert!(app.state.project.pattern(unique).is_none());

        app.state.playlist.header_selection = vec![1];
        command(&mut app, Command::Tracks);
        let selected = super::super::snapshot(&app.state).unwrap();
        assert_eq!(
            selected
                .notes
                .iter()
                .map(|n| n.note.position)
                .collect::<Vec<_>>(),
            [768, 792]
        );
    }

    #[test]
    fn score_composition_previews_repeated_clips_and_edits_two_patterns_in_one_undo() {
        let mut app = fixture();
        let pat = app.state.project.pattern(1).unwrap().clone();
        let mut second = pat;
        second.number = 2;
        app.state.project.patterns.push(second);
        pl::arr_mut(&mut app.state)
            .clips
            .push(Clip::new(396, 360, 2, ClipSource::Pattern(2)));
        let req = super::super::snapshot(&app.state).unwrap();
        let song = req.song.as_ref().unwrap();
        let first = song
            .notes
            .iter()
            .position(|n| n.pattern == 1 && n.channel == 0)
            .unwrap();
        let second = song
            .notes
            .iter()
            .position(|n| n.pattern == 2 && n.channel == 1)
            .unwrap();
        app.state.notation.started = Some(req.clone());
        app.state.notation.frame = Some(engrave::Frame {
            w: 1,
            h: 1,
            pixels: vec![0; 4],
            notes: Vec::new(),
            bars: Vec::new(),
            staves: Vec::new(),
        });
        command(&mut app, Command::Note(first));
        app.ctrl = true;
        command(&mut app, Command::Note(second));
        app.ctrl = false;
        let original = app.state.project.patterns.clone();
        command(&mut app, Command::Pitch(3));
        assert!(app.state.notation.song.as_ref().unwrap().pending.is_some());
        assert_eq!(app.state.project.patterns, original);
        app.answer(crate::dialog::Reply::Button(crate::dialog::Answer::Cancel));
        assert_eq!(app.state.project.patterns, original);
        command(&mut app, Command::Pitch(3));
        app.answer(crate::dialog::Reply::Button(crate::dialog::Answer::Yes));
        assert_eq!(app.state.project.pattern(1).unwrap().notes[0].key, 53);
        assert_eq!(app.state.project.pattern(2).unwrap().notes[1].key, 72);
        piano::undo(&mut app.state);
        assert_eq!(app.state.project.patterns, original);
    }

    #[test]
    fn score_entry_uses_engraved_staff_bounds_and_exports_individual_print_parts() {
        if !engrave::available() {
            return;
        }
        let mut app = fixture();
        let req = super::super::snapshot(&app.state).unwrap();
        let frame = engrave::render(&req).unwrap();
        assert_eq!(
            frame.staves.len(),
            8,
            "two editable staff hit areas in each of four measures"
        );
        app.state.notation.started = Some(req.clone());
        app.state.notation.latest = Some(req);
        app.state.notation.frame = Some(frame);
        app.state.raise(crate::app::Win::Score);
        app.state.native_view = Some(crate::app::Win::Score);
        command(&mut app, Command::Entry);
        let mut canvas = os_ui::Canvas::new(1100, 650, 1.).unwrap();
        app.paint(&mut canvas, None);
        let (action, x, y) = app
            .hits
            .iter()
            .find_map(|h| match h.action {
                Action::Score(Command::Place(0, 2, hit, lines)) => {
                    Some((h.action.clone(), hit.x + hit.w / 8., lines.bottom()))
                }
                _ => None,
            })
            .expect("note entry hit on the bass staff of measure two");
        app.mouse = (x, y);
        app.act(action);
        let placed = pl::clips(&app.state)
            .iter()
            .find(|c| c.position == 384 && c.track == 0)
            .unwrap();
        let ClipSource::Pattern(id) = placed.source else {
            panic!("pattern")
        };
        assert_eq!(app.state.project.pattern(id).unwrap().notes[0].key, 43);
        piano::undo(&mut app.state);
        assert!(app.state.project.pattern(id).is_none());
        let full = whole_score(&app.state).unwrap();
        let xml = super::super::export::musicxml(&full, "Export & verify").unwrap();
        assert!(xml.contains("<score-partwise version=\"4.0\">"));
        assert!(xml.contains("Test") || xml.contains("Low &amp; &lt;keys&gt;"));
        assert_eq!(xml.matches("<part id=").count(), 2);
        let html = super::super::export::html(&full, "Export & verify").unwrap();
        assert_eq!(html.matches("<section>").count(), 2);
        assert_eq!(html.matches("data:image/svg+xml;base64,").count(), 2);
        if let Some(root) = std::env::var_os("LEMON_SCORE_TEST_EXPORTS") {
            let root = std::path::PathBuf::from(root);
            std::fs::write(root.join("integration.musicxml"), xml).unwrap();
            std::fs::write(root.join("integration-parts.html"), html).unwrap();
        }
    }

    #[test]
    fn arrangement_score_follows_pages_across_meter_changes() {
        let regions = [(0, 384, (4, 4)), (768, 288, (3, 4))];
        assert_eq!(page_at(&regions, 1343), 0);
        assert_eq!(page_at(&regions, 1344), 1);
        assert_eq!(page_bars(&regions, 1)[0].meter, (3, 4));
    }
}
