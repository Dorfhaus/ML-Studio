// SPDX-License-Identifier: GPL-3.0-or-later

use super::{Origin, Sources, locked};
use crate::{
    app::{Action, App, State, Ui},
    dialog::{Answer, Kind, Msg, Then},
    history::Snap,
    notation::{Command, Request, edited_note, engrave::Frame},
    piano, playlist as pl,
};
use os_model::{Clip, ClipSource, Note, Pattern};
use os_ui::{Color, Rect};
use std::collections::{BTreeMap, BTreeSet};

type Target = (u16, usize);
type Placements = Vec<(usize, usize, Clip)>;

#[derive(Clone)]
pub(in crate::notation) struct Choice {
    pub(in crate::notation) source: Origin,
    note: Note,
}

impl Choice {
    pub(in crate::notation) fn target(&self) -> Target {
        (self.source.pattern, self.source.index)
    }
    pub fn matches(&self, o: &Origin) -> bool {
        self.source.pattern == o.pattern
            && self.source.index == o.index
            && self.source.channel == o.channel
    }
    pub(in crate::notation) fn locked(&self, s: &State, source: &Sources) -> bool {
        source
            .clips
            .get(&self.source.clip)
            .is_some_and(|clip| locked(s, clip))
    }
}

pub(super) struct Plan {
    arrangement: usize,
    before: BTreeMap<u16, Vec<Note>>,
    after: BTreeMap<u16, Vec<Note>>,
    clips: Placements,
}

pub(in crate::notation) fn choices(s: &State, src: &Sources) -> Vec<Choice> {
    let channel = piano::channel(s);
    let active = s.piano.selection(s.pattern, channel);
    let mut items = BTreeMap::new();
    let existing = s
        .notation
        .song
        .as_ref()
        .into_iter()
        .flat_map(|editor| &editor.selection);
    for item in existing {
        let (pattern, i) = item.target();
        if (pattern != s.pattern || Some(item.source.channel) != channel || active.contains(&i))
            && s.project.pattern(pattern).and_then(|p| p.notes.get(i)) == Some(&item.note)
            && src.notes.iter().any(|o| item.matches(o))
        {
            items.insert(item.target(), item.clone());
        }
    }
    for o in &src.notes {
        if o.pattern == s.pattern && Some(o.channel) == channel && active.contains(&o.index) {
            if let Some(&note) = piano::notes(s)
                .get(o.index)
                .filter(|n| n.channel == o.channel)
            {
                items.entry((o.pattern, o.index)).or_insert_with(|| Choice {
                    source: o.clone(),
                    note,
                });
            }
        }
    }
    items.into_values().collect()
}

pub(in crate::notation) fn selection_count(s: &State, req: &Request) -> usize {
    req.song.as_ref().map_or(0, |src| choices(s, src).len())
}

pub(super) fn select(app: &mut App, src: &Sources, origin: &Origin, hit: usize) {
    let s = &mut app.state;
    let Some(row) = s
        .project
        .channels
        .iter()
        .position(|c| c.index == origin.channel)
    else {
        return;
    };
    let Some(clip) = src.clips.get(&origin.clip) else {
        return;
    };
    let Some(&note) = s
        .project
        .pattern(origin.pattern)
        .and_then(|p| p.notes.get(origin.index))
    else {
        return;
    };
    let mut list = choices(s, src);
    let selected = list.iter().position(|choice| choice.matches(origin));
    let choice = Choice {
        source: origin.clone(),
        note,
    };
    if app.ctrl || app.shift {
        if let Some(i) = selected {
            list.remove(i);
        } else {
            list.push(choice);
        }
    } else {
        if let Some(i) = selected {
            list[i] = choice;
        } else {
            list = vec![choice];
        }
        s.notation.drag = Some((app.mouse.1, hit));
    }
    s.piano.clip_session = None;
    s.pattern = origin.pattern;
    s.selected_channel = row;
    s.rack.select_only(row);
    s.piano.select(
        origin.pattern,
        origin.channel,
        list.iter()
            .filter(|o| o.source.pattern == origin.pattern && o.source.channel == origin.channel)
            .map(|o| o.source.index)
            .collect(),
    );
    let editor = s.notation.song.as_mut().unwrap();
    editor.selection = list;
    editor.anchor = Some((origin.clip, clip.clone()));
    app.invalidate();
}

