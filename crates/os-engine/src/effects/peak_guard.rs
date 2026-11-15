// SPDX-License-Identifier: GPL-3.0-or-later

const TAPS: usize = 24;
const PHASES: usize = 3;
pub(super) const LATENCY: usize = TAPS - 1;

/// Looks ahead using a four-phase peak estimate (the original sample and
/// three interpolated positions). Gain follows the larger stereo peak.
pub(super) struct PeakGuard {
    fir: [[f32; TAPS]; PHASES],
    history: [[f32; 2]; TAPS * 2],
    cursor: usize,
    minimum_gain: f32,
    hold: usize,
    release: f32,
}

impl PeakGuard {
    pub fn new(rate: f64) -> Self {
        let fir = std::array::from_fn::<_, PHASES, _>(|phase| {
            let center = (TAPS / 2 - 1) as f64 + (phase + 1) as f64 / 4.0;
            let mut taps = std::array::from_fn::<_, TAPS, _>(|k| {
                let x = k as f64 - center;
                let w = std::f64::consts::PI * x / (TAPS as f64 * 0.5);
                let window = 0.42 + 0.5 * w.cos() + 0.08 * (2.0 * w).cos();
                let y = std::f64::consts::PI * x;
                (window * y.sin() / y) as f32
            });
            let sum = taps.iter().sum::<f32>();
            for tap in &mut taps {
                *tap /= sum;
            }
            taps
        });
        let mut guard = Self {
            fir, history: [[0.0; 2]; TAPS * 2], cursor: 0, minimum_gain: 1.0, hold: 0, release: 0.0,
        };
        guard.set_rate(rate);
        guard
    }

    pub fn set_rate(&mut self, rate: f64) {
        self.release = (-1.0 / (rate.max(1.0) * 0.02)).exp() as f32;
        self.clear();
    }

    pub fn clear(&mut self) {
        self.history.fill([0.0; 2]);
        self.cursor = 0;
        self.minimum_gain = 1.0;
        self.hold = 0;
    }

    #[inline]
    pub fn frame(&mut self, input: [f32; 2], ceiling: f32) -> [f32; 2] {
        let input = input.map(|x| if x.is_finite() { x } else { 0.0 });
        self.history[self.cursor] = input;
        self.history[self.cursor + TAPS] = input;
        self.cursor = (self.cursor + 1) % TAPS;
        let window = &self.history[self.cursor..self.cursor + TAPS];
        let mut peak = input[0].abs().max(input[1].abs());
        for taps in &self.fir {
            let [l, r] = window.iter().zip(taps).fold([0.0f32; 2], |[l, r], ([x, y], h)| {
                [x.mul_add(*h, l), y.mul_add(*h, r)]
            });
            peak = peak.max(l.abs()).max(r.abs());
        }
        // Reserve a small margin for interpolation error and gain transitions.
        let available = ceiling.max(0.0) * 0.98;
        let gain = if peak > available { available / peak } else { 1.0 };
        if gain <= self.minimum_gain {
            self.minimum_gain = gain;
            self.hold = TAPS;
        } else if self.hold > 0 {
            self.hold -= 1;
        } else {
            self.minimum_gain = (1.0 + (self.minimum_gain - 1.0) * self.release).min(gain);
        }
        window[0].map(|x| x * self.minimum_gain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_inter_sample_sine_peaks_with_matched_latency() {
        let mut guard = PeakGuard::new(48000.0);
        for i in 0..2500 {
            // A quarter-rate sine at 45 degrees has equal-magnitude stored
            // samples and reconstructed peaks 3 dB above those samples.
            let input = if i % 4 < 2 { 0.9 } else { -0.9 };
            let [l, r] = guard.frame([input, -input], 0.99);
            assert_eq!(l, -r);
            if i < LATENCY {
                assert_eq!(l, 0.0);
            } else if i > 100 {
                assert!(l.abs() * std::f32::consts::SQRT_2 <= 0.99);
                assert!(l.abs() > 0.6);
            }
        }
    }
}
