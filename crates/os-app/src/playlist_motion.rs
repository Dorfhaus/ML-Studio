// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
//! Adapted from Lemon Studio motion.rs (clip_jumped/clip_glide), upstream
//! 73e6389821c59ffd5d738f0ca43e926d5ab84a89. Visual offsets never alter clip timing.
use crate::app::{App, State};
use crate::playlist::{self, PGeom};
use os_model::Clip;
use os_ui::Rect;
use std::{
    borrow::Cow,
    collections::HashMap,
    time::{Duration, Instant},
};
const SNAP_TAU: f64 = 0.024;
const SNAP_ARRIVED: f32 = 0.25;
const FIRST_STEP: f64 = 0.016;
#[derive(Default)]
pub struct Motion {
    context: Option<(Option<u16>, usize)>,
    glides: HashMap<usize, Glide>,
}
struct Glide {
    left: f32,
    right: f32,
    down: f32,
    at: Instant,
    shown: (f32, f32, f32),
}

impl Glide {
    fn sample(&self, now: Instant) -> (f32, f32, f32) {
        let k = (-now.saturating_duration_since(self.at).as_secs_f64() / SNAP_TAU).exp() as f32;
        (self.left * k, self.right * k, self.down * k)
    }
}
impl Motion {
    pub fn clear(&mut self) {
        self.glides.clear();
        self.context = None;
    }
    pub fn active(&self) -> bool {
        !self.glides.is_empty()
    }
    fn context(&mut self, arr: Option<u16>, count: usize) {
        if self.context != Some((arr, count)) {
            self.clear();
            self.context = Some((arr, count));
        }
    }
    pub fn jump(
        &mut self,
        arr: Option<u16>,
        count: usize,
        key: usize,
        (left, right, down): (f32, f32, f32),
        now: Instant,
    ) {
        if crate::smooth::smoothing() <= 0. {
            self.clear();
            return;
        }
        self.context(arr, count);
        let at = now
            .checked_sub(Duration::from_secs_f64(FIRST_STEP))
            .unwrap_or(now);
        let previous = self.glides.get(&key).map_or((0., 0., 0.), |g| g.shown);
        let mut glide = Glide {
            left: previous.0 + left,
            right: previous.1 + right,
            down: previous.2 + down,
            at,
            shown: (0., 0., 0.),
        };
        glide.shown = glide.sample(now);
        self.glides.insert(key, glide);
    }
    pub fn offset(
        &self,
        arr: Option<u16>,
        count: usize,
        key: usize,
    ) -> Option<(f32, f32, f32)> {
        if self.context != Some((arr, count)) {
            return None;
        }
        self.glides.get(&key).map(|g| g.shown)
    }

    fn advance(&mut self, now: Instant) {
        let enabled = crate::smooth::smoothing() > 0.;
        self.glides.retain(|_, glide| {
            let (left, right, down) = glide.sample(now);
            glide.shown = (left, right, down);
            enabled && left.abs().max(right.abs()).max(down.abs()) >= SNAP_ARRIVED
        });
    }
}

/// Painting and damage use the same interpolated clip placement.
pub fn placement<'a>(
    s: &State, g: &PGeom, clip: &'a Clip, (left, right, down): (f32, f32, f32),
) -> (PGeom, Cow<'a, Clip>) {
    let mut at = g.clone();
    at.scroll_y -= down;
    let drawn = if (left - right).abs() < 0.01 {
        at.scroll_x -= left;
        Cow::Borrowed(clip)
    } else {
        Cow::Owned(playlist::with_edges(s, g, clip, left, right))
    };
    (at, drawn)
}

fn bounds(s: &State, g: &PGeom, clip: &Clip, offset: Option<(f32, f32, f32)>) -> Rect {
    if let Some(offset) = offset {
        let (at, drawn) = placement(s, g, clip, offset);
        at.clip_rect(&drawn)
    } else {
        g.clip_rect(clip)
    }
}

pub fn damage_bounds(s: &State, g: &PGeom, index: usize, clip: &Clip) -> Option<Rect> {
    let offset = s.playlist_motion.borrow().offset(s.project.current_arrangement, playlist::clips(s).len(), index)?;
    Some(bounds(s, g, clip, Some(offset)).outset(6.0).intersect(&g.grid))
}

pub fn tick(app: &mut App) {
    advance(app, Instant::now());
}

fn advance(app: &mut App, now: Instant) {
    let mut motion = app.state.playlist_motion.borrow_mut();
    if !motion.active() {
        return;
    }
    let s = &app.state;
    let clips = playlist::clips(s);
    motion.context(s.project.current_arrangement, clips.len());
    let old: Vec<_> = motion.glides.iter().map(|(&i, glide)| (i, glide.shown)).collect();
    motion.advance(now);
    let Some(grid) = app.pl_grid().filter(|_| s.shown(crate::app::Win::Playlist)) else { return };
    let g = PGeom::new(s, grid);
    let mut dirty = Vec::new();
    for (index, before) in old {
        let after = motion.glides.get(&index).map(|glide| glide.shown);
        if Some(before) != after {
            if let Some(clip) = clips.get(index) {
                for offset in [Some(before), after] {
                    dirty.push(bounds(s, &g, clip, offset).outset(6.0).intersect(&grid));
                }
            }
        }
    }
    drop(motion);
    for rect in dirty {
        app.damage_rect(rect);
    }
}