fn placements(s: &State, patterns: &BTreeMap<u16, Vec<Note>>) -> Placements {
    s.project
        .arrangements
        .iter()
        .enumerate()
        .flat_map(|(ai, arrangement)| {
            arrangement
                .clips
                .iter()
                .enumerate()
                .filter_map(move |(ci, clip)| {
                    matches!(clip.source, ClipSource::Pattern(p) if patterns.contains_key(&p))
                        .then(|| (ai, ci, clip.clone()))
                })
        })
        .collect()
}

pub(in crate::notation) fn submit(app: &mut App, after: BTreeMap<u16, Vec<Note>>) {
    if after.is_empty() {
        return;
    }
    let s = &app.state;
    let before = after
        .keys()
        .map(|&id| (id, s.project.pattern(id).unwrap().notes.clone()))
        .collect();
    let clips = placements(s, &after);
    let mut count = BTreeMap::<u16, usize>::new();
    for (_, _, clip) in &clips {
        if let ClipSource::Pattern(id) = clip.source {
            *count.entry(id).or_default() += 1;
        }
    }
    let plan = Plan {
        arrangement: s.project.arrangement_index(),
        before,
        after,
        clips,
    };
    if !count.values().any(|&n| n > 1) {
        commit(app, plan);
        return;
    }
    let shared = plan
        .clips
        .iter()
        .filter(|(_, _, c)| matches!(c.source, ClipSource::Pattern(p) if count[&p] > 1));
    let total = shared.clone().count();
    let mut lines = vec![(
        format!(
            "This updates shared patterns used by {total} Playlist placements. Use Unique first to edit just one."
        ),
        false,
    )];
    for (i, _, clip) in shared.take(10) {
        let arrangement = &s.project.arrangements[*i];
        let title = arrangement.name.as_deref().unwrap_or("Arrangement");
        let row = arrangement
            .tracks
            .get(usize::from(clip.track))
            .and_then(|r| r.name.as_deref());
        let ClipSource::Pattern(id) = clip.source else {
            continue;
        };
        let name = crate::patterns::name_of(s, id);
        let regions = s.project.marker_bars(&arrangement.markers);
        let mut bars = 0;
        for (ix, &(start, len, _)) in regions.iter().enumerate() {
            if let Some(next) = regions.get(ix + 1).filter(|next| next.0 <= clip.position) {
                bars += (next.0 - start).div_ceil(len.max(1));
            } else {
                bars += (clip.position - start) / len.max(1);
                break;
            }
        }
        lines.push((
            format!(
                "\n{name} · {title} · {} · Bar {}",
                row.map(str::to_owned)
                    .unwrap_or_else(|| format!("Track {}", clip.track + 1)),
                bars + 1
            ),
            false,
        ));
    }
    if total > 10 {
        lines.push((format!("\nAnd {} more placements.", total - 10), false));
    }
    app.state.notation.song.as_mut().unwrap().pending = Some(plan);
    app.show_msg(
        Msg::new(
            Kind::Question,
            "Edit shared patterns",
            lines,
            &[Answer::Yes, Answer::Cancel],
            Then::ScoreEdit,
        )
        .labelled(&["Apply", "Cancel"]),
    );
}

pub(crate) fn confirm_edit(app: &mut App, apply: bool) {
    let Some(plan) = app
        .state
        .notation
        .song
        .as_mut()
        .and_then(|editor| editor.pending.take())
    else {
        return;
    };
    if apply {
        commit(app, plan);
    }
}

