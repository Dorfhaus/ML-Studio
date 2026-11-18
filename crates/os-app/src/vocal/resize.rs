// SPDX-License-Identifier: GPL-3.0-or-later
//! Vertical resize of the inline vocal editor, independent of track height.

use crate::app::{App, Drag, State};
use crate::playlist::Rows;
use std::time::Instant;

pub(crate) const VISIBILITY: &str = "visible_playlist-vocal-resize";
const HEIGHT: &str = "vocal_editor_height";
const MAX_HEIGHT: f32 = 4096.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Size {
    height: f32,
    restore_scroll: Option<f32>,
    pub(crate) click: Option<(u16, Instant)>,
}

impl Size {
    pub(crate) fn load() -> Self {
        let height = crate::settings_file::get(HEIGHT)
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|height| (160.0..=MAX_HEIGHT).contains(height))
            .unwrap_or(super::EXTRA);
        Self {
            height,
            restore_scroll: None,
            click: None,
        }
    }

    pub(crate) fn extra(self, state: &State, base: f32) -> f32 {
        let Some(grid) = crate::views::playlist::grid_of(state) else {
            return self.height;
        };
        let minimum = super::view::minimum_height(grid.w);
        let height = if self.restore_scroll.is_some() {
            grid.h - base
        } else {
            self.height
        };
        height.max(minimum).min(MAX_HEIGHT).floor()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Gesture {
    row: u16,
    before: Size,
    y: f32,
    rendered_height: f32,
    changed: bool,
}

pub(crate) fn restore(state: &mut State) {
    state.playlist.vocal.size.click = None;
    if let Some(scroll) = state.playlist.vocal.size.restore_scroll.take() {
        state.playlist_scroll.1 = scroll;
    }
}

pub(crate) fn press(app: &mut App, row: u16) {
    if !super::expanded(&app.state, row) || !crate::workspace_visibility::shown(VISIBILITY) {
        return;
    }
    let now = Instant::now();
    let rows = Rows::new(&app.state);
    let size = &mut app.state.playlist.vocal.size;
    if size.click.take().is_some_and(|(last_row, last)| {
        last_row == row && now.duration_since(last).as_millis() < 400
    }) {
        if size.restore_scroll.is_some() {
            restore(&mut app.state);
        } else {
            size.restore_scroll = Some(app.state.playlist_scroll.1);
            app.state.playlist_scroll.1 = rows.top(row);
        }
    } else {
        app.drag = Some(Drag::VocalResize(Gesture {
            row,
            before: *size,
            y: app.mouse.1,
            rendered_height: rows.height(row) - rows.base_height(row),
            changed: false,
        }));
    }
    app.invalidate();
}

pub(crate) fn drag(app: &mut App, mut gesture: Gesture, y: f32) {
    if !super::expanded(&app.state, gesture.row)
        || (!gesture.changed && (y - gesture.y).abs() < 3.0)
    {
        return;
    }
    let Some(grid) = app.pl_grid() else { return };
    let height = (gesture.rendered_height + y - gesture.y)
        .clamp(super::view::minimum_height(grid.w), MAX_HEIGHT)
        .round();
    let size = &mut app.state.playlist.vocal.size;
    size.height = height;
    size.restore_scroll = None;
    gesture.changed = true;
    app.drag = Some(Drag::VocalResize(gesture));
    app.invalidate();
}

pub(crate) fn release(app: &mut App, gesture: Gesture) {
    if !super::expanded(&app.state, gesture.row) {
        return;
    }
    let size = &mut app.state.playlist.vocal.size;
    if gesture.changed {
        crate::settings_file::write(HEIGHT, &size.height.to_string());
    } else {
        size.click = Some((gesture.row, Instant::now()));
    }
}

pub(crate) fn cancel(app: &mut App) -> bool {
    let Some(Drag::VocalResize(gesture)) = app.drag else {
        return false;
    };
    app.state.playlist.vocal.size = gesture.before;
    app.drag = None;
    app.pressed = None;
    app.invalidate();
    true
}
