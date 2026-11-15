// SPDX-License-Identifier: GPL-3.0-or-later

//! Export the same written-score projection used by Verovio.

use super::{Command, Request, arrangement, engrave, written};
use crate::{
    app::{Action, App, Ui},
    dialog::Then,
    filedlg::{FileDlg, Mode},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use os_model::notation::Tuplet;
use os_ui::{Font, Rect};
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    path::{Path, PathBuf},
    sync::mpsc,
};

pub(super) type Job = mpsc::Receiver<Result<PathBuf, String>>;

pub(super) fn dialog(app: &mut App, print: bool) {
    if app.state.notation.export.is_some() {
        app.state.message = Some("A score export is already running.".into());
        app.invalidate();
        return;
    }
    let (title, extension) = if print {
        ("Export printable parts", "html")
    } else {
        ("Export MusicXML", "musicxml")
    };
    let path = app.state.path.as_deref();
    let stem = path
        .and_then(Path::file_stem)
        .and_then(|name| name.to_str())
        .unwrap_or("Arrangement");
    app.file_dialog(
        FileDlg::new(Mode::Save, title, "score", Then::ScoreExport(print))
            .filter(title, &[extension])
            .start(path.and_then(Path::parent))
            .name(&format!("{stem}.{extension}")),
    );
}

pub(crate) fn file_done(app: &mut App, path: PathBuf, print: bool) {
    if app.state.notation.export.is_some() {
        return;
    }
    let req = match arrangement::whole_score(&app.state) {
        Ok(req) => req,
        Err(error) => {
            app.state.message = Some(error);
            app.invalidate();
            return;
        }
    };
    let title = app
        .state
        .path
        .as_deref()
        .and_then(Path::file_stem)
        .and_then(|name| name.to_str())
        .unwrap_or("Arrangement")
        .to_owned();
    let path = if path.extension().is_none() {
        path.with_extension(if print { "html" } else { "musicxml" })
    } else {
        path
    };
    let (tx, rx) = mpsc::channel();
    app.state.notation.export = Some(rx);
    app.state.message = Some("Preparing score export…".into());
    std::thread::spawn(move || {
        let result = if print {
            html(&req, &title)
        } else {
            musicxml(&req, &title)
        };
        let result = result
            .and_then(|text| {
                crate::safe_save::write(&path, text.as_bytes()).map_err(|err| err.to_string())
            })
            .map(|()| path);
        let _ = tx.send(result);
        os_host::wake();
    });
    app.invalidate();
}

pub(super) fn poll(app: &mut App) {
    let Some(job) = &app.state.notation.export else {
        return;
    };
    let status = match job.try_recv() {
        Ok(Ok(path)) => format!("Exported {}", path.display()),
        Ok(Err(err)) => format!("Score export failed: {err}"),
        Err(mpsc::TryRecvError::Disconnected) => "The score export worker stopped.".into(),
        Err(mpsc::TryRecvError::Empty) => return,
    };
    app.state.notation.export = None;
    app.state.message = Some(status);
    app.invalidate();
}

fn buttons(s: &crate::app::State) -> impl Iterator<Item = (&'static str, Command)> {
    [
        (
            "Print parts",
            Command::Export(true),
            s.notation.song.is_some(),
            super::EXPORT_VISIBILITY,
        ),
        (
            "MusicXML",
            Command::Export(false),
            s.notation.song.is_some(),
            super::EXPORT_VISIBILITY,
        ),
        (
            "Import score",
            Command::Import,
            true,
            super::IMPORT_VISIBILITY,
        ),
        ("Notation", Command::Marks, true, super::MARKS_VISIBILITY),
    ]
    .into_iter()
    .filter_map(|(label, command, enabled, preference)| {
        (enabled && crate::workspace_visibility::shown(preference)).then_some((label, command))
    })
}

pub(super) fn controls_width(s: &crate::app::State) -> f32 {
    80. * buttons(s).count() as f32 + 4.
}

pub(super) fn controls(ui: &mut Ui, s: &crate::app::State, paper: Rect) {
    for (i, (label, command)) in buttons(s).enumerate() {
        let r = Rect::new(
            paper.right() - 80. * (i + 1) as f32,
            paper.bottom() + 5.,
            77.,
            23.,
        );
        let hot = ui.hit(r, Action::Score(command), label);
        ui.c.fill_rrect(r, 4., if hot { ui.t.button_hi } else { ui.t.toolbar });
        ui.c.text_center(Font::new(13), r, ui.t.text, label);
    }
}

fn escape(s: &str) -> String {
    s.chars()
        .filter(|&c| c >= ' ' || matches!(c, '\t' | '\r' | '\n'))
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn note_type(value: u32) -> &'static str {
    written::VALUES
        .iter()
        .find(|v| v.0 == value)
        .map_or("64th", |v| v.1)
}

