// SPDX-License-Identifier: GPL-3.0-or-later

use serde::{Deserialize, Serialize};

/// Written details travel with the note through moves, copies, pattern splits and history.
/// Zero values let old projects use automatic notation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteMarks {
    /// 1 = C through 7 = B. The sounding key determines the octave and alteration.
    pub letter: u8,
    /// Explicit accidental, including a courtesy natural.
    pub accidental: bool,
    pub articulation: u8,
    pub dynamic: u8,
    pub tuplet: Tuplet,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tuplet {
    pub actual: u8,
    pub normal: u8,
    pub value: u8,
    pub dots: u8,
    pub start: bool,
    pub end: bool,
}

impl Tuplet {
    pub fn valid(self) -> bool {
        (2..=15).contains(&self.actual)
            && (1..=15).contains(&self.normal)
            && self.actual != self.normal
            && matches!(self.value, 1 | 2 | 4 | 8 | 16 | 32 | 64)
            && self.dots <= 2
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Clef {
    Treble,
    Bass,
    Alto,
    Tenor,
}

impl Clef {
    pub fn sign_line(self) -> (char, u8) {
        match self {
            Self::Treble => ('G', 2),
            Self::Bass => ('F', 4),
            Self::Alto => ('C', 3),
            Self::Tenor => ('C', 4),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffChange {
    pub channel: u16,
    pub position: u32,
    pub fifths: Option<i8>,
    pub clef: Option<Clef>,
}

impl StaffChange {
    pub fn rescale(&mut self, from: u16, to: u16) {
        self.position = super::rescale_tick(self.position, from, to);
    }
}
