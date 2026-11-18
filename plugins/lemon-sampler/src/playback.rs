// SPDX-License-Identifier: GPL-3.0-or-later
//! Prepare linked sampler audio for the song tempo before the first note.
//! The host polls from its UI thread; all sample building uses the existing worker pool.

use super::{SamplerParams, Shared, queue_build};
use std::sync::{Arc, Mutex, Weak, atomic::{AtomicBool, Ordering}};

struct Entry {
    shared: Weak<Shared>,
    params: Weak<SamplerParams>,
    queued: Arc<AtomicBool>,
}

static SAMPLERS: Mutex<Vec<Entry>> = Mutex::new(Vec::new());

pub(super) fn register(shared: &Arc<Shared>, params: &Arc<SamplerParams>) {
    let mut entries = SAMPLERS.lock().unwrap();
    entries.retain(|entry| entry.shared.strong_count() > 0 && entry.params.strong_count() > 0);
    entries.push(Entry {
        shared: Arc::downgrade(shared),
        params: Arc::downgrade(params),
        queued: Arc::new(AtomicBool::new(false)),
    });
}

/// True once instances playing these files have their current audio at `tempo`.
/// Weak instance references preserve per-instance settings without retaining closed projects.
pub fn prepare(tempo: f64, paths: &[String]) -> bool {
    let samplers: Vec<_> = {
        let mut entries = SAMPLERS.lock().unwrap();
        entries.retain(|entry| entry.shared.strong_count() > 0 && entry.params.strong_count() > 0);
        entries.iter().filter_map(|entry| Some((
            entry.shared.upgrade()?, entry.params.upgrade()?, entry.queued.clone(),
        ))).collect()
    };
    let mut ready = true;
    for (shared, params, queued) in samplers {
        let path = shared.loaded_path();
        if path.is_empty() || !paths.contains(&path) {
            continue;
        }
        shared.set_tempo(tempo);
        if !shared.stale(&params) && !shared.busy.load(Ordering::Acquire) {
            continue;
        }
        ready = false;
        if shared.busy.load(Ordering::Acquire) {
            shared.again.store(true, Ordering::Release);
            continue;
        }
        if !queued.swap(true, Ordering::AcqRel) {
            queue_build(Box::new(move || {
                shared.precompute(&params);
                queued.store(false, Ordering::Release);
            }));
        }
    }
    ready
}