fn commit(app: &mut App, plan: Plan) {
    let s = &mut app.state;
    if plan.arrangement != s.project.arrangement_index()
        || placements(s, &plan.after) != plan.clips
        || plan
            .before
            .iter()
            .any(|(&id, notes)| s.project.pattern(id).is_none_or(|p| &p.notes != notes))
    {
        s.message = Some("The arrangement changed. Select the notes and try again.".into());
        app.invalidate();
        return;
    }
    let old: Vec<_> = plan
        .before
        .keys()
        .map(|&id| (id, s.project.pattern_length(id)))
        .collect();
    let snaps = old.iter().map(|&(id, _)| Snap::pattern(s, id)).collect();
    pl::snapshot_with(s, "Edit arrangement score", Snap::Several(snaps));
    for ((id, notes), (_, len)) in plan.after.into_iter().zip(old) {
        crate::patterns::pat_mut(s, id).notes = notes;
        pl::refit_pattern_clips(s, id, len);
    }
    s.piano.clear_selection();
    s.notation.song.as_mut().unwrap().selection.clear();
    s.notation.checked = None;
    app.invalidate();
}

pub(super) fn edit(app: &mut App, req: &Request, cmd: Command) {
    if !matches!(
        cmd,
        Command::Pitch(_)
            | Command::Move(_)
            | Command::Length(_)
            | Command::Velocity(_)
            | Command::Delete
    ) {
        return;
    }
    let s = &app.state;
    let src = req.song.as_ref().unwrap();
    let list = choices(s, src);
    let mut changes: BTreeMap<u16, BTreeMap<usize, Option<Note>>> = BTreeMap::new();
    for choice in list {
        let origin = src
            .notes
            .iter()
            .find(|o| o.clip == choice.source.clip && choice.matches(o))
            .or_else(|| src.notes.iter().find(|o| choice.matches(o)))
            .unwrap();
        let clip = &src.clips[&origin.clip];
        if locked(s, clip) {
            app.state.message =
                Some("Unlock or unfreeze the selected Playlist tracks first.".into());
            app.invalidate();
            return;
        }
        let step = ((req.ppq * req.grid) as f64 / 8.0 * clip.pattern_rate())
            .round()
            .max(1.) as u32;
        let length = if choice.note.length == 0 {
            (req.ppq / 4).max(1)
        } else {
            choice.note.length
        };
        let new = edited_note(choice.note, length, step, cmd);
        if cmd == Command::Delete || new != choice.note {
            changes
                .entry(origin.pattern)
                .or_default()
                .insert(origin.index, (cmd != Command::Delete).then_some(new));
        }
    }
    let after = changes
        .into_iter()
        .map(|(p, changes)| {
            let mut notes: Vec<_> = s
                .project
                .pattern(p)
                .unwrap()
                .notes
                .iter()
                .enumerate()
                .filter_map(|(i, &note)| changes.get(&i).copied().unwrap_or(Some(note)))
                .collect();
            notes.sort_by_key(|n| (n.position, n.channel));
            (p, notes)
        })
        .collect();
    submit(app, after);
}

pub(in crate::notation) fn entry_active(s: &State) -> bool {
    crate::workspace_visibility::shown(crate::notation::ENTRY_VISIBILITY)
        && s.notation.song.as_ref().is_some_and(|editor| editor.entry)
}

