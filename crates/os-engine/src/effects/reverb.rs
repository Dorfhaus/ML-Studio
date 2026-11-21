// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Lemon Studio's plate reverb (a recreation of FL's Fruity Reeverb 2 mixer effect), configured from its saved state.
//!
//! The two sides are summed to mono, filtered (high cut, low cut), diffused by four all-passes, delayed (pre-delay)
//! and fed into a tank of two halves that feed each other; seven taps per side make the reverb, one tap per side from a
//! line behind the first diffuser makes the early reflections.
//!
// spec: plugins/fruity-reeverb-2.md ("Behaviour: what both builds share", "The mixer effect"), plugins/lemon-reverb.md

use std::f64::consts::PI;

use super::Effect;

/// The knobs, in the saved state's order.
pub const PARAMS: usize = 15;
pub const LOW_CUT: usize = 0;
pub const HIGH_CUT: usize = 1;
pub const PREDELAY: usize = 2;
pub const ROOM: usize = 3;
pub const DIFFUSION: usize = 4;
pub const DECAY: usize = 5;
pub const DAMPING: usize = 6;
pub const BASS: usize = 7;
pub const CROSSOVER: usize = 8;
pub const SEPARATION: usize = 9;
pub const DRY: usize = 10;
pub const EARLY: usize = 11;
pub const WET: usize = 12;
pub const MOD_SPEED: usize = 13;
pub const MOD_DEPTH: usize = 14;

/// The knobs' ranges (stored values are used as they are, also outside them).
pub const RANGES: [(i32, i32); PARAMS] = [
    (19, 3000),
    (5, 221),
    (0, 384),
    (1, 100),
    (0, 100),
    (1, 200),
    (5, 221),
    (20, 300),
    (25, 2000),
    (-64, 64),
    (0, 160),
    (0, 160),
    (0, 160),
    (0, 2048),
    (0, 2048),
];

/// A fresh instance (FL's Default preset, with its wet level at 100 %).
pub const DEFAULTS: [i32; PARAMS] = [75, 40, 0, 50, 100, 15, 40, 100, 500, 0, 128, 128, 128, 684, 0];

/// The value that switches a filter off: low cut 19, high cut / high damping 221.
pub const LOW_CUT_OFF: i32 = 19;
pub const LOWPASS_OFF: i32 = 221;

/// Saved-state versions (the first int).
pub const VERSION: i32 = 10001;
pub const VERSION_NO_MOD: i32 = 10000;
/// The current saved state's size: the version, fifteen knobs, two switches.
pub const STATE_LEN: usize = 66;
pub const TEMPO_BYTE: usize = 64;
pub const MID_SIDE_BYTE: usize = 65;

/// Line lengths for the largest room at 20 kHz (lines 0..3 diffusers, 4..7 tank half A, 8..11 tank half B).
const LINES: [i32; 12] = [142, 107, 379, 277, 672, 4453, 1800, 3720, 908, 4217, 2656, 3163];
/// Reverb taps (line, offset at 20 kHz).
const TAPS_L: [(usize, i32); 7] = [(9, 266), (9, 2974), (10, 1913), (11, 2496), (5, 1690), (6, 187), (7, 1566)];
const TAPS_R: [(usize, i32); 7] = [(5, 3627), (5, 353), (6, 1728), (7, 3173), (9, 1811), (10, 335), (11, 621)];

/// Everything the saved state says.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub knobs: [i32; PARAMS],
    /// The pre-delay counts 1/192 beat steps instead of 1/384 s.
    pub tempo_sync: bool,
    /// The side signal (left − right) feeds the reverb instead of the sum.
    pub mid_side: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { knobs: DEFAULTS, tempo_sync: false, mid_side: false }
    }
}

fn int_at(b: &[u8], at: usize) -> Option<i32> {
    b.get(at..at + 4).map(|s| i32::from_le_bytes(s.try_into().unwrap()))
}

impl Settings {
    /// A saved state: version 10001 (fifteen knobs and two switches), 10000 (thirteen knobs, no modulation) or a
    /// state without a version (FL's factory presets: thirteen knobs and one byte).
    pub fn from_state(b: &[u8]) -> Option<Settings> {
        let mut s = Settings { knobs: DEFAULTS, tempo_sync: false, mid_side: false };
        let first = int_at(b, 0)?;
        let (at, n, bytes) = match first {
            VERSION => (4, 15, 2),
            VERSION_NO_MOD => (4, 13, 2),
            _ => (0, 13, 1),
        };
        for k in 0..n {
            s.knobs[k] = int_at(b, at + 4 * k)?;
        }
        if n == 13 {
            // No modulation: the speed knob's default, no depth.
            s.knobs[MOD_SPEED] = 684;
            s.knobs[MOD_DEPTH] = 0;
        }
        let sw = at + 4 * n;
        // The presets' single byte is 0 in all of FL's; read as the tempo switch.
        s.tempo_sync = b.get(sw).is_some_and(|&x| x != 0);
        if bytes == 2 {
            s.mid_side = b.get(sw + 1).is_some_and(|&x| x != 0);
        }
        Some(s)
    }

