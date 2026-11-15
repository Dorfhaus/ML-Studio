//! Local chord extraction using the existing FFT and project chord vocabulary.
use super::*;
use os_engine::Sample;

type ResultList = Result<Vec<(u32, u32, Vec<Chord>)>, String>;

enum Source {
    Audio(String, f64, f64, f64),
    Midi(Vec<(f64, f64, u16)>),
}

pub fn start(app: &mut App, i: usize) -> Option<String> {
    let s = &mut app.state;
    if s.playlist.scoring.analysis.is_some() {
        return Some("Chord extraction is already running.".into());
    }
    let selected = if s.playlist.sel.contains(&i) {
        s.playlist.sel.clone()
    } else {
        vec![i]
    };
    let map = s.project.tempo_map(s.tempo);
    let beat_ticks = f64::from(s.project.beat_ticks().max(1));
    let mut jobs = Vec::new();
    let mut originals = Vec::new();
    for clip in selected
        .into_iter()
        .filter_map(|i| pl::clips(s).get(i))
        .take(64)
    {
        let from = map.seconds(f64::from(clip.position));
        let to = map.seconds(f64::from(clip.end()));
        let source = match clip.source {
            ClipSource::Channel(id) => {
                let Some(path) = s
                    .project
                    .channel(id)
                    .filter(|ch| ch.kind.is_sampler())
                    .and_then(|ch| ch.sample_path.as_deref())
                else {
                    continue;
                };
                let (a, b) = pl::window(s, clip);
                let path = crate::plugins::resolve_sample(path);
                if b <= a || to <= from {
                    continue;
                }
                Source::Audio(path, a / 1000.0, b / 1000.0, to - from)
            }
            ClipSource::Pattern(n) => {
                let Some(pattern) = s.project.pattern(n) else {
                    continue;
                };
                let (a, b) = pl::window(s, clip);
                let notes = pattern
                    .notes
                    .iter()
                    .filter(|n| !n.muted())
                    .filter_map(|n| {
                        let lo = f64::from(n.position).max(a);
                        let hi = (f64::from(n.position) + f64::from(n.length.max(1))).min(b);
                        (hi > lo).then(|| {
                            let time = |t| {
                                map.seconds(
                                    f64::from(clip.position) + (t - a) / clip.pattern_rate(),
                                ) - from
                            };
                            (time(lo), time(hi), n.key)
                        })
                    })
                    .collect();
                Source::Midi(notes)
            }
        };
        jobs.push((clip.position, clip.end(), from, to, source));
        originals.push(clip.clone());
    }
    if jobs.is_empty() {
        return Some("Select audio or pattern clips to extract chords.".into());
    }
    let (tx, rx) = mpsc::sync_channel(1);
    let arrangement = s.project.arrangement_index();
    let previous = data(s).chords.clone();
    s.playlist.scoring.analysis = Some(Analysis {
        arrangement,
        clips: originals,
        previous,
        map: map.clone(),
        rx,
    });
    std::thread::spawn(move || {
        let result: ResultList = jobs
            .into_iter()
            .map(|(lo, hi, from, to, source)| {
                let grid = |local| {
                    let at = from + local;
                    let beat = (map.tick(at) / beat_ticks).round() * beat_ticks;
                    (map.seconds(beat) - at).abs()
                };
                let chords = match source {
                    Source::Audio(path, a, b, duration) => {
                        let path = std::path::Path::new(&path);
                        if std::fs::metadata(path).map_or(true, |m| m.len() > 384 * 1024 * 1024) {
                            return Err(
                                "The audio source is missing or too large to analyze at once."
                                    .into(),
                            );
                        }
                        let sample =
                            os_engine::sample::load_shared(path).map_err(|e| e.to_string())?;
                        let end = b.min(sample.frames() as f64 / f64::from(sample.rate));
                        if end <= a {
                            return Ok((lo, hi, Vec::new()));
                        }
                        audio(&sample, a, end, duration / (b - a), &grid)?
                    }
                    Source::Midi(notes) => midi(&notes, to - from, &grid)?,
                };
                let mut out: Vec<Chord> = chords
                    .into_iter()
                    .map(|(offset, symbol)| Chord {
                        tick: (map.tick(from + offset).round() as u32)
                            .clamp(lo, hi.saturating_sub(1).max(lo)),
                        symbol,
                    })
                    .collect();
                out.dedup_by(|next, previous| {
                    if next.tick == previous.tick {
                        std::mem::swap(&mut next.symbol, &mut previous.symbol);
                        true
                    } else {
                        next.symbol == previous.symbol
                    }
                });
                if out.last().is_some_and(|p| p.symbol != "N.C.") {
                    out.push(Chord {
                        tick: hi,
                        symbol: "N.C.".into(),
                    });
                }
                Ok((lo, hi, out))
            })
            .collect();
        let _ = tx.send(result);
    });
    Some("Extracting chords in the background...".into())
}

