use crate::*;
use os_dsp::snaptune::SnapTune;
use std::sync::atomic::{AtomicU32, Ordering};
pub type VocalTune = Fx<Vocal>;
pub struct Vocal {
    engine: Option<SnapTune>,
    rate: f32,
    delay: Vec<[f32; 2]>,
    at: usize,
    smooth: [f32; 2],
    request: Arc<AtomicU32>,
    result: Arc<AtomicU32>,
    seen: u32,
}
impl Default for Vocal {
    fn default() -> Self {
        Self {
            engine: None,
            rate: 48000.,
            delay: Vec::new(),
            at: 0,
            smooth: [1., 1.],
            request: Arc::new(AtomicU32::new(0)),
            result: Arc::new(AtomicU32::new(0)),
            seen: 0,
        }
    }
}
impl Unit for Vocal {
    const SPEC: &'static Spec = &specs::VOCAL;
    const ID: &'static str = "com.mlstudio.vocaltune";
    const CLASS: [u8; 16] = *b"MLVocalTune00001";
    fn latency(&self) -> u32 {
        self.delay.len() as u32
    }
    fn prepare(&mut self, rate: f32, p: &FxParams) {
        self.rate = rate;
        let engine = SnapTune::new(rate as u32, [65535, 0, 0, 0], 0);
        self.delay = vec![[0.; 2]; engine.latency() as usize];
        self.engine = Some(engine);
        self.reset(p);
    }
    fn reset(&mut self, p: &FxParams) {
        self.delay.fill([0.; 2]);
        self.at = 0;
        self.smooth = [
            if p.v(0) == 0 {
                0.
            } else {
                p.v(4) as f32 / 1000.
            },
            10f32.powf(p.v(5) as f32 / 200.),
        ];
        if let Some(e) = &mut self.engine {
            e.configure(e.amount, e.key, e.scale, e.voice, 0, false);
            e.configure(e.amount, e.key, e.scale, e.voice, 0, true);
        }
        self.result.store(0, Ordering::Release);
        self.seen = self.request.load(Ordering::Acquire);
    }
    fn audio(&mut self, l: &mut [f32], r: &mut [f32], p: &FxParams) {
        let Some(e) = &mut self.engine else { return };
        let request = self.request.load(Ordering::Acquire);
        if request != self.seen {
            self.seen = request;
            self.result.store(1, Ordering::Release);
        }
        let learn = if self.result.load(Ordering::Acquire) == 1 {
            request
        } else {
            0
        };
        // Keep the engine's wet path and reported delay stable, including Amount 0.
        e.configure(
            (p.v(0) as f32 / 1000.).max(0.000001),
            p.v(1) as u8,
            p.v(2) as u8,
            p.v(3) as u8,
            learn,
            true,
        );
        let target_mix = if p.v(0) == 0 {
            0.
        } else {
            p.v(4) as f32 / 1000.
        };
        let gain = 10f32.powf(p.v(5) as f32 / 200.);
        let coeff = 1. - (-1. / (0.01 * self.rate)).exp();
        for start in (0..l.len()).step_by(256) {
            let n = (l.len() - start).min(256);
            let mut wet = [0.; 512];
            let mut dry = [[0.; 2]; 256];
            for i in 0..n {
                let x = [l[start + i], r[start + i]].map(|v| if v.is_finite() { v } else { 0. });
                wet[2 * i] = x[0];
                wet[2 * i + 1] = x[1];
                dry[i] = self.delay[self.at];
                self.delay[self.at] = x;
                self.at = (self.at + 1) % self.delay.len();
            }
            e.process(&mut wet[..2 * n]);
            for i in 0..n {
                self.smooth[0] += coeff * (target_mix - self.smooth[0]);
                self.smooth[1] += coeff * (gain - self.smooth[1]);
                l[start + i] =
                    (dry[i][0] + (wet[2 * i] - dry[i][0]) * self.smooth[0]) * self.smooth[1];
                r[start + i] =
                    (dry[i][1] + (wet[2 * i + 1] - dry[i][1]) * self.smooth[0]) * self.smooth[1];
            }
        }
        if learn != 0 {
            if let Some((key, scale)) = e.suggestion() {
                self.result
                    .store(2 + key as u32 + scale as u32 * 12, Ordering::Release);
            }
        }
    }
    fn panel(&self, p: Arc<FxParams>) -> Arc<dyn Extension> {
        Arc::new(Panel {
            p,
            request: self.request.clone(),
            result: self.result.clone(),
        })
    }
}
struct Panel {
    p: Arc<FxParams>,
    request: Arc<AtomicU32>,
    result: Arc<AtomicU32>,
}
impl Extension for Panel {
    fn paint(&self, c: &mut Canvas, _: &dyn lemon_small_fx::view::Model) {
        if !os_plugkit::visibility::stock_dsp_controls() {
            return;
        }
        let v = self.result.load(Ordering::Acquire);
        button(
            c,
            Rect::new(16., 327., 140., 30.),
            if v == 1 {
                "Cancel learning"
            } else {
                "Learn key"
            },
        );
        if v >= 2 {
            button(c, Rect::new(166., 327., 140., 30.), "Use suggestion");
        }
        let line = if v >= 2 {
            let x = v - 2;
            format!(
                "Suggested: {} {}",
                os_dsp::snaptune::KEYS[(x % 12) as usize],
                os_dsp::snaptune::SCALES[(x / 12) as usize]
            )
        } else if v == 1 {
            "Listening: play a varied vocal phrase (at least six seconds of voiced notes).".into()
        } else {
            "Choose Key and Scale, or learn a suggestion from a varied vocal phrase.".into()
        };
        text(c, 16., 374., &line);
        text(
            c,
            16.,
            398.,
            "Same SnapTune engine as the mixer. Amount controls speed and correction strength.",
        );
        text(
            c,
            16.,
            422.,
            "Single voice • Amount 0% / Mix 0% is delay-aligned dry • fixed latency for stable timing",
        );
    }
    fn click(&self, x: f32, y: f32, ctx: Option<&Arc<dyn GuiContext>>) -> bool {
        if !os_plugkit::visibility::stock_dsp_controls() {
            return false;
        }
        if Rect::new(16., 327., 140., 30.).contains(x, y) {
            if self.result.load(Ordering::Acquire) == 1 {
                self.result.store(0, Ordering::Release);
            } else {
                self.request.fetch_add(1, Ordering::AcqRel);
            }
            return true;
        }
        let v = self.result.load(Ordering::Acquire);
        if v >= 2 && Rect::new(166., 327., 140., 30.).contains(x, y) {
            let values = [((v - 2) % 12) as i32, ((v - 2) / 12) as i32];
            if let Some(c) = ctx {
                let mut state = c.get_state();
                state.params.insert(
                    "key".into(),
                    nih_plug::wrapper::state::ParamValue::I32(values[0]),
                );
                state.params.insert(
                    "scale".into(),
                    nih_plug::wrapper::state::ParamValue::I32(values[1]),
                );
                self.p.live.clear();
                c.set_state(state);
            } else {
                for (i, v) in values.iter().enumerate() {
                    self.p.live.set(
                        self.p.p[i + 1].as_ptr(),
                        self.p.p[i + 1].preview_normalized(*v),
                    );
                }
            }
            self.result.store(0, Ordering::Release);
            return true;
        }
        false
    }
}
