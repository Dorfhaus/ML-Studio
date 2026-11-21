// Filter-bank/envelope algorithm adapted from mda Vocoder by Paul Kellett / Shane Dunne (MIT).
// ML: explicit stereo carrier sidechain, internal band-limited carrier, sample-rate-scaled envelopes,
// Nyquist guards, smoothed output and transition fades; no upstream VST shell or callback mutex.
use crate::specs::VOCODER;
use lemon_small_fx::{
    Shared,
    editor::{Extension, FxEditor},
    params::FxParams,
};
use nih_plug::prelude::*;
use os_ui::Canvas;
use std::{
    num::NonZeroU32,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};
#[derive(Clone)]
pub(crate) struct Bank {
    f: [[f64; 13]; 16],
    bands: usize,
    k: usize,
    held: f64,
    gain: f64,
    thru: f64,
    high: f64,
}
impl Default for Bank {
    fn default() -> Self {
        Self {
            f: [[0.; 13]; 16],
            bands: 16,
            k: 0,
            held: 0.,
            gain: 0.,
            thru: 0.,
            high: 0.,
        }
    }
}
impl Bank {
    pub fn configure(&mut self, rate: f64, params: [f64; 5], bands: usize) {
        let [response, q, shift, sibilance, level] = params;
        self.bands = bands;
        self.gain = 10f64.powf(2. * level - 3. * q - 2.);
        let thru = 10f64.powf(0.5 + 2. * level);
        self.thru = sibilance.powi(3) * thru;
        self.high = 0.4f64.powi(3) * thru;
        let frequencies: &[f64] = if bands == 8 {
            &[0., 3000., 2200., 1500., 1080., 700., 390., 190.]
        } else {
            &[
                0., 5000., 4000., 3250., 2750., 2300., 2000., 1750., 1500., 1250., 1000., 750.,
                540., 350., 195., 95.,
            ]
        };
        let envelope = 10f64.powf(-1.7 - 2.7 * response);
        self.f[0][12] = 1. - (1. - 0.5 * envelope).powf(44100. / rate);
        let radius = 1. - 10f64.powf(-1. - 1.2 * q);
        let shift = 2f64.powf(3. * shift - 1.);
        for (i, &freq) in frequencies.iter().enumerate().skip(1) {
            let speed = envelope.min(0.025 - 0.022 / bands as f64 * i as f64);
            self.f[i][12] = 1. - (1. - speed).powf(44100. / rate);
            let hz = (freq * shift).min(rate * 0.23);
            let w = std::f64::consts::TAU / rate;
            self.f[i][0] = 4. * radius * radius * (w * hz).cos() / (1. + radius * radius);
            self.f[i][1] = -radius * radius;
            self.f[i][2] = 4. * radius * radius * (w * hz * 0.96).cos() / (1. + radius * radius);
        }
    }
    pub fn clear(&mut self) {
        for f in &mut self.f {
            f[3..12].fill(0.);
        }
        self.k = 0;
        self.held = 0.;
    }
    pub fn sample(&mut self, modulator: f64, carrier: f64) -> f32 {
        let f = &mut self.f;
        let a = modulator - f[0][7];
        f[0][7] = modulator;
        f[0][11] -= f[0][12] * (f[0][11] - a.abs());
        let mut out = f[0][11] * (self.thru * a + self.high * (carrier - f[0][3]));
        f[0][3] = carrier;
        self.k ^= 1;
        if self.k != 0 {
            self.held = 0.;
            let aa = a + f[0][9] - 2. * f[0][8];
            f[0][9] = f[0][8];
            f[0][8] = a;
            let bb = carrier + f[0][5] - 2. * f[0][4];
            f[0][5] = f[0][4];
            f[0][4] = carrier;
            for f in f.iter_mut().take(self.bands).skip(1) {
                let mut t = f[0] * f[3] + f[1] * f[4] + bb;
                f[4] = f[3];
                f[3] = t;
                t += f[2] * f[5] + f[1] * f[6];
                f[6] = f[5];
                f[5] = t;
                let mut t = f[0] * f[7] + f[1] * f[8] + aa;
                f[8] = f[7];
                f[7] = t;
                t += f[2] * f[9] + f[1] * f[10];
                f[10] = f[9];
                f[9] = t;
                f[11] -= f[12] * (f[11] - t.abs());
                self.held += f[5] * f[11];
            }
        }
        out += self.held * self.gain;
        if out.is_finite() {
            out.clamp(-1., 1.) as f32
        } else {
            self.clear();
            0.
        }
    }
}
pub(crate) struct Readings {
    main: AtomicU32,
    carrier: AtomicU32,
}
pub(crate) struct Panel {
    pub(crate) readings: Arc<Readings>,
    pub(crate) params: Arc<FxParams>,
}
impl Extension for Panel {
    fn paint(&self, c: &mut Canvas, _: &dyn lemon_small_fx::view::Model) {
        if !os_plugkit::visibility::stock_dsp_controls() {
            return;
        }
        let main = f32::from_bits(self.readings.main.load(Ordering::Relaxed));
        let carrier = f32::from_bits(self.readings.carrier.load(Ordering::Relaxed));
        crate::text(
            c,
            16.,
            328.,
            "1. Put ML Vocoder on the voice/modulator mixer track.",
        );
        crate::text(
            c,
            16.,
            350.,
            "2. Internal: choose a note. Sidechain: route your synth to this effect.",
        );
        let status = if main < 1e-5 {
            "Waiting for voice / modulator input"
        } else if self.params.v(0) == 1 && carrier < 1e-5 {
            "Voice detected — waiting for synth in sidechain"
        } else {
            "Voice and carrier detected"
        };
        crate::text(c, 16., 379., status);
        crate::text(
            c,
            16.,
            402.,
            "Mix blends the original voice. Sibilance restores consonants.",
        );
    }
    fn click(&self, _: f32, _: f32, _: Option<&Arc<dyn GuiContext>>) -> bool {
        false
    }
}
pub struct Vocoder {
    pub(crate) params: Arc<FxParams>,
    banks: [Bank; 2],
    previous: [Bank; 2],
    previous_source: i32,
    bank_blend: f32,
    rate: f32,
    phase: f64,
    smoothed: [f32; 3],
    last: [i32; 9],
    fade: f32,
    pub(crate) readings: Arc<Readings>,
}
impl Default for Vocoder {
    fn default() -> Self {
        Self {
            params: Arc::new(FxParams::new(&VOCODER)),
            banks: Default::default(), previous: Default::default(), previous_source: 0, bank_blend: 1.,
            rate: 48000.,
            phase: 0.,
            smoothed: [1., 1., 130.8128],
            last: [i32::MIN; 9],
            fade: 0.,
            readings: Arc::new(Readings {
                main: AtomicU32::new(0),
                carrier: AtomicU32::new(0),
            }),
        }
    }
}
fn blep(t: f64, dt: f64) -> f64 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.
    } else if t > 1. - dt {
        let x = (t - 1.) / dt;
        x * x + x + x + 1.
    } else {
        0.
    }
}
impl Vocoder {
    pub(crate) fn prepare(&mut self, rate: f32) {
        self.rate = rate;
        self.reset();
    }
    fn update(&mut self) {
        let p: [i32; 9] = std::array::from_fn(|i| self.params.v(i));
        if p != self.last {
            let switching = p[0] != self.last[0] || p[2..7] != self.last[2..7];
            if switching && self.last[0] >= 0 {
                // Finish the current transition before adopting another filter bank.
                if self.bank_blend < 1. { return; }
                self.previous.clone_from(&self.banks);
                self.previous_source = self.last[0];
                self.bank_blend = 0.;
            }
            if p[0] != self.last[0] || p[2] != self.last[2] {
                for b in &mut self.banks {
                    b.clear();
                }
                self.fade = 0.;
            }
            for b in &mut self.banks {
                b.configure(
                    self.rate as f64,
                    [
                        p[3] as f64 / 1000.,
                        p[4] as f64 / 1000.,
                        p[5] as f64 / 1000.,
                        p[6] as f64 / 1000.,
                        0.5,
                    ],
                    if p[2] == 0 { 8 } else { 16 },
                );
            }
            self.last = p;
        }
    }
    pub(crate) fn audio(
        &mut self,
        l: &mut [f32],
        r: &mut [f32],
        carrier: Option<(&[f32], &[f32])>,
    ) {
        self.update();
        let target = [
            self.last[7] as f32 / 1000.,
            10f32.powf(self.last[8] as f32 / 200.),
            440. * 2f32.powf((self.last[1] as f32 - 69.) / 12.),
        ];
        let smooth = 1. - (-1. / (0.005 * self.rate)).exp();
        let mut mp = 0f32;
        let mut cp = 0f32;
        for i in 0..l.len() {
            let input = [
                if l[i].is_finite() { l[i] } else { 0. },
                if r[i].is_finite() { r[i] } else { 0. },
            ];
            let m = (input[0] + input[1]) * 0.5;
            mp = mp.max(m.abs());
            for k in 0..3 {
                self.smoothed[k] += smooth * (target[k] - self.smoothed[k]);
            }
            let step = (self.smoothed[2] / self.rate).min(0.45) as f64;
            let saw = ((2. * self.phase - 1.) - blep(self.phase, step)) as f32 * 0.3;
            self.phase = (self.phase + step).fract();
            self.fade = (self.fade + 1. / (0.01 * self.rate)).min(1.);
            for ch in 0..2 {
                let c = if self.last[0] == 0 {
                    saw
                } else {
                    carrier.map_or(0., |(a, b)| {
                        if ch == 0 { a.get(i) } else { b.get(i) }
                            .copied()
                            .unwrap_or(0.)
                    })
                };
                let c = if c.is_finite() { c } else { 0. };
                cp = cp.max(c.abs());
                let mut wet = self.banks[ch].sample(m as f64, c as f64) * self.fade;
                if self.bank_blend < 1. {
                    let old_carrier = if self.previous_source == self.last[0] { c }
                        else if self.previous_source == 0 { saw }
                        else { carrier.and_then(|(a,b)| if ch == 0 { a.get(i) } else { b.get(i) }).copied().filter(|x| x.is_finite()).unwrap_or(0.) };
                    let old = self.previous[ch].sample(m as f64, old_carrier as f64);
                    wet = old + self.bank_blend * (wet - old);
                }
                let v = (input[ch] * (1. - self.smoothed[0]) + wet * self.smoothed[0])
                    * self.smoothed[1];
                if ch == 0 { l[i] = v } else { r[i] = v }
            }
            self.bank_blend = (self.bank_blend + 1. / (0.005 * self.rate)).min(1.);
        }
        self.readings.main.store(mp.to_bits(), Ordering::Relaxed);
        self.readings.carrier.store(cp.to_bits(), Ordering::Relaxed);
    }
}
impl Plugin for Vocoder {
    const NAME: &'static str = "ML Vocoder";
    const VENDOR: &'static str = "ML Studio";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        aux_input_ports: &[new_nonzero_u32(2)],
        ..AudioIOLayout::const_default()
    }];
    type SysExMessage = ();
    type BackgroundTask = ();
    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    fn editor(&mut self, _: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let mut s = Shared::default();
        s.extension = Some(Arc::new(Panel {
            params: self.params.clone(),
            readings: self.readings.clone(),
        }));
        Some(Box::new(FxEditor::new(
            &VOCODER,
            self.params.clone(),
            Arc::new(s),
        )))
    }
    fn initialize(
        &mut self,
        _: &AudioIOLayout,
        c: &BufferConfig,
        ctx: &mut impl InitContext<Self>,
    ) -> bool {
        if !(8000.0..=192000.0).contains(&c.sample_rate) {
            return false;
        }
        ctx.set_latency_samples(0);
        self.prepare(c.sample_rate);
        true
    }
    fn reset(&mut self) {
        for b in &mut self.banks {
            b.clear();
        }
        self.phase = 0.;
        self.fade = 0.;
        self.bank_blend = 1.;
        self.last = [i32::MIN; 9];
        self.smoothed = [
            self.params.v(7) as f32 / 1000.,
            10f32.powf(self.params.v(8) as f32 / 200.),
            440. * 2f32.powf((self.params.v(1) as f32 - 69.) / 12.),
        ];
        self.readings.main.store(0, Ordering::Relaxed);
        self.readings.carrier.store(0, Ordering::Relaxed);
    }
    fn process(
        &mut self,
        b: &mut Buffer,
        a: &mut AuxiliaryBuffers,
        _: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let carrier = a.inputs.first_mut().map(|b| b.as_slice());
        let carrier = carrier.as_ref().and_then(|b| {
            if b.len() >= 2 {
                Some((&b[0][..], &b[1][..]))
            } else {
                None
            }
        });
        let c = b.as_slice();
        if c.len() >= 2 {
            let (l, r) = c.split_at_mut(1);
            self.audio(l[0], r[0], carrier);
        }
        ProcessStatus::KeepAlive
    }
}
impl ClapPlugin for Vocoder {
    const CLAP_ID: &'static str = "com.mlstudio.vocoder";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("8/16-band vocoder with stereo carrier sidechain");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}
impl Vst3Plugin for Vocoder {
    const VST3_CLASS_ID: [u8; 16] = *b"MLVocoder0000001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}