pub fn poll(app: &mut App) {
    let Some(job) = &app.state.playlist.scoring.analysis else {
        return;
    };
    let result = match job.rx.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => Err("Chord extraction stopped.".into()),
    };
    let s = &mut app.state;
    let job = s.playlist.scoring.analysis.take().unwrap();
    if s.project.arrangement_index() != job.arrangement
        || job.previous != data(s).chords
        || job.map != s.project.tempo_map(s.tempo)
        || !job.clips.iter().all(|clip| pl::clips(s).contains(clip))
    {
        s.message = Some(
            "The selected clips or chord chart changed. Extract again on the current selection."
                .into(),
        );
    } else {
        match result {
            Ok(changes)
                if changes
                    .iter()
                    .any(|(_, _, list)| list.iter().any(|c| c.symbol != "N.C.")) =>
            {
                update(s, "Extract chords", |timeline| {
                    for (start, end, list) in changes {
                        if list.is_empty() {
                            continue;
                        }
                        timeline.chords.retain(|p| p.tick < start || p.tick >= end);
                        for chord in list {
                            if chord.tick != end || !timeline.chords.iter().any(|p| p.tick == end) {
                                timeline.chords.push(chord);
                            }
                        }
                    }
                });
                Lane::Chords.show(true);
                s.message = Some("Chords added to the Playlist chord chart.".into());
            }
            Ok(_) => {
                s.message = Some("No confident chords were found in the selected clips.".into())
            }
            Err(error) => s.message = Some(error),
        }
    }
    app.invalidate();
}

const NAMES: [&str; 12] = [
    "C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B",
];
const VOCABULARY: [(usize, &str); 15] = [
    (1, ""),
    (5, "m"),
    (2, "sus2"),
    (3, "sus4"),
    (6, "dim"),
    (7, "aug"),
    (10, "6"),
    (13, "m6"),
    (15, "7"),
    (27, "maj7"),
    (32, "m7"),
    (33, "m7b5"),
    (42, "add9"),
    (52, "madd9"),
    (40, "9"),
];

const PITCH_CLASSES: usize = 12;
const NO_CHORD: usize = PITCH_CLASSES * VOCABULARY.len();
const STATES: usize = NO_CHORD + 1;

// Each observation carries the evidence for *all* chord labels. Tracking just the
// winning label at each hop loses useful evidence when two chords are close.
struct Observation {
    time: f64,
    chroma: [f32; 12],
    bass: Option<usize>,
    quiet: bool,
}

struct Template {
    classes: [bool; 12],
    root: usize,
    quality: usize,
    notes: usize,
}

fn emission(obs: &Observation, template: &Template, precise: bool) -> f32 {
    if obs.quiet {
        return -3.0;
    }
    let threshold = if precise { 0.20 } else { 0.27 };
    let mut error = 0.0;
    for (pc, &weight) in obs.chroma.iter().enumerate() {
        if template.classes[pc] {
            error += (threshold - weight).max(0.0) * 1.25;
        } else {
            // Passing pitches are useful evidence, but one bright note should
            // not make a seventh or ninth more likely than a sustained triad.
            error += weight.min(0.70) * 0.43;
        }
    }
    let extensions = template.notes.saturating_sub(3) as f32;
    let bass = if obs.bass == Some(template.root) {
        0.12
    } else {
        0.0
    };
    let root = obs.chroma[template.root] * 0.12;
    let extension_cost = if precise { 0.18 } else { 0.24 };
    (0.64 - error - extensions * extension_cost + bass + root).clamp(-2.0, 1.0)
}

