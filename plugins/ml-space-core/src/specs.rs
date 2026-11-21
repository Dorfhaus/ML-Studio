use lemon_small_fx::{Card, Control, Mark, Setting, Spec};
pub static CONVOLVER: Spec = Spec {
    name: "ML Space",
    word: "SPACE",
    about: "ML Studio Convolver. See included third-party notices.",
    settings: &[
        Setting {
            id: "mix",
            name: "Mix",
            range: (0, 1000),
            start: 250,
            magnet: None,
        },
        Setting {
            id: "predelay",
            name: "Pre-delay",
            range: (0, 2500),
            start: 0,
            magnet: None,
        },
        Setting {
            id: "lowcut",
            name: "Low cut",
            range: (0, 1000),
            start: 0,
            magnet: None,
        },
        Setting {
            id: "highcut",
            name: "High cut",
            range: (1000, 20000),
            start: 20000,
            magnet: None,
        },
        Setting {
            id: "output",
            name: "Output",
            range: (-240, 120),
            start: 0,
            magnet: None,
        },
    ],
    studio_settings: 5,
    version: None,
    any_version: false,
    fields: &[],
    tail: &[],
    size: (640, 400),
    cards: &[
        Card {
            x0: 8.,
            x1: 126.,
            y0: 28.,
            y1: 152.,
            heading: "MIX",
        },
        Card {
            x0: 134.,
            x1: 252.,
            y0: 28.,
            y1: 152.,
            heading: "PRE-DELAY",
        },
        Card {
            x0: 260.,
            x1: 378.,
            y0: 28.,
            y1: 152.,
            heading: "LOW CUT",
        },
        Card {
            x0: 386.,
            x1: 504.,
            y0: 28.,
            y1: 152.,
            heading: "HIGH CUT",
        },
        Card {
            x0: 512.,
            x1: 630.,
            y0: 28.,
            y1: 152.,
            heading: "OUTPUT",
        },
    ],
    controls: &[
        Control::Knob {
            id: "mix",
            cx: 67.,
            cy: 85.,
            r: 24.,
            caption: "",
            bipolar: false,
        },
        Control::Knob {
            id: "predelay",
            cx: 193.,
            cy: 85.,
            r: 24.,
            caption: "",
            bipolar: false,
        },
        Control::Knob {
            id: "lowcut",
            cx: 319.,
            cy: 85.,
            r: 24.,
            caption: "",
            bipolar: false,
        },
        Control::Knob {
            id: "highcut",
            cx: 445.,
            cy: 85.,
            r: 24.,
            caption: "",
            bipolar: false,
        },
        Control::Knob {
            id: "output",
            cx: 571.,
            cy: 85.,
            r: 24.,
            caption: "",
            bipolar: true,
        },
    ],
    display: None,
    mark: Mark {
        x: 624.,
        y: 382.,
        anchor: 1.,
        stacked: false,
    },
    hint: None,
    labels: &[],
    text: convolver_text,
    parse: convolver_parse,
    sound: |_, _| unreachable!(),
    rings: true,
};
fn convolver_text(k: usize, v: i32) -> String {
    match k {
        0 => format!("{:.1}%", v as f64 / 10.),
        1 => format!("{:.1} ms", v as f64 / 10.),
        2 => {
            if v == 0 {
                "Off".into()
            } else {
                format!("{v} Hz")
            }
        }
        3 => {
            if v == 0 {
                "Off".into()
            } else {
                format!("{v} Hz")
            }
        }
        4 => format!("{:+.1} dB", v as f64 / 10.),
        _ => String::new(),
    }
}
fn convolver_parse(k: usize, s: &str) -> Option<i32> {
    if k == 2 && s.trim().eq_ignore_ascii_case("off") {
        return Some(0);
    }
    let v = s
        .trim()
        .split_whitespace()
        .next()?
        .trim_end_matches('%')
        .parse::<f64>()
        .ok()?;
    if !v.is_finite() {
        return None;
    }
    let scale = match k {
        0 | 1 | 4 => 10.,
        _ => 1.,
    };
    let setting = CONVOLVER.settings.get(k)?;
    Some(((v * scale).round() as i32).clamp(setting.range.0, setting.range.1))
}
pub static VOCODER: Spec = Spec {
    name: "ML Vocoder",
    word: "VOCODER",
    about: "ML Studio Vocoder. See included third-party notices.",
    settings: &[
        Setting {
            id: "carrier",
            name: "Carrier",
            range: (0, 1),
            start: 0,
            magnet: None,
        },
        Setting {
            id: "note",
            name: "Note",
            range: (24, 96),
            start: 48,
            magnet: None,
        },
        Setting {
            id: "bands",
            name: "Bands",
            range: (0, 1),
            start: 1,
            magnet: None,
        },
        Setting {
            id: "response",
            name: "Response",
            range: (50, 1000),
            start: 160,
            magnet: None,
        },
        Setting {
            id: "tone",
            name: "Resonance",
            range: (0, 1000),
            start: 550,
            magnet: None,
        },
        Setting {
            id: "shift",
            name: "Formant",
            range: (0, 1000),
            start: 667,
            magnet: None,
        },
        Setting {
            id: "sibilance",
            name: "Sibilance",
            range: (0, 1000),
            start: 250,
            magnet: None,
        },
        Setting {
            id: "mix",
            name: "Mix",
            range: (0, 1000),
            start: 1000,
            magnet: None,
        },
        Setting {
            id: "output",
            name: "Output",
            range: (-240, 120),
            start: 0,
            magnet: None,
        },
    ],
    studio_settings: 9,
    version: None,
    any_version: false,
    fields: &[],
    tail: &[],
    size: (640, 454),
    cards: &[
        Card {
            x0: 8.,
            x1: 126.,
            y0: 28.,
            y1: 152.,
            heading: "CARRIER",
        },
        Card {
            x0: 134.,
            x1: 252.,
            y0: 28.,
            y1: 152.,
            heading: "NOTE",
        },
        Card {
            x0: 260.,
            x1: 378.,
            y0: 28.,
            y1: 152.,
            heading: "BANDS",
        },
        Card {
            x0: 386.,
            x1: 504.,
            y0: 28.,
            y1: 152.,
            heading: "RESPONSE",
        },
        Card {
            x0: 512.,
            x1: 630.,
            y0: 28.,
            y1: 152.,
            heading: "RESONANCE",
        },
        Card {
            x0: 8.,
            x1: 126.,
            y0: 184.,
            y1: 308.,
            heading: "FORMANT",
        },
        Card {
            x0: 134.,
            x1: 252.,
            y0: 184.,
            y1: 308.,
            heading: "SIBILANCE",
        },
        Card {
            x0: 260.,
            x1: 378.,
            y0: 184.,
            y1: 308.,
            heading: "MIX",
        },
        Card {
            x0: 386.,
            x1: 504.,
            y0: 184.,
            y1: 308.,
            heading: "OUTPUT",
        },
    ],
    controls: &[
        Control::Choice {
            id: "carrier",
            x: 20.,
            top: 64.,
            names: &["Internal", "Sidechain"],
        },
        Control::Knob {
            id: "note",
            cx: 193.,
            cy: 85.,
            r: 24.,
            caption: "",
            bipolar: false,
        },
        Control::Choice {
            id: "bands",
            x: 272.,
            top: 64.,
            names: &["8", "16"],
        },
        Control::Knob {
            id: "response",
            cx: 445.,
            cy: 85.,
            r: 24.,
            caption: "",
            bipolar: false,
        },
        Control::Knob {
            id: "tone",
            cx: 571.,
            cy: 85.,
            r: 24.,
            caption: "",
            bipolar: false,
        },
        Control::Knob {
            id: "shift",
            cx: 67.,
            cy: 241.,
            r: 24.,
            caption: "",
            bipolar: false,
        },
        Control::Knob {
            id: "sibilance",
            cx: 193.,
            cy: 241.,
            r: 24.,
            caption: "",
            bipolar: false,
        },
        Control::Knob {
            id: "mix",
            cx: 319.,
            cy: 241.,
            r: 24.,
            caption: "",
            bipolar: false,
        },
        Control::Knob {
            id: "output",
            cx: 445.,
            cy: 241.,
            r: 24.,
            caption: "",
            bipolar: true,
        },
    ],
    display: None,
    mark: Mark {
        x: 624.,
        y: 436.,
        anchor: 1.,
        stacked: false,
    },
    hint: None,
    labels: &[],
    text: vocoder_text,
    parse: vocoder_parse,
    sound: |_, _| unreachable!(),
    rings: true,
};
fn vocoder_text(k: usize, v: i32) -> String {
    match k {
        0 => {
            if v == 0 {
                "Internal".into()
            } else {
                "Sidechain".into()
            }
        }
        1 => format!(
            "{}{}",
            [
                "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"
            ][(v % 12) as usize],
            v / 12 - 1
        ),
        2 => {
            if v == 0 {
                "8".into()
            } else {
                "16".into()
            }
        }
        3 => format!("{:.1}%", v as f64 / 10.),
        4 => format!("{:.1}%", v as f64 / 10.),
        5 => format!("{:.1}%", v as f64 / 10.),
        6 => format!("{:.1}%", v as f64 / 10.),
        7 => format!("{:.1}%", v as f64 / 10.),
        8 => format!("{:+.1} dB", v as f64 / 10.),
        _ => String::new(),
    }
}
fn vocoder_parse(k: usize, s: &str) -> Option<i32> {
    if k == 0 {
        return match s.trim().to_lowercase().as_str() {
            "internal" | "0" => Some(0),
            "sidechain" | "1" => Some(1),
            _ => None,
        };
    }
    if k == 2 {
        return match s.trim() {
            "8" => Some(0),
            "16" => Some(1),
            _ => None,
        };
    }
    if k == 1 {
        let upper = s.trim().to_uppercase();
        for n in 24..=96 {
            if vocoder_text(1, n) == upper {
                return Some(n);
            }
        }
    }
    let v = s
        .trim()
        .split_whitespace()
        .next()?
        .trim_end_matches('%')
        .parse::<f64>()
        .ok()?;
    if !v.is_finite() {
        return None;
    }
    let scale = match k {
        3 | 4 | 5 | 6 | 7 | 8 => 10.,
        _ => 1.,
    };
    let setting = VOCODER.settings.get(k)?;
    Some(((v * scale).round() as i32).clamp(setting.range.0, setting.range.1))
}
