// Copyright (C) 2026 ItsAlphaNeon
// Copyright (C) 2026 thacer13
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Pattern selector and PATTERNS menu commands: find empty, rename / colour, clone, insert, delete, move,
//! transpose, plus the name box and the delete confirmation.
// spec: ui/pattern-menu.md

use os_model::{ClipSource, Pattern, Rgb};

use crate::app::State;

/// Highest pattern number (§1).
pub const MAX: u16 = 999;
/// Pixels of vertical drag per pattern step on the name box (§1, OS: linear).
pub const DRAG_STEP: f32 = 8.0;
/// Name box size (§4).
pub const BOX_W: f32 = 251.0;
pub const BOX_H: f32 = 51.0;
/// Names remembered for the suggestions column (OS).
const RECENT: usize = 5;

/// Generic name suggestions (OS word list, §4).
pub const WORDS: [&[&str]; 4] = [
    &["Drums", "Kick", "Snare", "Clap", "Hats", "Toms", "Perc", "Cymbal", "Shaker", "FX"],
    &["Bass", "Sub", "Lead", "Pad", "Keys", "Piano", "Pluck", "Chords", "Arp", "Strings"],
    &["Vocal", "Choir", "Guitar", "Brass", "Bells", "Organ", "Synth", "Sample", "Loop", "Fill"],
    &["Intro", "Verse", "Pre-chorus", "Chorus", "Hook", "Bridge", "Drop", "Breakdown", "Build", "Outro"],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    FindFirst,
    FindNext,
    FindNextNoName,
    SelectInPlaylist,
    Rename,
    ChangeColor,
    RandomColor,
    Transpose,
    InsertOne,
    Clone,
    Delete,
    MoveUp,
    MoveDown,
    /// Set time signature...: a dialog, run by `App::pattern_cmd` (spec: ui/pattern-time-signature.md).
    TimeSignature,
    /// Open in project browser: the browser's Patterns section, run by `App::pattern_cmd`
    /// (spec: ui/browser-commands.md §4).
    OpenInBrowser,
    /// Split by channel: one pattern per channel with notes (spec: ui/split-by-channel.md §2).
    SplitByChannel,
    /// Quick render as audio clip and Render and replace: run by `App::pattern_cmd` (spec: ui/pattern-render.md).
    QuickRender,
    RenderReplace,
    /// Render as audio clip...: the Export form in its render mode first.
    RenderAsClip,
}

impl Cmd {
    /// The command behind a PATTERNS menu label.
    pub fn from_label(label: &str) -> Option<Cmd> {
        Some(match label {
            "Find first empty..." => Cmd::FindFirst,
            "Find next empty..." => Cmd::FindNext,
            "Find next empty (no naming)" => Cmd::FindNextNoName,
            "Select in playlist" => Cmd::SelectInPlaylist,
            "Rename and color..." => Cmd::Rename,
            "Change color..." => Cmd::ChangeColor,
            "Random color" => Cmd::RandomColor,
            "Transpose..." => Cmd::Transpose,
            "Insert one" => Cmd::InsertOne,
            "Clone" => Cmd::Clone,
            "Delete" => Cmd::Delete,
            "Move up" => Cmd::MoveUp,
            "Move down" => Cmd::MoveDown,
            "Set time signature..." => Cmd::TimeSignature,
            "Open in project browser" => Cmd::OpenInBrowser,
            "Split by channel" => Cmd::SplitByChannel,
            "Quick render as audio clip" => Cmd::QuickRender,
            "Render as audio clip..." => Cmd::RenderAsClip,
            "Render and replace..." => Cmd::RenderReplace,
            _ => return None,
        })
    }
}