fn duration_markup(xml: &mut String, d: written::Duration) {
    write!(xml, "<type>{}</type>", note_type(d.value)).unwrap();
    for _ in 0..d.dots {
        xml.push_str("<dot/>");
    }
}

fn ratio_markup(xml: &mut String, ratio: Option<(u32, u32)>) {
    if let Some((actual, normal)) = ratio {
        write!(xml, "<time-modification><actual-notes>{actual}</actual-notes><normal-notes>{normal}</normal-notes></time-modification>").unwrap();
    }
}

fn rests(xml: &mut String, units: u32, voice: usize) {
    for d in written::durations(units, Tuplet::default()) {
        write!(
            xml,
            "<note><rest/><duration>{}</duration><voice>{voice}</voice>",
            d.ticks
        )
        .unwrap();
        duration_markup(xml, d);
        ratio_markup(xml, d.ratio);
        xml.push_str("</note>");
    }
}

pub(super) fn musicxml(req: &Request, title: &str) -> Result<String, String> {
    let parts = &req
        .song
        .as_ref()
        .ok_or("No arrangement in this score.")?
        .staves;
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<score-partwise version=\"4.0\"><work><work-title>{}</work-title></work><part-list>",
        escape(title)
    );
    for (i, part) in parts.iter().enumerate() {
        write!(
            xml,
            "<score-part id=\"P{}\"><part-name>{}</part-name></score-part>",
            i + 1,
            escape(&part.label)
        )
        .unwrap();
    }
    xml.push_str("</part-list>");
    for staff in 0..parts.len() {
        write!(xml, "<part id=\"P{}\">", staff + 1).unwrap();
        let mut remaining = BTreeMap::<usize, usize>::new();
        let layouts: Vec<_> = req
            .bars
            .iter()
            .map(|bar| {
                let voices = engrave::staff_notes(req, bar, staff, engrave::capacity(req, bar));
                for voice in &voices {
                    for (from, to, notes) in voice {
                        let segments = written::durations(to - from, notes[0].marks.tuplet).count();
                        for n in notes {
                            *remaining.entry(n.index).or_default() += segments;
                        }
                    }
                }
                voices
            })
            .collect();
        let mut started = std::collections::BTreeSet::new();
        let mut previous = None;
        for (bar, voices) in req.bars.iter().zip(layouts) {
            let cap = engrave::capacity(req, bar);
            let expected =
                u32::from(bar.meter.0) * (4 * written::DIVISIONS) / u32::from(bar.meter.1.max(1));
            let implicit = if cap != expected {
                " implicit=\"yes\""
            } else {
                ""
            };
            write!(xml, "<measure number=\"{}\"{implicit}>", bar.number).unwrap();
            let (clef, key) = req.staff_style(staff, bar.start);
            if previous != Some((bar.meter, clef, key)) {
                let (beats, den) = bar.meter;
                let (sign, line) = clef.sign_line();
                write!(xml, "<attributes><divisions>{}</divisions><key><fifths>{key}</fifths></key><time><beats>{beats}</beats><beat-type>{den}</beat-type></time><clef><sign>{sign}</sign><line>{line}</line></clef></attributes>", written::DIVISIONS).unwrap();
                previous = Some((bar.meter, clef, key));
            }
            if staff == 0 {
                for point in req
                    .tempo
                    .iter()
                    .filter(|p| (bar.start..bar.end).contains(&p.tick))
                {
                    let offset = engrave::units(point.tick - bar.start, req.ppq, 0);
                    let bpm = point.bpm;
                    if bpm.is_finite() && bpm > 0. {
                        write!(xml, "<direction><direction-type><metronome><beat-unit>quarter</beat-unit><per-minute>{bpm}</per-minute></metronome></direction-type><offset>{offset}</offset><sound tempo=\"{bpm}\"/></direction>").unwrap();
                    }
                }
            }
            for (index, items) in voices.iter().enumerate() {
                if index > 0 {
                    write!(xml, "<backup><duration>{cap}</duration></backup>").unwrap();
                }
                let voice = index + 1;
                let mut cursor = 0;
                let mut accidentals = written::Accidentals::default();
                let mut group_end = None;
                for (event, (from, to, notes)) in items.iter().enumerate() {
                    rests(&mut xml, from - cursor, voice);
                    let tuplet = notes[0].marks.tuplet;
                    if tuplet.start {
                        group_end = engrave::tuplet_end(items, event);
                    }
                    let starts = tuplet.start && group_end.is_some();
                    let ends = group_end == Some(event);
                    if let Some(dynamic) = notes
                        .iter()
                        .filter(|n| !started.contains(&n.index))
                        .find_map(|n| {
                            written::DYNAMICS
                                .get(usize::from(n.marks.dynamic))
                                .filter(|name| !name.is_empty())
                        })
                    {
                        write!(xml, "<direction placement=\"below\"><direction-type><dynamics><{dynamic}/></dynamics></direction-type><voice>{voice}</voice></direction>").unwrap();
                    }
                    for d in written::durations(to - from, tuplet) {
                        for (i, n) in notes.iter().enumerate() {
                            let (step, alter, octave, accidental) =
                                accidentals.pitch(n.key, n.marks, key);
                            let step = written::LETTERS[step];
                            if !(0..=9).contains(&octave) {
                                return Err("MusicXML supports written octaves 0 through 9.".into());
                            }
                            let count = remaining.get_mut(&n.index).unwrap();
                            *count -= 1;
                            let first = started.insert(n.index);
                            let ties =
                                [(!first).then_some("stop"), (*count > 0).then_some("start")];
                            let chord = if i > 0 { "<chord/>" } else { "" };
                            write!(xml, "<note>{chord}<pitch><step>{step}</step><alter>{alter}</alter><octave>{octave}</octave></pitch><duration>{}</duration>", d.ticks).unwrap();
                            for tie in ties.into_iter().flatten() {
                                write!(xml, "<tie type=\"{tie}\"/>").unwrap();
                            }
                            write!(xml, "<voice>{voice}</voice>").unwrap();
                            duration_markup(&mut xml, d);
                            if accidental && first {
                                let sign =
                                    ["flat-flat", "flat", "natural", "sharp", "double-sharp"]
                                        [(alter + 2) as usize];
                                write!(xml, "<accidental>{sign}</accidental>").unwrap();
                            }
                            ratio_markup(&mut xml, d.ratio);
                            let tuplet_mark = first && i == 0 && (starts || ends);
                            let art = written::ARTICULATIONS
                                .get(usize::from(n.marks.articulation))
                                .map_or("", |a| a.0);
                            if ties.iter().any(Option::is_some)
                                || tuplet_mark
                                || first && !art.is_empty()
                            {
                                xml.push_str("<notations>");
                                for tie in ties.into_iter().flatten() {
                                    write!(xml, "<tied type=\"{tie}\"/>").unwrap();
                                }
                                if tuplet_mark && starts {
                                    xml.push_str("<tuplet type=\"start\"/>");
                                }
                                if tuplet_mark && ends {
                                    xml.push_str("<tuplet type=\"stop\"/>");
                                }
                                if first && !art.is_empty() {
                                    write!(xml, "<articulations><{art}/></articulations>").unwrap();
                                }
                                xml.push_str("</notations>");
                            }
                            xml.push_str("</note>");
                        }
                    }
                    if ends {
                        group_end = None;
                    }
                    cursor = *to;
                }
                if cursor == 0 && voice == 1 {
                    write!(xml, "<note><rest measure=\"yes\"/><duration>{cap}</duration><voice>1</voice></note>").unwrap();
                } else {
                    rests(&mut xml, cap - cursor, voice);
                }
            }
            xml.push_str("</measure>");
        }
        xml.push_str("</part>");
    }
    xml.push_str("</score-partwise>\n");
    Ok(xml)
}