fn observation(time: f64, chroma: [f32; 12], bass: Option<usize>) -> Observation {
    let max = chroma.iter().copied().fold(0.0, f32::max);
    Observation {
        time,
        chroma: chroma.map(|x| if max > 1e-8 { x / max } else { 0.0 }),
        bass,
        quiet: max <= 1e-8,
    }
}

fn track(
    frames: &[Observation],
    duration: f64,
    precise: bool,
    distance_to_beat: &impl Fn(f64) -> f64,
) -> Vec<(f64, String)> {
    let templates: Vec<_> = (0..12)
        .flat_map(|root| {
            VOCABULARY
                .iter()
                .enumerate()
                .map(move |(quality, &(entry, _))| {
                    let classes = os_model::chords::classes(entry, root as u8);
                    Template {
                        classes,
                        root,
                        quality,
                        notes: classes.iter().filter(|&&on| on).count(),
                    }
                })
        })
        .collect();

    let mut scores = [0.0_f32; STATES];
    let mut back = Vec::with_capacity(frames.len());
    for (i, obs) in frames.iter().enumerate() {
        let dt = frames.get(i + 1).map_or(duration, |f| f.time) - obs.time;
        let weight = (dt / 0.128).clamp(0.0, 40.0) as f32;
        let mut best_chord = 0;
        for state in 1..NO_CHORD {
            if scores[state] > scores[best_chord] {
                best_chord = state;
            }
        }
        // Use the project beat grid as a preference only. Strong off-grid
        // changes can still land where the performance actually changes.
        let beat = distance_to_beat(obs.time);
        let switch =
            if precise { 0.88 } else { 1.25 } + (beat / 0.075).clamp(0.0, 1.0) as f32 * 0.32;
        let mut previous = [0_u8; STATES];
        let mut next = [0.0; STATES];
        let pitched = obs.chroma.iter().filter(|&&v| v >= 0.17).count();
        for state in 0..STATES {
            let mut stay = scores[state];
            let mut parent = state;
            // Best previous chord plus a cost for changing labels. An actual
            // silence is easier to enter or leave than an ambiguous note.
            let silence_cost = switch * 0.75;
            let chord_cost = if obs.quiet || state == NO_CHORD {
                silence_cost
            } else {
                switch
            };
            let change_chord = scores[best_chord] - chord_cost;
            let from_silence = scores[NO_CHORD] - silence_cost;
            if change_chord > stay {
                stay = change_chord;
                parent = best_chord;
            }
            if from_silence > stay {
                stay = from_silence;
                parent = NO_CHORD;
            }
            let evidence = if state == NO_CHORD {
                if obs.quiet {
                    1.0
                } else if pitched < 3 {
                    0.18
                } else {
                    -0.16
                }
            } else {
                let template = &templates[state];
                let count = template
                    .classes
                    .iter()
                    .zip(obs.chroma)
                    .filter(|&(on, v)| *on && v >= 0.17)
                    .count();
                let third_or_suspension = if template.quality == 1 {
                    3
                } else if template.quality == 2 {
                    2
                } else if template.quality == 3 {
                    5
                } else {
                    4
                };
                let needs_third = matches!(template.quality, 0..=3)
                    && obs.chroma[(template.root + third_or_suspension) % 12] < 0.12;
                let penalty = if count < 3 || needs_third { 0.42 } else { 0.0 };
                emission(obs, template, precise) - penalty
            };
            next[state] = stay + evidence * weight;
            previous[state] = parent as u8;
        }
        let max = next.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        scores = next.map(|v| v - max);
        back.push(previous);
    }
    let mut state = scores.iter().position(|&v| v == 0.0).unwrap_or(NO_CHORD);
    let mut path = vec![NO_CHORD; frames.len()];
    for (i, prev) in back.iter().enumerate().rev() {
        path[i] = state;
        state = usize::from(prev[state]);
    }

    let mut out = Vec::new();
    let mut start = 0;
    while start < path.len() {
        let state = path[start];
        let end = path[start..]
            .iter()
            .position(|&v| v != state)
            .map_or(path.len(), |n| start + n);
        let symbol = if state == NO_CHORD {
            "N.C.".into()
        } else {
            let t = &templates[state];
            let mut symbol = format!("{}{}", NAMES[t.root], VOCABULARY[t.quality].1);
            // Retain a clearly sustained inversion. An alternating or unclear
            // bass line does not split a chord into additional chart entries.
            let mut bass_time = [0.0; 12];
            let till = frames.get(end).map_or(duration, |f| f.time);
            for i in start..end {
                let dt = frames.get(i + 1).map_or(duration, |f| f.time) - frames[i].time;
                if let Some(b) = frames[i].bass {
                    bass_time[b] += dt;
                }
            }
            if let Some((bass, _)) = bass_time.into_iter().enumerate().find(|&(p, secs)| {
                p != t.root
                    && t.classes[p]
                    && secs >= 0.30
                    && secs >= (till - frames[start].time) * 0.80
            }) {
                symbol.push('/');
                symbol.push_str(NAMES[bass]);
            }
            symbol
        };
        out.push((frames[start].time, symbol));
        start = end;
    }
    out
}

