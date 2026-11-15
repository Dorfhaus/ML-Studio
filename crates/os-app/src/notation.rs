// SPDX-License-Identifier: GPL-3.0-or-later

//! A score projection of the active Piano Roll channel. Engraving is a worker
//! task; every musical edit is made in the existing pattern on the UI thread.

mod arrangement;
mod engrave;
mod export;
mod import;
mod marks;
mod written;
pub(crate) use arrangement::confirm_edit;
pub(crate) use export::file_done as export_done;
pub(crate) use import::file_done as import_done;
pub(crate) use marks::context as context_menu;

use crate::{
    app::{Action, App, State, Ui, Win},
    piano, views,
};
use os_model::Note;
use os_ui::{Color, Font, Rect};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
use winit::keyboard::{Key, NamedKey};

pub const VISIBILITY: &str = "visible_score_editor";
pub const ENTRY_VISIBILITY: &str = "visible_score_note_entry";
pub const EXPORT_VISIBILITY: &str = "visible_score_export";
pub const MARKS_VISIBILITY: &str = "visible_score_notation_tools";
pub const IMPORT_VISIBILITY: &str = "visible_score_import";
pub const ARRANGEMENT_VISIBILITY: &str = "visible_arrangement_score";
pub const SHORTCUT_VISIBILITY: &str = "visible_win-score-editor";

pub(crate) fn shortcut_shown() -> bool {
    available()
        && crate::workspace_visibility::shown(VISIBILITY)
        && crate::workspace_visibility::shown(SHORTCUT_VISIBILITY)
}
pub(crate) fn available() -> bool {
    engrave::available()
}

pub(crate) fn project_changed(app: &mut App) {
    let editor = &mut app.state.notation;
    editor.import = None;
    editor.song = None;
    editor.started = None;
    editor.latest = None;
    editor.checked = None;
}

const BARS: usize = 4;
const MAX_NOTES: usize = 640;
const GRIDS: [u32; 4] = [2, 1, 4, 0];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Page(i32),
    Quantize,
    Note(usize),
    Blank,
    Pitch(i32),
    Move(i32),
    Length(bool),
    Velocity(i32),
    Add,
    Delete,
    Mode,
    Follow,
    Tracks,
    Instruments(i32),
    MakeUnique,
    Entry,
    Place(usize, u32, Rect, Rect),
    Export(bool),
    Import,
    Marks,
    Staff(usize, u32),
    SetMarks(marks::Edit),
}

#[derive(Clone, Debug, PartialEq)]
struct Measure {
    start: u32,
    end: u32,
    number: u32,
    meter: (u16, u16),
}

