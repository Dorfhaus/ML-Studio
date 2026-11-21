// SPDX-License-Identifier: GPL-3.0-or-later
//! Ruler gestures shared by the Playlist and Piano Roll.

use std::time::Instant;

pub(crate) type RulerClick = (Instant, f32);

pub(crate) fn ruler_double_click(last: &mut Option<RulerClick>, x: f32) -> bool {
    let now = Instant::now();
    let double = last.is_some_and(|(at, previous_x)| now.duration_since(at).as_millis() < 450
        && (x - previous_x).abs() <= 6.0);
    *last = (!double).then_some((now, x));
    double
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum RangeDrag {
    Draw { anchor: u32, unchanged: Option<(u32, u32)> },
    Move { anchor: u32, range: (u32, u32) },
}

impl RangeDrag {
    pub(crate) fn at(self, tick: u32) -> Option<(u32, u32)> {
        match self {
            Self::Draw { anchor, unchanged } => {
                if anchor == tick { unchanged } else { Some((anchor.min(tick), anchor.max(tick))) }
            }
            Self::Move { anchor, range: (start, end) } => {
                let shift = (i64::from(tick) - i64::from(anchor))
                    .clamp(-i64::from(start), i64::from(u32::MAX - end));
                Some(((i64::from(start) + shift) as u32, (i64::from(end) + shift) as u32))
            }
        }
    }
}