fn midi(
    notes: &[(f64, f64, u16)],
    duration: f64,
    grid: &impl Fn(f64) -> f64,
) -> Result<Vec<(f64, String)>, String> {
    if notes.len() > 40_000 {
        return Err("Analyze a smaller MIDI region.".into());
    }
    let mut events = Vec::with_capacity(notes.len() * 2 + 1);
    events.push((0.0, 0u16, 0i32));
    for &(a, b, key) in notes {
        if a < duration && b > a && key < 128 {
            events.extend([(a, key, 1), (b.min(duration), key, -1)]);
        }
    }
    events.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut keys = [0i32; 128];
    let mut frames = Vec::new();
    let mut cursor = 0;
    while cursor < events.len() {
        let t = events[cursor].0;
        while cursor < events.len() && (events[cursor].0 - t).abs() < 1e-6 {
            let (_, key, change) = events[cursor];
            keys[usize::from(key)] += change;
            cursor += 1;
        }
        if t >= duration {
            break;
        }
        let mut chroma = [0.0; 12];
        for (key, &count) in keys.iter().enumerate().filter(|&(_, n)| *n > 0) {
            chroma[key % 12] += (count as f32).sqrt();
        }
        frames.push(observation(
            t,
            chroma,
            keys.iter().position(|&n| n > 0).map(|v| v % 12),
        ));
    }
    Ok(track(&frames, duration, true, grid))
}

