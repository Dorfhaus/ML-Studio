// SPDX-License-Identifier: GPL-3.0-or-later

use os_engine::effects::eq::HalfBand;

pub(crate) const OVERSAMPLING: [&str; 3] = ["1x", "2x", "4x"];

#[derive(Clone, Default)]
pub(crate) struct Oversampler {
    stages: usize,
    up: [[HalfBand; 2]; 2],
    down: [[HalfBand; 2]; 2],
}

impl Oversampler {
    pub fn set(&mut self, value: i32) {
        let stages = value.clamp(0, 2) as usize;
        if self.stages != stages {
            self.stages = stages;
            self.reset();
        }
    }

    pub fn reset(&mut self) {
        self.up = Default::default();
        self.down = Default::default();
    }

    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], shape: impl Fn(f32) -> f32) {
        for (channel, samples) in [left, right].into_iter().enumerate() {
            match self.stages {
                0 => samples.iter_mut().for_each(|x| *x = shape(*x)),
                1 => {
                    let (up, down) = (&mut self.up[channel][0], &mut self.down[channel][0]);
                    for x in samples {
                        let [a, b] = up.up(*x);
                        *x = down.down(shape(a), shape(b));
                    }
                }
                _ => {
                    let [up, up_high] = &mut self.up[channel];
                    let [down, down_high] = &mut self.down[channel];
                    for x in samples {
                        let [a, b] = up.up(*x).map(|v| {
                            let [a, b] = up_high.up(v);
                            down_high.down(shape(a), shape(b))
                        });
                        *x = down.down(a, b);
                    }
                }
            }
        }
        if self.stages > 0 {
            for channel in 0..2 {
                for stage in 0..self.stages {
                    self.up[channel][stage].tidy();
                    self.down[channel][stage].tidy();
                }
            }
        }
    }
}

/// The old and new waveshaping paths see the same input while a control changes. No lookahead or heap work in process().
pub(crate) struct ControlTransition<T, const N: usize> {
    active: T,
    previous: T,
    applied: [i32; N],
    target: [i32; N],
    length: usize,
    remaining: usize,
}

impl<T: super::Sound + Clone + 'static, const N: usize> ControlTransition<T, N> {
    pub fn boxed(active: T, values: &[i32], rate: u32) -> Box<dyn super::Sound> {
        let target = std::array::from_fn(|i| values.get(i).copied().unwrap_or_default());
        Box::new(Self { previous: active.clone(), active, applied: target, target,
            length: (rate / 250).max(1) as usize, remaining: 0 })
    }
}

impl<T: super::Sound + Clone, const N: usize> super::Sound for ControlTransition<T, N> {
    fn set(&mut self, values: &[i32]) {
        self.target = std::array::from_fn(|i| values.get(i).copied().unwrap_or_default());
    }

    fn set_sample_rate(&mut self, rate: u32) {
        self.length = (rate / 250).max(1) as usize;
        self.active.set_sample_rate(rate);
        self.previous.set_sample_rate(rate);
        self.reset();
    }

    fn reset(&mut self) {
        self.active.set(&self.target);
        self.applied = self.target;
        self.active.reset();
        self.remaining = 0;
    }

    fn process(&mut self, mut left: &mut [f32], mut right: &mut [f32]) {
        while !left.is_empty() {
            if self.remaining == 0 {
                if self.target == self.applied {
                    self.active.process(left, right);
                    return;
                }
                self.previous.clone_from(&self.active);
                self.active.set(&self.target);
                self.applied = self.target;
                self.remaining = self.length;
            }
            let n = self.remaining.min(64).min(left.len());
            let mut old_l = [0.; 64];
            let mut old_r = [0.; 64];
            let (l, next_l) = left.split_at_mut(n);
            let (r, next_r) = right.split_at_mut(n);
            old_l[..n].copy_from_slice(l);
            old_r[..n].copy_from_slice(r);
            self.previous.process(&mut old_l[..n], &mut old_r[..n]);
            self.active.process(l, r);
            for i in 0..n {
                let old = self.remaining as f32 / self.length as f32;
                l[i] += (old_l[i] - l[i]) * old;
                r[i] += (old_r[i] - r[i]) * old;
                self.remaining -= 1;
            }
            left = next_l;
            right = next_r;
        }
    }
}
