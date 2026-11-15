// SPDX-License-Identifier: GPL-3.0-or-later

/// Four-point Catmull-Rom interpolation between the two middle samples.
#[inline]
pub(super) fn cubic(a: f32, b: f32, c: f32, d: f32, f: f32) -> f32 {
    b + 0.5 * f * ((c - a) + f * ((2.0 * a - 5.0 * b + 4.0 * c - d) +
        f * (3.0 * (b - c) + d - a)))
}
