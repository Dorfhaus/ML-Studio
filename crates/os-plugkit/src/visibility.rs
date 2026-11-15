// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::Cell;
use std::time::{Duration, Instant};

pub const STOCK_DSP_CONTROLS: &str = "visible_stock-dsp-controls";

pub fn stock_dsp_controls() -> bool {
    thread_local! {
        static CACHE: Cell<Option<(Instant, bool)>> = const { Cell::new(None) };
    }
    CACHE.with(|cache| {
        if let Some((at, value)) = cache.get() {
            if at.elapsed() < Duration::from_millis(300) {
                return value;
            }
        }
        let hidden = os_paths::user_file("settings.txt")
            .and_then(|path| std::fs::read_to_string(path).ok())
            .is_some_and(|text| text.lines().rev()
                .filter_map(|line| line.split_once('='))
                .find(|(key, _)| key.trim() == STOCK_DSP_CONTROLS)
                .is_some_and(|(_, value)| value.trim() == "0"));
        cache.set(Some((Instant::now(), !hidden)));
        !hidden
    })
}