#[derive(Clone, Debug, PartialEq)]
struct PartNote {
    index: usize,
    note: Note,
    length: u32,
    staff: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct Request {
    pattern: u16,
    channel: u16,
    ppq: u32,
    grid: u32,
    bars: Vec<Measure>,
    notes: Vec<PartNote>,
    song: Option<arrangement::Sources>,
    staff_notation: Vec<os_model::notation::StaffChange>,
    tempo: Vec<os_model::timeline::TempoPoint>,
}

impl Request {
    fn staff_style(&self, staff: usize, tick: u32) -> (os_model::notation::Clef, i8) {
        use os_model::notation::Clef;
        if let Some(staff) = self.song.as_ref().and_then(|song| song.staves.get(staff)) {
            written::settings(
                &self.staff_notation,
                staff.channel,
                tick,
                if staff.bass { Clef::Bass } else { Clef::Treble },
            )
        } else {
            (if staff == 0 { Clef::Treble } else { Clef::Bass }, 0)
        }
    }
}

#[derive(Default)]
pub struct Editor {
    page: usize,
    resolution: usize,
    context: Option<(u16, u16)>,
    latest: Option<Request>,
    started: Option<Request>,
    frame: Option<engrave::Frame>,
    worker: Option<mpsc::Receiver<Result<engrave::Frame, String>>>,
    problem: Option<String>,
    checked: Option<Instant>,
    drag: Option<(f32, usize)>,
    song: Option<arrangement::Editor>,
    export: Option<export::Job>,
    import: Option<import::Job>,
}

fn page_bars(changes: &[(u32, u32, (u16, u16))], page: usize) -> Vec<Measure> {
    let mut skip = page.saturating_mul(BARS);
    let mut number = 1u32;
    let mut out = Vec::with_capacity(BARS);
    for (i, &(first, length, meter)) in changes.iter().enumerate() {
        let stop = changes.get(i + 1).map_or(u32::MAX, |x| x.0);
        let length = length.max(1);
        let count = (stop - first).div_ceil(length) as usize;
        let advance = skip.min(count);
        skip -= advance;
        number = number.saturating_add(advance as u32);
        for offset in advance..count.min(advance + (BARS - out.len())) {
            let start = first.saturating_add((offset as u32).saturating_mul(length));
            out.push(Measure {
                start,
                end: start.saturating_add(length).min(stop),
                number,
                meter,
            });
            number = number.saturating_add(1);
        }
        if out.len() == BARS {
            break;
        }
    }
    out
}

fn snapshot(s: &State) -> Option<Request> {
    if let Some(editor) = &s.notation.song {
        return arrangement::snapshot(s, editor);
    }
    let channel = piano::channel(s)?;
    let bars = page_bars(&s.project.pattern_bars(s.pattern), s.notation.page);
    let (start, end) = (bars.first()?.start, bars.last()?.end);
    let ppq = u32::from(s.project.ppq.max(4));
    let grid = GRIDS[s.notation.resolution]; // eighths of a quarter note, or exact ticks at zero
    let mut notes = Vec::new();
    for (index, &note) in piano::notes(s).iter().enumerate() {
        if note.position >= end {
            break;
        }
        if note.channel != channel {
            continue;
        }
        let length = piano::shown_len(s, &note).max(1);
        if note.position.saturating_add(length) > start {
            notes.push(PartNote {
                index,
                note,
                length,
                staff: usize::from(note.key < 60),
            });
            if notes.len() > MAX_NOTES {
                break;
            }
        }
    }
    Some(Request {
        pattern: s.pattern,
        channel,
        ppq,
        grid,
        bars,
        notes,
        song: None,
        staff_notation: Vec::new(),
        tempo: Vec::new(),
    })
}

pub fn tick(app: &mut App) {
    export::poll(app);
    import::poll(app);
    if !app.state.shown(Win::Score) {
        let score = &mut app.state.notation;
        if score.problem.is_some() {
            score.started = None;
        }
        return;
    }
    if !crate::workspace_visibility::shown(VISIBILITY) {
        app.state.hide(Win::Score);
        app.invalidate();
        return;
    }
    arrangement::tick(app);
    let context = app
        .state
        .notation
        .song
        .is_none()
        .then(|| piano::channel(&app.state).map(|ch| (app.state.pattern, ch)))
        .flatten();
    let editor = &mut app.state.notation;
    if editor.context != context {
        editor.context = context;
        if editor.song.is_none() {
            editor.page = 0;
        }
        editor.drag = None;
        editor.checked = None;
    }
    if editor
        .checked
        .is_some_and(|t| t.elapsed() < Duration::from_millis(75))
    {
        return;
    }
    editor.checked = Some(Instant::now());
    let request = snapshot(&app.state);
    let editor = &mut app.state.notation;
    let mut repaint = editor.latest != request;
    editor.latest = request;
    if let Some(job) = &editor.worker {
        match job.try_recv() {
            Ok(Ok(frame)) => {
                if editor.started == editor.latest {
                    editor.frame = Some(frame);
                    editor.problem = None;
                    repaint = true;
                }
                editor.worker = None;
            }
            Ok(Err(error)) => {
                if editor.started == editor.latest {
                    editor.problem = Some(error);
                    repaint = true;
                }
                editor.worker = None;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                editor.problem =
                    Some("The notation renderer stopped. Reopen the Score Editor to retry.".into());
                editor.worker = None;
                repaint = true;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    if editor.worker.is_none() && editor.latest != editor.started {
        editor.frame = None;
        editor.problem = None;
        editor.started = editor.latest.clone();
        if let Some(request) = editor.latest.clone() {
            let (tx, rx) = mpsc::channel();
            editor.worker = Some(rx);
            std::thread::spawn(move || {
                let _ = tx.send(engrave::render(&request));
                os_host::wake();
            });
        }
        repaint = true;
    }
    if repaint {
        app.invalidate();
    }
}

fn selected(s: &State, req: &Request) -> Vec<usize> {
    if let Some(song) = &req.song {
        return song.selected(s);
    }
    let notes = piano::notes(s);
    let mut sel = s.piano.selection(req.pattern, Some(req.channel));
    sel.retain(|&i| notes.get(i).is_some_and(|note| note.channel == req.channel));
    sel.sort_unstable();
    sel.dedup();
    sel
}

pub fn run(app: &mut App, cmd: Command) {
    match cmd {
        Command::Import => return import::dialog(app),
        Command::Marks => return marks::menu(app, None),
        Command::Staff(..) => return run(app, Command::Blank),
        Command::SetMarks(edit) => return marks::apply(app, edit),
        _ => {}
    }
    if let Command::Export(print) = cmd {
        export::dialog(app, print);
        return;
    }
    if arrangement::command(app, cmd) {
        return;
    }
    let s = &mut app.state;
    let Some(req) = snapshot(s) else { return };
    match cmd {
        Command::Page(delta) => {
            s.notation.page = s
                .notation
                .page
                .saturating_add_signed(delta as isize)
                .min(16_383);
            s.notation.checked = None;
        }
        Command::Quantize => {
            s.notation.resolution = (s.notation.resolution + 1) % GRIDS.len();
            s.notation.checked = None;
        }
        Command::Blank => s.piano.clear_selection(),
        Command::Note(index) => {
            if s.notation.started.as_ref() != Some(&req)
                || s.notation.frame.is_none()
                || !req.notes.iter().any(|note| note.index == index)
            {
                return;
            }
            let mut selection = selected(s, &req);
            if app.ctrl || app.shift {
                if let Some(i) = selection.iter().position(|&i| i == index) {
                    selection.remove(i);
                } else {
                    selection.push(index);
                }
            } else {
                if !selection.contains(&index) {
                    selection = vec![index];
                }
                s.notation.drag = Some((app.mouse.1, index));
            }
            s.piano.select(req.pattern, req.channel, selection);
        }
        edit => apply(s, &req, edit),
    }
    app.invalidate();
}

fn apply(s: &mut State, req: &Request, edit: Command) {
    let selection = selected(s, req);
    let step = (req.ppq * req.grid / 8).max(1);
    if edit == Command::Add {
        let at = selection
            .first()
            .and_then(|&i| piano::notes(s).get(i))
            .map_or(req.bars[0].start, |note| note.position.saturating_add(step));
        let at = at
            .max(req.bars[0].start)
            .min(req.bars.last().unwrap().end - 1);
        let key = selection
            .first()
            .and_then(|&i| piano::notes(s).get(i))
            .map_or(60, |note| note.key);
        piano::snapshot_as(s, "Add score note");
        let pattern = piano::pattern_mut(s);
        let index = pattern.notes.len();
        pattern.notes.push(Note {
            key,
            length: step,
            ..Note::step(req.channel, at)
        });
        s.piano.select(req.pattern, req.channel, vec![index]);
        piano::finish_edit(s);
        s.notation.checked = None;
        return;
    }
    edit_notes(s, selection, step, edit);
}

fn edited_note(mut note: Note, length: u32, step: u32, edit: Command) -> Note {
    match edit {
        Command::Pitch(n) => note.key = (i32::from(note.key) + n).clamp(0, 127) as u16,
        Command::Move(n) => {
            note.position = note
                .position
                .saturating_add_signed(n * step as i32)
                .min(u32::MAX - 65536)
        }
        Command::Length(longer) => {
            note.length = if longer {
                length.saturating_mul(2).min(u32::MAX - note.position)
            } else {
                (length / 2).max(1)
            }
        }
        Command::Velocity(n) => note.velocity = (i32::from(note.velocity) + n).clamp(0, 128) as u8,
        _ => {}
    }
    note
}

fn edit_notes(s: &mut State, selection: Vec<usize>, step: u32, edit: Command) {
    if selection.is_empty() {
        return;
    }
    if edit == Command::Delete {
        piano::snapshot_as(s, "Delete score notes");
        let mut at = 0usize;
        piano::pattern_mut(s).notes.retain(|_| {
            let keep = selection.binary_search(&at).is_err();
            at += 1;
            keep
        });
        s.piano.clear_selection();
        piano::finish_edit(s);
        s.notation.checked = None;
        return;
    }
    let changes = selection
        .into_iter()
        .filter_map(|i| {
            let note = piano::notes(s)[i];
            let new = edited_note(note, piano::shown_len(s, &note).max(1), step, edit);
            (new != note).then_some((i, new))
        })
        .collect::<Vec<_>>();
    if changes.is_empty() {
        return;
    }
    piano::snapshot_as(s, "Edit score notes");
    let pattern = piano::pattern_mut(s);
    for (i, note) in changes {
        pattern.notes[i] = note;
    }
    piano::finish_edit(s);
    s.notation.checked = None;
}

pub fn finish_drag(app: &mut App) {
    let Some((start, index)) = app.state.notation.drag.take() else {
        return;
    };
    let semitones = ((start - app.mouse.1) / 8.).round().clamp(-48., 48.) as i32;
    if semitones == 0 || app.state.native_view.is_some_and(|w| w != Win::Score) {
        return;
    }
    let Some(req) = snapshot(&app.state) else {
        return;
    };
    if app.state.notation.started.as_ref() == Some(&req)
        && selected(&app.state, &req).contains(&index)
    {
        run(app, Command::Pitch(semitones));
        app.invalidate();
    }
}

pub fn key(app: &mut App, key: &Key, ctrl: bool, alt: bool) -> bool {
    let s = &app.state;
    if !s.shown(Win::Score) || s.playlist.popup.is_some() || s.menu_open.is_some() || ctrl || alt {
        return false;
    }
    let focus = s.native_view.or_else(|| {
        s.z.last()
            .copied()
            .filter(|w| !s.native_detached.contains(w))
    });
    if focus != Some(Win::Score) {
        return false;
    }
    let cmd = match key {
        Key::Named(NamedKey::ArrowUp) => Command::Pitch(if app.shift { 12 } else { 1 }),
        Key::Named(NamedKey::ArrowDown) => Command::Pitch(if app.shift { -12 } else { -1 }),
        Key::Named(NamedKey::ArrowRight) => Command::Move(1),
        Key::Named(NamedKey::ArrowLeft) => Command::Move(-1),
        Key::Named(NamedKey::PageUp) => Command::Page(-1),
        Key::Named(NamedKey::PageDown) => Command::Page(1),
        Key::Named(NamedKey::Insert) => Command::Add,
        Key::Named(NamedKey::Delete) => Command::Delete,
        Key::Named(NamedKey::Escape) if app.state.notation.drag.is_some() => {
            app.state.notation.drag = None;
            return true;
        }
        _ => return false,
    };
    run(app, cmd);
    true
}

pub fn paint(ui: &mut Ui, s: &State, outer: Rect) {
    let (_, client) = views::framed(ui, s, Win::Score, outer, "Score Editor", 14., true);
    views::resize_zones(ui, s, Win::Score, outer);
    if client.h < 80. {
        return;
    }
    let editor = &s.notation;
    let font = Font::new(14);
    ui.c.fill_rect(client, ui.t.sunken);
    let label = s
        .project
        .channels
        .get(s.selected_channel)
        .and_then(|c| c.name.as_deref())
        .unwrap_or("Instrument");
    ui.c.text(
        font,
        client.x + 12.,
        client.y + 8.,
        ui.t.text,
        &if editor.song.is_some() {
            "Arrangement Score".into()
        } else {
            format!("{label}   •   {}", crate::patterns::name_of(s, s.pattern))
        },
    );
    let width = |label: &str| if label.chars().count() <= 2 { 28. } else { 55. };
    arrangement::controls(ui, s, client);
    let mut x = client.x + 10.;
    let mut button = |label: &str, command: Command, y: f32, hint: &'static str| {
        let r = Rect::new(x, client.y + y, width(label), 24.);
        let hot = ui.hit(r, Action::Score(command), hint);
        ui.c.fill_rrect(r, 4., if hot { ui.t.sunken } else { ui.t.toolbar });
        ui.c.text_center(font, r, ui.t.text, label);
        x += r.w + 4.;
    };
    button("‹", Command::Page(-1), 31., "Previous four measures");
    button("›", Command::Page(1), 31., "Next four measures");
    button(
        ["1/16", "1/32", "1/8", "Exact"][editor.resolution],
        Command::Quantize,
        31.,
        "Change display quantization",
    );
    if editor.song.is_some() {
        if crate::workspace_visibility::shown(ENTRY_VISIBILITY) {
            button(
                if arrangement::entry_active(s) {
                    "Entering"
                } else {
                    "Entry"
                },
                Command::Entry,
                31.,
                "Toggle staff note entry",
            );
        }
        button(
            "Unique",
            Command::MakeUnique,
            31.,
            "Make the selected Playlist occurrence unique",
        );
    } else {
        button("Add", Command::Add, 31., "Add a note after the selection");
    }
    button("Delete", Command::Delete, 31., "Delete selected notes");
    button(
        "½ time",
        Command::Length(false),
        31.,
        "Halve selected note lengths",
    );
    button(
        "2× time",
        Command::Length(true),
        31.,
        "Double selected note lengths",
    );
    button(
        "Vel −",
        Command::Velocity(-5),
        31.,
        "Decrease selected note velocity",
    );
    button(
        "Vel +",
        Command::Velocity(5),
        31.,
        "Increase selected note velocity",
    );
    let paper = Rect::new(
        client.x + 10.,
        client.y + 65.,
        client.w - 20.,
        client.h - 101.,
    );
    if paper.w < 80. || paper.h < 60. {
        return;
    }
    export::controls(ui, s, paper);
    ui.c.fill_rrect(paper, 6., Color::rgb(0xf8f5ed));
    ui.hit(paper, Action::Score(Command::Blank), "Clear note selection");
    if let Some(error) = &editor.problem {
        ui.c.text_center(font, paper, Color::rgb(0x38322c), error);
        return;
    }
    let (Some(req), Some(frame)) = (&editor.latest, &editor.frame) else {
        let message = if piano::channel(s).is_some() {
            "Engraving score…"
        } else {
            "Select an instrument in the Channel Rack."
        };
        ui.c.text_center(font, paper, Color::rgb(0x38322c), message);
        return;
    };
    if editor.latest != editor.started {
        ui.c.text_center(font, paper, Color::rgb(0x38322c), "Updating score…");
        return;
    }
    let mut selection = selected(s, req);
    if req.song.is_none()
        && (req.channel != piano::channel(s).unwrap_or(u16::MAX) || req.pattern != s.pattern)
    {
        selection.clear();
    }
    let ratio = ((paper.w - 4.) / frame.w as f32).min((paper.h - 4.) / frame.h as f32);
    let sheet = Rect::new(
        paper.cx() - frame.w as f32 * ratio / 2.,
        paper.y + 2.,
        frame.w as f32 * ratio,
        frame.h as f32 * ratio,
    );
    ui.c.image_rgba(sheet, frame.w, frame.h, &frame.pixels);
    arrangement::paint_playhead(ui, s, req, frame, sheet, ratio);
    let entry = arrangement::entry_active(s);
    arrangement::entry_hits(ui, req, frame, sheet, ratio, entry);
    for &(i, bounds) in &frame.notes {
        let scaled = Rect::new(
            sheet.x + bounds.x * ratio,
            sheet.y + bounds.y * ratio,
            bounds.w * ratio,
            bounds.h * ratio,
        );
        let hit = Rect::new(
            scaled.cx() - scaled.w.max(12.) / 2.,
            scaled.cy() - scaled.h.min(38.).max(12.) / 2.,
            scaled.w.max(12.),
            scaled.h.min(38.).max(12.),
        );
        let hover = !entry
            && ui.hit(
                hit,
                Action::Score(Command::Note(i)),
                "Select note • Shift-click adds notes • Drag up or down to transpose",
            );
        if selection.contains(&i) || hover {
            ui.c.fill_rrect(
                hit,
                4.,
                Color::rgb(0x2976ae).with_alpha(if hover || arrangement::anchored(s, req, i) {
                    95
                } else {
                    60
                }),
            );
        }
    }
    let selected_count = if req.song.is_some() {
        arrangement::selection_count(s, req)
    } else {
        selection.len()
    };
    let velocity = selection
        .first()
        .and_then(|&i| {
            if req.song.is_some() {
                req.notes.get(i).map(|part| &part.note)
            } else {
                piano::notes(s).get(i)
            }
        })
        .map_or(String::new(), |note| {
            if selected_count == 1 {
                format!("   •   Velocity {}", note.velocity)
            } else {
                format!("   •   {selected_count} selected")
            }
        });
    ui.c.text_clip(
        font,
        paper.x + 2.,
        paper.bottom() + 9.,
        (paper.w - export::controls_width(s)).max(1.),
        ui.t.text_dim,
        &format!(
            "Measures {}–{}{}{}",
            req.bars[0].number,
            req.bars.last().unwrap().number,
            velocity,
            req.song
                .as_ref()
                .map_or(String::new(), |song| song.status())
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> App {
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state.selected_channel = 0;
        app.state.pattern = 1;
        let pattern = piano::pattern_mut(&mut app.state);
        pattern.notes = vec![
            Note {
                key: 49,
                length: 96,
                ..Note::step(0, 0)
            },
            Note {
                key: 60,
                length: 96,
                ..Note::step(0, 0)
            },
            Note {
                key: 64,
                length: 96,
                ..Note::step(0, 0)
            },
            Note {
                key: 67,
                length: 96,
                ..Note::step(1, 0)
            },
            Note {
                key: 72,
                length: 120,
                ..Note::step(0, 350)
            },
        ];
        app
    }

    #[test]
    fn score_edits_share_piano_notes_selection_and_history() {
        let mut app = fixture();
        let original = piano::notes(&app.state).to_vec();
        run(&mut app, Command::Quantize);
        assert_eq!(piano::notes(&app.state), original);
        app.state.piano.select(1, 0, vec![1, 2]);
        run(&mut app, Command::Pitch(2));
        let snapshot = snapshot(&app.state).unwrap();
        assert_eq!(
            snapshot
                .notes
                .iter()
                .map(|n| n.note.key)
                .collect::<Vec<_>>(),
            [49, 62, 66, 72]
        );
        assert_eq!(piano::notes(&app.state)[3], original[3]);
        assert_eq!(selected(&app.state, &snapshot), [1, 2]);
        assert!(piano::undo(&mut app.state));
        assert_eq!(piano::notes(&app.state), original);
        let before = engrave::test_markup(&super::snapshot(&app.state).unwrap());
        piano::pattern_mut(&mut app.state).notes[0].key = 50;
        let after = engrave::test_markup(&super::snapshot(&app.state).unwrap());
        assert_ne!(before, after, "the score follows a Piano Roll pitch edit");
        assert!(
            after.contains("<tie startid="),
            "notes crossing a measure use ties"
        );
        assert!(after.contains("<chord dur="));
    }

    #[test]
    fn score_bundled_verovio_renders_and_detached_hits_edit_real_notes() {
        if !engrave::available() {
            return;
        }
        let mut app = fixture();
        let request = snapshot(&app.state).unwrap();
        let frame = engrave::render(&request).expect("native Verovio renders generated MEI");
        for index in [0, 1, 2, 4] {
            assert!(
                frame
                    .notes
                    .iter()
                    .any(|&(i, bounds)| i == index && bounds.w > 0. && bounds.h > 0.),
                "note {index} lost its hit area"
            );
        }
        assert!(
            frame
                .pixels
                .chunks_exact(4)
                .any(|px| px[0] < 80 && px[1] < 80 && px[2] < 80)
        );
        let editor = &mut app.state.notation;
        editor.frame = Some(frame);
        editor.started = Some(request.clone());
        editor.latest = Some(request);
        app.act(Action::ToggleWin(Win::Score));
        app.act(Action::DetachWin(Win::Score));
        assert!(app.state.native_detached.contains(&Win::Score));
        app.state.native_view = Some(Win::Score);
        let mut canvas = os_ui::Canvas::new(1100, 650, 1.).unwrap();
        app.paint(&mut canvas, None);
        let hit = app
            .hits
            .iter()
            .find(|hit| hit.action == Action::Score(Command::Note(0)))
            .expect("note hit on the detached canvas")
            .rect;
        app.mouse_move(hit.cx(), hit.cy());
        app.mouse_down();
        app.mouse_up();
        assert_eq!(app.state.piano.selection(1, Some(0)), [0]);
        assert!(key(&mut app, &Key::Named(NamedKey::ArrowUp), false, false));
        assert_eq!(piano::notes(&app.state)[0].key, 50);
    }
}
