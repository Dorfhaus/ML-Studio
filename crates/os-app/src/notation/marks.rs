// SPDX-License-Identifier: GPL-3.0-or-later

use super::{Command, MARKS_VISIBILITY, Request, arrangement, snapshot, written};
use crate::{
    app::{App, State},
    history::{self, Snap},
    playlist::{Cmd, PItem, Popup},
};
use os_model::{
    Note,
    notation::{Clef, StaffChange, Tuplet},
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Edit {
    Letter(u8),
    Accidental(bool),
    Articulation(u8),
    Dynamic(u8),
    Tuplet(u8, u8, u8),
    Key(u16, u32, Option<i8>),
    Clef(u16, u32, Option<Clef>),
}

fn pick(label: impl Into<std::borrow::Cow<'static, str>>, e: Edit) -> PItem {
    PItem::new(label, Cmd::Score(Command::SetMarks(e)))
}

fn staff_items(req: &Request, staff: usize, number: u32) -> Option<Vec<PItem>> {
    let bar = req.bars.iter().find(|bar| bar.number == number)?;
    let part = req.song.as_ref()?.staves.get(staff)?;
    let (channel, tick) = (part.channel, bar.start);
    let key_names = [
        "C♭ / A♭ minor",
        "G♭ / E♭ minor",
        "D♭ / B♭ minor",
        "A♭ / F minor",
        "E♭ / C minor",
        "B♭ / G minor",
        "F / D minor",
        "C / A minor",
        "G / E minor",
        "D / B minor",
        "A / F♯ minor",
        "E / C♯ minor",
        "B / G♯ minor",
        "F♯ / D♯ minor",
        "C♯ / A♯ minor",
    ];
    let mut keys = vec![pick(
        "Remove change at this bar",
        Edit::Key(channel, tick, None),
    )];
    for (i, name) in key_names.into_iter().enumerate() {
        keys.push(pick(name, Edit::Key(channel, tick, Some(i as i8 - 7))));
    }
    let mut clefs = vec![pick(
        "Remove change at this bar",
        Edit::Clef(channel, tick, None),
    )];
    for (name, clef) in [
        ("Treble", Clef::Treble),
        ("Bass", Clef::Bass),
        ("Alto", Clef::Alto),
        ("Tenor", Clef::Tenor),
    ] {
        clefs.push(pick(name, Edit::Clef(channel, tick, Some(clef))));
    }
    Some(vec![
        PItem::head(format!("{} · Bar {number}", part.label)),
        PItem::sub("Key signature", keys),
        PItem::sub("Clef", clefs),
    ])
}

pub(super) fn menu(app: &mut App, target: Option<(usize, u32)>) {
    if !crate::workspace_visibility::shown(MARKS_VISIBILITY) {
        return;
    }
    let Some(req) = snapshot(&app.state) else {
        return;
    };
    if app.state.notation.started.as_ref() != Some(&req) {
        return;
    }
    let selected = super::selected(&app.state, &req);
    let mut items = Vec::new();
    if target.is_none() && !selected.is_empty() {
        let mut spelling = vec![
            pick("Automatic spelling", Edit::Letter(0)),
            pick("Show accidental", Edit::Accidental(true)),
            pick("Automatic accidental", Edit::Accidental(false)),
        ];
        let pitch = req
            .notes
            .iter()
            .find(|n| selected.contains(&n.index))
            .map(|n| n.note.key);
        if let Some(key) = pitch {
            for (i, letter) in written::LETTERS.iter().enumerate() {
                let marks = os_model::notation::NoteMarks {
                    letter: i as u8 + 1,
                    ..Default::default()
                };
                let (step, alter, ..) = written::pitch(key, marks, 0);
                if i != step {
                    continue;
                }
                let sign = ["𝄫", "♭", "", "♯", "𝄪"][(alter + 2) as usize];
                spelling.push(pick(
                    format!("Spell as {letter}{sign}"),
                    Edit::Letter(i as u8 + 1),
                ));
            }
        }
        items.extend([
            PItem::sub("Pitch spelling", spelling),
            PItem::sub(
                "Dynamics",
                ["None", "ppp", "pp", "p", "mp", "mf", "f", "ff", "fff"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| pick(v, Edit::Dynamic(i as u8)))
                    .collect(),
            ),
            PItem::sub(
                "Articulation",
                [
                    "None",
                    "Staccato",
                    "Accent",
                    "Tenuto",
                    "Staccatissimo",
                    "Marcato",
                ]
                .into_iter()
                .enumerate()
                .map(|(i, v)| pick(v, Edit::Articulation(i as u8)))
                .collect(),
            ),
            PItem::sub(
                "Tuplets",
                vec![
                    pick("Triplet eighths · 3:2", Edit::Tuplet(3, 2, 8)),
                    pick("Quintuplet sixteenths · 5:4", Edit::Tuplet(5, 4, 16)),
                    pick("Septuplet sixteenths · 7:4", Edit::Tuplet(7, 4, 16)),
                    pick("Remove tuplet notation", Edit::Tuplet(0, 0, 0)),
                ],
            ),
        ]);
    }
    let on_note = req
        .notes
        .iter()
        .find(|n| selected.contains(&n.index))
        .and_then(|n| {
            let index = req.bars.iter().find(|b| b.end > n.note.position)?.number;
            Some((n.staff, index))
        });
    if let Some((staff, bar)) = target
        .or(on_note)
        .or_else(|| req.bars.first().map(|b| (0, b.number)))
        && let Some(staff) = staff_items(&req, staff, bar)
    {
        if !items.is_empty() {
            items.push(PItem::sep());
        }
        items.extend(staff);
    }
    if items.is_empty() {
        items.push(PItem::head(
            "Select a note, or open Arrangement mode for staff settings",
        ));
    }
    app.state.playlist.popup = Some(Popup {
        x: app.mouse.0,
        y: app.mouse.1,
        items,
        sub: None,
        scroll: [0; 2],
        cascade: Vec::new(),
    });
    app.invalidate();
}

fn update_staff(s: &mut State, edit: Edit) {
    let (channel, position) = match edit {
        Edit::Key(ch, pos, _) | Edit::Clef(ch, pos, _) => (ch, pos),
        _ => return,
    };
    let i = s.project.arrangement_index();
    let Some(a) = s.project.arrangements.get(i) else {
        return;
    };
    let mut changes = a.staff_notation.clone();
    let mut change = StaffChange {
        channel,
        position,
        ..Default::default()
    };
    changes.retain(|m| {
        if (m.channel, m.position) != (channel, position) {
            return true;
        }
        change.fifths = m.fifths.or(change.fifths);
        change.clef = m.clef.or(change.clef);
        false
    });
    match edit {
        Edit::Key(_, _, key) => change.fifths = key,
        Edit::Clef(_, _, clef) => change.clef = clef,
        _ => return,
    }
    if change.clef.is_some() || change.fifths.is_some() {
        changes.push(change);
    }
    changes.sort_by_key(|m| (m.position, m.channel));
    if changes != a.staff_notation {
        let before = a.staff_notation.clone();
        history::record(s, "Edit staff notation", Snap::StaffNotation(i, before));
        s.project.arrangements[i].staff_notation = changes;
    }
}

fn write_notes(
    notes: &mut [Note],
    indices: &[usize],
    edit: Edit,
    ppq: u32,
    bar_end: u32,
) -> Result<(), &'static str> {
    if let Edit::Tuplet(actual, normal, value) = edit
        && actual > 0
    {
        let mut beats = BTreeMap::<u32, Vec<usize>>::new();
        let channel = notes[indices[0]].channel;
        for &i in indices {
            if notes[i].channel != channel {
                return Err("Select a tuplet within one instrument and pattern.");
            }
            beats.entry(notes[i].position).or_default().push(i);
        }
        if beats.len() != usize::from(actual) {
            return Err(
                "Select the same number of note starts as the tuplet numerator. Chords count as one start.",
            );
        }
        let start = *beats.first_key_value().unwrap().0;
        let last = *beats.last_key_value().unwrap().0;
        let num = written::nominal(u32::from(value), 0, ppq) * u32::from(normal);
        let at = |i| (num * i + u32::from(actual) / 2) / u32::from(actual);
        let end = start
            .checked_add(num)
            .ok_or("The tuplet is past the end of the timeline.")?;
        if end > bar_end || last >= bar_end {
            return Err("Keep the selected group within one measure.");
        }
        if num < u32::from(actual)
            || notes.iter().enumerate().any(|(i, n)| {
                n.channel == channel
                    && !indices.contains(&i)
                    && n.position >= start
                    && n.position < end.max(last.saturating_add(1))
            })
        {
            return Err("Select consecutive note starts with room for the tuplet.");
        }
        for (i, selected) in beats.values().enumerate() {
            let i = i as u32;
            for &n in selected {
                notes[n].position = start + at(i);
                notes[n].length = at(i + 1) - at(i);
                notes[n].notation.tuplet = Tuplet {
                    actual,
                    normal,
                    value,
                    dots: 0,
                    start: i == 0,
                    end: i + 1 == u32::from(actual),
                };
            }
        }
        return Ok(());
    }
    for &index in indices {
        let note = &mut notes[index];
        match edit {
            Edit::Letter(letter) => {
                let mut notation = note.notation;
                notation.letter = letter;
                if letter != 0 && written::pitch(note.key, notation, 0).0 != usize::from(letter - 1)
                {
                    return Err(
                        "That letter cannot spell each selected pitch with a standard accidental.",
                    );
                }
                note.notation = notation;
            }
            Edit::Accidental(value) => note.notation.accidental = value,
            Edit::Articulation(value) => note.notation.articulation = value,
            Edit::Dynamic(value) => note.notation.dynamic = value,
            Edit::Tuplet(..) => note.notation.tuplet = Tuplet::default(),
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn apply(app: &mut App, edit: Edit) {
    if !crate::workspace_visibility::shown(MARKS_VISIBILITY) {
        return;
    }
    let s = &mut app.state;
    let Some(req) = snapshot(s) else {
        return;
    };
    if s.notation.started.as_ref() != Some(&req) {
        return;
    }
    if matches!(edit, Edit::Clef(..) | Edit::Key(..)) {
        update_staff(s, edit);
        s.notation.checked = None;
        app.invalidate();
        return;
    }
    let mut groups = BTreeMap::<u16, Vec<usize>>::new();
    if let Some(song) = &req.song {
        let notes = arrangement::note_choices(s, song);
        if notes.iter().any(|n| n.locked(s, song)) {
            s.message = Some("Unlock the selected Playlist tracks first.".into());
            app.invalidate();
            return;
        }
        for choice in notes {
            let (pattern, index) = choice.target();
            groups.entry(pattern).or_default().push(index);
        }
    } else {
        let notes = super::selected(s, &req);
        if !notes.is_empty() {
            groups.insert(req.pattern, notes);
        }
    }
    let mut after = BTreeMap::new();
    for (pattern, indices) in groups {
        let Some(before) = s.project.pattern(pattern) else {
            return;
        };
        let mut notes = before.notes.clone();
        let bar_end = indices
            .iter()
            .map(|&i| notes[i].position)
            .min()
            .map_or(u32::MAX, |tick| {
                s.project.pattern_bar_end(pattern, tick.saturating_add(1))
            });
        if let Err(message) = write_notes(&mut notes, &indices, edit, req.ppq, bar_end) {
            s.message = Some(message.into());
            app.invalidate();
            return;
        }
        notes.sort_by_key(|n| n.position);
        if notes != before.notes {
            after.insert(pattern, notes);
        }
    }
    if req.song.is_some() {
        arrangement::submit_notes(app, after);
    } else if let Some(notes) = after.remove(&req.pattern) {
        let pattern = req.pattern;
        let old = s.project.pattern_length(pattern);
        let snap = Snap::pattern(s, pattern);
        crate::playlist::snapshot_with(s, "Edit written notation", snap);
        crate::patterns::pat_mut(s, pattern).notes = notes;
        crate::playlist::refit_pattern_clips(s, pattern, old);
        s.notation.checked = None;
        if matches!(edit, Edit::Tuplet(..)) {
            s.piano.clear_selection();
        }
        app.invalidate();
    }
}

pub(crate) fn context(app: &mut App, command: Command) -> bool {
    if !crate::workspace_visibility::shown(MARKS_VISIBILITY) {
        return false;
    }
    match command {
        Command::Staff(staff, bar) => menu(app, Some((staff, bar))),
        Command::Note(index) => {
            if snapshot(&app.state)
                .is_some_and(|req| !super::selected(&app.state, &req).contains(&index))
            {
                super::run(app, command);
                app.state.notation.drag = None;
            }
            menu(app, None);
        }
        _ => return false,
    }
    true
}
