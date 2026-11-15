//! Musical ticks and absolute media time share one monotonic tempo map.
use serde::{Deserialize, Serialize};

pub const MIN_BPM: f64 = 10.0;
pub const MAX_BPM: f64 = 522.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TempoPoint {
    pub tick: u32,
    pub bpm: f64,
    /// Interpolate BPM in musical time from this point to the next.
    pub ramp: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ramp_end: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chord {
    pub tick: u32,
    pub symbol: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Video {
    pub path: String,
    pub start: f64,
    pub duration: f64,
    pub fps: f64,
    #[serde(default)]
    pub source_in: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_duration: Option<f64>,
}

impl Video {
    pub fn end(&self) -> f64 {
        self.start + self.duration
    }

    pub fn source_length(&self) -> f64 {
        self.source_duration.unwrap_or(self.duration)
    }

    pub fn source_time(&self, seconds: f64) -> Option<f64> {
        (seconds >= self.start && seconds < self.end())
            .then(|| self.source_in + seconds - self.start)
    }
}

fn deserialize_videos<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Video>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum SavedVideo {
        Clips(Vec<Video>),
        Legacy(Video),
    }
    Ok(match Option::<SavedVideo>::deserialize(d)? {
        Some(SavedVideo::Clips(clips)) => clips,
        Some(SavedVideo::Legacy(clip)) => vec![clip],
        None => Vec::new(),
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualTemplate {
    #[default]
    Cover,
    Spectrum,
    Pulse,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisualScene {
    pub tick: u32,
    pub end: u32,
    pub template: VisualTemplate,
    pub title: String,
    pub artist: String,
    pub cover: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub automation_id: u16,
    #[serde(default, skip_serializing_if = "VisualStyle::is_default")]
    pub style: VisualStyle,
}

fn is_zero_u16(value: &u16) -> bool {
    *value == 0
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VisualStyle {
    pub fade_in: u32,
    pub fade_out: u32,
    pub crossfade: bool,
    pub values: [f32; 3],
}

impl Default for VisualStyle {
    fn default() -> Self {
        Self {
            fade_in: 0,
            fade_out: 0,
            crossfade: false,
            values: [1., 0.5, 1.],
        }
    }
}

impl VisualStyle {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Timeline {
    pub tempo: Vec<TempoPoint>,
    pub chords: Vec<Chord>,
    #[serde(
        alias = "video",
        deserialize_with = "deserialize_videos",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub videos: Vec<Video>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub visuals: Vec<VisualScene>,
}

impl Timeline {
    pub fn rescale(&mut self, from: u16, to: u16) {
        for point in &mut self.tempo {
            point.tick = crate::rescale_tick(point.tick, from, to);
        }
        for chord in &mut self.chords {
            chord.tick = crate::rescale_tick(chord.tick, from, to);
        }
        for scene in &mut self.visuals {
            scene.tick = crate::rescale_tick(scene.tick, from, to);
            scene.end = crate::rescale_tick(scene.end, from, to).max(scene.tick.saturating_add(1));
            scene.style.fade_in = crate::rescale_tick(scene.style.fade_in, from, to);
            scene.style.fade_out = crate::rescale_tick(scene.style.fade_out, from, to);
        }
        self.normalize();
    }

    pub fn normalize(&mut self) {
        self.tempo.retain(|p| (MIN_BPM..=MAX_BPM).contains(&p.bpm));
        for point in &mut self.tempo {
            point.ramp_end = point
                .ramp_end
                .filter(|bpm| (MIN_BPM..=MAX_BPM).contains(bpm));
        }
        self.tempo.sort_by_key(|p| p.tick);
        self.tempo.dedup_by_key(|p| p.tick);
        self.chords.retain(|p| !p.symbol.trim().is_empty());
        self.chords.sort_by_key(|p| p.tick);
        self.chords.dedup_by_key(|p| p.tick);
        self.visuals.retain(|scene| scene.tick < scene.end);
        self.visuals.sort_by_key(|scene| scene.tick);
        self.visuals.dedup_by_key(|scene| scene.tick);
        // A later scene cuts the previous one. An empty section remains available between clips.
        for i in 1..self.visuals.len() {
            self.visuals[i - 1].end = self.visuals[i - 1].end.min(self.visuals[i].tick);
        }
        for scene in &mut self.visuals {
            let length = scene.end - scene.tick;
            scene.style.fade_in = scene.style.fade_in.min(length);
            scene.style.fade_out = scene.style.fade_out.min(length);
            for (value, default) in scene
                .style
                .values
                .iter_mut()
                .zip(VisualStyle::default().values)
            {
                *value = if value.is_finite() {
                    value.clamp(0., 1.)
                } else {
                    default
                };
            }
        }
        self.videos.retain(|v| {
            !v.path.is_empty()
                && (0.0..=86400.0).contains(&v.start)
                && (0.001..=86400.0).contains(&v.duration)
                && (1.0..=240.0).contains(&v.fps)
                && (0.001..=86400.0).contains(&v.source_length())
                && (0.0..=v.source_length() + 1e-9 - v.duration).contains(&v.source_in)
        });
        self.videos.sort_by(|a, b| a.start.total_cmp(&b.start));
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Segment {
    point: TempoPoint,
    seconds: f64,
    slope: f64,
}

/// Segment start times are precomputed when the project changes, including for the render engine.
/// Linear BPM ramps integrate in logarithmic time. The inverse uses the same curve.
#[derive(Clone, Debug, PartialEq)]
pub struct TempoMap {
    scale: f64,
    segments: Vec<Segment>,
}

impl TempoMap {
    pub fn new(ppq: u16, steps: u16, base: f64, points: &[TempoPoint]) -> Self {
        let scale = 240.0 / (f64::from(ppq.max(1)) * f64::from(steps.max(1)));
        let mut segments = Vec::with_capacity(points.len() + 1);
        let mut point = TempoPoint {
            tick: 0,
            bpm: if base.is_finite() {
                base.max(1.0)
            } else {
                130.0
            },
            ramp: false,
            ramp_end: None,
        };
        let mut seconds = 0.0;
        for &next in points {
            if !(MIN_BPM..=MAX_BPM).contains(&next.bpm) || next.tick < point.tick {
                continue;
            }
            if next.tick > point.tick {
                let ticks = f64::from(next.tick - point.tick);
                let slope = if point.ramp {
                    (point.ramp_end.unwrap_or(next.bpm) - point.bpm) / ticks
                } else {
                    0.0
                };
                let segment = Segment {
                    point,
                    seconds,
                    slope,
                };
                seconds += span(scale, segment, ticks);
                segments.push(segment);
            }
            point = next;
        }
        segments.push(Segment {
            point,
            seconds,
            slope: 0.0,
        });
        Self { scale, segments }
    }

    fn segment(&self, tick: f64) -> Segment {
        self.segments[self
            .segments
            .partition_point(|s| f64::from(s.point.tick) <= tick)
            .saturating_sub(1)]
    }

    pub fn bpm(&self, tick: f64) -> f64 {
        let s = self.segment(tick);
        s.point.bpm + s.slope * (tick - f64::from(s.point.tick)).max(0.0)
    }

    pub fn seconds(&self, tick: f64) -> f64 {
        let s = self.segment(tick);
        s.seconds + span(self.scale, s, (tick - f64::from(s.point.tick)).max(0.0))
    }

    pub fn tick(&self, seconds: f64) -> f64 {
        let s = self.segments[self
            .segments
            .partition_point(|s| s.seconds <= seconds)
            .saturating_sub(1)];
        let time = (seconds - s.seconds).max(0.0) / self.scale;
        let ticks = if s.slope.abs() < 1e-12 {
            time * s.point.bpm
        } else {
            (time * s.slope).exp_m1() * s.point.bpm / s.slope
        };
        f64::from(s.point.tick) + ticks
    }

    pub fn frame(&self, tick: u64, rate: u32) -> u64 {
        (self.seconds(tick as f64) * f64::from(rate)).round_ties_even() as u64
    }

    /// Fit the interval from the preceding tempo point to a selected beat, keeping all earlier
    /// cue intervals unchanged. The selected beat becomes the next anchor.
    pub fn warp(&self, points: &[TempoPoint], tick: u32, seconds: f64) -> Option<Vec<TempoPoint>> {
        if tick == 0 || !seconds.is_finite() {
            return None;
        }
        let previous = self.segment(f64::from(tick) - 0.5);
        let dt = seconds - previous.seconds;
        if dt <= 0.0 {
            return None;
        }
        let from = previous.point.tick;
        let bpm = self.scale * f64::from(tick - from) / dt;
        if !(MIN_BPM..=MAX_BPM).contains(&bpm) {
            return None;
        }
        let at = self.segment(f64::from(tick));
        let after = TempoPoint {
            tick,
            bpm: self.bpm(f64::from(tick)),
            ramp: at.slope != 0.0,
            ramp_end: at.point.ramp_end,
        };
        let mut out = points.to_vec();
        // Preserve the earlier ramp's arrival tempo before the new step at the anchor.
        if let Some(prior) = out
            .iter_mut()
            .rev()
            .find(|p| p.tick < from)
            .filter(|p| p.ramp)
        {
            prior.ramp_end = Some(prior.ramp_end.unwrap_or(previous.point.bpm));
        }
        out.retain(|p| p.tick != from && p.tick != tick);
        out.extend([
            TempoPoint {
                tick: from,
                bpm,
                ramp: false,
                ramp_end: None,
            },
            after,
        ]);
        out.sort_by_key(|p| p.tick);
        Some(out)
    }
}

fn span(scale: f64, s: Segment, ticks: f64) -> f64 {
    if s.slope.abs() < 1e-12 {
        scale * ticks / s.point.bpm
    } else {
        scale * (s.slope * ticks / s.point.bpm).ln_1p() / s.slope
    }
}
