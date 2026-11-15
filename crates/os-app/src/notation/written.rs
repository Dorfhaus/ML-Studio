// SPDX-License-Identifier: GPL-3.0-or-later

use os_model::notation::{Clef, NoteMarks, StaffChange, Tuplet};
use std::collections::BTreeMap;

pub(super) const UNIT: u32 = 840;
pub(super) const DIVISIONS: u32 = 8 * UNIT;
pub(super) const LETTERS: [char; 7] = ['C', 'D', 'E', 'F', 'G', 'A', 'B'];
pub(super) const NATURAL: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
pub(super) const DYNAMICS: [&str; 9] = ["", "ppp", "pp", "p", "mp", "mf", "f", "ff", "fff"];
pub(super) const ARTICULATIONS: [(&str, &str); 6] = [
    ("", ""),
    ("staccato", "stacc"),
    ("accent", "acc"),
    ("tenuto", "ten"),
    ("staccatissimo", "stacciss"),
    ("strong-accent", "marc"),
];
pub(super) const VALUES: [(u32, &str); 7] = [
    (1, "whole"),
    (2, "half"),
    (4, "quarter"),
    (8, "eighth"),
    (16, "16th"),
    (32, "32nd"),
    (64, "64th"),
];

pub(super) fn alteration(fifths: i8, step: usize) -> i32 {
    let order = if fifths >= 0 {
        [3, 0, 4, 1, 5, 2, 6]
    } else {
        [6, 2, 5, 1, 4, 0, 3]
    };
    if order[..usize::from(fifths.unsigned_abs().min(7))].contains(&step) {
        i32::from(fifths.signum())
    } else {
        0
    }
}

pub(super) fn pitch(key: u16, style: NoteMarks, fifths: i8) -> (usize, i32, i32, bool) {
    let key = i32::from(key);
    let named = |step: usize| {
        let octave = (key - NATURAL[step] + 2).div_euclid(12);
        let alter = key - NATURAL[step] - 12 * octave;
        (
            step,
            alter,
            octave - 1,
            style.accidental || alter != alteration(fifths, step),
        )
    };
    if let Some(step) = style.letter.checked_sub(1).filter(|&step| step < 7) {
        let p = named(usize::from(step));
        if p.1.abs() <= 2 {
            return p;
        }
    }
    if let Some(step) =
        (0..7).find(|&i| (NATURAL[i] + alteration(fifths, i)).rem_euclid(12) == key % 12)
    {
        return named(step);
    }
    let sharp = [0, 0, 1, 1, 2, 3, 3, 4, 4, 5, 5, 6];
    let flat = [0, 1, 1, 2, 2, 3, 4, 4, 5, 5, 6, 6];
    named(if fifths < 0 { flat } else { sharp }[key.rem_euclid(12) as usize])
}

#[derive(Default)]
pub(super) struct Accidentals(BTreeMap<(usize, i32), i32>);

impl Accidentals {
    pub fn pitch(&mut self, key: u16, style: NoteMarks, signature: i8) -> (usize, i32, i32, bool) {
        let (step, alter, octave, _) = pitch(key, style, signature);
        let previous = self
            .0
            .insert((step, octave), alter)
            .unwrap_or_else(|| alteration(signature, step));
        (step, alter, octave, style.accidental || previous != alter)
    }
}

pub(super) fn settings(
    changes: &[StaffChange],
    channel: u16,
    tick: u32,
    default: Clef,
) -> (Clef, i8) {
    let (mut clef, mut key) = (default, 0);
    for change in changes
        .iter()
        .filter(|c| c.channel == channel && c.position <= tick)
    {
        if let Some(v) = change.clef {
            clef = v;
        }
        if let Some(v) = change.fifths {
            key = v.clamp(-7, 7);
        }
    }
    (clef, key)
}

#[derive(Clone, Copy)]
pub(super) struct Duration {
    pub value: u32,
    pub dots: u8,
    pub ticks: u32,
    /// Ratio for a written subdivision. Irregular rests can use an invisible ratio.
    pub ratio: Option<(u32, u32)>,
}

pub(super) fn nominal(value: u32, dots: u8, quarter: u32) -> u32 {
    let base = (4 * quarter) / value;
    base + if dots >= 1 { base / 2 } else { 0 } + if dots >= 2 { base / 4 } else { 0 }
}

pub(super) fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

pub(super) fn durations(mut ticks: u32, tuplet: Tuplet) -> impl Iterator<Item = Duration> {
    let marked = tuplet
        .valid()
        .then(|| {
            let (actual, normal) = (u32::from(tuplet.actual), u32::from(tuplet.normal));
            let value = u32::from(tuplet.value);
            let expected = nominal(value, tuplet.dots, DIVISIONS) * normal / actual;
            (ticks.abs_diff(expected) <= 1).then_some(Duration {
                value,
                dots: tuplet.dots,
                ticks,
                ratio: Some((actual, normal)),
            })
        })
        .flatten();
    std::iter::from_fn(move || {
        if ticks == 0 {
            return None;
        }
        let d = if let Some(d) = marked {
            d
        } else {
            let mut choices = VALUES.iter().flat_map(|&(value, _)| {
                [1, 0]
                    .into_iter()
                    .map(move |dots| (nominal(value, dots, DIVISIONS), value, dots))
            });
            if let Some((ticks, value, dots)) = choices.find(|&(size, _, _)| size <= ticks) {
                Duration {
                    value,
                    dots,
                    ticks,
                    ratio: None,
                }
            } else {
                let base = nominal(64, 0, DIVISIONS);
                let factor = gcd(base, ticks);
                Duration {
                    value: 64,
                    dots: 0,
                    ticks,
                    ratio: Some((base / factor, ticks / factor)),
                }
            }
        };
        ticks -= d.ticks;
        Some(d)
    })
}
