use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::{io::Read, path::Path};
pub const MAX_FILE: usize = 16 * 1024 * 1024;
#[derive(Clone)]
pub struct Impulse {
    pub name: String,
    pub rate: u32,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}
#[derive(Serialize, Deserialize)]
struct Saved {
    name: String,
    rate: u32,
    audio: String,
}
impl Impulse {
    pub fn validate(&self) -> Result<(), String> {
        if !(8000..=192000).contains(&self.rate)
            || self.name.len() > 256
            || self.left.is_empty()
            || self.left.len() != self.right.len()
            || self.left.len() > self.rate as usize * 10
            || self
                .left
                .iter()
                .chain(&self.right)
                .any(|v| !v.is_finite() || v.abs() > 16.)
        {
            return Err("Invalid impulse response".into());
        }
        Ok(())
    }
    pub fn import(p: &Path) -> Result<Self, String> {
        let mut b = Vec::new();
        std::fs::File::open(p)
            .map_err(|e| e.to_string())?
            .take((MAX_FILE + 1) as u64)
            .read_to_end(&mut b)
            .map_err(|e| e.to_string())?;
        Self::decode(
            &b,
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .chars()
                .take(100)
                .collect(),
        )
    }
    pub fn save(&self) -> String {
        let mut b = Vec::with_capacity(self.left.len() * 8);
        for (&l, &r) in self.left.iter().zip(&self.right) {
            b.extend(l.to_le_bytes());
            b.extend(r.to_le_bytes());
        }
        serde_json::to_string(&Saved {
            name: self.name.clone(),
            rate: self.rate,
            audio: STANDARD.encode(b),
        })
        .unwrap()
    }
    pub fn load(s: &str) -> Result<Self, String> {
        if s.len() > 24 * 1024 * 1024 {
            return Err("IR state too large".into());
        }
        let v: Saved = serde_json::from_str(s).map_err(|e| e.to_string())?;
        let bytes = STANDARD.decode(v.audio).map_err(|e| e.to_string())?;
        if bytes.len() % 8 != 0 {
            return Err("Incomplete IR state".into());
        }
        let mut ir = Self {
            name: v.name,
            rate: v.rate,
            left: Vec::new(),
            right: Vec::new(),
        };
        for f in bytes.chunks_exact(8) {
            ir.left.push(f32::from_le_bytes(f[..4].try_into().unwrap()));
            ir.right
                .push(f32::from_le_bytes(f[4..].try_into().unwrap()));
        }
        ir.validate()?;
        Ok(ir)
    }
    pub fn room() -> Self {
        let rate = 48000;
        let mut left = vec![0.; 57600];
        let mut right = left.clone();
        let mut seed = 17u32;
        for i in 600..left.len() {
            for out in [&mut left, &mut right] {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                out[i] =
                    seed as i32 as f32 / i32::MAX as f32 * 0.012 * (-7. * i as f32 / 57600.).exp();
            }
        }
        for (i, a) in [(700, 0.35), (1350, 0.25), (2100, 0.2)] {
            left[i] += a;
            right[i + 137] += a;
        }
        Self {
            name: "ML Small Room".into(),
            rate,
            left,
            right,
        }
    }
    pub fn at_rate(&self, rate: u32) -> [Vec<f32>; 2] {
        if rate == self.rate {
            return [self.left.clone(), self.right.clone()];
        }
        let ratio = self.rate as f64 / rate as f64;
        let cutoff = (1. / ratio).min(1.) * 0.95;
        let n = (self.left.len() as f64 / ratio).ceil() as usize;
        std::array::from_fn(|ch| {
            let src = if ch == 0 { &self.left } else { &self.right };
            (0..n)
                .map(|i| {
                    let pos = i as f64 * ratio;
                    let center = pos.floor() as isize;
                    let mut sum = 0.;
                    for k in -48..=48 {
                        let idx = center + k;
                        let x = pos - idx as f64;
                        if x.abs() > 48. {
                            continue;
                        }
                        let h = if x.abs() < 1e-10 {
                            cutoff
                        } else {
                            (std::f64::consts::PI * x * cutoff).sin() / (std::f64::consts::PI * x)
                        };
                        let w = 0.5 + 0.5 * (std::f64::consts::PI * x / 48.).cos();
                        if idx >= 0 && (idx as usize) < src.len() {
                            sum += src[idx as usize] as f64 * h * w;
                        }
                    }
                    (sum * ratio) as f32
                })
                .collect()
        })
    }
    pub fn decode(bytes: &[u8], name: String) -> Result<Self, String> {
        let err = |s: &str| Err(s.to_string());
        if bytes.len() > MAX_FILE {
            return err("WAV exceeds 16 MB");
        }
        if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return err("Choose a RIFF WAV file (PCM or 32-bit float)");
        }
        let u16_at = |b: &[u8], i| u16::from_le_bytes(b[i..i + 2].try_into().unwrap());
        let u32_at = |b: &[u8], i| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
        let end = u32_at(bytes, 4) as usize + 8;
        if end > bytes.len() || end < 12 {
            return err("Truncated WAV header");
        }
        let (mut fmt, mut data) = (None, None);
        let mut p = 12;
        while p + 8 <= end {
            let len = u32_at(bytes, p + 4) as usize;
            let stop = p
                .checked_add(8)
                .and_then(|p| p.checked_add(len))
                .ok_or("WAV chunk overflow")?;
            if stop > end {
                return err("Truncated WAV chunk");
            }
            match &bytes[p..p + 4] {
                b"fmt " => fmt = Some(&bytes[p + 8..stop]),
                b"data" => data = Some(&bytes[p + 8..stop]),
                _ => {}
            }
            p = stop + len % 2;
        }
        let fmt = fmt.ok_or("WAV has no format chunk")?;
        let data = data.ok_or("WAV has no audio chunk")?;
        if fmt.len() < 16 {
            return err("Truncated WAV format");
        }
        let (mut kind, channels, align, bits) = (
            u16_at(fmt, 0),
            u16_at(fmt, 2) as usize,
            u16_at(fmt, 12) as usize,
            u16_at(fmt, 14),
        );
        if kind == 0xfffe {
            if fmt.len() < 40
                || u16_at(fmt, 16) < 22
                || u16_at(fmt, 18) != bits
                || fmt[26..40] != [0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113]
            {
                return err("Unsupported extensible WAV format");
            }
            kind = u16_at(fmt, 24);
        }
        if !(channels == 1 || channels == 2)
            || !((kind == 1 && [8, 16, 24, 32].contains(&bits)) || (kind == 3 && bits == 32))
            || align != channels * (bits as usize / 8)
            || u32_at(fmt, 4) == 0
        {
            return err("Use mono/stereo PCM 8/16/24/32-bit or float32 WAV");
        }
        if data.len() % align != 0 {
            return err("Incomplete WAV sample");
        }