/// Clicks inside the name box, its popups and the confirm dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Accept,
    Cancel,
    Suggestions,
    /// Column, row of a suggestion (column 0 = recent names).
    Suggest(usize, usize),
    Color,
    Swatch(usize),
    CustomSwatch(usize),
    EditSwatch(usize),
    SwatchWheel,
    SwatchBrightness,
    SaveSwatch,
    ClearSwatch,
    CancelSwatch,
    ConfirmOk,
    ConfirmCancel,
    /// The confirmation's Remember my choice check.
    ConfirmRemember,
    Inside,
    Text(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxTarget {
    AudioClip(usize),
    MixerTrack(u16),
    Name(u16),
    Transpose(u16),
    /// The channel rack's boxes (spec: ui/channel-rack.md §13, §17, §21).
    Rack(crate::rack::RBox),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NameBox {
    pub target: BoxTarget,
    pub text: String,
    /// The text is selected: the first typed character replaces it.
    pub fresh: bool,
    pub editor: crate::dialog::Field,
    /// Top-left on screen.
    pub at: (f32, f32),
    pub suggestions: bool,
    pub palette: bool,
    pub custom: Option<(usize, [f32; 3])>,
    /// Opened for a new pattern (find empty, insert one): accepting it makes the pattern the playlist's clip
    /// source, listed in the picker even while empty, so it can be placed at once.
    pub created: bool,
}

#[derive(Debug, Default)]
pub struct PatternUi {
    pub name_box: Option<NameBox>,
    /// "The selected pattern is about to be deleted." is showing.
    pub confirm: bool,
    /// The confirmation's "Remember my choice" check: off each time it opens; Ok with it ticked turns the
    /// confirmation's Manage messages entry off, Cancel never does (spec: ui/settings-choices.md "Hints and warnings:
    /// Manage messages and Project data size warning (2026-10-16)").
    pub remember: bool,
    pub recent: Vec<String>,
    /// Where the PATTERNS menu opens (`None` = under its menu-bar title).
    pub menu_at: Option<(f32, f32)>,
}

/// Palette for the colour tab and "Change color..." (OS swatches, §4).
pub const SWATCHES: [u32; 20] = [
    0x925755, 0xa65d3f, 0xa8813f, 0x9a9a45, 0x6f9446, 0x45946a, 0x3f9493, 0x4679a2, 0x4b4977, 0x6d4f91,
    0x905887, 0x764955, 0x5f6f75, 0xc06060, 0xd08850, 0xd0c060, 0x80c070, 0x60b0c0, 0x7090e0, 0xb080d0,
];

// ---------------------------------------------------------------------------------------------------------

/// A pattern with notes, control points or its own name (listed in menus, §2).
pub fn used(p: &Pattern) -> bool {
    !p.notes.is_empty() || !p.controls.is_empty() || p.name.is_some()
}

/// Numbers of the listed patterns, ascending: the used ones, those placed in the playlist, and the playlist's clip
/// source (a new pattern, empty until notes are added).
pub fn listed(s: &State) -> Vec<u16> {
    let mut v: Vec<u16> = s.project.patterns.iter().filter(|p| used(p)).map(|p| p.number).collect();
    v.extend(crate::playlist::clips(s).iter().filter_map(|c| if let ClipSource::Pattern(n) = c.source { Some(n) } else { None }));
    if let Some(ClipSource::Pattern(n)) = s.playlist.source {
        v.push(n);
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// A new pattern's name box: accepting it lists the pattern and makes it the playlist's clip source.
fn created_box(s: &mut State) {
    if let Some(b) = s.pat_ui.name_box.as_mut() {
        b.created = true;
    }
}

pub fn name_of(s: &State, n: u16) -> String {
    s.project.pattern(n).and_then(|p| p.name.clone()).unwrap_or_else(|| format!("Pattern {n}"))
}

fn is_used(s: &State, n: u16) -> bool {
    s.project.pattern(n).is_some_and(used)
}

/// Lowest unused number.
pub fn first_empty(s: &State) -> u16 {
    (1..=MAX).find(|&n| !is_used(s, n)).unwrap_or(MAX)
}

/// Lowest unused number above `after`.
pub fn next_empty(s: &State, after: u16) -> u16 {
    (after.saturating_add(1)..=MAX).find(|&n| !is_used(s, n)).unwrap_or(MAX)
}

/// The pattern `n`, added when the project has none with that number.
pub fn pat_mut(s: &mut State, n: u16) -> &mut Pattern {
    let ps = &mut s.project.patterns;
    let i = match ps.iter().position(|p| p.number == n) {
        Some(i) => i,
        None => {
            let at = ps.partition_point(|p| p.number < n);
            ps.insert(at, Pattern { number: n, ..Pattern::default() });
            at
        }
    };
    &mut ps[i]
}

/// Renumber patterns (`None` removes one): playlist clips follow (clips of removed patterns go). Numbers that
/// end up without a pattern keep an empty entry so saving clears their blocks.
fn renumber(s: &mut State, map: impl Fn(u16) -> Option<u16>) {
    let old: Vec<u16> = s.project.patterns.iter().map(|p| p.number).collect();
    let mut pats: Vec<Pattern> = std::mem::take(&mut s.project.patterns)
        .into_iter()
        .filter_map(|mut p| {
            let n = map(p.number)?;
            p.number = n;
            Some(p)
        })
        .collect();
    for n in old {
        if !pats.iter().any(|p| p.number == n) {
            pats.push(Pattern { number: n, ..Pattern::default() });
        }
    }
    pats.sort_by_key(|p| p.number);
    s.project.patterns = pats;
    s.project.clip_picker_order.retain_mut(|source| match source {
        ClipSource::Pattern(id) => map(*id).map(|new| *id = new).is_some(),
        ClipSource::Channel(_) => true,
    });
    s.playlist.picker_ui.selected.clear();
    s.playlist.picker_ui.stop_previews();
    if let Some(a) = s.project.arrangement_mut() {
        remap_pattern_clips(a, &map);
    }
    s.playlist.sel.clear();
    s.playlist.hover_clip = None;
    s.piano.clear_selection();
    s.piano.dirty = true;
}

fn remap_pattern_clips(arrangement: &mut os_model::Arrangement, map: &impl Fn(u16) -> Option<u16>) {
    for clips in std::iter::once(&mut arrangement.clips)
        .chain(arrangement.tracks.iter_mut().filter_map(|track| track.frozen.as_mut()))
    {
        clips.retain_mut(|clip| match clip.source {
            ClipSource::Pattern(n) => map(n).map(|new| clip.source = ClipSource::Pattern(new)).is_some(),
            ClipSource::Channel(_) => true,
        });
    }
}

/// "<name> #k" with the lowest free k >= 2 (§3 Clone).
pub fn clone_name(s: &State, n: u16) -> String {
    let name = name_of(s, n);
    let base = match name.rsplit_once(" #") {
        Some((b, k)) if k.parse::<u32>().is_ok() => b.to_string(),
        _ => name,
    };
    let taken = |cand: &str| s.project.patterns.iter().any(|p| p.name.as_deref() == Some(cand));
    (2..).map(|k| format!("{base} #{k}")).find(|c| !taken(c)).expect("unbounded")
}

/// Swap patterns `a` and `b`.
fn swap(s: &mut State, a: u16, b: u16) {
    renumber(s, |n| Some(if n == a { b } else if n == b { a } else { n }));
}

/// Transpose every note of pattern `n` by `semis`.
pub fn transpose(s: &mut State, n: u16, semis: i32) {
    if let Some(p) = s.project.patterns.iter_mut().find(|p| p.number == n) {
        for note in &mut p.notes {
            note.key = (i32::from(note.key) + semis).clamp(0, 131) as u16;
        }
        s.piano.dirty = true;
    }
}

pub(crate) fn random_color(seed: u64) -> Rgb {
    // A random hue at a muted saturation / lightness like the default pattern colours.
    let mut x = seed ^ 0x9e37_79b9_7f4a_7c15;
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    let h = (x % 360) as f32;
    let (sat, l) = (0.35f32, 0.45f32);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * sat;
    let hp = h / 60.0;
    let xx = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, xx, 0.0),
        1 => (xx, c, 0.0),
        2 => (0.0, c, xx),
        3 => (0.0, xx, c),
        4 => (xx, 0.0, c),
        _ => (c, 0.0, xx),
    };
    let m = l - c / 2.0;
    let u = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb::new(u(r), u(g), u(b))
}

/// Open the name box for pattern `n` at `at`.
pub fn open_name_box(s: &mut State, target: BoxTarget, at: (f32, f32)) {
    let at = if matches!(target, BoxTarget::AudioClip(_) | BoxTarget::MixerTrack(_)) {
        (at.0.clamp(0., (s.screen.0 - BOX_W).max(0.)),
         at.1.clamp(0., (s.screen.1 - BOX_H).max(0.)))
    } else {
        at
    };
    let text = match target {
        BoxTarget::Name(n) => name_of(s, n),
        BoxTarget::MixerTrack(n) => crate::mixer::name(s, n),
        BoxTarget::AudioClip(i) => crate::playlist::clips(s).get(i).map_or_else(String::new, |c| crate::playlist::clip_name(s, c).into_owned()),
        BoxTarget::Transpose(_) => String::new(),
        BoxTarget::Rack(b) => crate::rack::box_text(s, b),
    };
    s.pat_ui.name_box = Some(NameBox { target, editor: { let mut f = crate::dialog::Field::default(); f.set(&text); f.select_all(); f }, text, fresh: true, at, suggestions: false, palette: false, custom: None, created: false });
}

/// Where a box opened from a menu / key goes: the mouse over its colour tab (§4).
pub fn box_at_mouse(mouse: (f32, f32)) -> (f32, f32) {
    ((mouse.0 - 241.0).max(0.0), (mouse.1 - 41.0).max(0.0))
}

/// Pattern `n` picked in the pattern selector (its menu, a drag or the wheel): it becomes the current pattern and the
/// playlist's clip source, so Draw and Paint place it, as FL does (issue #94).
pub fn select(s: &mut State, n: u16) {
    s.piano.dirty |= s.pattern != n;
    s.pattern = n;
    s.playlist.source = Some(ClipSource::Pattern(n));
}

/// Box position for "+" (§4).
pub const PLUS_BOX_AT: (f32, f32) = (949.0, 46.0);

/// Run a command; `at` is where a name box opens. Returns a status message.
pub fn run(s: &mut State, cmd: Cmd, at: (f32, f32)) -> Option<String> {
    let cur = s.pattern.max(1);
    match cmd {
        Cmd::FindFirst => {
            s.pattern = first_empty(s);
            open_name_box(s, BoxTarget::Name(s.pattern), at);
            created_box(s);
        }
        Cmd::FindNext => {
            s.pattern = next_empty(s, cur);
            open_name_box(s, BoxTarget::Name(s.pattern), at);
            created_box(s);
        }
        Cmd::FindNextNoName => {
            s.pattern = next_empty(s, cur);
            s.playlist.source = Some(ClipSource::Pattern(s.pattern));
        }
        Cmd::SelectInPlaylist => {
            let sel: Vec<usize> = crate::playlist::clips(s)
                .iter()
                .enumerate()
                .filter(|(_, c)| c.source == ClipSource::Pattern(cur))
                .map(|(i, _)| i)
                .collect();
            let n = sel.len();
            s.playlist.sel = sel;
            if !s.shown(crate::app::Win::Playlist) || !s.in_front(crate::app::Win::Playlist) {
                s.raise(crate::app::Win::Playlist);
            }
            return Some(format!("Selected {n} clip(s) of {}", name_of(s, cur)));
        }
        Cmd::Rename => open_name_box(s, BoxTarget::Name(cur), at),
        Cmd::ChangeColor => {
            open_name_box(s, BoxTarget::Name(cur), at);
            if let Some(b) = s.pat_ui.name_box.as_mut() {
                b.palette = true;
            }
        }
        Cmd::RandomColor => {
            let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
            let selected = &s.playlist.picker_ui.selected;
            let targets: Vec<_> = if selected.contains(&ClipSource::Pattern(cur)) {
                selected.iter().filter_map(|src| if let ClipSource::Pattern(n) = src { Some(*n) } else { None }).collect()
            } else { vec![cur] };
            let snaps=targets.iter().map(|&n|crate::history::Snap::source_color(s,ClipSource::Pattern(n))).collect();
            crate::history::record(s,"Random pattern colors",crate::history::Snap::Several(snaps));
            for n in targets { pat_mut(s, n).color = Some(random_color(seed ^ u64::from(n))); }
            s.piano.dirty = true;
        }
        Cmd::Transpose => open_name_box(s, BoxTarget::Transpose(cur), at),
        Cmd::InsertOne => {
            renumber(s, |n| Some(if n >= cur { (n + 1).min(MAX) } else { n }));
            open_name_box(s, BoxTarget::Name(cur), at);
            created_box(s);
        }
        Cmd::Clone => {
            if cur >= MAX {
                return None;
            }
            let mut copy = s.project.pattern(cur).cloned().unwrap_or(Pattern { number: cur, ..Pattern::default() });
            copy.name = Some(clone_name(s, cur));
            renumber(s, |n| Some(if n > cur { (n + 1).min(MAX) } else { n }));
            copy.number = cur + 1;
            *pat_mut(s, cur + 1) = copy;
            s.pattern = cur + 1;
        }
        // Its Manage messages entry off: as if Ok was pressed.
        Cmd::Delete if !crate::messages::on(&crate::messages::DELETE_PATTERN) => delete_pattern(s),
        Cmd::Delete => open_confirm(s),
        Cmd::MoveUp if cur > 1 => {
            swap(s, cur, cur - 1);
            s.pattern = cur - 1;
        }
        Cmd::MoveDown if cur < MAX => {
            swap(s, cur, cur + 1);
            s.pattern = cur + 1;
        }
        Cmd::MoveUp | Cmd::MoveDown => {}
        Cmd::SplitByChannel => split_by_channel(s),
        // Dialogs, the browser and the renders: `App::pattern_cmd` runs them.
        Cmd::TimeSignature | Cmd::OpenInBrowser | Cmd::QuickRender | Cmd::RenderReplace | Cmd::RenderAsClip => {}
    }
    None
}

/// Shows the delete confirmation (the pattern's, or the channels' waiting in `rack.pending_delete`), its Remember my
/// choice off.
pub fn open_confirm(s: &mut State) {
    s.pat_ui.confirm = true;
    s.pat_ui.remember = false;
}

/// The Manage messages entry of the delete confirmation showing: the channels' or the pattern's.
fn confirming(s: &State) -> &'static crate::messages::Message {
    if s.rack.pending_delete.is_some() {
        &crate::messages::DELETE_CHANNEL
    } else {
        &crate::messages::DELETE_PATTERN
    }
}

/// PATTERNS > Split by channel (spec: ui/split-by-channel.md §2). Channels with notes in the current pattern, in rack
/// order: the pattern keeps the first one's notes and takes its name; each further one gets a pattern of its own named
/// after it, right after the current one in channel order (later patterns move up with their clips, as Insert one),
/// with the original's colour, markers and other settings but only that channel's notes. Every playlist clip of the
/// original gets one clip per new pattern at the same place, under it. No notes: nothing happens. One undo step.
pub const SPLIT_VISIBILITY: &str = "visible_split-by-channel";

pub fn split_by_channel(s: &mut State) {
    let _ = split(s, None);
}

pub fn confirm_split(app: &mut crate::app::App) -> Option<String> {
    let s = &mut app.state;
    let cur = s.pattern.max(1);
    if !crate::workspace_visibility::shown(SPLIT_VISIBILITY) { return None; }
    let count = s.project.pattern(cur).map_or(0, |p| s.project.channels.iter()
        .filter(|ch| p.notes.iter().any(|n| n.channel == ch.index)).take(2).count());
    if count == 2 && s.project.arrangements.iter().any(|a| a.clips.iter().any(|c| c.source == ClipSource::Pattern(cur))) {
        use crate::dialog::{Answer, Kind, Msg, Then};
        app.show_msg(Msg::new(Kind::Question, "Split by channel", vec![
            ("Create an instrument track for each pattern, or group the new tracks below the first pattern.".into(), false)
        ], &[Answer::Yes, Answer::No, Answer::Cancel], Then::SplitPattern(cur))
            .labelled(&["Instrument tracks", "Group below first", "Cancel"]));
        None
    } else {
        split(s, None)
    }
}

pub fn split(s: &mut State, group: Option<bool>) -> Option<String> {
    let cur = s.pattern.max(1);
    let orig = s.project.pattern(cur).cloned()?;
    let chans: Vec<(u16, String)> = s
        .project
        .channels
        .iter()
        .enumerate()
        .filter(|(_, c)| orig.notes.iter().any(|n| n.channel == c.index))
        .map(|(r, c)| (c.index, crate::rack::channel_name(s, r)))
        .collect();
    let ((first, name), rest) = chans.split_first()?;
    let k = rest.len() as u16;
    // Room for the new numbers: no used pattern may move past the last one.
    let placed = s.project.arrangements.iter().flat_map(|a| {
        a.clips.iter().chain(a.tracks.iter().filter_map(|t| t.frozen.as_ref()).flatten())
    }).filter_map(|clip| match clip.source {
        ClipSource::Pattern(n) => Some(n),
        _ => None,
    });
    let last = s.project.patterns.iter().filter(|p| used(p)).map(|p| p.number)
        .chain(placed).chain(std::iter::once(cur)).max().unwrap_or(cur);
    if usize::from(last) + rest.len() > usize::from(MAX) {
        return Some("There are too many patterns to split this pattern.".into());
    }
    let extra = crate::history::Snap::Patterns { patterns: s.project.patterns.clone(), current: cur };
    let undo = if group.is_some() {
        for arr in &s.project.arrangements {
            let rows = split_rows(arr, cur);
            if rows.is_empty() { continue; }
            let last_clip = arr.clips.iter().map(|c| c.track as usize).max().unwrap_or(0);
            let last_track = arr.tracks.iter().enumerate().rposition(|(i, t)| *t != os_model::PlaylistTrack::new(i as u32 + 1)).unwrap_or(0);
            if last_clip.max(last_track) + rows.len() * rest.len() >= usize::from(crate::playlist::TRACKS) {
                return Some("There are too few free Playlist rows to fit the split patterns.".into());
            }
            if rows.iter().any(|&row| arr.tracks.get(usize::from(row)).is_some_and(|t| t.locked || t.frozen.is_some() || t.folder || t.comp.is_some())) {
                return Some("Unlock or unfreeze the source Playlist track before splitting.".into());
            }
        }
        crate::history::Snap::Several(vec![extra, crate::history::Snap::PatternArrangements(s.project.arrangements.clone())])
    } else { extra };
    crate::playlist::snapshot_with(s, "Split by channel", undo);
    // Notes of a channel no longer in the rack stay with the original.
    let p = pat_mut(s, cur);
    p.notes.retain(|n| n.channel == *first || !chans.iter().any(|c| c.0 == n.channel));
    p.name = Some(name.clone());
    s.piano.dirty = true;
    if k == 0 {
        return None;
    }
    let mapped = |n| if n > cur { n + k } else { n };
    let active = s.project.arrangement().map(|arr| arr.number);
    renumber(s, |n| Some(mapped(n)));
    if group.is_some() {
        for arrangement in s.project.arrangements.iter_mut().filter(|a| Some(a.number) != active) {
            for clip in arrangement.clips.iter_mut()
                .chain(arrangement.tracks.iter_mut().filter_map(|t| t.frozen.as_mut()).flatten()) {
                if let ClipSource::Pattern(n) = &mut clip.source {
                    *n = mapped(*n);
                }
            }
        }
    }
    for lane in &mut s.project.instrument_lanes {
        lane.3 = mapped(lane.3);
    }
    for (j, (ch, name)) in (1..).zip(rest) {
        // Control points stay with the original (only notes are divided).
        let notes = orig.notes.iter().filter(|n| n.channel == *ch).copied().collect();
        *pat_mut(s, cur + j) = Pattern { number: cur + j, name: Some(name.clone()), notes, controls: Vec::new(), ..orig.clone() };
    }
    if let Some(grouped) = group {
        let part_tracks = chans.iter().map(|(id, name)| {
            let channel = s.project.channel(*id).unwrap();
            (*id, name.clone(), channel.mixer_track, channel.color)
        }).collect::<Vec<_>>();
        for number in s.project.arrangements.iter().map(|arr| arr.number).collect::<Vec<_>>() {
            let rows = split_rows(s.project.arrangements.iter().find(|a| a.number == number).unwrap(), cur);
            for row in rows.into_iter().rev() {
                if Some(number) == active {
                    while crate::playlist::arr_mut(s).tracks.len() < usize::from(crate::playlist::TRACKS) {
                        let a = crate::playlist::arr_mut(s);
                        a.tracks.push(os_model::PlaylistTrack::new(a.tracks.len() as u32 + 1));
                    }
                    for _ in 0..k { crate::playlist::insert_track(s, row + 1); }
                } else {
                    let arr = s.project.arrangements.iter_mut().find(|a| a.number == number).unwrap();
                    let shift = |track: u16| if track > row { track + k } else { track };
                    for clip in &mut arr.clips { clip.track = shift(clip.track); }
                    for t in &mut arr.tracks {
                        if let Some(clips) = &mut t.frozen {
                            for clip in clips { clip.track = shift(clip.track); }
                        }
                    }
                    arr.attached_order.iter_mut().for_each(|r| *r = shift(*r));
                    for link in &mut s.project.instrument_lanes {
                        if link.1 == number { link.2 = shift(link.2); }
                    }
                    while arr.tracks.len() <= usize::from(row) {
                        arr.tracks.push(os_model::PlaylistTrack::new(arr.tracks.len() as u32 + 1));
                    }
                    for _ in 0..k { arr.tracks.insert(usize::from(row + 1), os_model::PlaylistTrack::new(0)); }
                    arr.tracks.truncate(usize::from(crate::playlist::TRACKS));
                    for (i, track) in arr.tracks.iter_mut().enumerate().skip(usize::from(row + 1)) {
                        track.number = i as u32 + 1;
                    }
                }
                let arr = s.project.arrangements.iter_mut().find(|a| a.number == number).unwrap();
                let original = arr.clips.iter().filter(|clip| clip.track == row && clip.source == ClipSource::Pattern(cur)).cloned().collect::<Vec<_>>();
                for (i, (id, name, track, color)) in part_tracks.iter().enumerate() {
                    let part = i as u16;
                    let lane = row + part;
                    let track_info = &mut arr.tracks[usize::from(lane)];
                    if part != 0 || track_info.name.is_none() { track_info.name = Some(name.clone()); }
                    if part != 0 || track_info.color.is_none() { track_info.color = *color; }
                    if grouped {
                        track_info.grouped = part != 0;
                    } else {
                        track_info.mode = os_model::PlaylistTrackMode::Instrument;
                        track_info.attached_mixer = Some(*track);
                        s.project.instrument_lanes.retain(|x| x.1 != number || x.2 != lane);
                        s.project.instrument_lanes.push((*id, number, lane, cur + part));
                    }
                    if part == 0 { continue; }
                    arr.clips.extend(original.iter().cloned().map(|mut c| {
                        c.source = ClipSource::Pattern(cur + part);
                        c.track = lane;
                        c.name = None;
                        c
                    }));
                }
                arr.clips.sort_by_key(|c| c.position);
            }
        }
        return None;
    }
    if let Some(a) = s.project.arrangement_mut() {
        // Clips are drawn in record order: the new ones go just before their original, which stays on top.
        let mut clips = Vec::with_capacity(a.clips.len());
        for c in std::mem::take(&mut a.clips) {
            if c.source == ClipSource::Pattern(cur) {
                clips.extend((1..=k).map(|j| os_model::Clip { source: ClipSource::Pattern(cur + j), name: c.name.clone(), ..c }));
            }
            clips.push(c);
        }
        a.clips = clips;
    }
    None
}

fn split_rows(arrangement: &os_model::Arrangement, pattern: u16) -> Vec<u16> {
    let mut rows = arrangement.clips.iter().filter(|c| c.source == ClipSource::Pattern(pattern))
        .map(|c| c.track).collect::<Vec<_>>();
    rows.sort_unstable();
    rows.dedup();
    rows
}

/// Ok in the delete confirmation (channels waiting in the rack go to the app, which removes them). With Remember my
/// choice ticked, the confirmation's Manage messages entry turns off.
pub fn delete_current(s: &mut State) {
    if s.pat_ui.remember {
        crate::messages::set(confirming(s), false);
    }
    s.pat_ui.confirm = false;
    s.pat_ui.remember = false;
    if let Some(rows) = s.rack.pending_delete.take() {
        s.rack.delete_now = Some(rows);
        return;
    }
    delete_pattern(s);
}

/// Cancel in the delete confirmation: nothing is deleted and Remember my choice is not kept.
fn cancel_confirm(s: &mut State) {
    s.pat_ui.confirm = false;
    s.pat_ui.remember = false;
    s.rack.pending_delete = None;
}

/// Deletes the current pattern: later ones move down a number, its clips go.
fn delete_pattern(s: &mut State) {
    let cur = s.pattern.max(1);
    renumber(s, |n| if n == cur { None } else if n > cur { Some(n - 1) } else { Some(n) });
}

/// Delete selected picker patterns in one pass, including placements in other arrangements.
pub(crate) fn delete_picker_patterns(s: &mut State, sorted: &[u16]) {
    use crate::history::Snap;

    let undo = Snap::Several(vec![
        Snap::Patterns { patterns: s.project.patterns.clone(), current: s.pattern },
        Snap::PatternArrangements(s.project.arrangements.clone()),
        Snap::ClipPickerOrder(s.project.clip_picker_order.clone()),
    ]);
    crate::history::record(s, "Delete selected patterns", undo);
    let map = |n| sorted.binary_search(&n).err().map(|before| n - before as u16);
    let active = s.project.arrangement().map(|a| a.number);
    renumber(s, map);
    for arrangement in s.project.arrangements.iter_mut().filter(|a| Some(a.number) != active) {
        remap_pattern_clips(arrangement, &map);
    }
    s.pattern = (s.pattern - sorted.partition_point(|&n| n < s.pattern) as u16).max(1);
}

/// Accept the name box.
pub fn accept(s: &mut State) {
    let Some(b) = s.pat_ui.name_box.take() else { return };
    match b.target {
        BoxTarget::AudioClip(i) => crate::playlist::set_audio_clip_name(s, i, &b.text),
        BoxTarget::MixerTrack(n) => crate::mixer::rename_track(s, n, &b.text),
        BoxTarget::Name(n) => {
            let text = b.text.trim().to_string();
            let name = (!text.is_empty() && text != format!("Pattern {n}")).then_some(text.clone());
            if name.is_some() || s.project.pattern(n).is_some() {
                pat_mut(s, n).name = name;
            }
            if b.created {
                s.playlist.source = Some(ClipSource::Pattern(n));
            }
            if !text.is_empty() {
                s.pat_ui.recent.retain(|r| *r != text);
                s.pat_ui.recent.insert(0, text);
                s.pat_ui.recent.truncate(RECENT);
            }
            s.piano.dirty = true;
        }
        BoxTarget::Transpose(n) => {
            if let Ok(v) = b.text.trim().trim_start_matches('+').parse::<i32>() {
                transpose(s, n, v);
            }
        }
        BoxTarget::Rack(rb) => {
            crate::rack::box_accept(s, rb, &b.text);
            s.piano.dirty = true;
        }
    }
}

/// A suggestion picked: rename and close at once (§4).
pub fn suggest(s: &mut State, col: usize, row: usize) {
    let word = if col == 0 { s.pat_ui.recent.get(row).cloned() } else { WORDS.get(col - 1).and_then(|c| c.get(row)).map(|w| w.to_string()) };
    if let (Some(w), Some(b)) = (word, s.pat_ui.name_box.as_mut()) {
        b.text = w;
        accept(s);
    }
}

pub const CUSTOM_SWATCHES: usize = 6;

pub fn custom_swatch(i: usize) -> Option<u32> {
    crate::prefs::text(&format!("clip_custom_swatch_{i}")).and_then(|v| hex_color(&v))
}

fn hex_color(text: &str) -> Option<u32> {
    let text = text.strip_prefix('#').unwrap_or(text);
    if text.len() == 6 && text.bytes().all(|b| b.is_ascii_hexdigit()) {
        u32::from_str_radix(text, 16).ok()
    } else {
        None
    }
}

pub fn set_color(s: &mut State, i: usize) {
    if let Some(&color) = SWATCHES.get(i) {
        apply_swatch(s, color);
    }
}

fn apply_swatch(s: &mut State, c: u32) {
    let target = s.pat_ui.name_box.as_ref().map(|b| b.target);
    let color = Some(Rgb::new((c >> 16) as u8, (c >> 8) as u8, c as u8));
    if matches!(target, Some(BoxTarget::AudioClip(_) | BoxTarget::MixerTrack(_))) {
        match target {
            Some(BoxTarget::AudioClip(i)) => crate::playlist::set_audio_clip_color(s, i, color),
            Some(BoxTarget::MixerTrack(n)) => crate::mixer::set_track_color(s, n, color),
            _ => unreachable!(),
        }
        if let Some(b) = &mut s.pat_ui.name_box {
            b.palette = false;
            b.custom = None;
        }
        return;
    }
    let source = match target {
        Some(BoxTarget::Name(n)) => os_model::ClipSource::Pattern(n),
        Some(BoxTarget::Rack(crate::rack::RBox::Channel(r))) => {
            let Some(ch) = s.project.channels.get(r) else { return };
            os_model::ClipSource::Channel(ch.index)
        }
        _ => return,
    };
    let before = crate::history::Snap::source_color(s, source);
    if crate::playlist::source_color(s, source) != color {
        crate::history::record(s, "Change clip source color", before);
        match target {
            Some(BoxTarget::Name(n)) => pat_mut(s, n).color = color,
            Some(BoxTarget::Rack(crate::rack::RBox::Channel(r))) => {
                let ch = &mut s.project.channels[r];
                ch.color = color;
                if matches!(ch.kind, os_model::ChannelKind::AudioClip | os_model::ChannelKind::Automation) { ch.rack.stored_color = color; }
            }
            _ => {}
        }
        s.piano.dirty = true;
    }
    if let Some(b) = s.pat_ui.name_box.as_mut() {
        b.palette = false;
        b.custom = None;
    }
}

fn edit_swatch(s: &mut State, i: usize) {
    if let Some(b) = &mut s.pat_ui.name_box {
        let color = custom_swatch(i).map(os_ui::Color::rgb).unwrap_or(os_ui::Color::rgb(0xffffff));
        b.custom = Some((i, crate::theme_adjust::hsv(color)));
    }
}

fn save_swatch(s: &mut State, clear: bool) {
    let Some(b) = &mut s.pat_ui.name_box else { return };
    let Some((i, hsv)) = b.custom else { return };
    if clear {
        crate::prefs::set_text(&format!("clip_custom_swatch_{i}"), "");
    } else {
        let c = crate::theme_adjust::from_hsv(hsv);
        let color = (u32::from(c.r) << 16) | (u32::from(c.g) << 8) | u32::from(c.b);
        crate::prefs::set_text(&format!("clip_custom_swatch_{i}"), &format!("{color:06X}"));
        if matches!(b.target, BoxTarget::MixerTrack(_)) {
            apply_swatch(s, color);
            return;
        }
    }
    b.custom = None;
}

/// A click in the name box / its popups / the confirmation.
pub fn hit(s: &mut State, h: Hit) {
    match h {
        Hit::Accept => accept(s),
        Hit::Cancel => s.pat_ui.name_box = None,
        Hit::Suggestions => {
            if let Some(b) = s.pat_ui.name_box.as_mut() {
                b.suggestions = !b.suggestions;
                b.palette = false;
                b.custom = None;
            }
        }
        Hit::Suggest(c, r) => suggest(s, c, r),
        Hit::Color => {
            if let Some(b) = s.pat_ui.name_box.as_mut() {
                b.palette = !b.palette;
                b.suggestions = false;
                b.custom = None;
            }
        }
        Hit::Swatch(i) => set_color(s, i),
        Hit::CustomSwatch(i) => match custom_swatch(i) {
            Some(color) => apply_swatch(s, color),
            None => edit_swatch(s, i),
        },
        Hit::EditSwatch(i) => edit_swatch(s, i),
        Hit::SaveSwatch => save_swatch(s, false),
        Hit::ClearSwatch => save_swatch(s, true),
        Hit::CancelSwatch => {
            if let Some(b) = &mut s.pat_ui.name_box { b.custom = None; }
        }
        Hit::ConfirmOk => delete_current(s),
        Hit::ConfirmCancel => cancel_confirm(s),
        Hit::ConfirmRemember => s.pat_ui.remember = !s.pat_ui.remember,
        Hit::Text(_) | Hit::Inside | Hit::SwatchWheel | Hit::SwatchBrightness => {}
    }
}

/// Typing into the name box. Returns whether the key was taken.
pub fn key(s: &mut State, key: &winit::keyboard::Key, ctrl: bool) -> bool { key_mod(s,key,ctrl,false) }
pub fn key_mod(s: &mut State, key: &winit::keyboard::Key, ctrl: bool, shift: bool) -> bool {
    use winit::keyboard::{Key, NamedKey};
    if s.pat_ui.confirm {
        match key {
            Key::Named(NamedKey::Enter) => delete_current(s),
            Key::Named(NamedKey::Escape) => cancel_confirm(s),
            _ => {}
        }
        return true;
    }
    let Some(b) = s.pat_ui.name_box.as_mut() else { return false };
    if b.custom.is_some() {
        match key {
            Key::Named(NamedKey::Enter) => save_swatch(s, false),
            Key::Named(NamedKey::Escape) => b.custom = None,
            _ => {}
        }
        return true;
    }
    if b.editor.text != b.text { b.editor.set(&b.text); if b.fresh { b.editor.select_all(); } }
    match key {
        Key::Named(NamedKey::Enter) => accept(s),
        Key::Named(NamedKey::Escape) => s.pat_ui.name_box = None,
        _ => {
            b.editor.key_mod(key, ctrl, false, shift);
            b.text.clone_from(&b.editor.text);
            b.fresh = b.editor.all;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use os_model::{Clip, Note};

    fn app() -> App {
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        let s = &mut app.state;
        s.project.patterns = vec![
            Pattern { number: 1, name: Some("Kick".into()), notes: vec![Note::step(0, 0)], ..Pattern::default() },
            Pattern { number: 2, name: Some("Kick #2".into()), notes: vec![Note::step(0, 48)], ..Pattern::default() },
            Pattern { number: 3, notes: vec![Note::step(1, 0)], ..Pattern::default() },
            Pattern { number: 5, name: Some("Lead".into()), ..Pattern::default() },
        ];
        crate::playlist::arr_mut(s).clips = vec![Clip::new(0, 384, 0, ClipSource::Pattern(1)), Clip::new(0, 384, 1, ClipSource::Pattern(3))];
        app
    }

    fn clip_pats(s: &State) -> Vec<u16> {
        crate::playlist::clips(s).iter().filter_map(|c| if let ClipSource::Pattern(n) = c.source { Some(n) } else { None }).collect()
    }

    #[test]
    fn empty_numbers_and_listing() {
        let mut app = app();
        let s = &mut app.state;
        assert_eq!(listed(s), vec![1, 2, 3, 5]);
        assert_eq!(first_empty(s), 4);
        s.pattern = 5;
        assert_eq!(next_empty(s, 5), 6);
        run(s, Cmd::FindFirst, (0.0, 0.0));
        assert_eq!(s.pattern, 4);
        // Cancel keeps the number current but unlisted; accepting a name lists it.
        s.pat_ui.name_box = None;
        assert_eq!(listed(s), vec![1, 2, 3, 5]);
        run(s, Cmd::Rename, (0.0, 0.0));
        key(s, &winit::keyboard::Key::Character("Probe".into()), false);
        accept(s);
        assert_eq!(listed(s), vec![1, 2, 3, 4, 5]);
        assert_eq!(name_of(s, 4), "Probe");
    }

    /// A new pattern is placeable at once: accepting its name box, default name included, lists it in the picker
    /// and makes it the playlist's clip source while it is still empty; placed, it stays listed.
    #[test]
    fn a_new_empty_pattern_is_listed_and_picked() {
        let mut app = app();
        let s = &mut app.state;
        s.playlist.source = Some(ClipSource::Pattern(1));
        run(s, Cmd::FindFirst, (0.0, 0.0));
        accept(s);
        assert_eq!((s.pattern, s.playlist.source), (4, Some(ClipSource::Pattern(4))));
        assert_eq!(listed(s), vec![1, 2, 3, 4, 5]);
        assert_eq!(crate::playlist::source(s), ClipSource::Pattern(4));
        // Without a name box (Ctrl+F4) alike.
        run(s, Cmd::FindNextNoName, (0.0, 0.0));
        assert_eq!((s.pattern, s.playlist.source), (6, Some(ClipSource::Pattern(6))));
        assert_eq!(listed(s), vec![1, 2, 3, 5, 6]);
    }

    #[test]
    fn clone_insert_delete_move_renumber_clips() {
        let mut app = app();
        let s = &mut app.state;
        s.pattern = 1;
        run(s, Cmd::Clone, (0.0, 0.0));
        assert_eq!(s.pattern, 2);
        assert_eq!(name_of(s, 2), "Kick #3");
        assert_eq!(name_of(s, 3), "Kick #2");
        assert_eq!(clip_pats(s), vec![1, 4]);
        // Insert one at 2: everything from 2 moves up.
        run(s, Cmd::InsertOne, (0.0, 0.0));
        s.pat_ui.name_box = None;
        assert_eq!(name_of(s, 3), "Kick #3");
        assert!(!s.project.pattern(2).is_some_and(used));
        assert_eq!(clip_pats(s), vec![1, 5]);
        // Delete the empty 2: later ones move down; the clip follows.
        run(s, Cmd::Delete, (0.0, 0.0));
        assert!(s.pat_ui.confirm);
        delete_current(s);
        assert_eq!(name_of(s, 2), "Kick #3");
        assert_eq!(clip_pats(s), vec![1, 4]);
        // Move down swaps with the next number.
        run(s, Cmd::MoveDown, (0.0, 0.0));
        assert_eq!((s.pattern, name_of(s, 3), name_of(s, 2)), (3, "Kick #3".into(), "Kick #2".into()));
        // Deleting a pattern removes its clips.
        s.pattern = 1;
        delete_current(s);
        assert_eq!(clip_pats(s), vec![3]);
    }

    // spec: ui/settings-choices.md "Hints and warnings: Manage messages and Project data size warning (2026-10-16)"
    /// Delete's confirmation has Remember my choice, off each time it opens. Ticked with Cancel nothing goes and the
    /// "Delete pattern" entry stays on; ticked with Ok the pattern goes and the entry turns off; while it is off,
    /// Delete deletes at once with no box.
    #[test]
    fn delete_confirmation_remembers_ok_only() {
        use crate::messages::{on, set, DELETE_PATTERN};
        use winit::keyboard::{Key, NamedKey};
        let mut app = app();
        let s = &mut app.state;
        s.pattern = 2;
        run(s, Cmd::Delete, (0.0, 0.0));
        assert!(s.pat_ui.confirm && !s.pat_ui.remember);
        hit(s, Hit::ConfirmRemember);
        assert!(s.pat_ui.remember);
        key(s, &Key::Named(NamedKey::Escape), false);
        assert_eq!((listed(s), on(&DELETE_PATTERN), s.pat_ui.confirm), (vec![1, 2, 3, 5], true, false), "Cancel");
        run(s, Cmd::Delete, (0.0, 0.0));
        assert!(s.pat_ui.confirm && !s.pat_ui.remember, "Remember my choice is off again");
        hit(s, Hit::ConfirmRemember);
        hit(s, Hit::ConfirmOk);
        assert_eq!((listed(s), on(&DELETE_PATTERN), s.pat_ui.confirm), (vec![1, 2, 4], false, false), "Ok");
        // Off: as if Ok was pressed.
        s.pattern = 1;
        run(s, Cmd::Delete, (0.0, 0.0));
        assert!(!s.pat_ui.confirm);
        assert_eq!((listed(s), clip_pats(s)), (vec![1, 3], vec![1]));
        set(&DELETE_PATTERN, true);
    }

    #[test]
    fn stepping_the_selector_adds_no_picker_rows() {
        let mut app = app();
        let s = &mut app.state;
        let before = crate::views::playlist::pick_items(s);
        s.pattern = 40;
        assert_eq!(crate::views::playlist::pick_items(s), before);
        assert_eq!(s.project.patterns.len(), 4);
        // The footer "+" = find first empty, which lists the pattern once it is named.
        run(s, Cmd::FindFirst, (0.0, 0.0));
        key(s, &winit::keyboard::Key::Character("Bass".into()), false);
        accept(s);
        assert_eq!(crate::views::playlist::pick_items(s).len(), before.len() + 1);
    }

    #[test]
    fn transpose_and_suggestions() {
        let mut app = app();
        let s = &mut app.state;
        s.pattern = 1;
        let k0 = s.project.pattern(1).unwrap().notes[0].key;
        run(s, Cmd::Transpose, (0.0, 0.0));
        key(s, &winit::keyboard::Key::Character("-".into()), false);
        key(s, &winit::keyboard::Key::Character("3".into()), false);
        accept(s);
        assert_eq!(s.project.pattern(1).unwrap().notes[0].key, k0 - 3);
        run(s, Cmd::Rename, (0.0, 0.0));
        suggest(s, 4, 3);
        assert_eq!(name_of(s, 1), "Chorus");
        assert!(s.pat_ui.name_box.is_none());
    }
}


#[cfg(test)]
mod split_workflow_tests {
    use super::*;
    use crate::{app::App, dialog::{Answer, Reply}, playlist as pl};
    use os_model::{Clip, Note, PlaylistTrackMode};

    #[test]
    fn arranged_split_workflows_preserve_placements_lanes_and_undo() {
        for grouped in [false, true] {
            let mut app = App::new(None);
            app.state.project = crate::app::test_project();
            app.state.pattern = 1;
            pat_mut(&mut app.state, 1).notes = vec![Note::step(0, 0), Note::step(1, 24)];
            let mut source = Clip::new(384, 192, 2, ClipSource::Pattern(1));
            source.window = Some((24.0, 216.0));
            let original = vec![source.clone(), Clip::new(768, 192, 3, ClipSource::Pattern(2))];
            pl::arr_mut(&mut app.state).clips = original.clone();
            let mut alternate = pl::arr_mut(&mut app.state).clone();
            alternate.number = 2;
            app.state.project.arrangements.push(alternate);
            confirm_split(&mut app);
            app.answer(Reply::Button(if grouped { Answer::No } else { Answer::Yes }));
            for arrangement in &app.state.project.arrangements {
                let new_clip = &arrangement.clips[1];
                assert_eq!((new_clip.position, new_clip.length, new_clip.window, new_clip.track), (384, 192, source.window, 3));
                assert_eq!(new_clip.source, ClipSource::Pattern(2));
                assert_eq!(arrangement.clips[2].track, 4);
                assert_eq!(arrangement.clips[2].source, ClipSource::Pattern(3));
                assert_eq!(arrangement.tracks[3].grouped, grouped);
                if !grouped {
                    assert_eq!(arrangement.tracks[3].mode, PlaylistTrackMode::Instrument);
                    assert!(app.state.project.instrument_lanes.iter().any(|v| *v == (1, arrangement.number, 3, 2)));
                }
            }
            pl::run(&mut app, pl::Cmd::Undo);
            assert!(app.state.project.arrangements.iter().all(|a| a.clips == original));
            assert_eq!(app.state.project.pattern(1).unwrap().notes.len(), 2);
        }
    }
}
