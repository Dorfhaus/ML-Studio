// Adapted from B.Oops FxChopper.hpp and FxStutter.hpp, Sven Jaehnichen,
// GPL-3.0-or-later. Native transport, 16-step editor and safety bounds are ML additions.
use crate::*;
use std::sync::{
    Mutex,
    atomic::{AtomicU32, Ordering},
};
pub type TimeShaper = Fx<Time>;
const CYCLES: [f64; 5] = [0.25, 0.5, 1., 2., 4.];
const GRAPH: Rect = Rect::new(20., 386., 720., 154.);
pub struct Time {
    rate: f64,
    bpm: f64,
    phase: f64,
    host_phase: Option<f64>,
    host_sample: Option<i64>,
    expected: Option<i64>,
    line: Vec<[f32; 2]>,
    at: usize,
    valid: usize,
    smooth: [f64; 2],
    mode: f64,
    levels: [f64; 16],
    head: Arc<AtomicU32>,
}
impl Default for Time {
    fn default() -> Self {
        Self {
            rate: 48000.,
            bpm: 120.,
            phase: 0.,
            host_phase: None,
            host_sample: None,
            expected: None,
            line: Vec::new(),
            at: 0,
            valid: 0,
            smooth: [1., 1.],
            mode: 0.,
            levels: [1.; 16],
            head: Arc::new(AtomicU32::new(0)),
        }
    }
}
/// B.Oops' centered, piecewise-linear transition between adjacent chop gains.
pub(crate) fn envelope(levels: &[f64; 16], phase: f64, smoothing: f64) -> f64 {
    let t = (phase + 1e-10).rem_euclid(1.) * 16.;
    let step = t.floor() as usize;
    let frac = t - step as f64;
    let v = levels[step];
    if smoothing > 0. && frac < 0.5 * smoothing {
        v + (levels[(step + 15) % 16] - v) * (0.5 - frac / smoothing)
    } else if smoothing > 0. && frac > 1. - 0.5 * smoothing {
        v + (levels[(step + 1) % 16] - v) * (0.5 - (1. - frac) / smoothing)
    } else {
        v
    }
}
impl Time {
    fn read(&self, back: f64) -> [f64; 2] {
        let d = back.max(0.);
        let a = d.floor() as usize;
        let frac = d - a as f64;
        let get = |n: usize| {
            if n < self.valid {
                self.line[(self.at + self.line.len() - n) % self.line.len()]
            } else {
                [0.; 2]
            }
        };
        let x = get(a);
        let y = get(a + 1);
        std::array::from_fn(|i| x[i] as f64 + (y[i] - x[i]) as f64 * frac)
    }
}
impl Unit for Time {
    const SPEC: &'static Spec = &specs::TIME;
    const ID: &'static str = "com.mlstudio.timeshaper";
    const CLASS: [u8; 16] = *b"MLTimeShaper0001";
    fn prepare(&mut self, rate: f32, p: &FxParams) {
        self.rate = rate as f64;
        self.line = vec![[0.; 2]; rate as usize * 8 + 8];
        self.reset(p);
    }
    fn reset(&mut self, p: &FxParams) {
        self.valid = 0;
        self.at = 0;
        self.phase = 0.;
        self.expected = None;
        self.host_phase = None;
        self.host_sample = None;
        self.mode = p.v(1) as f64;
        self.smooth = [p.v(4) as f64 / 1000., 10f64.powf(p.v(5) as f64 / 200.)];
        self.levels = std::array::from_fn(|i| p.v(6 + i) as f64 / 1000.);
    }
    fn transport(&mut self, t: &Transport) {
        self.bpm = t
            .tempo
            .filter(|v| v.is_finite())
            .unwrap_or(120.)
            .clamp(40., 300.);
        self.host_phase = if t.playing {
            t.pos_beats().filter(|v| v.is_finite())
        } else {
            None
        };
        self.host_sample = if t.playing { t.pos_samples() } else { None };
    }
    fn audio(&mut self, l: &mut [f32], r: &mut [f32], p: &FxParams) {
        let cycle = CYCLES[p.v(0).clamp(0, 4) as usize];
        let step = self.bpm / 60. / self.rate / cycle;
        if let (Some(pos), Some(expected)) = (self.host_sample, self.expected) {
            if pos != expected {
                self.valid = 0;
            }
        }
        self.expected = self.host_sample.map(|n| n + l.len() as i64);
        if let Some(beats) = self.host_phase {
            self.phase = (beats / cycle).rem_euclid(1.);
        }
        let length = 1. / step;
        let repeats = p.v(2) as f64;
        let slice = length / repeats;
        let smoothing = p.v(3) as f64 / 1000.;
        let coeff = 1. - (-1. / (0.005 * self.rate)).exp();
        let targets = [p.v(4) as f64 / 1000., 10f64.powf(p.v(5) as f64 / 200.)];
        for i in 0..l.len() {
            let x = [l[i], r[i]].map(|v| if v.is_finite() { v } else { 0. });
            self.line[self.at] = x;
            self.valid = (self.valid + 1).min(self.line.len() - 1);
            for k in 0..2 {
                self.smooth[k] += coeff * (targets[k] - self.smooth[k]);
            }
            for k in 0..16 {
                self.levels[k] += coeff * (p.v(6 + k) as f64 / 1000. - self.levels[k]);
            }
            let mut wet = x.map(|v| v as f64);
            self.mode += coeff * (p.v(1) as f64 - self.mode);
            if self.mode > 1e-9 {
                let slice_phase = (self.phase + 1e-10).rem_euclid(1.) * repeats;
                let nr = slice_phase.floor();
                let frac = slice_phase.fract();
                wet = self.read(nr * slice);
                // Same adjacent-tap interpolation as B.Oops; bounds suppress unavailable startup history.
                if smoothing > 0. && frac < 0.5 * smoothing {
                    let old = self.read((nr - 1.).max(0.) * slice);
                    let f = 0.5 - frac / smoothing;
                    for ch in 0..2 {
                        wet[ch] += (old[ch] - wet[ch]) * f;
                    }
                } else if smoothing > 0. && frac > 1. - 0.5 * smoothing {
                    let next = self.read((nr + 1.) * slice);
                    let f = 0.5 - (1. - frac) / smoothing;
                    for ch in 0..2 {
                        wet[ch] += (next[ch] - wet[ch]) * f;
                    }
                }
            }
            for ch in 0..2 {
                wet[ch] = x[ch] as f64 + (wet[ch] - x[ch] as f64) * self.mode;
            }
            let level = envelope(&self.levels, self.phase, smoothing);
            l[i] = ((x[0] as f64 + (wet[0] * level - x[0] as f64) * self.smooth[0])
                * self.smooth[1]) as f32;
            r[i] = ((x[1] as f64 + (wet[1] * level - x[1] as f64) * self.smooth[0])
                * self.smooth[1]) as f32;
            self.at = (self.at + 1) % self.line.len();
            self.phase = (self.phase + step).rem_euclid(1.);
        }
        self.head
            .store((self.phase as f32).to_bits(), Ordering::Relaxed);
    }
    fn panel(&self, p: Arc<FxParams>) -> Arc<dyn Extension> {
        Arc::new(Panel {
            p,
            head: self.head.clone(),
            drag: Mutex::new(None),
        })
    }
}
struct Panel {
    p: Arc<FxParams>,
    head: Arc<AtomicU32>,
    drag: Mutex<Option<(usize, i32, Option<Arc<dyn GuiContext>>)>>,
}
impl Panel {
    fn set(&self, i: usize, v: i32, ctx: Option<&Arc<dyn GuiContext>>) {
        self.p
            .live
            .set(self.p.p[i].as_ptr(), self.p.p[i].preview_normalized(v));
        if let Some(c) = ctx {
            ParamSetter::new(c.as_ref()).set_parameter(&self.p.p[i], v);
        }
    }
    fn finish(&self, cancel: bool) {
        if let Some((i, old, ctx)) = self.drag.lock().unwrap().take() {
            if cancel {
                self.set(i, old, ctx.as_ref());
            }
            if let Some(c) = ctx {
                ParamSetter::new(c.as_ref()).end_set_parameter(&self.p.p[i]);
            }
        }
    }
    fn preset(&self, kind: usize, ctx: Option<&Arc<dyn GuiContext>>) {
        let values: [i32; 16] = std::array::from_fn(|i| match kind {
            0 => 1000,
            1 => {
                if i % 2 == 0 {
                    1000
                } else {
                    0
                }
            }
            2 => (i % 4) as i32 * 1000 / 3,
            _ => 0,
        });
        if let Some(c) = ctx {
            let mut state = c.get_state();
            for (i, v) in values.into_iter().enumerate() {
                state.params.insert(
                    format!("step{i}"),
                    nih_plug::wrapper::state::ParamValue::I32(v),
                );
            }
            self.p.live.clear();
            c.set_state(state);
        } else {
            for (i, v) in values.into_iter().enumerate() {
                self.set(6 + i, v, None);
            }
        }
    }
}
impl Extension for Panel {
    fn paint(&self, c: &mut Canvas, _: &dyn lemon_small_fx::view::Model) {
        if !os_plugkit::visibility::stock_dsp_controls() {
            return;
        }
        for (i, label) in ["All on", "Chop", "Pump", "Clear"].iter().enumerate() {
            button(c, Rect::new(20. + i as f32 * 120., 330., 110., 28.), label);
        }
        text(
            c,
            20.,
            367.,
            "STEP LEVELS — drag a bar vertically; Escape cancels; presets and edits support Undo",
        );
        c.fill_rect(GRAPH, Color::rgb(0x1d1d1d));
        for i in 0..16 {
            let x = GRAPH.x + i as f32 * 45.;
            let h = self.p.v(i + 6) as f32 / 1000. * GRAPH.h;
            c.fill_rect(
                Rect::new(x + 3., GRAPH.y + GRAPH.h - h, 39., h),
                Color::rgb(0x249ce0),
            );
            text(c, x + 15., GRAPH.y + GRAPH.h + 7., &(i + 1).to_string());
        }
        let x = GRAPH.x + f32::from_bits(self.head.load(Ordering::Relaxed)) * GRAPH.w;
        c.line(x, GRAPH.y, x, GRAPH.y + GRAPH.h, 2., Color::rgb(0xffffff));
        text(
            c,
            20.,
            578.,
            "Cycle follows song beats (40–300 BPM). Repeat replays the first slice of each cycle.",
        );
        text(
            c,
            20.,
            602.,
            "Stopped transport runs freely • Smooth softens edges • Mix 0% is dry • no fixed latency",
        );
    }
    fn click(&self, x: f32, y: f32, ctx: Option<&Arc<dyn GuiContext>>) -> bool {
        if !os_plugkit::visibility::stock_dsp_controls() {
            return false;
        }
        for i in 0..4 {
            if Rect::new(20. + i as f32 * 120., 330., 110., 28.).contains(x, y) {
                self.preset(i, ctx);
                return true;
            }
        }
        if GRAPH.contains(x, y) {
            let i = 6 + (((x - GRAPH.x) / 45.) as usize).min(15);
            if let Some(c) = ctx {
                ParamSetter::new(c.as_ref()).begin_set_parameter(&self.p.p[i]);
            }
            *self.drag.lock().unwrap() = Some((i, self.p.v(i), ctx.cloned()));
            self.moved(x, y);
            return true;
        }
        false
    }
    fn moved(&self, _: f32, y: f32) -> bool {
        let d = self.drag.lock().unwrap();
        if let Some((i, _, ctx)) = &*d {
            let v = ((1. - (y - GRAPH.y) / GRAPH.h) * 1000.)
                .round()
                .clamp(0., 1000.) as i32;
            self.set(*i, v, ctx.as_ref());
            return true;
        }
        false
    }
    fn dragging(&self) -> bool {
        self.drag.lock().unwrap().is_some()
    }
    fn release(&self, _: Option<&Arc<dyn GuiContext>>) -> bool {
        let had = self.dragging();
        self.finish(false);
        had
    }
    fn character(&self, c: char) -> bool {
        if c == '\u{1b}' && self.dragging() {
            self.finish(true);
            return true;
        }
        false
    }
    fn editor_closed(&self) {
        self.finish(false);
    }
}