        let mut left = Vec::with_capacity(data.len() / align);
        let mut right = Vec::with_capacity(data.len() / align);
        let rate = u32_at(fmt, 4);
        if !(8000..=192000).contains(&rate) || data.len() / align > rate as usize * 10 {
            return err("IR must be at most 10 seconds, 8–192 kHz");
        }
        for sample in data.chunks_exact(align) {
            let mut stereo = [0.; 2];
            for (channel, b) in sample.chunks_exact(bits as usize / 8).enumerate() {
                let value = match (kind, bits) {
                    (3, 32) => f32::from_le_bytes(b.try_into().unwrap()) as f64,
                    (_, 8) => (b[0] as f64 - 128.) / 128.,
                    (_, 16) => i16::from_le_bytes(b.try_into().unwrap()) as f64 / 32768.,
                    (_, 24) => {
                        ((i32::from_le_bytes([b[0], b[1], b[2], 0]) << 8) >> 8) as f64 / 8388608.
                    }
                    (_, 32) => i32::from_le_bytes(b.try_into().unwrap()) as f64 / 2147483648.,
                    _ => unreachable!(),
                };
                if !value.is_finite() || value.abs() > 16. {
                    return err("WAV contains invalid or excessive sample values");
                }
                stereo[channel] = value as f32;
            }
            left.push(stereo[0]);
            right.push(if channels == 1 { stereo[0] } else { stereo[1] });
        }

        let source = Self {
            name,
            rate,
            left,
            right,
        };
        source.validate()?;
        Ok(source)
    }
}
