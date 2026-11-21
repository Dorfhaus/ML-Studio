//! Stereo output pairs captured for consecutive mixer inserts, or folded into the main insert.
#[derive(Default)]
pub struct InstrumentOutputs {
    pub split: bool,
    pairs: Vec<Vec<[f32; 2]>>,
    frames: usize,
}
impl InstrumentOutputs {
    pub fn configure(&mut self, buses: usize, max_frames: usize) {
        self.pairs = (0..buses.min(15)).map(|_| vec![[0.; 2]; max_frames]).collect();
        self.frames = 0;
    }
    pub fn count(&self) -> usize {
        if self.split { self.pairs.len() } else { 0 }
    }
    pub fn begin(&mut self, frames: usize) {
        self.frames = frames;
        if self.split {
            for pair in &mut self.pairs {
                pair.resize(frames, [0.; 2]);
                pair.fill([0.; 2]);
            }
        }
    }
    pub fn route(&mut self, port: usize, offset: usize, src: (&[f32], &[f32]), main: (&mut [f32], &mut [f32])) -> bool {
        let mut sounding = false;
        if let Some(pair) = self.pairs.get_mut(port).filter(|_| self.split) {
            for (dst, (&l, &r)) in pair[offset..].iter_mut().zip(src.0.iter().zip(src.1)) {
                *dst = [l, r];
                sounding |= l.abs().max(r.abs()) > 1e-5;
            }
        } else {
            for ((l, r), (&a, &b)) in main.0.iter_mut().zip(main.1).zip(src.0.iter().zip(src.1)) {
                *l += a;
                *r += b;
                sounding |= a.abs().max(b.abs()) > 1e-5;
            }
        }
        sounding
    }
    pub fn read(&self, port: usize, l: &mut [f32], r: &mut [f32]) -> bool {
        if port >= self.count() || self.frames != l.len() || r.len() != l.len() {
            return false;
        }
        for ((l, r), s) in l.iter_mut().zip(r).zip(&self.pairs[port]) {
            *l = s[0];
            *r = s[1];
        }
        true
    }
}