pub(in crate::notation) fn entry_hits(
    ui: &mut Ui,
    req: &Request,
    frame: &Frame,
    sheet: Rect,
    scale: f32,
    entry: bool,
) {
    if req.song.is_none()
        || !entry && !crate::workspace_visibility::shown(crate::notation::MARKS_VISIBILITY)
    {
        return;
    }
    let step = (req.ppq * req.grid.max(1) / 8).max(1);
    let center = |r: Rect| sheet.y + (r.y + r.h / 2.) * scale;
    for (i, &(number, staff, bounds)) in frame.staves.iter().enumerate() {
        let Some(bar) = req.bars.iter().find(|bar| bar.number == number) else {
            continue;
        };
        let lines = Rect::new(
            sheet.x + bounds.x * scale,
            sheet.y + bounds.y * scale,
            bounds.w * scale,
            bounds.h * scale,
        );
        let mut top = lines.y - lines.h * 0.75;
        let mut bottom = lines.bottom() + lines.h * 0.75;
        if let Some((_, _, previous)) = i
            .checked_sub(1)
            .and_then(|j| frame.staves.get(j))
            .filter(|s| s.0 == number)
        {
            top = top.max((center(bounds) + center(*previous)) / 2.);
        }
        if let Some((_, _, next)) = frame.staves.get(i + 1).filter(|s| s.0 == number) {
            bottom = bottom.min((center(bounds) + center(*next)) / 2.);
        }
        let hit = Rect::new(lines.x, top, lines.w, bottom - top);
        if !entry {
            ui.hit(
                hit,
                Action::Score(Command::Staff(staff, number)),
                "Right-click for staff notation settings",
            );
            continue;
        }
        if ui.hit(
            hit,
            Action::Score(Command::Place(staff, number, hit, lines)),
            "Click a staff position to enter a note",
        ) {
            let ink = Color::rgb(0x3f81bf);
            let span = bar.end - bar.start;
            let pos = offset(ui.mouse.0, hit, span, step);
            let x = hit.x + hit.w * (pos as f32 + step as f32 / 2.) / span as f32;
            let y = lines.bottom() - staff_step(ui.mouse.1, lines) * (lines.h / 8.).max(1.);
            ui.c.stroke_rrect(hit.inset(1.), 1., 1., ink.with_alpha(150));
            ui.c.fill_rrect(
                Rect::new(x.min(hit.right() - 5.) - 4., y - 3., 8., 6.),
                3.,
                ink,
            );
        }
    }
}

fn staff_step(y: f32, lines: Rect) -> f32 {
    ((lines.bottom() - y) / (lines.h / 8.).max(1.)).round()
}

fn pitch(clef: os_model::notation::Clef, key_signature: i8, y: f32, lines: Rect) -> (u16, u8) {
    use crate::notation::written::{NATURAL, alteration};
    use os_model::notation::Clef;
    let bottom = match clef {
        Clef::Treble => 30,
        Clef::Bass => 18,
        Clef::Alto => 24,
        Clef::Tenor => 22,
    };
    let diatonic = bottom + staff_step(y, lines) as i32;
    let octave = diatonic.div_euclid(7);
    let step = diatonic.rem_euclid(7) as usize;
    let semitone = NATURAL[step] + alteration(key_signature, step);
    (
        ((octave + 1) * 12 + semitone).clamp(0, 127) as u16,
        step as u8 + 1,
    )
}

fn offset(x: f32, hit: Rect, span: u32, step: u32) -> u32 {
    let pos = (x - hit.x) / hit.w;
    ((pos * span as f32 / step as f32).floor().max(0.) as u32)
        .saturating_mul(step)
        .min(span.saturating_sub(1))
}

pub(super) fn place(
    app: &mut App,
    req: &Request,
    staff: usize,
    number: u32,
    hit: Rect,
    lines: Rect,
) {
    let s = &app.state;
    if !entry_active(s) || !hit.contains(app.mouse.0, app.mouse.1) {
        return;
    }
    let Some(instrument) = req.song.as_ref().and_then(|song| song.staves.get(staff)) else {
        return;
    };
    let Some(bar) = req.bars.iter().find(|b| b.number == number) else {
        return;
    };
    let step = (req.ppq * req.grid.max(1) / 8).max(1);
    let offset = offset(app.mouse.0, hit, bar.end - bar.start, step);
    let (clef, signature) = req.staff_style(staff, bar.start);
    insert(
        app,
        instrument.channel,
        bar.start + offset,
        bar.start,
        bar.end,
        step,
        pitch(clef, signature, app.mouse.1, lines),
    );
}

