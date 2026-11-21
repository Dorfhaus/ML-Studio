use super::*;
use signalsmith_stretch::Stretch;
pub struct PitchShift;
impl Tool for PitchShift {
    type Dsp = Pitch;
    const SPEC: &'static Spec = &specs::PITCH;
    const ID: &'static str = "com.mlstudio.pitch-shift";
    const CLASS: [u8; 16] = *b"MLStudioPitch001";
    fn paint(c: &mut Canvas, d: &Readings) {
        readout(c, 430., 112., "HOST LATENCY", d.get(0), "ms");
        c.text(
            Font::new(11),
            16.,
            185.,
            Color::rgb(0xaaaaaa),
            "Stereo-linked pitch shift. Dry/wet stays latency-aligned.",
        );
    }
}
pub struct Pitch {
    stretch: Stretch,
    data: Arc<Readings>,
    rate: u32,
    delay: Vec<[f32; 2]>,
    pos: usize,
    latency: usize,
    mix: f32,
    gain: f32,
    shifted: f32,
    smooth: f32,
    phase: f64,
    last: [i32; 5],
}
impl Processor for Pitch {
    fn new(rate: u32, data: Arc<Readings>) -> Self {
        let stretch = Stretch::preset_default(2, rate);
        let latency = stretch.input_latency() + stretch.output_latency();
        Self {
            stretch,
            data,
            rate,
            delay: vec![[0.; 2]; latency.max(1)],
            pos: 0,
            latency,
            mix: 1.,
            gain: 1.,
            shifted: 0.,
            smooth: 1. - (-1. / (rate as f32 * 0.01)).exp(),
            phase:0.,last: [i32::MIN; 5],
        }
    }
    fn latency(&self) -> u32 {
        self.latency as u32
    }
    fn reset(&mut self, p: &FxParams) {
        self.stretch.reset();
        self.delay.fill([0.; 2]);
        self.pos = 0;
        self.mix = p.v(3) as f32 / 1000.;
        self.gain = 10f32.powf(p.v(4) as f32 / 200.);
        self.shifted = if p.v(0) != 0 || p.v(1) != 0 { 1. } else { 0. };
        self.phase=0.; self.last = [i32::MIN; 5];
        self.data
            .set(0, self.latency as f32 * 1000. / self.rate as f32);
    }
    fn process(&mut self, l: &mut [f32], r: &mut [f32], p: &FxParams) {
        let settings = [p.v(0), p.v(1), p.v(2),p.v(5),p.v(6)];
        if settings != self.last {
            self.stretch.set_transpose_factor_semitones(
                settings[0] as f32 + settings[1] as f32 / 100.,
                None,
            );
            self.stretch.set_formant_factor_semitones(settings[3] as f32/10., settings[2] != 0);
            self.last = settings;
        }
        let shifted = if settings[0] != 0 || settings[1] != 0 || settings[3]!=0 || settings[4]!=0 {
            1.
        } else {
            0.
        };
        let mix = p.v(3) as f32 / 1000.;
        let gain = 10f32.powf(p.v(4) as f32 / 200.);
        let mut input = [0f32; 512];
        let mut output = [0f32; 512];
        for (ll, rr) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            if settings[4]>0 {
                let modulation=self.phase.sin() as f32*settings[4] as f32/100.;
                self.stretch.set_transpose_factor_semitones(settings[0] as f32+settings[1] as f32/100.+modulation,None);
                self.phase=(self.phase+std::f64::consts::TAU*p.v(7) as f64*0.1*ll.len() as f64/self.rate as f64)%std::f64::consts::TAU;
            }
            for i in 0..ll.len() {
                input[2 * i] = ll[i];
                input[2 * i + 1] = rr[i];
            }
            self.stretch
                .process(&input[..ll.len() * 2], &mut output[..ll.len() * 2]);
            for i in 0..ll.len() {
                let dry = self.delay[self.pos];
                self.delay[self.pos] = [ll[i], rr[i]];
                self.pos = (self.pos + 1) % self.delay.len();
                self.mix += (mix - self.mix) * self.smooth;
                self.gain += (gain - self.gain) * self.smooth;
                self.shifted += (shifted - self.shifted) * self.smooth;
                let wet = self.mix * self.shifted;
                ll[i] = (dry[0] + (output[2 * i] - dry[0]) * wet) * self.gain;
                rr[i] = (dry[1] + (output[2 * i + 1] - dry[1]) * wet) * self.gain;
            }
        }
    }
}
