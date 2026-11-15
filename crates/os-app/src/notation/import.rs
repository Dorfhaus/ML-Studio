// SPDX-License-Identifier: GPL-3.0-or-later

//! Read MusicXML into the same notes, patterns and Playlist arrangements used by playback.
use super::{Command, IMPORT_VISIBILITY, written};
use crate::{
    app::{App, Win},
    dialog::Then,
    filedlg::{FileDlg, Mode},
    history::{self, Snap},
};
use os_model::{
    Clip, ClipSource, Note, Pattern, TimeMarker,
    notation::{Clef, NoteMarks, StaffChange, Tuplet},
    timeline::{MAX_BPM, MIN_BPM, TempoPoint},
};
use roxmltree::{Document, Node, ParsingOptions};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc,
};

const MAX_BYTES: u64 = 24 * 1024 * 1024;
const MAX_PARTS: usize = 100;
const MAX_NOTES: usize = 200_000;
const MAX_BARS: usize = 1024;

type Element<'a, 'input> = Node<'a, 'input>;

pub(super) type Job = mpsc::Receiver<Result<Score, String>>;

#[derive(Default)]
pub(super) struct Score {
    title: String,
    parts: Vec<Part>,
    meters: Vec<TimeMarker>,
    tempo: Vec<TempoPoint>,
    length: u32,
    omitted: BTreeSet<&'static str>,
    ppq: u16,
}

#[derive(Default)]
struct Part {
    name: String,
    notes: Vec<Note>,
    staff: Vec<StaffChange>,
    velocity: Option<u8>,
    written_dynamic: u8,
}

pub(super) fn dialog(app: &mut App) {
    if !crate::workspace_visibility::shown(IMPORT_VISIBILITY) {
        return;
    }
    if app.state.notation.import.is_some() {
        app.state.message = Some("A score import is already running.".into());
        app.invalidate();
        return;
    }
    app.file_dialog(
        FileDlg::new(Mode::Open, "Import MusicXML", "score", Then::ScoreImport)
            .filter("MusicXML score", &["musicxml", "xml", "mxl"])
            .start(app.state.path.as_deref().and_then(Path::parent)),
    );
}

pub(crate) fn file_done(app: &mut App, path: PathBuf) {
    if app.state.notation.import.is_some() {
        return;
    }
    let ppq = app.state.project.ppq.max(4);
    let (tx, rx) = mpsc::channel();
    app.state.notation.import = Some(rx);
    app.state.message = Some("Reading MusicXML…".into());
    std::thread::spawn(move || {
        let result = read_score(&path, ppq);
        let _ = tx.send(result);
        os_host::wake();
    });
    app.invalidate();
}

pub(super) fn poll(app: &mut App) {
    if app.modal_open() {
        return;
    }
    let Some(rx) = &app.state.notation.import else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => Err("The MusicXML import worker stopped.".into()),
    };
    app.state.notation.import = None;
    if let Err(error) = result.and_then(|score| install(app, score)) {
        app.state.message = Some(format!("MusicXML import failed: {error}"));
        app.invalidate();
    }
}

fn limited(reader: impl Read) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("MusicXML exceeds 24 MiB.".into());
    }
    Ok(bytes)
}

fn text(bytes: Vec<u8>) -> Result<String, String> {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let body = &bytes[2..];
        if !body.len().is_multiple_of(2) {
            return Err("The UTF-16 file is truncated.".into());
        }
        let le = bytes[0] == 0xff;
        let units = body
            .chunks_exact(2)
            .map(|s| {
                let bytes = [s[0], s[1]];
                if le {
                    u16::from_le_bytes(bytes)
                } else {
                    u16::from_be_bytes(bytes)
                }
            })
            .collect::<Vec<_>>();
        return String::from_utf16(&units).map_err(|e| e.to_string());
    }
    String::from_utf8(bytes).map_err(|e| format!("Use UTF-8 or UTF-16 encoding. {e}"))
}

fn xml_document(text: &str) -> Result<Document<'_>, String> {
    // Standard MusicXML files can include a DOCTYPE. roxmltree does not fetch an external DTD.
    Document::parse_with_options(
        text.trim_start_matches('\u{feff}'),
        ParsingOptions {
            allow_dtd: true,
            nodes_limit: 1_000_000,
        },
    )
    .map_err(|e| e.to_string())
}

