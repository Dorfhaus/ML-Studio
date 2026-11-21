// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Editor edits the host has not handed back yet (spec: plugins/lemon-sampler.md §4 "Edits apply at once").
//!
//! The editor tells the host about every edit; nih-plug then changes the parameter only when the host passes the edit
//! back (VST3 while processing) or flushes (CLAP), and some hosts do neither. So the sound reads parameters through
//! [`LiveEdits::value`]: the editor's newest value wins until the host's own value moves to it (confirmed) or away from
//! the value it had at the edit (automation / undo in the host).
//!
//! Each plugin instance's params object owns one [`LiveEdits`] and hands it to its editor's [`crate::vals::Vals`], so
//! instances never see or wait on each other's edits. Entries are keyed by the parameter's address, which stays put
//! while the params object lives (behind the plugin's `Arc`).

use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};

use nih_plug::prelude::*;

// Each touched parameter has a stable slot for the lifetime of its params object.
const INACTIVE: u64 = u64::MAX;
type Link = OnceLock<Box<LiveEdit>>;
struct LiveEdit {
    addr: usize,
    // One atomic publication of the host baseline and the editor target.
    values: AtomicU64,
    next: Link,
}

#[derive(Default)]
pub struct LiveEdits {
    writers: Mutex<()>,
    buckets: [Link; 32],
}

impl LiveEdits {
    fn bucket(&self, addr: usize) -> &Link {
        &self.buckets[((addr >> 3) ^ (addr >> 10)) & 31]
    }

    /// A full patch recall supersedes edits awaiting host confirmation.
    pub fn clear(&self) {
        let _write = self.writers.lock().unwrap();
        for bucket in &self.buckets {
            let mut link = bucket;
            while let Some(slot) = link.get() {
                slot.values.store(INACTIVE, Ordering::Release);
                link = &slot.next;
            }
        }
    }
    /// Records an editor edit: `p` sounds at `norm` (normalised) until its host value moves from what it is now.
    pub fn set(&self, p: ParamPtr, norm: f32) {
        // SAFETY: callers hold pointers from a live params object.
        let host = unsafe { p.unmodulated_normalized_value() };
        self.set_at(param_addr(p), norm, host);
    }

    fn set_at(&self, addr: usize, norm: f32, host: f32) {
        let _write = self.writers.lock().unwrap();
        let values = if (norm - host).abs() <= 1e-7 { INACTIVE } else {
            (u64::from(host.to_bits()) << 32) | u64::from(norm.to_bits())
        };
        let mut link = self.bucket(addr);
        loop {
            if values == INACTIVE && link.get().is_none() { return; }
            let slot = link.get_or_init(|| Box::new(LiveEdit {
                addr, values: AtomicU64::new(INACTIVE), next: Link::new(),
            }));
            if slot.addr == addr {
                slot.values.store(values, Ordering::Release);
                return;
            }
            link = &slot.next;
        }
    }

    /// Audio reads never wait for the editor and never drop a pending edit on contention.
    fn norm_at(&self, addr: usize, host: f32) -> Option<f32> {
        let mut link = self.bucket(addr);
        while let Some(slot) = link.get() {
            if slot.addr == addr {
                let values = slot.values.load(Ordering::Acquire);
                if values == INACTIVE { return None; }
                let base = f32::from_bits((values >> 32) as u32);
                let norm = f32::from_bits(values as u32);
                if (host-base).abs() <= 1e-7 && (host-norm).abs() > 1e-7 {
                    return Some(norm);
                }
                // Never acknowledge a newer edit published during this read.
                let _ = slot.values.compare_exchange(values, INACTIVE, Ordering::AcqRel, Ordering::Relaxed);
                return None;
            }
            link = &slot.next;
        }
        None
    }

    /// The normalised value the sound uses for `p` (its pending editor edit, else the host's value).
    pub fn norm(&self, p: ParamPtr) -> f32 {
        // SAFETY: callers hold pointers from a live params object.
        let host = unsafe { p.unmodulated_normalized_value() };
        self.norm_at(param_addr(p), host).unwrap_or(host)
    }

    /// The plain value the sound uses for `p` (its pending editor edit, else the host's value).
    pub fn value<P: Param>(&self, p: &P) -> P::Plain {
        match self.norm_at(p as *const P as usize, p.unmodulated_normalized_value()) {
            Some(n) => p.preview_plain(n),
            None => p.modulated_plain_value(),
        }
    }
}

/// The address a parameter's edits are keyed by.
fn param_addr(p: ParamPtr) -> usize {
    match p {
        ParamPtr::FloatParam(p) => p as usize,
        ParamPtr::IntParam(p) => p as usize,
        ParamPtr::BoolParam(p) => p as usize,
        ParamPtr::EnumParam(p) => p as usize,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param() -> IntParam {
        IntParam::new("p", 0, IntRange::Linear { min: 0, max: 4 })
    }

    #[test]
    fn edits_sound_until_the_host_confirms_or_moves() {
        let live = LiveEdits::default();
        let p = param();
        live.set(p.as_ptr(), 0.5);
        assert_eq!(live.value(&p), 2);
        assert_eq!(live.norm(p.as_ptr()), 0.5);
        // The host confirms: its value from then on.
        let a = param_addr(p.as_ptr());
        assert_eq!(live.norm_at(a, 0.5), None);
        assert_eq!(live.norm_at(a, 0.0), None);
        // The host moves elsewhere (automation): the edit is dropped for good, also when the host comes back.
        live.set_at(a, 0.75, 0.0);
        assert_eq!(live.norm_at(a, 0.25), None);
        assert_eq!(live.norm_at(a, 0.0), None);
        // An edit back to the host value is no edit.
        live.set_at(a, 0.0, 0.0);
        assert_eq!(live.norm_at(a, 0.0), None);
    }

    #[test]
    fn instances_keep_their_own_edits() {
        let (a, b) = (LiveEdits::default(), LiveEdits::default());
        let p = param();
        a.set(p.as_ptr(), 0.5);
        assert_eq!(b.value(&p), 0);
        // Another instance holding its list (its editor, or its audio thread) never hides this instance's edit.
        b.set(p.as_ptr(), 1.0);
        let _held = a.writers.lock().unwrap();
        assert_eq!(a.value(&p), 2);
        assert_eq!(b.value(&p), 4);
    }
}