pub(super) fn html(req: &Request, title: &str) -> Result<String, String> {
    let mut html = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>{}</title><style>@page{{size:A4;margin:10mm}}body{{margin:0;font:16px system-ui}}header{{margin:12px}}section{{break-before:page}}section:first-of-type{{break-before:auto}}figure{{margin:0;break-inside:avoid;text-align:center}}img{{width:100%;max-height:252mm;object-fit:contain}}@media print{{header{{display:none}}}}h2{{margin:0 0 4mm}}</style><header><button onclick=\"window.print()\">Print / Save as PDF</button></header>",
        escape(title)
    );
    let root = engrave::engine()?;
    let sources = req.song.as_ref().ok_or("No arrangement in this score.")?;
    for (i, staff) in sources.staves.iter().enumerate() {
        let part = arrangement::single_part(req, i);
        let svg = engrave::svg(&root, &engrave::mei(&part)?.0, true)?;
        write!(
            html,
            "<section><h2>{} — {}</h2>",
            escape(title),
            escape(&staff.label)
        )
        .unwrap();
        let mut pages = 0;
        let mut remaining = svg.as_str();
        while let Some(from) = remaining.find("<svg ") {
            let start = &remaining[from..];
            let end = start
                .find("\n</svg>")
                .map(|pos| pos + "\n</svg>".len())
                .ok_or("Invalid print page from Verovio.")?;
            write!(
                html,
                "<figure><img alt=\"{} page {}\" src=\"data:image/svg+xml;base64,{}\"></figure>",
                escape(&staff.label),
                pages + 1,
                STANDARD.encode(&start[..end])
            )
            .unwrap();
            remaining = &start[end..];
            pages += 1;
        }
        if pages == 0 {
            return Err("Verovio generated no print pages.".into());
        }
        html.push_str("</section>");
        if html.len() > 64 * 1024 * 1024 {
            return Err("Printable parts exceed 64 MiB. Narrow the score to selected rows.".into());
        }
    }
    Ok(html)
}