fn read_score(path: &Path, ppq: u16) -> Result<Score, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let bytes = if path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("mxl"))
    {
        let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        let manifest = text(limited(
            zip.by_name("META-INF/container.xml")
                .map_err(|e| e.to_string())?,
        )?)?;
        let doc = xml_document(&manifest)?;
        let root = doc
            .descendants()
            .find_map(|n| {
                n.has_tag_name("rootfile")
                    .then(|| n.attribute("full-path"))
                    .flatten()
            })
            .ok_or("Missing MusicXML root file.")?;
        if root.starts_with('/') || root.contains('\\') || root.split('/').any(|p| p == "..") {
            return Err("Invalid score path in the archive.".into());
        }
        limited(zip.by_name(root).map_err(|e| e.to_string())?)?
    } else {
        limited(file)?
    };
    let fallback = path
        .file_stem()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    parse(&text(bytes)?, &fallback, ppq)
}

fn child<'a, 'i>(node: Element<'a, 'i>, name: &str) -> Option<Element<'a, 'i>> {
    node.children().find(|n| n.has_tag_name(name))
}

fn value<'a>(node: Element<'a, '_>, name: &str) -> Option<&'a str> {
    child(node, name)?.text().map(str::trim)
}

fn integer(node: Element<'_, '_>, name: &str) -> Option<i32> {
    value(node, name)?.parse().ok()
}

fn number(node: Element<'_, '_>, name: &str) -> Option<f64> {
    value(node, name)?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

fn staff_no(node: Element<'_, '_>) -> u16 {
    integer(node, "staff")
        .unwrap_or(1)
        .clamp(1, MAX_PARTS as i32) as u16
}

fn tick(beats: f64, ppq: u16) -> u32 {
    (beats * f64::from(ppq)).round().max(0.) as u32
}

fn parse(xml: &str, fallback: &str, ppq: u16) -> Result<Score, String> {
    let document = xml_document(xml)?;
    let root = document.root_element();
    let timewise = root.has_tag_name("score-timewise");
    if !timewise && !root.has_tag_name("score-partwise") {
        return Err("Choose a MusicXML partwise or timewise score.".into());
    }
    let title = value(root, "movement-title")
        .or_else(|| value(child(root, "work")?, "work-title"))
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback)
        .chars()
        .take(140)
        .collect();
    let mut score = Score {
        title,
        ppq,
        ..Default::default()
    };
    let mut parts = Vec::<(&str, Vec<(Element<'_, '_>, bool)>)>::new();
    if timewise {
        let bars: Vec<_> = root
            .children()
            .filter(|n| n.has_tag_name("measure"))
            .collect();
        if bars.len() > MAX_BARS {
            return Err("The score has more than 1,024 measures.".into());
        }
        let ids: Vec<_> = bars
            .first()
            .ok_or("The score has no measures.")?
            .children()
            .filter(|n| n.has_tag_name("part"))
            .filter_map(|n| n.attribute("id"))
            .collect();
        if ids.len() > MAX_PARTS {
            return Err("The score has more than 100 parts.".into());
        }
        for id in ids {
            let mut measures = Vec::new();
            for bar in &bars {
                let piece = bar
                    .children()
                    .find(|n| n.has_tag_name("part") && n.attribute("id") == Some(id))
                    .ok_or("A timewise measure is missing a part.")?;
                measures.push((piece, bar.attribute("implicit") == Some("yes")));
            }
            parts.push((id, measures));
        }
    } else {
        for part in root.children().filter(|n| n.has_tag_name("part")) {
            let id = part.attribute("id").unwrap_or("");
            let bars = part
                .children()
                .filter(|n| n.has_tag_name("measure"))
                .map(|n| (n, n.attribute("implicit") == Some("yes")))
                .collect();
            parts.push((id, bars));
        }
    }
    if parts.len() > MAX_PARTS {
        return Err("The score has more than 100 parts.".into());
    }
    let names = child(root, "part-list");
    let mut note_count = 0;
    let mut boundaries = Vec::<(f64, (u8, u8))>::new();
    for (index, (id, bars)) in parts.into_iter().enumerate() {
        if bars.len() > MAX_BARS {
            return Err("The score has more than 1,024 measures.".into());
        }
        let name = names
            .and_then(|n| {
                n.children()
                    .find(|n| n.has_tag_name("score-part") && n.attribute("id") == Some(id))
            })
            .and_then(|n| value(n, "part-name"))
            .filter(|n| !n.is_empty())
            .unwrap_or(id);
        let mut context = PartReader::default();
        let mut time = 0.;
        for (i, (bar, implicit)) in bars.into_iter().enumerate() {
            let start = tick(time, ppq);
            let used = context.read_measure(bar, time, ppq, &mut score)?;
            let (beats, den) = context.meter;
            let expected = f64::from(beats) * 4. / f64::from(den);
            let length = if implicit && used > 0. {
                used
            } else {
                expected.max(used)
            };
            if index == 0 {
                if i == 0
                    || context.meter != boundaries[i - 1].1
                    || (boundaries[i - 1].0 - f64::from(beats) * 4. / f64::from(den)).abs() > 1e-7
                {
                    let mut marker = TimeMarker::new(start, &format!("{beats}/{den}"));
                    marker.kind = TimeMarker::SIGNATURE;
                    marker.numerator = beats;
                    marker.denominator = den;
                    score.meters.push(marker);
                }
                boundaries.push((length, context.meter));
            } else if let Some(&(shared, meter)) = boundaries.get(i) {
                if meter != context.meter || used > shared + 1e-7 {
                    return Err(
                        "The parts have incompatible bar lengths or time signatures.".into(),
                    );
                }
            } else {
                return Err("The parts have different numbers of measures.".into());
            }
            time += boundaries[i].0;
        }
        score.length = score.length.max(tick(time, ppq));
        let multiple = context.parts.len() > 1;
        for (staff, mut part) in context.parts {
            if part.notes.is_empty() {
                continue;
            }
            let title: String = name.chars().take(100).collect();
            part.name = if multiple {
                format!("{title} · Staff {staff}")
            } else {
                title
            };
            part.notes.sort_by_key(|n| n.position);
            part.staff.sort_by_key(|c| c.position);
            note_count += part.notes.len();
            if note_count > MAX_NOTES || score.parts.len() >= MAX_PARTS {
                return Err("The score exceeds the supported note or staff count.".into());
            }
            score.parts.push(part);
        }
    }
    if score.parts.is_empty() {
        return Err("The score has no pitched notes to import.".into());
    }
    score.tempo.sort_by_key(|t| t.tick);
    score.tempo.dedup_by_key(|t| t.tick);
    Ok(score)
}

