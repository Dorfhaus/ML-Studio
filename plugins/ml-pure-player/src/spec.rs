use lemon_small_fx::{Card, Control, Mark, Setting, Spec};
pub static SPEC: Spec = Spec {
    name: "ML Pure Player",
    word: "PURE PLAYER",
    about: "ML Pure Player / FluidSynth 2.6.2 (LGPL-2.1-or-later). Optional GeneralUser GS by S. Christian Collins.",
    settings: &[
        Setting {
            id: "bank",
            name: "Bank",
            range: (0, 16383),
            start: 0,
            magnet: None,
        },
        Setting {
            id: "program",
            name: "Program",
            range: (0, 127),
            start: 0,
            magnet: None,
        },
        Setting {
            id: "gain",
            name: "Output",
            range: (0, 200),
            start: 80,
            magnet: Some(100),
        },
        Setting {
            id: "reverb",
            name: "Reverb",
            range: (0, 100),
            start: 20,
            magnet: None,
        },
        Setting {
            id: "chorus",
            name: "Chorus",
            range: (0, 100),
            start: 10,
            magnet: None,
        },
    ],
    studio_settings: 5,
    version: None,
    any_version: false,
    fields: &[],
    tail: &[],
    size: (780, 530),
    cards: &[
        Card {
            x0: 16.,
            x1: 112.,
            y0: 28.,
            y1: 119.,
            heading: "OUTPUT",
        },
        Card {
            x0: 124.,
            x1: 220.,
            y0: 28.,
            y1: 119.,
            heading: "REVERB",
        },
        Card {
            x0: 232.,
            x1: 328.,
            y0: 28.,
            y1: 119.,
            heading: "CHORUS",
        },
    ],
    controls: &[
        Control::Knob {
            id: "gain",
            cx: 64.,
            cy: 68.,
            r: 24.,
            caption: "",
            bipolar: false,
        },
        Control::Knob {
            id: "reverb",
            cx: 172.,
            cy: 68.,
            r: 24.,
            caption: "",
            bipolar: false,
        },
        Control::Knob {
            id: "chorus",
            cx: 280.,
            cy: 68.,
            r: 24.,
            caption: "",
            bipolar: false,
        },
    ],
    display: None,
    mark: Mark {
        x: 764.,
        y: 509.,
        anchor: 1.,
        stacked: false,
    },
    hint: None,
    labels: &[],
    text: |k, v| {
        if k < 2 {
            format!("{v}")
        } else {
            format!("{v}%")
        }
    },
    parse: |_, s| {
        s.trim()
            .trim_end_matches('%')
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .map(|n| n.round() as i32)
    },
    sound: |_, _| unreachable!(),
    rings: true,
};