    /// The current saved-state layout (66 bytes).
    pub fn to_state(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(STATE_LEN);
        b.extend_from_slice(&VERSION.to_le_bytes());
        for v in self.knobs {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.push(u8::from(self.tempo_sync));
        b.push(u8::from(self.mid_side));
        b
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Coefficients

/// `single(1 / irate)`.
fn q_of(irate: i32) -> f32 {
    (1.0 / f64::from(irate)) as f32
}

/// The one-pole low-pass coefficient for `h` (in 100 Hz); 1 (no filtering) for 221.
pub fn lowpass(h: i32, irate: i32) -> f32 {
    if h == LOWPASS_OFF {
        return 1.0;
    }
    let q = f64::from(q_of(irate));
    let x = (2.0 - (f64::from(h) * 628.3185307179587 * q).cos()) as f32;
    let xx = (f64::from(x) * f64::from(x)) as f32;
    (1.0 - (f64::from(x) - (f64::from(xx) - 1.0).sqrt())) as f32
}

/// The low cut's all-pass coefficient; 0 (off) for 19.
pub fn low_cut(hz: i32, irate: i32) -> f32 {
    if hz == LOW_CUT_OFF {
        return 0.0;
    }
    let t = f64::from((f64::from(hz) * PI * f64::from(q_of(irate))).tan() as f32);
    ((t - 1.0) / (t + 1.0)) as f32
}

const LN001: f64 = -6.907755278982137; // ln(0.001)

/// A diffuser / tank all-pass coefficient for `n` samples of line at diffusion `d` (0 when `d` is 0).
fn diff_coef(n: i32, spread: f64, d: f32, q: f32) -> f32 {
    if d == 0.0 {
        return 0.0;
    }
    let a = (f64::from(n) * f64::from(q * 0.5)) as f32;
    (LN001 * f64::from(a) / (f64::from(d) * spread)).exp() as f32
}

/// `single(exp(0.25 * ln(exp(ln001 * loop / t))))`: the gain for a quarter of the tank's loop at decay time `t`.
fn quarter_gain(lp: f32, t: f32) -> f32 {
    (0.25 * (LN001 * f64::from(lp) / f64::from(t)).exp().ln()).exp() as f32
}

/// `slider(x)`: the level knobs' law.
pub fn slider(x: f32) -> f32 {
    if x == 1.0 {
        return 1.0;
    }
    let ln11 = 11f64.ln() as f32;
    (((f64::from(x * ln11)).exp() - 1.0) * f64::from(0.1f32)) as f32
}

/// A 2x2 output matrix `(LL, RR, RL, LR)` for level `v` and separation `sep` (−64..64).
pub fn matrix(v: f32, separation: i32, mid_side: bool) -> [f32; 4] {
    let sep = separation as f32 / 64.0;
    let mut m = if sep == 0.0 {
        [v, v, 0.0, 0.0]
    } else if v == 0.0 {
        [0.0; 4]
    } else {
        // The pan's cosine and sine stay double until the product is rounded (FL renders: a single cosine leaves
        // l one step below v and the separated cases 1 ulp off).
        let u = v * (1.0 / (PI / 4.0).cos()) as f32;
        let l = (f64::from(u) * (PI / 4.0).cos()) as f32;
        let r = (f64::from(u) * (PI / 4.0).sin()) as f32;
        let merge = if sep < 0.0 { sep } else { ((f64::from(sep) * 5f64.ln()).exp() - 1.0) as f32 * 0.25 }.min(1.0);
        [l * (1.0 - merge * 0.5), r * (1.0 - merge * 0.5), l * merge * 0.5, r * merge * 0.5]
    };
    if mid_side {
        m[1] = -m[1];
        m[3] = -m[3];
    }
    m
}

/// The pre-delay in samples (at least 1).
pub fn predelay_samples(steps: i32, tempo_sync: bool, irate: i32, bpm: f64) -> usize {
    let p = if tempo_sync {
        (f64::from(steps) * f64::from(irate) * 0.3125 / f64::from(bpm as f32)) as f32
    } else {
        (steps as i64 * i64::from(irate)) as f32 / 384.0
    };
    (p.round_ties_even() as i64).max(1) as usize
}

/// Line lengths and tap offsets for a room size.
#[derive(Clone, Debug, PartialEq)]
pub struct Geometry {
    pub lengths: [usize; 12],
    pub taps_l: [(usize, usize); 7],
    pub taps_r: [(usize, usize); 7],
    /// Early reflections: offsets (samples ago) and gains per side.
    pub early: (usize, usize, f32, f32),
}

impl Geometry {
    pub fn new(room: i32, irate: i32) -> Geometry {
        let scale = i64::from(room + 15) * i64::from(irate) / 115;
        let len = |t: i32| ((scale * i64::from(t) / 20000).max(1)) as usize;
        let off = |t: i32| (scale * i64::from(t) / 20000).max(0) as usize;
        let lengths = LINES.map(len);
        let taps_l = TAPS_L.map(|(l, o)| (l, off(o)));
        let taps_r = TAPS_R.map(|(l, o)| (l, off(o)));
        let e = (f64::from(irate) * f64::from(room + 3) * 3.3022917905026087e-7) as f32;
        let ol = ((1102.0 * e).round_ties_even() as i64).max(1) as usize;
        let or = ((837.0 * e).round_ties_even() as i64).max(1) as usize;
        let lo = ol.min(or) as f64;
        let gl = ((lo / ol as f64) * 0.5) as f32;
        let gr = ((lo / or as f64) * 0.36) as f32;
        Geometry { lengths, taps_l, taps_r, early: (ol, or, gl, gr) }
    }
}

/// The per-sample constants for a setting.
#[derive(Clone, Debug, PartialEq)]
pub struct Coefs {
    pub high_cut: f32,
    pub low_cut: f32,
    pub damping: f32,
    pub diffuser: f32,
    pub t0: f32,
    pub t1: f32,
    pub decay: f32,
    pub shelf: f32,
    pub shelf_gain: f32,
    pub predelay: usize,
    pub step: f32,
    pub target: f32,
    pub slew: f32,
    pub dry: f32,
    pub early: [f32; 4],
    pub reverb: [f32; 4],
    pub mid_side: bool,
}

impl Coefs {
    pub fn new(s: &Settings, g: &Geometry, irate: i32, bpm: f64) -> Coefs {
        let k = &s.knobs;
        let q = q_of(irate);
        let rate = irate as f32;
        let len = |i: usize| g.lengths[i] as i32;

        let d = (f64::from(k[DIFFUSION]) * 0.01) as f32;
        let diffuser = diff_coef(len(0) + len(1), 0.07, d, q);
        let t0 = diff_coef(len(4) + len(8), 0.35, d, q);
        let t1 = diff_coef(len(6) + len(10), 0.51, d, q);

        let lp = (len(5) + len(7) + len(9) + len(11)) as f32;
        let t = (k[DECAY] as i64 * i64::from(irate)) as f32 * 0.1;
        let decay = quarter_gain(lp, t);
        let tb = t * (k[BASS] as f32 * 0.01f32);
        let mut bass = quarter_gain(lp, tb) / decay;
        let shelf_gain = ((f64::from(bass) - 1.0) * 0.5) as f32;
        bass = bass.min(1.0);
        let tc = (f64::from(k[CROSSOVER]) * PI * f64::from(q)).tan() as f32;
        let shelf = (tc - bass) / (tc + bass);

        let sp = k[MOD_SPEED] as f32 / 2048.0;
        let step = (sp * sp * sp * 9.9 + 0.1) / rate;
        let target = (k[MOD_DEPTH] as f32 / 2048.0) * 0.004_535_147_5 * rate;
        let slew = 1.0 - 0.01f64.powf(f64::from(1.0 / (rate * 0.1))) as f32;

        Coefs {
            high_cut: lowpass(k[HIGH_CUT], irate),
            low_cut: low_cut(k[LOW_CUT], irate),
            damping: lowpass(k[DAMPING], irate),
            diffuser,
            t0,
            t1,
            decay,
            shelf,
            shelf_gain,
            predelay: predelay_samples(k[PREDELAY], s.tempo_sync, irate, bpm),
            step,
            target,
            slew,
            dry: slider(k[DRY] as f32 / 128.0),
            early: matrix(slider(k[EARLY] as f32 / 256.0), k[SEPARATION], s.mid_side),
            reverb: matrix(slider(k[WET] as f32 / 256.0), k[SEPARATION], s.mid_side),
            mid_side: s.mid_side,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Lines

/// A line of fixed length: reads what was written `len` samples ago, then takes the new value.
#[derive(Clone, Debug, Default)]
struct Line {
    buf: Vec<f32>,
    pos: usize,
    length: usize,
    previous: usize,
}

impl Line {
    fn new(capacity: usize, len: usize) -> Self {
        let length = len.max(1);
        Self { buf: vec![0.; capacity.max(length)], pos: 0, length, previous: length }
    }
    #[inline]
    fn read(&self, age: usize) -> f32 {
        let i = if self.pos >= age { self.pos - age } else { self.buf.len() - (age - self.pos) };
        self.buf[i]
    }
    #[inline]
    fn out(&self, blend: f32) -> f32 {
        let current = self.read(self.length);
        if blend == 0. { current } else { current + blend * (self.read(self.previous) - current) }
    }
    #[inline]
    fn push(&mut self, v: f32) {
        self.buf[self.pos] = v;
        self.pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
        }
    }
    /// A delay: the input `len` samples ago.
    #[inline]
    fn delay(&mut self, x: f32, blend: f32) -> f32 {
        let y = self.out(blend);
        self.push(x);
        y
    }
    /// An all-pass with forward `f` and feedback `b`.
    #[inline]
    fn allpass(&mut self, x: f32, f: f32, b: f32, blend: f32) -> f32 {
        let y = self.out(blend) - f * x;
        self.push(b * y + x);
        y
    }
    /// The value stored `k` writes ago, this sample's write counting as 1 (`k = 0`: the oldest).
    #[inline]
    fn tap(&self, k: usize, length: usize) -> f32 {
        let age = if k > length { k % length } else { k };
        self.read(if age == 0 { length } else { age })
    }
    fn clear(&mut self) {
        self.buf.iter_mut().for_each(|v| *v = 0.0);
    }
}

/// A history: the input `k` samples ago (k ≥ 1 before this sample's write, k ≥ 0 after it).
#[derive(Clone, Debug, Default)]
struct History {
    buf: Vec<f32>,
    /// Where the next value goes.
    w: usize,
}

impl History {
    fn new(len: usize) -> History {
        History { buf: vec![0.0; len.max(2)], w: 0 }
    }

    #[inline]
    fn ago(&self, k: usize) -> f32 {
        let n = self.buf.len();
        self.buf[(self.w + n - 1 - (k % n)) % n]
    }
    /// Before this sample's write: the input `k` samples ago.
    #[inline]
    fn before(&self, k: usize) -> f32 {
        let n = self.buf.len();
        self.buf[(self.w + n - (k % n)) % n]
    }
    #[inline]
    fn push(&mut self, v: f32) {
        self.buf[self.w] = v;
        self.w += 1;
        if self.w == self.buf.len() {
            self.w = 0;
        }
    }
    fn grow(&mut self, len: usize) {
        if len > self.buf.len() {
            // Keep what was written, in order (the newest last).
            let n = self.buf.len();
            let mut v: Vec<f32> = (0..n).map(|i| self.buf[(self.w + i) % n]).collect();
            v.splice(0..0, std::iter::repeat_n(0.0, len - n));
            self.buf = v;
            self.w = 0;
        }
    }
    fn clear(&mut self) {
        self.buf.iter_mut().for_each(|v| *v = 0.0);
    }
}

/// One half of the tank's filter states.
#[derive(Clone, Copy, Debug, Default)]
struct Half {
    x: f32,
    s: f32,
    d: f32,
}

/// Level ramps for knob moves while the reverb plays (Lemon Studio's own: FL ramps its levels, how is not described).
#[derive(Clone, Debug)]
struct Ramp {
    from: [f32; 9],
    left: u32,
}

const RAMP: u32 = 1024;

/// The reverb.
#[derive(Clone)]
pub struct Reverb {
    irate: i32,
    bpm: f64,
    settings: Settings,
    geom: Geometry,
    c: Coefs,
    previous_room: Geometry,
    room_fade: u32,
    room_fade_length: u32,
    room_playing: bool,
    room: Option<i32>,
    lines: Vec<Line>,
    early: History,
    pre: History,
    mod_a: History,
    mod_b: History,
    half_a: Half,
    half_b: Half,
    lp: f32,
    lc_x: f32,
    lc_a: f32,
    ph: f32,
    depth: f32,
    ramp: Option<Ramp>,
    /// Peak of the reverb output since the editor last asked (for its display).
    pub peak: f32,
    pub duck_mode: u8,
    pub duck_amount: f32,
    reduction: f32,
    duck_env: f32,
    hard_hold_left: u32,
    hard_hold_samples: u32,
    hard_attack_step: f32,
    hard_release: f32,
    duck_slew: [f32;2],
}

impl Reverb {
    pub fn new(sample_rate: u32, settings: Settings, bpm: f64) -> Reverb {
        let irate = sample_rate.max(1) as i32;
        let geom = Geometry::new(settings.knobs[ROOM], irate);
        let c = Coefs::new(&settings, &geom, irate, bpm);
        let mod_len = (0.004_535_147_5 * f64::from(irate)).ceil() as usize + 4;
        let settings_room = settings.knobs[ROOM];
        let largest = Geometry::new(RANGES[ROOM].1, irate);
        let early = History::new(largest.early.0.max(largest.early.1).max(geom.early.0).max(geom.early.1) + 2);
        let lines = geom.lengths.iter().zip(largest.lengths).map(|(&n, max)| Line::new(max, n)).collect();
        Reverb {
            irate,
            bpm,
            settings,
            previous_room: geom.clone(), room_fade: 0,
            room_fade_length: (sample_rate / 200).max(1), room_playing: false,
            geom: geom.clone(),
            c,
            room: Some(settings_room),
            lines,
            early,
            pre: History::new(2 * irate as usize + 2),
            mod_a: History::new(mod_len),
            mod_b: History::new(mod_len),
            half_a: Half::default(),
            half_b: Half::default(),
            lp: 0.0,
            lc_x: 0.0,
            lc_a: 0.0,
            ph: 0.0,
            depth: 0.0,
            ramp: None,
            peak: 0.0,
            hard_hold_left: 0,
            hard_hold_samples: (sample_rate as f32 * 0.035).ceil() as u32,
            hard_attack_step: 1.0 / (sample_rate.max(1) as f32 * 0.002),
            hard_release: 1.0 - (-1.0 / (sample_rate.max(1) as f32 * 0.085)).exp(),
            duck_mode: 0, duck_amount: 1., reduction: 0., duck_env:0., duck_slew:[0.008,0.24].map(|s|1.-(-1./(sample_rate.max(1) as f32*s)).exp()),
        }
    }

    pub fn duck_settings(state: &[u8]) -> (u8,u8) {
        let tail = state.get(STATE_LEN..).and_then(|t| t.strip_prefix(b"DUCK")).unwrap_or_default();
        let mode = tail.first().copied().unwrap_or(0).min(2);
        let amount = tail.get(1..).and_then(|t|t.strip_prefix(b"AMT"))
            .and_then(|t| t.first()).copied().unwrap_or(100).min(100);
        (mode,amount)
    }

    pub fn from_state(state: &[u8], sample_rate: u32) -> Option<Reverb> {
        let mut reverb = Reverb::new(sample_rate, Settings::from_state(state)?, 120.0);
        let (mode,amount) = Self::duck_settings(state);
        reverb.duck_mode=mode;
        reverb.duck_amount=f32::from(amount)/100.;
        Some(reverb)
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn coefs(&self) -> &Coefs {
        &self.c
    }

    /// Keep the tank history and crossfade the old and new read positions when the room changes.
    fn size_lines(&mut self) {
        let room = self.settings.knobs[ROOM];
        if self.room == Some(room) || self.room_fade > 0 {
            return;
        }
        self.room = Some(room);
        self.previous_room = self.geom.clone();
        self.room_fade = if self.room_playing { self.room_fade_length } else { 0 };
        self.geom = Geometry::new(room, self.irate);
        for (line, length) in self.lines.iter_mut().zip(self.geom.lengths) {
            line.previous = line.length;
            line.length = length.max(1);
            if line.length > line.buf.len() { line.buf.resize(line.length, 0.); }
        }
        let (ol, or, _, _) = self.geom.early;
        if ol.max(or) + 2 > self.early.buf.len() { self.early.grow(ol.max(or) + 2); }
    }

    /// New settings (from the editor or the host). Levels glide when they change.
    pub fn set(&mut self, s: Settings) {
        if s == self.settings {
            return;
        }
        let old = self.levels();
        self.settings = s;
        self.size_lines();
        self.c = Coefs::new(&self.settings, &self.geom, self.irate, self.bpm);
        if old != self.levels() {
            self.ramp = Some(Ramp { from: self.current_levels(old), left: RAMP });
        }
    }

    /// The host's tempo (for the tempo-based pre-delay).
    pub fn set_tempo(&mut self, bpm: f64) {
        if bpm > 0.0 && bpm != self.bpm {
            self.bpm = bpm;
            if self.settings.tempo_sync {
                self.c.predelay = predelay_samples(self.settings.knobs[PREDELAY], true, self.irate, bpm);
            }
        }
    }

    fn levels(&self) -> [f32; 9] {
        let c = &self.c;
        [c.dry, c.early[0], c.early[1], c.early[2], c.early[3], c.reverb[0], c.reverb[1], c.reverb[2], c.reverb[3]]
    }

    /// The levels heard right now (a glide in progress counts where it is).
    fn current_levels(&self, target_before: [f32; 9]) -> [f32; 9] {
        match &self.ramp {
            Some(r) => {
                let a = r.left as f32 / RAMP as f32;
                std::array::from_fn(|i| r.from[i] * a + target_before[i] * (1.0 - a))
            }
            None => target_before,
        }
    }

    /// Empties every line and filter (the modulation's phase runs on).
    pub fn clear(&mut self) {
        self.room_playing = false;
        self.room_fade = 0;
        self.size_lines();
        self.c = Coefs::new(&self.settings, &self.geom, self.irate, self.bpm);
        self.duck_env=0.;
        self.hard_hold_left = 0;
        self.reduction=0.;
        self.lines.iter_mut().for_each(Line::clear);
        self.early.clear();
        self.pre.clear();
        self.mod_a.clear();
        self.mod_b.clear();
        self.half_a = Half::default();
        self.half_b = Half::default();
        self.lp = 0.0;
        self.lc_x = 0.0;
        self.lc_a = 0.0;
        self.depth = 0.0;
    }

    /// One sample: the early reflections and the reverb signal per side (they do not depend on this sample's input),
    /// then the input `m` is fed in.
    #[inline]
    fn tick(&mut self, m: f32) -> (f32, f32, f32, f32) {
        self.room_playing = true;
        if self.room_fade == 0 && self.room != Some(self.settings.knobs[ROOM]) {
            self.size_lines();
            self.c = Coefs::new(&self.settings, &self.geom, self.irate, self.bpm);
        }
        let blend = self.room_fade as f32 / self.room_fade_length as f32;
        self.room_fade = self.room_fade.saturating_sub(1);
        let c = &self.c;
        // 1. Modulation.
        let sine = (f64::from(self.ph) * 2.0 * PI).sin() as f32;
        self.ph += c.step;
        self.ph -= self.ph.trunc();
        self.depth = if (c.target - self.depth).abs() < 0.01 {
            c.target
        } else {
            (c.target - self.depth) * c.slew + self.depth + 1e-30
        };
        let sweep = self.depth * ((sine + 1.0) * 0.5);
        let n = sweep.trunc();
        let f = sweep - n;
        let n = n as usize;

        // 2. Early reflections.
        let (ol, or, gl, gr) = self.geom.early;
        let mut er_l = self.early.before(ol) * gl;
        let mut er_r = self.early.before(or) * gr;
        if blend > 0. {
            let (pl, pr, gain_l, gain_r) = self.previous_room.early;
            er_l += blend * (self.early.before(pl) * gain_l - er_l);
            er_r += blend * (self.early.before(pr) * gain_r - er_r);
        }

        // 3. Tank.
        let pre = self.pre.before(c.predelay) + 1e-20;
        let l = &mut self.lines;
        let x = l[11].out(blend) * c.decay;
        let y = half(&mut self.half_a, x, pre, c, &mut self.mod_a, n, f);
        let y = l[4].allpass(y, c.t0, c.t0, blend);
        let y = l[5].delay(y, blend) * c.decay;
        let y = l[6].allpass(y, c.t1, c.t1, blend);
        l[7].push(y);
        let x = l[7].out(blend) * c.decay;
        let y = half(&mut self.half_b, x, pre, c, &mut self.mod_b, n, f);
        let y = l[8].allpass(y, c.t0, c.t0, blend);
        let y = l[9].delay(y, blend) * c.decay;
        let y = l[10].allpass(y, c.t1, c.t1, blend);
        l[11].push(y);

        let k = 1.0 - c.decay;
        let c06 = k * 0.6 + c.decay;
        let c025 = k * 0.25 + c.decay;
        let c07 = k * 0.7 + c.decay;
        let c02 = k * 0.2 + c.decay;
        let c05 = k * 0.5 + c.decay;
        let tap = |(idx,k): (usize,usize), old: (usize,usize)| {
            let line = &l[idx];
            let current = line.tap(k, line.length);
            if blend == 0. { current } else {
                current + blend * (l[old.0].tap(old.1, l[old.0].previous) - current)
            }
        };
        let tl = |i: usize| tap(self.geom.taps_l[i], self.previous_room.taps_l[i]);
        let tr = |i: usize| tap(self.geom.taps_r[i], self.previous_room.taps_r[i]);
        let left = (((tl(0) - c025 * tl(1)) - c07 * tl(2)) - c02 * tl(3)) + c06 * tl(4) + tl(5) + c05 * tl(6);
        let right = ((((c025 * tr(0) + tr(1)) - c07 * tr(2)) - c02 * tr(3)) - c06 * tr(4)) + tr(5) + c05 * tr(6);

        // 4. Feeding the input.
        let x = ((m + 1e-30) - self.lp) * c.high_cut + self.lp;
        self.lp = x;
        let a = (x - self.lc_a) * c.low_cut + self.lc_x;
        self.lc_x = x;
        self.lc_a = a;
        let mut x = if c.low_cut != 0.0 { (x - a) * 0.5 + 1e-30 } else { x };
        x = l[0].allpass(x, c.diffuser, c.diffuser, blend);
        self.early.push(x);
        x = l[1].allpass(x, c.diffuser, c.diffuser, blend);
        x = l[2].allpass(x, c.diffuser, c.diffuser, blend);
        x = l[3].allpass(x, c.diffuser, c.diffuser, blend);
        self.pre.push(x);

        (er_l, er_r, left, right)
    }

    /// One stereo frame through the reverb.
    #[inline]
    pub fn frame(&mut self, il: f32, ir: f32) -> (f32, f32) {
        self.frame_observed(il, ir, |_, _, _| {})
    }

    #[inline]
    fn frame_observed(&mut self, il: f32, ir: f32, observe: impl FnOnce(f32, f32, f32)) -> (f32, f32) {
        let m = if self.c.mid_side { il - ir } else { il + ir };
        let (el, er, wl, wr) = self.tick(m);
        let (dry, e, w) = match &mut self.ramp {
            None => (self.c.dry, self.c.early, self.c.reverb),
            Some(rp) => {
                rp.left -= 1;
                let a = rp.left as f32 / RAMP as f32;
                let c = &self.c;
                let to = [c.dry, c.early[0], c.early[1], c.early[2], c.early[3], c.reverb[0], c.reverb[1], c.reverb[2], c.reverb[3]];
                let v: [f32; 9] = std::array::from_fn(|i| rp.from[i] * a + to[i] * (1.0 - a));
                if rp.left == 0 {
                    self.ramp = None;
                }
                (v[0], [v[1], v[2], v[3], v[4]], [v[5], v[6], v[7], v[8]])
            }
        };
        let (mut ol, mut or) = if dry == 0.0 {
            (0.0, 0.0)
        } else if dry == 1.0 {
            (il, ir)
        } else {
            (il * dry, ir * dry)
        };
        let (dry_left, dry_right) = (ol, or);
        let peak = il.abs().max(ir.abs());
        if self.duck_mode == 2 {
            // -72 dBFS wakes the gate. Hold through zero crossings and short waveform gaps.
            if peak >= 0.000_251_189 {
                self.hard_hold_left = self.hard_hold_samples;
            }
            let active = self.hard_hold_left > 0;
            self.hard_hold_left = self.hard_hold_left.saturating_sub(1);
            let target = if active { self.duck_amount } else { 0.0 };
            if target > self.reduction {
                self.reduction = (self.reduction + self.hard_attack_step * target).min(target);
            } else {
                self.reduction += (target - self.reduction) * self.hard_release;
            }
        } else {
            self.hard_hold_left = 0;
            self.duck_env += (peak - self.duck_env) * self.duck_slew[usize::from(peak < self.duck_env)];
            let target = if self.duck_mode == 1 {
                self.duck_amount * (1.0 - 1.0 / (1.0 + self.duck_env * 3.0))
            } else { 0.0 };
            self.reduction += (target - self.reduction) * self.duck_slew[0];
        }
        let gain = 1. - self.reduction;
        mix(&mut ol, &mut or, el*gain, er*gain, &e);
        mix(&mut ol, &mut or, wl*gain, wr*gain, &w);
        self.peak = self.peak.max(wl.abs()).max(wr.abs()).max(el.abs()).max(er.abs());
        observe(ol - dry_left, or - dry_right, self.reduction);
        (ol, or)
    }

    /// Makes room for the pre-delay in force (allocates only when it grew).
    fn fit_predelay(&mut self) {
        if self.pre.buf.len() < self.c.predelay + 1 {
            self.pre.grow(self.c.predelay + 1);
        }
    }

    /// Processes a stereo block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.fit_predelay();
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            (*l, *r) = self.frame(*l, *r);
        }
    }

    /// Peak wet returns (after mixing and ducking), and maximum applied gain reduction.
    /// Used only while an editor is listening. All three values are linear.
    pub fn process_metered(&mut self, left: &mut [f32], right: &mut [f32]) -> [f32; 3] {
        self.fit_predelay();
        let mut levels = [0.0_f32; 3];
        for (l, r) in left.iter_mut().zip(right) {
            (*l, *r) = self.frame_observed(*l, *r, |wl, wr, reduction| {
                levels[0] = levels[0].max(wl.abs());
                levels[1] = levels[1].max(wr.abs());
                levels[2] = levels[2].max(reduction);
            });
        }
        levels
    }

    /// Runs `frames` samples of silence through the reverb (what FL processes after loading, before the song).
    pub fn preroll(&mut self, frames: usize) {
        let mut a = vec![0.0f32; 2048];
        let mut b = vec![0.0f32; 2048];
        let mut left = frames;
        while left > 0 {
            let n = left.min(2048);
            a[..n].fill(0.0);
            b[..n].fill(0.0);
            self.process(&mut a[..n], &mut b[..n]);
            left -= n;
        }
    }
}

/// A tank half's input stage: bass shelf, damping and the modulated delay.
#[inline]
fn half(h: &mut Half, x: f32, pre: f32, c: &Coefs, md: &mut History, n: usize, f: f32) -> f32 {
    let s = (x - h.s) * c.shelf + h.x;
    h.x = x;
    h.s = s;
    let y = (((s + x) * c.shelf_gain + x + pre) - h.d) * c.damping + h.d;
    h.d = y;
    md.push(y);
    f * md.ago(n + 1) + (1.0 - f) * md.ago(n)
}

/// Adds a signal pair through a matrix `(LL, RR, RL, LR)`.
#[inline]
fn mix(ol: &mut f32, or: &mut f32, xl: f32, xr: f32, m: &[f32; 4]) {
    if m[2] == 0.0 && m[3] == 0.0 {
        if m[0] == 0.0 && m[1] == 0.0 {
            return;
        }
        *ol = xl * m[0] + *ol;
        *or = xr * m[1] + *or;
    } else {
        *ol = xl * m[0] + xr * m[2] + *ol;
        *or = xr * m[1] + xl * m[3] + *or;
    }
}

impl Effect for Reverb {
    fn process(&mut self, buf: &mut [f32]) {
        self.fit_predelay();
        for f in buf.chunks_exact_mut(2) {
            (f[0], f[1]) = self.frame(f[0], f[1]);
        }
    }

    fn reset(&mut self) {
        self.clear();
    }

    fn set_tempo(&mut self, bpm: f64) {
        Reverb::set_tempo(self, bpm);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_of_every_version_load() {
        let s = Settings { knobs: [33, 21, 27, 90, 100, 45, 31, 150, 298, 0, 128, 128, 90, 700, 20], tempo_sync: true, mid_side: true };
        let b = s.to_state();
        assert_eq!(b.len(), STATE_LEN);
        assert_eq!(Settings::from_state(&b), Some(s));
        // Version 10000: thirteen knobs, no modulation.
        let mut old = 10000i32.to_le_bytes().to_vec();
        for v in &s.knobs[..13] {
            old.extend_from_slice(&v.to_le_bytes());
        }
        old.extend_from_slice(&[0, 1]);
        let o = Settings::from_state(&old).unwrap();
        assert_eq!((o.knobs[MOD_SPEED], o.knobs[MOD_DEPTH], o.tempo_sync, o.mid_side), (684, 0, false, true));
        // A factory preset (no version, one byte).
        let mut p = Vec::new();
        for v in [400, 60, 31, 60, 65, 23, 37, 70, 330, 0, 128, 128, 102] {
            p.extend_from_slice(&i32::to_le_bytes(v));
        }
        p.push(0);
        let v = Settings::from_state(&p).unwrap();
        assert_eq!(v.knobs[LOW_CUT], 400);
        assert_eq!(v.knobs[WET], 102);
        assert!(!v.tempo_sync && !v.mid_side);
    }

    #[test]
    fn line_geometry_follows_the_room() {
        let g = Geometry::new(100, 20000);
        assert_eq!(g.lengths, LINES.map(|v| v as usize));
        assert_eq!(g.taps_l[0], (9, 266));
        let g = Geometry::new(50, 44100);
        // scale = 65 * 44100 / 115 = 24926
        assert_eq!(g.lengths[0], 24926 * 142 / 20000);
        // The early side with the smaller offset gets exactly 0.36.
        assert!(g.early.0 > g.early.1);
        assert_eq!(g.early.3, 0.36f32);
    }

    #[test]
    fn levels_follow_the_slider_law() {
        assert_eq!(slider(1.0), 1.0);
        assert_eq!(slider(0.0), 0.0);
        assert!((slider(0.5) - 0.2317).abs() < 1e-3);
        let m = matrix(0.5, 0, false);
        assert_eq!(m, [0.5, 0.5, 0.0, 0.0]);
        let m = matrix(0.5, 0, true);
        assert_eq!(m, [0.5, -0.5, 0.0, -0.0]);
        let m = matrix(0.5, -64, false);
        assert!(m[2] < 0.0 && m[0] > 0.5);
        let m = matrix(0.5, 64, false);
        assert!((m[0] - m[2]).abs() < 1e-6, "fully merged: the cross terms equal the direct ones");
        // The pan's cosine and sine stay double: FL's entries at wet 50 %, half merged and fully separated (fitted to
        // its renders).
        let v = slider(0.5);
        let m = matrix(v, 32, false);
        assert_eq!((m[0].to_bits(), m[2].to_bits()), (0x3e48_91cd, 0x3d12_9c8c));
        assert_eq!(matrix(v, -64, false)[0], v * 1.5);
    }

    #[test]
    fn the_output_does_not_depend_on_this_samples_input() {
        let mut a = Reverb::new(44100, Settings::default(), 120.0);
        let mut b = Reverb::new(44100, Settings { knobs: { let mut k = DEFAULTS; k[DRY] = 0; k }, ..Settings::default() }, 120.0);
        let _ = &mut a;
        let mut l = vec![1.0f32];
        let mut r = vec![0.5f32];
        b.process(&mut l, &mut r);
        assert_eq!((l[0], r[0]), (0.0, 0.0));
        // and later samples do hear it
        let mut l = vec![0.0f32; 8000];
        let mut r = vec![0.0f32; 8000];
        b.process(&mut l, &mut r);
        assert!(l.iter().any(|v| v.abs() > 1e-6));
    }

    #[test]
    fn a_tail_decays() {
        let mut rv = Reverb::new(44100, Settings::default(), 120.0);
        let mut l = vec![0.0f32; 44100 * 4];
        let mut r = vec![0.0f32; 44100 * 4];
        l[0] = 1.0;
        r[0] = 1.0;
        rv.process(&mut l, &mut r);
        let e = |a: usize, b: usize| l[a..b].iter().map(|v| v * v).sum::<f32>();
        let first = e(4410, 22050);
        let last = e(22050 * 6, 22050 * 7);
        assert!(first > 0.0 && last < first * 0.01, "{first} {last}");
    }

    #[test]
    fn pre_delay_counts_steps() {
        assert_eq!(predelay_samples(0, false, 44100, 120.0), 1);
        assert_eq!(predelay_samples(384, false, 44100, 120.0), 44100);
        // 192 steps = one beat: 0.5 s at 120 BPM.
        assert_eq!(predelay_samples(192, true, 44100, 120.0), 22050);
    }
}