struct PartReader {
    parts: BTreeMap<u16, Part>,
    ties: BTreeMap<(u16, String, u16), usize>,
    divisions: f64,
    meter: (u8, u8),
    key: Option<i8>,
    transpose: i32,
}

impl Default for PartReader {
    fn default() -> Self {
        Self {
            parts: BTreeMap::new(),
            ties: BTreeMap::new(),
            divisions: 1.,
            meter: (4, 4),
            key: None,
            transpose: 0,
        }
    }
}

impl PartReader {
    fn staff(&mut self, id: u16) -> &mut Part {
        self.parts.entry(id).or_insert_with(|| Part {
            staff: self
                .key
                .map(|key| {
                    vec![StaffChange {
                        fifths: Some(key),
                        ..Default::default()
                    }]
                })
                .unwrap_or_default(),
            ..Default::default()
        })
    }

    fn read_measure(
        &mut self,
        bar: Element<'_, '_>,
        base: f64,
        ppq: u16,
        out: &mut Score,
    ) -> Result<f64, String> {
        let mut cursor: f64 = 0.;
        let mut last: f64 = 0.;
        let mut far: f64 = 0.;
        for item in bar.children().filter(Node::is_element) {
            let delta = number(item, "duration").unwrap_or(0.).max(0.) / self.divisions;
            match item.tag_name().name() {
                "attributes" => self.attributes(item, tick(base + cursor, ppq), cursor, out)?,
                "backup" => cursor = (cursor - delta).max(0.),
                "forward" => cursor += delta,
                "direction" => {
                    let offset = number(item, "offset").unwrap_or(0.) / self.divisions;
                    let staff = self.staff(staff_no(item));
                    if let Some(dynamic) = item
                        .descendants()
                        .find(|n| n.has_tag_name("dynamics"))
                        .and_then(dynamic_mark)
                    {
                        staff.written_dynamic = dynamic;
                    }
                    if let Some(sound) = child(item, "sound") {
                        if let Some(bpm) = sound
                            .attribute("tempo")
                            .and_then(|t| t.parse::<f64>().ok())
                            .filter(|t| (MIN_BPM..=MAX_BPM).contains(t))
                        {
                            out.tempo.push(TempoPoint {
                                tick: tick(base + cursor + offset, ppq),
                                bpm,
                                ramp: false,
                                ramp_end: None,
                            });
                        }
                        if let Some(vel) = sound
                            .attribute("dynamics")
                            .and_then(|v| v.parse::<f64>().ok())
                            .filter(|v| v.is_finite())
                        {
                            staff.velocity = Some((vel * 0.9).round().clamp(1., 127.) as u8);
                        }
                    }
                }
                "note" => {
                    if child(item, "grace").is_some() {
                        out.omitted.insert("grace notes");
                        continue;
                    }
                    if delta <= 0. {
                        continue;
                    }
                    let onset = if child(item, "chord").is_some() {
                        last
                    } else {
                        last = cursor;
                        cursor += delta;
                        last
                    };
                    far = far.max(onset + delta);
                    self.read_note(
                        item,
                        tick(base + onset, ppq),
                        tick(base + onset + delta, ppq),
                        out,
                    )?;
                }
                "barline" if child(item, "repeat").is_some() || child(item, "ending").is_some() => {
                    out.omitted.insert("repeat and ending signs");
                }
                "harmony" => {
                    out.omitted.insert("chord symbols");
                }
                _ => {}
            }
            far = far.max(cursor);
            if far > 256. {
                return Err(
                    "A measure exceeds the supported length. Check its divisions and durations."
                        .into(),
                );
            }
        }
        Ok(far)
    }

