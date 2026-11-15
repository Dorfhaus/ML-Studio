// SPDX-License-Identifier: GPL-3.0-or-later

use super::{
    MAX_NOTES, Measure, Request,
    written::{self, DIVISIONS, Duration as WrittenDuration, UNIT},
};
use os_model::notation::{NoteMarks, Tuplet};
use os_ui::Rect;
use resvg::{tiny_skia, usvg};
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs::{self, File},
    io::{self, Cursor},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const BUNDLE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/score-engine.zip"));
const VERSION: &str = "6.3.0-425dd7b";

pub(super) struct Frame {
    pub w: usize,
    pub h: usize,
    pub pixels: Vec<u8>,
    pub notes: Vec<(usize, Rect)>,
    pub bars: Vec<(u32, Rect)>,
    pub staves: Vec<(u32, usize, Rect)>,
}

#[derive(Clone, Copy)]
pub(super) struct Slice {
    pub index: usize,
    pub key: u16,
    pub marks: NoteMarks,
    from: u32,
    to: u32,
}

fn silence(xml: &mut String, units: u32, voice: usize) {
    let tag = if voice == 0 { "rest" } else { "space" };
    for d in written::durations(units, Tuplet::default()) {
        if let Some(ratio) = d.ratio {
            open_tuplet(xml, ratio, false);
        }
        write!(xml, "<{tag}{}/>", mei_duration(d)).unwrap();
        if d.ratio.is_some() {
            xml.push_str("</tuplet>");
        }
    }
}