fn audio(
    sample: &Sample,
    from: f64,
    to: f64,
    time_ratio: f64,
    grid: &impl Fn(f64) -> f64,
) -> Result<Vec<(f64, String)>, String> {
    // Resample by averaging native frames before the FFT, preserving both stereo channels.
    const OUT: f64 = 12_000.0;
    const N: usize = 4096;
    const HOP: usize = 1536;
    let duration = to - from;
    if duration > 1800.0 {
        return Err("Split audio longer than 30 minutes before extracting chords.".into());
    }
    let mut left = vec![0.0; N];
    let mut right = vec![0.0; N];
    let mut imaginary = vec![0.0; N];
    let window: Vec<f32> = (0..N)
        .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / N as f32).cos())
        .collect();
    let mut frames = Vec::new();
    let total = (duration * OUT).ceil() as usize;
    let scale = f64::from(sample.rate) / OUT;
    for center in (0..total).step_by(HOP) {
        let mut energy = 0.0;
        for i in 0..N {
            let local = center as isize + i as isize - N as isize / 2;
            if !(0..total as isize).contains(&local) {
                left[i] = 0.0;
                right[i] = 0.0;
                continue;
            }
            let f = from * f64::from(sample.rate) + local as f64 * scale;
            let a = (f as usize).min(sample.frames());
            let b = (((f + scale) as usize).max(a + 1)).min(sample.frames());
            let width = (b - a).max(1) as f32;
            left[i] = sample.left[a..b].iter().sum::<f32>() / width * window[i];
            right[i] = sample.right[a..b].iter().sum::<f32>() / width * window[i];
            energy += left[i].powi(2) + right[i].powi(2);
        }
        let time = center as f64 / OUT * time_ratio;
        if energy < 0.0002 {
            frames.push(observation(time, [0.0; 12], None));
            continue;
        }
        imaginary.fill(0.0);
        lemon_parametric_eq_2::analyzer::fft(&mut left, &mut imaginary);
        let power: Vec<f32> = left
            .iter()
            .zip(&imaginary)
            .take(550)
            .map(|(r, i)| r * r + i * i)
            .collect();
        imaginary.fill(0.0);
        lemon_parametric_eq_2::analyzer::fft(&mut right, &mut imaginary);
        let magnitudes: Vec<f32> = power
            .into_iter()
            .zip(right.iter().zip(&imaginary))
            .map(|(p, (r, i))| (p + r * r + i * i).sqrt())
            .collect();
        let mut chroma = [0.0; 12];
        let mut bass = [0.0; 12];
        let max = magnitudes.iter().copied().fold(0.0, f32::max);
        for bin in 14..magnitudes.len() - 1 {
            let (l, m, r) = (magnitudes[bin - 1], magnitudes[bin], magnitudes[bin + 1]);
            if m < max * 0.075 || m < l || m <= r {
                continue;
            }
            let delta = (0.5 * (l - r) / (l - 2.0 * m + r)).clamp(-0.5, 0.5);
            let hz = (bin as f32 + delta) * OUT as f32 / N as f32;
            let note = 69.0 + 12.0 * (hz / 440.0).log2();
            if (note - note.round()).abs() > 0.4 {
                continue;
            }
            let pc = (note.round() as i32).rem_euclid(12) as usize;
            chroma[pc] += m / (hz / 220.0).max(1.0).sqrt();
            if hz < 330.0 {
                bass[pc] += m / hz;
            }
        }
        let low = bass
            .into_iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .filter(|p| p.1 > 0.0)
            .map(|p| p.0);
        frames.push(observation(time, chroma, low));
    }
    Ok(track(&frames, duration * time_ratio, false, grid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_sustained_harmony_through_passing_notes_and_marks_silence() {
        let mut frames = Vec::new();
        let mut add = |t, notes: &[usize], bass| {
            let mut chroma = [0.0; 12];
            for &pitch in notes {
                chroma[pitch] = 1.0;
            }
            frames.push(observation(t, chroma, bass));
        };
        add(0.0, &[0, 4, 7], Some(4));
        add(0.60, &[0, 4, 7, 11], Some(4));
        add(0.64, &[0, 4, 7], Some(4));
        add(0.85, &[0, 4, 7, 9], Some(4));
        add(0.89, &[0, 4, 7], Some(4));
        add(1.00, &[7, 10, 2], Some(7));
        add(2.00, &[], None);
        assert_eq!(
            track(&frames, 3.0, true, &|_| 0.0),
            vec![
                (0.0, "C/E".into()),
                (1.0, "Gm".into()),
                (2.0, "N.C.".into())
            ]
        );
    }

    #[test]
    fn extracts_c_major_then_g_minor_from_stereo_harmony() {
        let rate = 24_000;
        let mut data = Vec::new();
        for keys in [[48, 52, 55], [55, 58, 62]] {
            for n in 0..rate {
                data.push(
                    keys.into_iter()
                        .map(|key| {
                            let hz = 440.0 * 2.0_f32.powf((key as f32 - 69.0) / 12.0);
                            (std::f32::consts::TAU * hz * n as f32 / rate as f32).sin() * 0.2
                        })
                        .sum::<f32>(),
                );
            }
        }
        let sample = Sample {
            rate,
            left: data.clone(),
            right: data.into_iter().map(|x| -x).collect(),
        };
        let chords = audio(&sample, 0.0, 2.0, 1.0, &|_| 0.0).unwrap();
        assert!(
            chords.iter().any(|(t, ch)| *t < 0.25 && ch == "C"),
            "{chords:?}"
        );
        assert!(
            chords
                .iter()
                .any(|(t, ch)| (0.85..1.35).contains(t) && ch == "Gm"),
            "{chords:?}"
        );
    }
}