    fn attributes(
        &mut self,
        item: Element<'_, '_>,
        pos: u32,
        cursor: f64,
        out: &mut Score,
    ) -> Result<(), String> {
        if let Some(divisions) = number(item, "divisions") {
            if !(1. ..=1_000_000.).contains(&divisions) {
                return Err("Invalid MusicXML divisions.".into());
            }
            self.divisions = divisions;
        }
        if let Some(time) = child(item, "time") {
            let beats = value(time, "beats").and_then(|s| {
                s.split('+')
                    .try_fold(0u16, |a, v| a.checked_add(v.trim().parse().ok()?))
            });
            let denominator = integer(time, "beat-type");
            if let (Some(beats @ 1..=32), Some(den @ (1 | 2 | 4 | 8 | 16 | 32))) =
                (beats, denominator)
            {
                if cursor > 1e-7 {
                    return Err("A time signature changes inside a measure.".into());
                }
                self.meter = (beats as u8, den as u8);
            } else {
                return Err("Unsupported MusicXML time signature.".into());
            }
        }
        if let Some(transpose) = child(item, "transpose") {
            let chromatic = integer(transpose, "chromatic").unwrap_or(0).clamp(-24, 24);
            self.transpose = chromatic
                + 12 * integer(transpose, "octave-change")
                    .unwrap_or(0)
                    .clamp(-3, 3);
            if self.transpose != 0 {
                out.omitted
                    .insert("written transpositions converted to concert pitch");
            }
        }
        let count = integer(item, "staves")
            .unwrap_or(1)
            .clamp(1, MAX_PARTS as i32) as u16;
        for id in 1..=count {
            self.staff(id);
        }
        for node in item.children().filter(Node::is_element) {
            let staff = node
                .attribute("number")
                .and_then(|s| s.parse::<u16>().ok())
                .filter(|&n| (1..=100).contains(&n));
            match node.tag_name().name() {
                "key" => {
                    if cursor > 1e-7 {
                        out.omitted
                            .insert("mid-measure key changes moved to the next bar");
                    }
                    let Some(fifths) = integer(node, "fifths").filter(|n| (-7..=7).contains(n))
                    else {
                        out.omitted.insert("custom key signatures");
                        continue;
                    };
                    if staff.is_none() {
                        self.key = Some(fifths as i8);
                    }
                    let ids: Vec<_> =
                        staff.map_or_else(|| self.parts.keys().copied().collect(), |id| vec![id]);
                    let fifths = if self.transpose == 0 {
                        fifths as i8
                    } else {
                        concert_key(fifths, self.transpose)
                    };
                    for id in ids {
                        self.staff(id).staff.push(StaffChange {
                            position: pos,
                            fifths: Some(fifths),
                            ..Default::default()
                        });
                    }
                }
                "clef" => {
                    if cursor > 1e-7 {
                        out.omitted
                            .insert("mid-measure clef changes moved to the next bar");
                    }
                    let clef = match (value(node, "sign"), integer(node, "line")) {
                        (Some("G"), Some(2) | None) => Clef::Treble,
                        (Some("F"), Some(4) | None) => Clef::Bass,
                        (Some("C"), Some(3)) => Clef::Alto,
                        (Some("C"), Some(4)) => Clef::Tenor,
                        _ => {
                            out.omitted.insert("unsupported clefs");
                            continue;
                        }
                    };
                    self.staff(staff.unwrap_or(1)).staff.push(StaffChange {
                        position: pos,
                        clef: Some(clef),
                        ..Default::default()
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn read_note(
        &mut self,
        node: Element<'_, '_>,
        start: u32,
        end: u32,
        out: &mut Score,
    ) -> Result<(), String> {
        let Some(pitch) = child(node, "pitch") else {
            if child(node, "unpitched").is_some() {
                out.omitted.insert("unpitched percussion");
            }
            return Ok(());
        };
        // The letter itself is a single ASCII MusicXML pitch name.
        let step = value(pitch, "step")
            .and_then(|s| {
                written::LETTERS
                    .iter()
                    .position(|c| s.starts_with(*c) && s.len() == 1)
            })
            .ok_or("Invalid MusicXML pitch step.")?;
        let octave = integer(pitch, "octave")
            .filter(|n| (0..=9).contains(n))
            .ok_or("Invalid MusicXML pitch octave.")?;
        let alter = number(pitch, "alter").unwrap_or(0.);
        if alter.fract() != 0. {
            out.omitted.insert("microtonal pitch rounded to semitones");
        }
        let key = (octave + 1) * 12
            + written::NATURAL[step]
            + (alter.round() as i32).clamp(-12, 12)
            + self.transpose;
        if !(0..=127).contains(&key) {
            return Err("A transposed note falls outside the MIDI pitch range.".into());
        }
        let key = key as u16;
        let staff = staff_no(node);
        let link = (staff, value(node, "voice").unwrap_or("1").to_owned(), key);
        let tie = |kind| {
            node.children()
                .any(|n| n.has_tag_name("tie") && n.attribute("type") == Some(kind))
                || child(node, "notations").is_some_and(|notations| {
                    notations
                        .children()
                        .any(|n| n.has_tag_name("tied") && n.attribute("type") == Some(kind))
                })
        };
        let continued = tie("stop").then(|| self.ties.remove(&link)).flatten();
        if let Some(i) = continued {
            let prior = &mut self.staff(staff).notes[i];
            if prior.position.saturating_add(prior.length).abs_diff(start) <= 1 {
                prior.length = end.saturating_sub(prior.position).max(1);
                if tie("start") {
                    self.ties.insert(link, i);
                }
                return Ok(());
            }
        }
        let notation = child(node, "notations");
        let artic = notation
            .and_then(|n| child(n, "articulations"))
            .map(|edge| {
                let id = written::ARTICULATIONS
                    .iter()
                    .position(|(xml, _)| !xml.is_empty() && child(edge, xml).is_some());
                if id.is_none() {
                    out.omitted.insert("unsupported articulations");
                }
                id.unwrap_or(0) as u8
            })
            .unwrap_or(0);
        let ratio = child(node, "time-modification");
        let tuplet = ratio
            .map(|ratio| {
                let has = |kind| {
                    notation.is_some_and(|n| {
                        n.children()
                            .any(|n| n.has_tag_name("tuplet") && n.attribute("type") == Some(kind))
                    })
                };
                Tuplet {
                    actual: integer(ratio, "actual-notes")
                        .and_then(|n| n.try_into().ok())
                        .unwrap_or(0),
                    normal: integer(ratio, "normal-notes")
                        .and_then(|n| n.try_into().ok())
                        .unwrap_or(0),
                    value: written::VALUES
                        .iter()
                        .find(|v| Some(v.1) == value(node, "type"))
                        .map_or(0, |v| v.0 as u8),
                    dots: node
                        .children()
                        .filter(|n| n.has_tag_name("dot"))
                        .count()
                        .min(3) as u8,
                    start: has("start"),
                    end: has("stop"),
                }
            })
            .filter(|r| r.valid())
            .unwrap_or_default();
        if ratio.is_some() && !tuplet.valid() {
            out.omitted.insert("unsupported tuplet markings");
        }
        for (tag, label) in [
            ("lyric", "lyrics"),
            ("ornaments", "ornaments"),
            ("slur", "slurs"),
            ("technical", "instrument techniques"),
        ] {
            if child(node, tag).is_some() || notation.is_some_and(|n| child(n, tag).is_some()) {
                out.omitted.insert(label);
            }
        }
        let letter = if self.transpose == 0 {
            step as u8 + 1
        } else {
            0
        };
        let part = self.staff(staff);
        let dynamic = notation
            .and_then(|n| child(n, "dynamics"))
            .and_then(dynamic_mark)
            .unwrap_or_else(|| std::mem::take(&mut part.written_dynamic));
        let index = part.notes.len();
        part.notes.push(Note {
            key,
            length: end.saturating_sub(start).max(1),
            velocity: part.velocity.unwrap_or(100),
            notation: NoteMarks {
                letter,
                accidental: child(node, "accidental").is_some(),
                articulation: artic,
                dynamic,
                tuplet,
            },
            ..Note::step(0, start)
        });
        if tie("start") {
            self.ties.insert(link, index);
        }
        Ok(())
    }
}

fn concert_key(written: i32, offset: i32) -> i8 {
    let tonic = (7 * written + offset).rem_euclid(12);
    (-7i32..=7)
        .filter(|&n| (7 * n).rem_euclid(12) == tonic)
        .min_by_key(|i| i.abs())
        .unwrap_or(0) as i8
}

fn dynamic_mark(node: Element<'_, '_>) -> Option<u8> {
    written::DYNAMICS
        .iter()
        .position(|name| !name.is_empty() && child(node, name).is_some())
        .map(|n| n as u8)
}

fn install(app: &mut App, mut score: Score) -> Result<(), String> {
    let s = &app.state;
    if score.ppq != s.project.ppq.max(4) {
        return Err("The project timebase changed. Import the score again.".into());
    }
    let reserved: BTreeSet<_> = s
        .project
        .patterns
        .iter()
        .map(|p| p.number)
        .chain(
            s.project
                .arrangements
                .iter()
                .flat_map(|a| &a.clips)
                .filter_map(|c| match c.source {
                    ClipSource::Pattern(p) => Some(p),
                    _ => None,
                }),
        )
        .collect();
    let ids: Vec<_> = (1..=crate::patterns::MAX)
        .filter(|n| !reserved.contains(n))
        .take(score.parts.len())
        .collect();
    let tracks = score.parts.len();
    if ids.len() != tracks {
        return Err("Free up some pattern numbers before importing this score.".into());
    }
    let number = (0..=255u16)
        .find(|n| !s.project.arrangements.iter().any(|a| a.number == *n))
        .ok_or("There are no free arrangement numbers.")?;
    if usize::from(
        s.project
            .channels
            .iter()
            .map(|c| c.index)
            .max()
            .unwrap_or(0),
    ) + tracks
        >= 4096
    {
        return Err("There is not enough room for the score instruments.".into());
    }
    app.transport.stop();
    let s = &mut app.state;
    let base = s.project.channels.len();
    let undo = Snap::Several(vec![
        Snap::ScoreArrangement(
            s.project.arrangements.clone(),
            s.project.current_arrangement,
            s.song_mode,
        ),
        Snap::Patterns {
            patterns: s.project.patterns.clone(),
            current: s.pattern,
        },
        Snap::Tail(Box::new(history::Tail::now(s, base))),
    ]);
    history::record(s, "Import MusicXML", undo);
    let mut a = os_flp::project::default_arrangement();
    a.name = Some(score.title.clone());
    a.number = number;
    a.markers = score.meters;
    a.timeline.tempo = score.tempo;
    let length = score.length.max(1);
    for (i, (mut part, pattern)) in score.parts.drain(..).zip(ids.iter().copied()).enumerate() {
        let row = crate::app::add_plugin_channel(&mut s.project, crate::osc3::LEMON_TRIO);
        let ch = &mut s.project.channels[row];
        ch.name = Some(part.name.clone());
        let channel = ch.index;
        for note in &mut part.notes {
            note.channel = channel;
        }
        for change in &mut part.staff {
            change.channel = channel;
        }
        a.staff_notation.extend(part.staff);
        a.tracks[i].name = Some(part.name.clone());
        a.clips
            .push(Clip::new(0, length, i as u16, ClipSource::Pattern(pattern)));
        s.project.patterns.push(Pattern {
            number: pattern,
            name: Some(part.name),
            notes: part.notes,
            length: Some(length),
            markers: a.markers.clone(),
            ..Default::default()
        });
    }
    a.staff_notation.sort_by_key(|c| (c.position, c.channel));
    s.project.arrangements.push(a);
    s.project.current_arrangement = Some(number);
    crate::playlist::load_view(s);
    s.playlist.header_selection.clear();
    s.playlist.sel.clear();
    s.playlist.pos = 0;
    s.song_mode = true;
    s.selected_channel = base;
    s.rack.filter = crate::rack::Filter::All;
    s.rack.select_only(base);
    crate::rack::autosize(s);
    crate::patterns::select(s, ids[0]);
    s.piano.clear_selection();
    s.piano.dirty = true;
    let editor = &mut s.notation;
    editor.song = None;
    editor.page = 0;
    editor.resolution = 3;
    editor.started = None;
    editor.checked = None;
    s.raise(Win::Score);
    if crate::workspace_visibility::shown(super::ARRANGEMENT_VISIBILITY) {
        super::run(app, Command::Mode);
    }
    let omitted = if score.omitted.is_empty() {
        String::new()
    } else {
        format!(
            ". Simplified: {}",
            score.omitted.into_iter().collect::<Vec<_>>().join(", ")
        )
    };
    app.state.message = Some(format!(
        "Imported {} as a new arrangement · {tracks} staves{omitted}",
        score.title
    ));
    app.invalidate();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notation::{arrangement, engrave, export};
    use std::io::Write;

    const MUSIC: &str = r#"<?xml version="1.0"?>
<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">
<score-partwise version="4.0">
<work><work-title>Two Parts</work-title></work>
<part-list><score-part id="P1"><part-name>Lead</part-name></score-part><score-part id="P2"><part-name>Bass</part-name></score-part></part-list>
<part id="P1">
 <measure number="1">
  <attributes><divisions>12</divisions><key><fifths>1</fifths></key><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>G</sign><line>2</line></clef></attributes>
  <direction><direction-type><dynamics><mf/></dynamics></direction-type><sound tempo="108" dynamics="80"/></direction>
  <note><pitch><step>F</step><alter>1</alter><octave>4</octave></pitch><duration>4</duration><voice>1</voice><type>eighth</type><time-modification><actual-notes>3</actual-notes><normal-notes>2</normal-notes></time-modification><notations><tuplet type="start"/><articulations><staccato/></articulations></notations></note>
  <note><pitch><step>G</step><octave>4</octave></pitch><duration>4</duration><voice>1</voice><type>eighth</type><time-modification><actual-notes>3</actual-notes><normal-notes>2</normal-notes></time-modification></note>
  <note><pitch><step>A</step><octave>4</octave></pitch><duration>4</duration><voice>1</voice><type>eighth</type><time-modification><actual-notes>3</actual-notes><normal-notes>2</normal-notes></time-modification><notations><tuplet type="stop"/></notations></note>
  <note><pitch><step>E</step><alter>-1</alter><octave>4</octave></pitch><duration>36</duration><tie type="start"/><voice>1</voice><type>half</type><dot/><accidental>flat</accidental><notations><tied type="start"/></notations></note>
 </measure>
 <measure number="2">
  <attributes><key><fifths>-2</fifths></key><clef><sign>C</sign><line>3</line></clef></attributes>
  <note><pitch><step>E</step><alter>-1</alter><octave>4</octave></pitch><duration>12</duration><tie type="stop"/><voice>1</voice><type>quarter</type><notations><tied type="stop"/></notations></note>
  <note><rest/><duration>36</duration><voice>1</voice><type>half</type><dot/></note>
 </measure>
</part>
<part id="P2">
 <measure number="1">
  <attributes><divisions>12</divisions><time><beats>4</beats><beat-type>4</beat-type></time><clef><sign>F</sign><line>4</line></clef></attributes>
  <note><pitch><step>G</step><octave>2</octave></pitch><duration>48</duration><type>whole</type></note>
  <note><chord/><pitch><step>D</step><octave>3</octave></pitch><duration>48</duration><type>whole</type></note>
 </measure>
 <measure number="2">
  <note><pitch><step>C</step><octave>3</octave></pitch><duration>48</duration><type>whole</type></note>
 </measure>
</part>
</score-partwise>"#;

    #[test]
    fn import_transfers_midi_marks_and_undo_and_round_trips() {
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state.project.ppq = 96;
        let before = app.state.project.clone();
        let score = parse(MUSIC, "Test", 96).unwrap();
        assert_eq!(score.parts.len(), 2);
        assert_eq!(
            score.parts[0]
                .notes
                .iter()
                .map(|n| (n.key, n.position, n.length))
                .collect::<Vec<_>>(),
            [(66, 0, 32), (67, 32, 32), (69, 64, 32), (63, 96, 384)]
        );
        assert_eq!(score.parts[0].notes[0].notation.articulation, 1);
        assert_eq!(score.parts[0].notes[0].notation.dynamic, 5);
        assert_eq!(score.parts[0].staff.last().unwrap().clef, Some(Clef::Alto));
        install(&mut app, score).unwrap();
        let index = app.state.project.arrangement_index();
        let arrangement = app.state.project.arrangement().unwrap();
        assert_eq!(arrangement.clips.len(), 2);
        assert_eq!(arrangement.name.as_deref(), Some("Two Parts"));
        assert_eq!(arrangement.timeline.tempo.first().unwrap().bpm, 108.);
        let req = arrangement::whole_score(&app.state).unwrap();
        let mei = engrave::test_markup(&req);
        assert!(mei.contains(r#"num="3" numbase="2" bracket.visible="true""#));
        assert!(mei.contains("artic=\"stacc\"") && mei.contains(">mf</dynam>"));
        assert!(mei.contains("key.sig=\"2f\"") && mei.contains("clef.line=\"3\""));
        if engrave::available() {
            assert!(engrave::render(&req).unwrap().notes.len() >= 7);
        }
        let xml = export::musicxml(&req, "Two Parts").unwrap();
        let back = parse(&xml, "", 96).unwrap();
        assert_eq!(
            back.parts[0]
                .notes
                .iter()
                .map(|n| (n.key, n.position, n.length))
                .collect::<Vec<_>>(),
            [(66, 0, 32), (67, 32, 32), (69, 64, 32), (63, 96, 384)]
        );
        assert_eq!(back.parts[0].notes[0].notation.articulation, 1);
        assert_eq!(back.parts[0].notes[0].notation.dynamic, 5);
        assert!(back.parts[0].notes[2].notation.tuplet.end);
        assert_eq!(back.parts[0].staff.last().unwrap().clef, Some(Clef::Alto));
        let file = os_flp::project::create(&app.state.project).unwrap();
        let reread = os_flp::project::load(&file).unwrap();
        assert_eq!(
            reread.pattern(app.state.pattern).unwrap().notes,
            app.state.project.pattern(app.state.pattern).unwrap().notes
        );
        assert_eq!(
            reread.arrangements[index].staff_notation,
            app.state.project.arrangements[index].staff_notation
        );
        assert!(crate::piano::undo(&mut app.state));
        assert_eq!(app.state.project.arrangements, before.arrangements);
        assert_eq!(app.state.project.channels, before.channels);
        assert_eq!(app.state.project.patterns, before.patterns);
        assert!(crate::history::redo(&mut app.state).is_some());
        assert_eq!(app.state.project.arrangement_index(), index);
        if engrave::available() {
            let req = super::super::snapshot(&app.state).unwrap();
            app.state.notation.frame = Some(engrave::render(&req).unwrap());
            app.state.notation.started = Some(req.clone());
            app.state.notation.latest = Some(req);
            app.state.native_view = Some(Win::Score);
            app.state.native_input = Some(Win::Score);
            let mut canvas = os_ui::Canvas::new(1100, 650, 1.).unwrap();
            app.paint(&mut canvas, None);
            let action = app
                .hits
                .iter()
                .find(|h| h.action == crate::app::Action::Score(Command::Note(0)))
                .unwrap()
                .action
                .clone();
            app.act(action);
            super::super::run(&mut app, Command::Marks);
            assert!(
                app.state
                    .playlist
                    .popup
                    .as_ref()
                    .is_some_and(|p| p.items.iter().any(|i| i.label == "Pitch spelling"))
            );
            app.state.playlist.popup = None;
            super::super::run(
                &mut app,
                Command::SetMarks(super::super::marks::Edit::Letter(5)),
            );
            let n = app.state.project.pattern(app.state.pattern).unwrap().notes[0];
            assert_eq!(
                (n.key, n.notation.letter),
                (66, 5),
                "F sharp respells as G flat at the same MIDI key"
            );
            assert!(crate::piano::undo(&mut app.state));
        }
        if let Some(dir) = std::env::var_os("LEMON_SCORE_TEST_EXPORTS") {
            let dir = PathBuf::from(dir);
            std::fs::write(dir.join("composer-roundtrip.musicxml"), xml).unwrap();
            if engrave::available() {
                let html = export::html(&req, "Two Parts").unwrap();
                std::fs::write(dir.join("composer-parts.html"), html).unwrap();
            }
        }
    }

    #[test]
    fn imports_standard_compressed_musicxml() {
        let work = tempfile::tempdir().unwrap();
        let path = work.path().join("Fixture.mxl");
        let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("META-INF/container.xml", options).unwrap();
        zip.write_all(br#"<container><rootfiles><rootfile full-path="scores/main.musicxml"/></rootfiles></container>"#).unwrap();
        zip.start_file("scores/main.musicxml", options).unwrap();
        zip.write_all(MUSIC.as_bytes()).unwrap();
        zip.finish().unwrap();
        let score = read_score(&path, 96).unwrap();
        assert_eq!(score.parts.len(), 2);
        assert_eq!(score.length, 768);
    }
}