fn mei_duration(d: WrittenDuration) -> String {
    if d.dots == 0 {
        format!(r#" dur="{}""#, d.value)
    } else {
        format!(r#" dur="{}" dots="{}""#, d.value, d.dots)
    }
}

fn open_tuplet(xml: &mut String, (actual, normal): (u32, u32), visible: bool) {
    write!(xml, r#"<tuplet num="{actual}" numbase="{normal}" bracket.visible="{visible}" num.visible="{visible}">"#).unwrap();
}

fn note(
    xml: &mut String,
    slice: Slice,
    duration: &str,
    part: u16,
    previous: &mut BTreeMap<usize, String>,
    controls: &mut String,
    key_signature: i8,
    accidentals: &mut written::Accidentals,
) {
    let id = format!("ml-{}-{part}", slice.index);
    let (step, alter, octave, visible) = accidentals.pitch(slice.key, slice.marks, key_signature);
    let pitch = written::LETTERS[step].to_ascii_lowercase();
    let accidental = ["ff", "f", "n", "s", "ss"][(alter + 2) as usize];
    write!(
        xml,
        r#"<note xml:id="{id}" pname="{pitch}" oct="{octave}"{duration}"#
    )
    .unwrap();
    if previous.insert(slice.index, id.clone()).is_some() {
        write!(
            controls,
            r##"<tie startid="#ml-{}-{}" endid="#{id}"/>"##,
            slice.index,
            part - 1
        )
        .unwrap();
    }
    if part == 0 {
        if let Some(&(_, artic)) = written::ARTICULATIONS
            .get(usize::from(slice.marks.articulation))
            .filter(|x| !x.1.is_empty())
        {
            write!(xml, r#" artic="{artic}""#).unwrap();
        }
        if let Some(dynamic) = written::DYNAMICS
            .get(usize::from(slice.marks.dynamic))
            .filter(|x| !x.is_empty())
        {
            write!(
                controls,
                r##"<dynam startid="#{id}" place="below">{dynamic}</dynam>"##
            )
            .unwrap();
        }
    }
    if visible {
        write!(xml, r#"><accid accid="{accidental}"/></note>"#).unwrap();
    } else {
        write!(xml, r#" accid.ges="{accidental}"/>"#).unwrap();
    }
}

pub(super) fn units(ticks: u32, ppq: u32, grid: u32) -> u32 {
    if grid == 0 {
        return precise_units(ticks, ppq);
    }
    let denominator = u64::from(ppq * grid);
    (((u64::from(ticks) * 8 + denominator / 2) / denominator) * u64::from(grid * UNIT))
        .min(u64::from(u32::MAX)) as u32
}

pub(super) fn capacity(req: &Request, bar: &Measure) -> u32 {
    let minimum = if req.grid == 0 { 1 } else { UNIT };
    units(bar.end - bar.start, req.ppq, u32::from(req.grid != 0)).clamp(minimum, 512 * UNIT)
}

fn precise_units(ticks: u32, ppq: u32) -> u32 {
    ((u64::from(ticks) * u64::from(DIVISIONS) + u64::from(ppq / 2)) / u64::from(ppq))
        .min(u64::from(u32::MAX)) as u32
}

pub(super) fn staff_notes(
    req: &Request,
    bar: &Measure,
    staff: usize,
    capacity: u32,
) -> Vec<Vec<(u32, u32, Vec<Slice>)>> {
    let mut chords = BTreeMap::<(u32, u32), Vec<Slice>>::new();
    for n in &req.notes {
        if n.staff != staff || n.note.key > 127 {
            continue;
        }
        let start = n.note.position.max(bar.start);
        let end = n.note.position.saturating_add(n.length).min(bar.end);
        if start >= end {
            continue;
        }
        let mut marks = n.note.notation;
        let minimum = if req.grid == 0 { 1 } else { UNIT.min(capacity) };
        let tuplet = marks.tuplet;
        let (from, to) = if tuplet.valid() && start == n.note.position && end - start == n.length {
            let target = written::nominal(u32::from(tuplet.value), tuplet.dots, DIVISIONS)
                * u32::from(tuplet.normal)
                / u32::from(tuplet.actual);
            let error = (DIVISIONS / req.ppq).max(1);
            let raw = precise_units(start - bar.start, req.ppq);
            let quantum = written::gcd(UNIT, target).max(1);
            let snapped = ((raw + quantum / 2) / quantum) * quantum;
            let from = if raw.abs_diff(snapped) <= error {
                snapped
            } else {
                raw
            };
            if target > 0
                && from.saturating_add(target) <= capacity
                && precise_units(n.length, req.ppq).abs_diff(target) <= error
            {
                (from, from + target)
            } else {
                marks.tuplet = Tuplet::default();
                let from = units(start - bar.start, req.ppq, req.grid).min(capacity - minimum);
                (
                    from,
                    units(end - bar.start, req.ppq, req.grid).clamp(from + minimum, capacity),
                )
            }
        } else {
            marks.tuplet = Tuplet::default();
            let from = units(start - bar.start, req.ppq, req.grid).min(capacity - minimum);
            (
                from,
                units(end - bar.start, req.ppq, req.grid).clamp(from + minimum, capacity),
            )
        };
        let notes = chords.entry((from, to)).or_default();
        // Unison MIDI notes remain individually selectable even if their heads coincide.
        notes.push(Slice {
            index: n.index,
            key: n.note.key,
            marks,
            from,
            to,
        });
    }
    let mut voices: Vec<Vec<(u32, u32, Vec<Slice>)>> = vec![Vec::new()];
    for ((from, to), notes) in chords {
        let mut groups: Vec<Vec<Slice>> = Vec::new();
        for note in notes {
            if let Some(group) = groups
                .iter_mut()
                .find(|group| group.iter().all(|other| other.key != note.key))
            {
                group.push(note);
            } else {
                groups.push(vec![note]);
            }
        }
        for group in groups {
            if let Some(voice) = voices
                .iter_mut()
                .find(|voice| voice.last().is_none_or(|entry| entry.1 <= from))
            {
                voice.push((from, to, group));
            } else {
                voices.push(vec![(from, to, group)]);
            }
        }
    }
    voices
}

pub(super) fn mei(req: &Request) -> Result<(String, Vec<(String, usize)>), String> {
    if req.bars.len() <= super::BARS && req.notes.len() > MAX_NOTES {
        return Err("Too many notes in these four measures. Try a less dense passage.".into());
    }
    let mut xml = String::from(
        r#"<mei xmlns="http://www.music-encoding.org/ns/mei" meiversion="5.0"><meiHead><fileDesc><titleStmt><title>ML Studio</title></titleStmt><pubStmt/></fileDesc></meiHead><music><body><mdiv><score>"#,
    );
    let (num, den) = req.bars[0].meter;
    write!(xml, r#"<scoreDef meter.count="{num}" meter.unit="{den}">"#).unwrap();
    let staff_count = if let Some(song) = &req.song {
        if song.staves.is_empty() {
            return Err("No playable pattern instruments in these Playlist rows.".into());
        }
        xml.push_str(r#"<staffGrp symbol="bracket" bar.thru="true">"#);
        for (i, staff) in song.staves.iter().enumerate() {
            let label = staff
                .label
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;");
            let (clef, key) = req.staff_style(i, req.bars[0].start);
            let (sign, line) = clef.sign_line();
            let key = mei_key(key);
            write!(xml, r#"<staffDef n="{}" lines="5" clef.shape="{sign}" clef.line="{line}" key.sig="{key}"><label>{label}</label></staffDef>"#, i + 1).unwrap();
        }
        song.staves.len()
    } else {
        xml.push_str(r#"<staffGrp symbol="brace" bar.thru="true"><staffDef n="1" lines="5" clef.shape="G" clef.line="2"/><staffDef n="2" lines="5" clef.shape="F" clef.line="4"/>"#);
        2
    };
    xml.push_str("</staffGrp></scoreDef><section>");
    let mut last = (num, den);
    let mut staff_settings: Vec<_> = (0..staff_count)
        .map(|staff| req.staff_style(staff, req.bars[0].start))
        .collect();
    let mut previous = BTreeMap::new();
    let mut pieces = BTreeMap::<usize, u16>::new();
    let mut ids = Vec::new();
    for bar in &req.bars {
        let next: Vec<_> = (0..staff_count)
            .map(|staff| req.staff_style(staff, bar.start))
            .collect();
        if bar.meter != last || next != staff_settings {
            let (num, den) = bar.meter;
            write!(
                xml,
                r#"<scoreDef meter.count="{num}" meter.unit="{den}"><staffGrp>"#
            )
            .unwrap();
            for (i, (&prev, &(clef, key))) in staff_settings.iter().zip(&next).enumerate() {
                if prev == (clef, key) {
                    continue;
                }
                write!(xml, "<staffDef n=\"{}\"", i + 1).unwrap();
                if clef != prev.0 {
                    let (sign, line) = clef.sign_line();
                    write!(xml, r#" clef.shape="{sign}" clef.line="{line}""#).unwrap();
                }
                if key != prev.1 {
                    write!(xml, r#" key.sig="{}""#, mei_key(key)).unwrap();
                }
                xml.push_str("/>");
            }
            xml.push_str("</staffGrp></scoreDef>");
            last = bar.meter;
            staff_settings = next;
        }
        let capacity = capacity(req, bar);
        write!(
            xml,
            r#"<measure n="{0}" xml:id="score-measure-{0}">"#,
            bar.number
        )
        .unwrap();
        let mut ties = String::new();
        for (staff, &(_, key_signature)) in staff_settings.iter().enumerate() {
            write!(
                xml,
                r#"<staff n="{0}" xml:id="score-staff-{1}-{0}">"#,
                staff + 1,
                bar.number
            )
            .unwrap();
            for (voice, items) in staff_notes(req, bar, staff, capacity)
                .into_iter()
                .enumerate()
            {
                if voice >= 16 {
                    return Err(
                        "This passage has more than 16 overlapping notation voices per staff."
                            .into(),
                    );
                }
                write!(xml, r#"<layer n="{}">"#, voice + 1).unwrap();
                let mut cursor = 0;
                let mut accidentals = written::Accidentals::default();
                let mut group_end = None;
                for (i, (from, to, notes)) in items.iter().enumerate() {
                    silence(&mut xml, *from - cursor, voice);
                    let marks = notes[0].marks.tuplet;
                    if marks.start {
                        group_end = tuplet_end(&items, i);
                        if group_end.is_some() {
                            open_tuplet(
                                &mut xml,
                                (u32::from(marks.actual), u32::from(marks.normal)),
                                true,
                            );
                        }
                    }
                    for d in written::durations(to - from, marks) {
                        let wrapped = d.ratio.filter(|_| group_end.is_none());
                        if let Some(ratio) = wrapped {
                            open_tuplet(&mut xml, ratio, false);
                        }
                        let duration = mei_duration(d);
                        if notes.len() > 1 {
                            write!(xml, "<chord{duration}>").unwrap();
                        }
                        for &n in notes {
                            debug_assert_eq!((n.from, n.to), (*from, *to));
                            let part = pieces.entry(n.index).or_default();
                            ids.push((format!("ml-{}-{part}", n.index), n.index));
                            note(
                                &mut xml,
                                n,
                                if notes.len() == 1 { &duration } else { "" },
                                *part,
                                &mut previous,
                                &mut ties,
                                key_signature,
                                &mut accidentals,
                            );
                            *part += 1;
                        }
                        if notes.len() > 1 {
                            xml.push_str("</chord>");
                        }
                        if wrapped.is_some() {
                            xml.push_str("</tuplet>");
                        }
                    }
                    if group_end == Some(i) {
                        xml.push_str("</tuplet>");
                        group_end = None;
                    }
                    cursor = *to;
                }
                if cursor == 0 && voice == 0 {
                    xml.push_str("<mRest/>");
                } else {
                    silence(&mut xml, capacity - cursor, voice);
                }
                xml.push_str("</layer>");
            }
            xml.push_str("</staff>");
        }
        for point in req
            .tempo
            .iter()
            .filter(|p| (bar.start..bar.end).contains(&p.tick))
        {
            let tstamp = 1.
                + f64::from(point.tick - bar.start) * f64::from(bar.meter.1)
                    / (4. * f64::from(req.ppq));
            let bpm = point.bpm;
            if bpm.is_finite() && bpm > 0. {
                write!(
                    ties,
                    r#"<tempo tstamp="{tstamp}" staff="1" mm="{bpm}" mm.unit="4">♩ = {bpm}</tempo>"#
                )
                .unwrap();
            }
        }
        xml.push_str(&ties);
        xml.push_str("</measure>");
    }
    xml.push_str("</section></score></mdiv></body></music></mei>");
    Ok((xml, ids))
}

fn mei_key(key: i8) -> String {
    if key == 0 {
        return "0".into();
    }
    format!("{}{}", key.unsigned_abs(), if key < 0 { 'f' } else { 's' })
}

pub(super) fn tuplet_end(items: &[(u32, u32, Vec<Slice>)], start: usize) -> Option<usize> {
    let (mut end, mark) = (items[start].0, items[start].2[0].marks.tuplet);
    for (i, (left, right, notes)) in items.iter().enumerate().skip(start) {
        let tag = notes[0].marks.tuplet;
        if !tag.valid()
            || tag.actual != mark.actual
            || tag.normal != mark.normal
            || *left != end
            || (i != start && tag.start)
        {
            return None;
        }
        if tag.end {
            return Some(i);
        }
        end = *right;
    }
    None
}

fn executable(dir: &Path) -> PathBuf {
    dir.join(if cfg!(windows) {
        "verovio.exe"
    } else {
        "verovio"
    })
}

fn show_error(error: impl std::fmt::Display) -> String {
    format!("Score renderer: {error}")
}

pub(super) fn available() -> bool {
    !BUNDLE.is_empty() || std::env::var_os("LEMON_VEROVIO_DIR").is_some()
}

pub(super) fn engine() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("LEMON_VEROVIO_DIR").map(PathBuf::from) {
        if executable(&path).is_file() && path.join("data").is_dir() {
            return Ok(path);
        }
        return Err(
            "The configured Verovio folder needs the Verovio executable and data folder.".into(),
        );
    }
    if BUNDLE.is_empty() {
        return Err("Verovio was not included in this build.".into());
    }
    let home = os_paths::user_dir()
        .ok_or("The ML Studio profile is unavailable.")?
        .join("Score Editor");
    fs::create_dir_all(&home).map_err(show_error)?;
    static KEY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let key = KEY.get_or_init(|| {
        use sha2::{Digest, Sha256};
        let hash = format!("{:x}", Sha256::digest(BUNDLE));
        format!("{VERSION}-{}", &hash[..12])
    });
    let root = home.join(key);
    if executable(&root).is_file() && root.join("data/Bravura.xml").is_file() {
        crate::job_scratch::keep_revision(&home, &root).map_err(show_error)?;
        return Ok(root);
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(BUNDLE)).map_err(show_error)?;
    let mut files = Vec::new();
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(show_error)?;
        let path = file.enclosed_name().ok_or("Invalid path in the score engine archive.")?;
        if !file.is_dir() {
            files.push(path.to_string_lossy().replace('\\', "/"));
        }
    }
    let mut staged_files: Vec<_> = files.iter().map(String::as_str).collect();
    staged_files.push(".ml-revision-files.json");
    let stage = crate::job_scratch::create(&home, "score-install", &staged_files).map_err(show_error)?;
    let mut bytes = 0;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(show_error)?;
        bytes += file.size();
        if bytes > 64 * 1024 * 1024 {
            return Err("Score renderer archive is too large.".into());
        }
        let rel = file
            .enclosed_name()
            .ok_or("Invalid path in the score engine archive.")?;
        let dest = stage.path().join(rel);
        if file.is_dir() {
            fs::create_dir_all(dest).map_err(show_error)?;
            continue;
        }
        fs::create_dir_all(dest.parent().unwrap()).map_err(show_error)?;
        io::copy(&mut file, &mut File::create(&dest).map_err(show_error)?).map_err(show_error)?;
        #[cfg(unix)]
        if dest == executable(stage.path()) {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&dest, fs::Permissions::from_mode(0o755)).map_err(show_error)?;
        }
    }
    if !executable(stage.path()).is_file() || !stage.path().join("data/Bravura.xml").is_file() {
        return Err("The bundled Verovio engine is incomplete.".into());
    }
    crate::job_scratch::mark_revision(stage.path(), &files).map_err(show_error)?;
    if let Err(error) = fs::rename(stage.path(), &root) {
        if !executable(&root).is_file() {
            return Err(show_error(error));
        }
    }
    let _ = fs::remove_file(root.join(".ml-studio-job"));
    crate::job_scratch::keep_revision(&home, &root).map_err(show_error)?;
    Ok(root)
}

pub(super) fn svg(root: &Path, mei: &str, print: bool) -> Result<String, String> {
    let work = crate::job_scratch::create(
        root.parent().unwrap_or(root), "score-render", &["score.mei", "score.svg"],
    ).map_err(show_error)?;
    let input = work.path().join("score.mei");
    let output = work.path().join("score.svg");
    fs::write(&input, mei).map_err(show_error)?;
    let mut child = Command::new(executable(root));
    child
        .current_dir(root)
        .args([
            "-r", "data", "-f", "mei", "-t", "svg", "-o", "-", "--stdin", "--header", "none",
            "--footer", "none",
        ])
        .stdin(File::open(input).map_err(show_error)?)
        .stdout(File::create(&output).map_err(show_error)?)
        .stderr(Stdio::null());
    if print {
        child.args([
            "--all-pages",
            "--breaks",
            "auto",
            "--page-width",
            "2100",
            "--page-height",
            "2970",
        ]);
    } else {
        child.args([
            "--breaks",
            "none",
            "--page-width",
            "1200",
            "--adjust-page-height",
        ]);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        child.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut process = child.spawn().map_err(show_error)?;
    let start = Instant::now();
    loop {
        match process.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) => return Err("Verovio could not engrave this passage.".into()),
            Ok(None) if start.elapsed() < Duration::from_secs(12) => {
                std::thread::sleep(Duration::from_millis(8))
            }
            _ => {
                let _ = process.kill();
                let _ = process.wait();
                return Err("Verovio did not finish engraving this passage.".into());
            }
        }
    }
    if fs::metadata(&output).map_err(show_error)?.len() > 8 * 1024 * 1024 {
        return Err("This passage needs too much space to display.".into());
    }
    fs::read_to_string(output).map_err(show_error)
}

fn fonts() -> std::sync::Arc<usvg::fontdb::Database> {
    use std::sync::{Arc, OnceLock};
    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut database = usvg::fontdb::Database::new();
            database.load_system_fonts();
            Arc::new(database)
        })
        .clone()
}

pub(super) fn render(req: &Request) -> Result<Frame, String> {
    let (markup, ids) = mei(req)?;
    let svg = svg(&engine()?, &markup, false)?;
    let options = usvg::Options {
        fontdb: fonts(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(&svg, &options).map_err(show_error)?;
    // Render at twice engraving size for sharp noteheads on high-DPI displays.
    let scale = 2.0f32.min(2400.0 / tree.size().width().max(tree.size().height()));
    let w = (tree.size().width() * scale).ceil() as u32;
    let h = (tree.size().height() * scale).ceil() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or("Cannot allocate the score page.")?;
    pixmap.fill(tiny_skia::Color::from_rgba8(248, 245, 237, 255));
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let notes = ids
        .into_iter()
        .filter_map(|(id, index)| {
            let rect = tree.node_by_id(&id)?.abs_bounding_box();
            Some((
                index,
                Rect::new(
                    rect.x() * scale,
                    rect.y() * scale,
                    rect.width() * scale,
                    rect.height() * scale,
                ),
            ))
        })
        .collect();
    let bars: Vec<(u32, Rect)> = req
        .bars
        .iter()
        .filter_map(|bar| {
            let rect = tree
                .node_by_id(&format!("score-measure-{}", bar.number))?
                .abs_bounding_box();
            Some((
                bar.number,
                Rect::new(
                    rect.x() * scale,
                    rect.y() * scale,
                    rect.width() * scale,
                    rect.height() * scale,
                ),
            ))
        })
        .collect();
    let mut staves = Vec::new();
    if let Some(song) = &req.song {
        for &(number, _) in &bars {
            for i in 0..song.staves.len() {
                let Some(usvg::Node::Group(staff)) =
                    tree.node_by_id(&format!("score-staff-{number}-{}", i + 1))
                else {
                    continue;
                };
                let mut lines = staff
                    .children()
                    .iter()
                    .filter_map(|node| match node {
                        // The five direct horizontal paths are the staff lines. Group bounds
                        // also include clefs, note glyphs and empty nodes, so can extend past
                        // the actual measure.
                        usvg::Node::Path(path) if path.data().bounds().height() == 0. => {
                            Some(path.abs_stroke_bounding_box())
                        }
                        _ => None,
                    })
                    .take(5);
                let Some(first) = lines.next() else { continue };
                let last = lines.last().unwrap_or(first);
                let top = first.y() + first.height() / 2.;
                let bottom = last.y() + last.height() / 2.;
                if bottom <= top {
                    continue;
                }
                let staff_lines = Rect::new(
                    first.x() * scale,
                    top * scale,
                    first.width() * scale,
                    (bottom - top) * scale,
                );
                staves.push((number, i, staff_lines));
            }
        }
    }
    Ok(Frame {
        w: w as usize,
        h: h as usize,
        pixels: pixmap.take(),
        notes,
        bars,
        staves,
    })
}

#[cfg(test)]
pub(super) fn test_markup(req: &Request) -> String {
    mei(req).unwrap().0
}