fn insert(
    app: &mut App,
    channel: u16,
    tick: u32,
    bar_start: u32,
    bar_end: u32,
    step: u32,
    (key, letter): (u16, u8),
) {
    let s = &app.state;
    let clips = pl::clips(s);
    let Some(a) = s.project.arrangement() else {
        return;
    };
    let in_scope = |row| {
        !s.notation.song.as_ref().unwrap().tracks || s.playlist.header_selection.contains(&row)
    };
    let carries = |clip: &Clip, pattern| {
        in_scope(clip.track)
            && (s
                .project
                .pattern(pattern)
                .is_some_and(|p| p.notes.iter().any(|n| n.channel == channel))
                || s.project
                    .instrument_lanes
                    .iter()
                    .any(|l| *l == (channel, a.number, clip.track, pattern)))
    };
    let plays = os_engine::engine::arrangement_pattern_plays(&s.project);
    let candidates: Vec<_> = plays
        .iter()
        .filter(|p| {
            let clip = &clips[p.clip];
            clip.position <= tick && u64::from(tick) < p.starts_until && carries(clip, p.pattern)
        })
        .collect();
    let anchor = s
        .notation
        .song
        .as_ref()
        .and_then(|e| e.anchor.as_ref())
        .filter(|(i, c)| clips.get(*i) == Some(c))
        .map(|(i, _)| *i);
    let selected = candidates
        .iter()
        .copied()
        .find(|p| Some(p.clip) == anchor)
        .or_else(|| (candidates.len() == 1).then(|| candidates[0]));
    if let Some(p) = selected {
        let clip = &clips[p.clip];
        if locked(s, clip) {
            app.state.message =
                Some("Unlock or unfreeze this Playlist track to enter notes.".into());
            app.invalidate();
            return;
        }
        let position = p
            .from
            .saturating_add((f64::from(tick - clip.position) * p.rate).round() as u64)
            .min(p.from + p.len.saturating_sub(1))
            .min(u64::from(u32::MAX)) as u32;
        let length = (f64::from(step) * p.rate).round().max(1.) as u32;
        let mut notes = s.project.pattern(p.pattern).unwrap().notes.clone();
        notes.push(Note {
            notation: os_model::notation::NoteMarks {
                letter,
                ..Default::default()
            },
            key,
            length,
            ..Note::step(channel, position)
        });
        notes.sort_by_key(|n| (n.position, n.channel));
        submit(app, [(p.pattern, notes)].into());
        return;
    }
    if !candidates.is_empty() {
        app.state.message = Some("Several patterns overlap here. Select a note in the intended clip, then enter the note.".into());
        app.invalidate();
        return;
    }
    let mut rows: BTreeSet<_> = plays
        .iter()
        .filter(|p| carries(&clips[p.clip], p.pattern))
        .map(|p| clips[p.clip].track)
        .collect();
    rows.extend(
        s.project
            .instrument_lanes
            .iter()
            .filter_map(|&(ch, arr, row, _)| {
                (ch == channel && arr == a.number && in_scope(row)).then_some(row)
            }),
    );
    let row = rows.into_iter().find(|&row| {
        pl::track_info(s, row)
            .is_some_and(|t| t.enabled && !t.locked && t.frozen.is_none() && !t.folder)
            && !clips
                .iter()
                .any(|c| c.track == row && c.position < bar_end && c.end() > bar_start)
    });
    let number = (1..=crate::patterns::MAX).find(|&id| {
        s.project.pattern(id).is_none()
            && !s
                .project
                .arrangements
                .iter()
                .flat_map(|a| &a.clips)
                .any(|c| c.source == ClipSource::Pattern(id))
    });
    let (Some(row), Some(number)) = (row, number) else {
        app.state.message =
            Some("This measure needs a free instrument row and pattern number.".into());
        app.invalidate();
        return;
    };
    let s = &mut app.state;
    let history = Snap::Patterns {
        patterns: s.project.patterns.clone(),
        current: s.pattern,
    };
    pl::snapshot_with(s, "Enter arrangement note", history);
    let name = s.project.channel(channel).and_then(|ch| ch.name.clone());
    let note = Note {
        notation: os_model::notation::NoteMarks {
            letter,
            ..Default::default()
        },
        key,
        length: step.min(bar_end - tick).max(1),
        ..Note::step(channel, tick - bar_start)
    };
    s.project.patterns.push(Pattern {
        number,
        name,
        length: Some(bar_end - bar_start),
        notes: vec![note],
        ..Default::default()
    });
    let arr = pl::arr_mut(s);
    arr.clips.push(Clip::new(
        bar_start,
        bar_end - bar_start,
        row,
        ClipSource::Pattern(number),
    ));
    arr.clips.sort_by_key(|c| c.position);
    s.piano.dirty = true;
    s.notation.checked = None;
    app.invalidate();
}