/// Upstream's magnet-follow rule: free movement stays under the pointer; visible snap/row jumps glide.
pub fn jumped(s: &State, g: &crate::playlist::PGeom, from: Vec<(usize, Rect)>, alt: bool) {
    const NO_STEP: f32 = 3.;
    let stepped =
        !alt && crate::playlist::grid_snap(s).ticks(g.ppq, g.bar_w) as f32 * g.ppt >= NO_STEP;
    let count = crate::playlist::clips(s).len();
    let now = Instant::now();
    for (i, r0) in from {
        let Some(c) = crate::playlist::clips(s).get(i) else {
            continue;
        };
        let r1 = g.clip_rect(c);
        let (left, right, down) = (r0.x - r1.x, r0.right() - r1.right(), r0.y - r1.y);
        let (left, right) = if stepped { (left, right) } else { (0., 0.) };
        if left != 0. || right != 0. || down != 0. {
            s.playlist_motion.borrow_mut().jump(
                s.project.current_arrangement,
                count,
                i,
                (left, right, down),
                now,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upstream_clip_glide_settles_and_respects_smoothing_and_identity() {
        crate::prefs::set_slider(&crate::prefs::SMOOTHING, 128);
        let mut motion = Motion::default();
        let now = Instant::now();
        motion.jump(Some(0), 3, 1, (-120., -120., 48.), now);
        let first = motion.offset(Some(0), 3, 1).unwrap();
        assert!(first.0 < 0. && first.0 > -120. && first.2 > 0.);
        motion.advance(now + Duration::from_millis(24));
        let later = motion.offset(Some(0), 3, 1).unwrap();
        assert!(later.0.abs() < first.0.abs());
        motion.advance(now + Duration::from_secs(1));
        assert!(motion.offset(Some(0), 3, 1).is_none());
        assert!(!motion.active());
        motion.jump(Some(0), 3, 1, (100., 100., 0.), now);
        assert!(
            motion.offset(Some(1), 3, 1).is_none(),
            "another arrangement never inherits a glide"
        );
        motion.jump(Some(1), 3, 1, (100., 100., 0.), now);
        assert!(
            motion.offset(Some(1), 4, 1).is_none(),
            "added/deleted clip resets identity"
        );
        crate::prefs::set_slider(&crate::prefs::SMOOTHING, 0);
        motion.jump(Some(1), 4, 1, (100., 100., 0.), now);
        assert!(!motion.active());
    }
    #[test]
    fn upstream_event_marker_dragging_preserves_groups_and_scroll_anchor() {
        use crate::playlist as pl;
        use crate::snap::EditorSnap;
        use os_model::{Clip, ClipSource, TimeMarker};
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state.project.arrangements = vec![os_flp::project::default_arrangement()];
        app.state.project.current_arrangement = Some(0);
        pl::arr_mut(&mut app.state).clips = vec![
            Clip::new(96, 96, 1, ClipSource::Pattern(1)),
            Clip::new(500, 100, 2, ClipSource::Pattern(1)),
            Clip::new(142, 96, 3, ClipSource::Pattern(1)),
        ];
        let grid = Rect::new(200., 100., 1000., 600.);
        let g = pl::PGeom::new(&app.state, grid);
        let orig = vec![(0, pl::clips(&app.state)[0].clone()), (2, pl::clips(&app.state)[2].clone())];
        let drag = pl::PlDrag::Move {
            x0: 300.,
            y0: g.y_of(1) + 10.,
            scroll_x: g.scroll_x,
            track0: 1,
            orig: orig.clone(),
            fresh: false,
        };
        // Keep the custom scroll anchor while using upstream's nearest edge catch.
        app.state.playlist_scroll.0 = 40.;
        app.mouse = (300. + 365. * g.ppt - 40., g.y_of(1) + 10.);
        app.state.playlist.snap = EditorSnap::Events;
        pl::drag_move(&mut app, drag.clone(), grid);
        assert_eq!(pl::clips(&app.state)[0].position, 500);
        assert_eq!(pl::clips(&app.state)[2].position, 546);
        assert_eq!(pl::clips(&app.state)[1].position, 500);
        app.alt = true;
        pl::drag_move(&mut app, drag.clone(), grid);
        assert_eq!(pl::clips(&app.state)[0].position, 461);
        app.alt = false;
        app.state.playlist.snap = EditorSnap::Markers;
        pl::arr_mut(&mut app.state)
            .markers
            .push(TimeMarker::new(480, "Verse"));
        pl::drag_move(&mut app, drag.clone(), grid);
        assert_eq!(pl::clips(&app.state)[0].position, 480);
        assert_eq!(pl::clips(&app.state)[2].position, 526);
        let g = pl::PGeom::new(&app.state, grid);
        let near = 480. + 3. / g.ppt;
        assert_eq!(pl::snap_place(&app.state, &g, near, false), 480);
        assert_ne!(pl::snap_place(&app.state, &g, near, true), 480);
        let menu = pl::snap_menu(&app.state, 0., 0.);
        assert!(
            menu.items
                .iter()
                .filter(|i| i.label == "Events" || i.label == "Markers")
                .all(|i| i.enabled)
        );
    }
}
