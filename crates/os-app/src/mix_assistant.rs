// SPDX-License-Identifier: GPL-3.0-or-later
//! Offline, editable balance and stock processing of the live Mixer graph.
mod genre;
mod processing;
mod quality;
mod reference;
mod setup;
use crate::{
    app::App,
    dialog::{Answer, Body, Modal, Reply},
    history::{self, Snap},
    mixer,
};
use ebur128::{EbuR128, Mode};
use genre::Genre;
use os_engine::{Engine, Sample, laws::fader_gain};
use os_model::{
    MixerTrack, Project,
    target::{Target, decode},
};
use processing::{Addition, Fx};
pub(crate) use reference::startup_profile_count;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

pub(crate) const VISIBILITY: &str = "visible_mixer-mix-assistant";
const HEADROOM: f64 = -6.0;
const MAX_MOVE: f64 = 6.0;
const EXCERPT_SECONDS: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Unknown,
    General,
    LeadVocal,
    BackingVocal,
    Kick,
    Drums,
    Bass,
    Lead,
    Music,
    Pad,
    Bus,
    DrumBus,
    VocalBus,
    InstrumentBus,
    BassBus,
    MixBus,
    Return,
}

impl Role {
    const ALL: [Self; 16] = [
        Self::General,
        Self::LeadVocal,
        Self::BackingVocal,
        Self::Kick,
        Self::Drums,
        Self::Bass,
        Self::Lead,
        Self::Music,
        Self::Pad,
        Self::Return,
        Self::Bus,
        Self::DrumBus,
        Self::VocalBus,
        Self::InstrumentBus,
        Self::BassBus,
        Self::MixBus,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Unknown => "Choose role",
            Self::General => "General track",
            Self::LeadVocal => "Lead vocal",
            Self::BackingVocal => "Backing vocals",
            Self::Kick => "Kick",
            Self::Drums => "Drums",
            Self::Bass => "Bass",
            Self::Lead => "Lead",
            Self::Music => "Instrument",
            Self::Pad => "Pad",
            Self::Bus => "General bus",
            Self::DrumBus => "Drum bus",
            Self::VocalBus => "Vocal bus",
            Self::InstrumentBus => "Instrument bus",
            Self::BassBus => "Bass bus",
            Self::MixBus => "Mix bus",
            Self::Return => "FX return",
        }
    }

    fn is_bus(self) -> bool {
        matches!(
            self,
            Self::Bus
                | Self::DrumBus
                | Self::VocalBus
                | Self::InstrumentBus
                | Self::BassBus
                | Self::MixBus
        )
    }

    /// Starting differences in active loudness, relative to a lead.
    fn target(self) -> f64 {
        match self {
            Self::LeadVocal | Self::Lead | Self::VocalBus => 0.0,
            Self::Drums | Self::DrumBus => -2.0,
            Self::Bass
            | Self::Kick
            | Self::Music
            | Self::Bus
            | Self::BassBus
            | Self::InstrumentBus
            | Self::MixBus => -4.0,
            Self::Unknown | Self::General => -5.0,
            Self::BackingVocal => -6.0,
            Self::Pad => -8.0,
            Self::Return => -12.0,
        }
    }

    fn guess(name: &str, receives_audio: bool) -> Self {
        // Split camel case as well as separators, e.g. VocalMix, VOX_BUS and BackMid.
        let mut separated = String::with_capacity(name.len());
        let mut chars = name.chars().peekable();
        let mut previous: Option<char> = None;
        while let Some(ch) = chars.next() {
            if ch.is_uppercase()
                && (previous.is_some_and(char::is_lowercase)
                    || previous.is_some_and(char::is_uppercase)
                        && chars.peek().is_some_and(|ch| ch.is_lowercase()))
            {
                separated.push(' ');
            }
            separated.extend(ch.to_lowercase());
            previous = Some(ch);
        }
        let words: Vec<_> = separated.split(|c: char| !c.is_alphabetic()).collect();
        let has = |tokens: &[&str]| words.iter().any(|word| tokens.contains(word));
        let vocal = has(&[
            "vocal", "vocals", "vox", "voice", "acapella", "gesang", "stimme",
        ]);
        let drums = has(&[
            "drum",
            "drums",
            "drm",
            "snare",
            "clap",
            "hihat",
            "hat",
            "hats",
            "oh",
            "overheads",
            "tom",
            "toms",
            "percussion",
            "perc",
            "cymbals",
            "ride",
            "shaker",
        ]);
        let bass = has(&["bass", "sub", "subbass"]) || separated.trim().starts_with("808");
        let backing = has(&["bgv", "bvox", "adlib", "adlibs", "choir", "backmid"])
            || (vocal
                && has(&[
                    "back",
                    "backing",
                    "harmony",
                    "harmonies",
                    "double",
                    "doubles",
                ]))
            || (has(&["back"]) && has(&["mid", "left", "right"]));
        let instrument = has(&[
            "instrument",
            "instruments",
            "inst",
            "guitar",
            "guitars",
            "git",
            "piano",
            "keys",
            "organ",
            "strings",
            "brass",
            "violins",
            "violin",
            "rhodes",
            "synth",
            "flute",
            "chords",
            "sax",
        ]);
        if has(&["return", "parallel"])
            || (has(&["fx", "effects", "aux", "reverb", "verb", "delay"]) && receives_audio)
        {
            return Self::Return;
        }
        if receives_audio
            || has(&["bus", "buss", "group", "submix", "premaster"])
            || has(&["mix"]) && (vocal || drums || bass || instrument)
        {
            return if vocal || backing {
                Self::VocalBus
            } else if drums {
                Self::DrumBus
            } else if bass {
                Self::BassBus
            } else if instrument {
                Self::InstrumentBus
            } else if has(&["mix", "mixbus", "premaster", "master", "stereo", "sum"]) {
                Self::MixBus
            } else {
                Self::Bus
            };
        }
        if backing {
            Self::BackingVocal
        } else if vocal {
            Self::LeadVocal
        } else if has(&["kick", "kicks", "bd"]) {
            Self::Kick
        } else if drums {
            Self::Drums
        } else if bass {
            Self::Bass
        } else if has(&["pad", "pads", "texture", "atmosphere"]) {
            Self::Pad
        } else if has(&["lead", "solo", "melody"]) {
            Self::Lead
        } else if instrument {
            Self::Music
        } else if has(&["reverb", "verb", "delay"]) {
            Self::Return
        } else {
            Self::Unknown
        }
    }
}

struct Meter {
    observer: quality::Observer,
    meter: EbuR128,
    window: usize,
    count: usize,
    values: Vec<f32>,
    peak: f32,
    low: [f64; 2],
    low_alpha: f64,
    low_energy: f64,
    sub: [f64; 2],
    treble: [f64; 2],
    sub_alpha: f64,
    treble_alpha: f64,
    sub_energy: f64,
    high_energy: f64,
    side_energy: f64,
    energy: f64,
}

impl Meter {
    fn new(rate: u32, mode: Mode) -> Result<Self, String> {
        Ok(Self {
            observer: quality::Observer::new(rate),
            meter: EbuR128::new(2, rate, mode | Mode::TRUE_PEAK).map_err(|e| e.to_string())?,
            window: ((rate as usize + 5) / 10) * 8,
            count: 0,
            values: Vec::new(),
            peak: 0.0,
            low: [0.0; 2],
            low_alpha: 1.0 - (-std::f64::consts::TAU * 150.0 / f64::from(rate)).exp(),
            low_energy: 0.0,
            sub: [0.; 2],
            treble: [0.; 2],
            sub_alpha: 1.0 - (-std::f64::consts::TAU * 30.0 / f64::from(rate)).exp(),
            treble_alpha: 1.0 - (-std::f64::consts::TAU * 5000.0 / f64::from(rate)).exp(),
            sub_energy: 0.,
            high_energy: 0.,
            side_energy: 0.,
            energy: 0.0,
        })
    }

    fn feed(&mut self, mut audio: &[f32]) -> Result<(), String> {
        if audio.iter().any(|sample| !sample.is_finite()) {
            return Err(
                "A Mixer track returned invalid audio. Check its effects and try again.".into(),
            );
        }
        self.observer.feed(audio);
        while !audio.is_empty() {
            let used = (self.window - self.count).min(audio.len());
            let (head, tail) = audio.split_at(used);
            for frame in head.chunks_exact(2) {
                self.side_energy += 0.5 * f64::from(frame[0] - frame[1]).powi(2);
                for (ch, (low, &sample)) in self.low.iter_mut().zip(frame).enumerate() {
                    self.peak = self.peak.max(sample.abs());
                    let sample = f64::from(sample);
                    *low += self.low_alpha * (sample - *low);
                    self.low_energy += *low * *low;
                    self.sub[ch] += self.sub_alpha * (sample - self.sub[ch]);
                    self.treble[ch] += self.treble_alpha * (sample - self.treble[ch]);
                    self.sub_energy += self.sub[ch].powi(2);
                    self.high_energy += (sample - self.treble[ch]).powi(2);
                    self.energy += sample * sample;
                }
            }
            self.meter.add_frames_f32(head).map_err(|e| e.to_string())?;
            self.count += used;
            if self.count == self.window {
                self.finish_window();
            }
            audio = tail;
        }
        Ok(())
    }

    fn finish_window(&mut self) {
        self.values
            .push(self.meter.loudness_momentary().unwrap_or(f64::NEG_INFINITY) as f32);
        self.count = 0;
    }

    fn finish(mut self) -> Option<Level> {
        self.observer.flush();
        let _ = self.meter.add_frames_f32(&[0.; 128]);
        let true_peak = db(self
            .meter
            .true_peak(0)
            .unwrap_or(0.)
            .max(self.meter.true_peak(1).unwrap_or(0.)));
        if self.count > 0 {
            self.finish_window();
        }
        let strongest = self
            .values
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        if strongest < -64.0 || self.peak < 0.0006 {
            return None;
        }
        let gate = (strongest - 20.0).max(-66.0);
        let mut active: Vec<_> = self
            .values
            .iter()
            .copied()
            .filter(|value| *value >= gate)
            .collect();
        active.sort_by(f32::total_cmp);
        let typical = f64::from(active[(active.len() - 1) * 2 / 3]);
        Some(Level {
            frames: self.observer.frames,
            true_peak,
            typical,
            windows: self.values,
            gate,
            crest: db(f64::from(self.peak)) - typical,
            low_fraction: self.low_energy / self.energy.max(f64::MIN_POSITIVE),
            sub_fraction: self.sub_energy / self.energy.max(f64::MIN_POSITIVE),
            high_fraction: self.high_energy / self.energy.max(f64::MIN_POSITIVE),
            side_fraction: self.side_energy / self.energy.max(f64::MIN_POSITIVE),
        })
    }
}

#[derive(Clone, Default)]
struct Level {
    frames: Vec<quality::Frame>,
    true_peak: f64,
    typical: f64,
    windows: Vec<f32>,
    gate: f32,
    crest: f64,
    low_fraction: f64,
    sub_fraction: f64,
    high_fraction: f64,
    side_fraction: f64,
}

impl Level {
    fn difference(&self, other: &Self) -> f64 {
        // Give active four-second phrases equal weight so a long chorus cannot
        // dominate a short verse solely because it contains more samples.
        let sections: Vec<_> = self
            .windows
            .chunks(10)
            .zip(other.windows.chunks(10))
            .filter_map(|(a, b)| {
                let values: Vec<_> = a
                    .iter()
                    .zip(b)
                    .filter(|(a, b)| **a >= self.gate && **b >= other.gate)
                    .map(|(a, b)| f64::from(a - b))
                    .collect();
                (values.len() >= 3).then(|| quality::quantile(values.into_iter(), 0.5))
            })
            .collect();
        if sections.len() >= 2 {
            return 0.8 * quality::quantile(sections.into_iter(), 0.5)
                + 0.2 * (self.typical - other.typical);
        }
        let mut overlap: Vec<_> = self
            .windows
            .iter()
            .zip(&other.windows)
            .filter(|(a, b)| **a >= self.gate && **b >= other.gate)
            .map(|(a, b)| f64::from(a - b))
            .collect();
        if overlap.len() < 2 {
            return self.typical - other.typical;
        }
        overlap.sort_by(f64::total_cmp);
        0.7 * overlap[overlap.len() / 2] + 0.3 * (self.typical - other.typical)
    }
}

pub(crate) struct RenderRequest {
    pub span: Option<(u64, u64)>,
    pub tracks: Vec<usize>,
    pub excerpt_at: Option<usize>,
    pub effects: Vec<(usize, usize, os_engine::SharedEffect)>,
}

pub(crate) struct Render {
    output: Option<Level>,
    levels: BTreeMap<u16, Level>,
    peak: f64,
    excerpt_at: usize,
    excerpt: Vec<f32>,
    rate: u32,
}

struct Excerpt {
    wanted: Option<usize>,
    length: usize,
    start: usize,
    current: Vec<f32>,
    score: f64,
    best: Vec<f32>,
    best_at: usize,
}

impl Excerpt {
    fn new(rate: u32, wanted: Option<usize>) -> Self {
        Self {
            wanted,
            length: rate as usize * 2 * EXCERPT_SECONDS,
            start: 0,
            current: Vec::new(),
            score: -1.0,
            best: Vec::new(),
            best_at: 0,
        }
    }

    fn feed(&mut self, mut offset: usize, mut samples: &[f32]) {
        if let Some(wanted) = self.wanted {
            let start = wanted.saturating_sub(offset).min(samples.len());
            let end = wanted
                .saturating_add(self.length)
                .saturating_sub(offset)
                .min(samples.len());
            if start < end {
                self.best.extend_from_slice(&samples[start..end]);
                self.best_at = wanted;
            }
            return;
        }
        while !samples.is_empty() {
            if self.current.is_empty() {
                self.start = offset;
            }
            let count = (self.length - self.current.len()).min(samples.len());
            self.current.extend_from_slice(&samples[..count]);
            samples = &samples[count..];
            offset += count;
            if self.current.len() == self.length {
                self.complete();
            }
        }
    }

    fn complete(&mut self) {
        if self.current.is_empty() {
            return;
        }
        let energy = self
            .current
            .iter()
            .map(|value| f64::from(*value).powi(2))
            .sum::<f64>();
        let score = energy / self.current.len().max(self.length / 2) as f64;
        if score > self.score {
            self.score = score;
            self.best_at = self.start;
            std::mem::swap(&mut self.best, &mut self.current);
        }
        self.current.clear();
    }
}

fn db(amplitude: f64) -> f64 {
    20.0 * amplitude.log10()
}
fn gain(db: f64) -> f64 {
    10_f64.powf(db / 20.0)
}
fn window(audio: &[f32], first: usize, last: usize) -> &[f32] {
    &audio[first.min(audio.len())..last.min(audio.len())]
}

pub(crate) fn render(
    mut engine: Engine,
    rate: u32,
    request: RenderRequest,
    cancelled: impl Fn() -> bool,
    progress: impl Fn(u32),
) -> Result<Render, String> {
    for (track, slot, effect) in &request.effects {
        if let Ok(mut fx) = effect.lock() {
            fx.reset_fx();
        }
        engine.set_effect(*track, *slot, effect.clone());
    }
    let (before, end) = match request.span {
        Some((start, end)) => {
            // Warm instruments and effects from the start, then measure the selected range.
            let duration = engine.length();
            engine.set_span(0, start.max(1));
            let before = if start == 0 { 0 } else { engine.frames_left() };
            engine.set_span(0, end);
            (before, engine.frames_left().min(duration))
        }
        None => {
            engine.follow_markers();
            (0, engine.frames_left())
        }
    };
    let speed = engine.speed_ratio();
    let end = (end as f64 / speed).round() as usize;
    let before = (before as f64 / speed).round() as usize;
    if end <= before {
        return Err("There is no audio in this range.".into());
    }
    let mut tracks = Vec::with_capacity(request.tracks.len() + 1);
    tracks.push(0);
    tracks.extend(request.tracks.iter().copied());
    engine.set_capture_tracks(Some(tracks));
    engine.set_track_capture(true);
    engine.capture_master_input();
    let mut meters: Vec<_> = request
        .tracks
        .into_iter()
        .map(|track| Ok((track, Meter::new(rate, Mode::M)?)))
        .collect::<Result<_, String>>()?;
    let mut master = EbuR128::new(2, rate, Mode::TRUE_PEAK).map_err(|e| e.to_string())?;
    let mut output = Meter::new(rate, Mode::M)?;
    let mut excerpt = Excerpt::new(rate, request.excerpt_at);
    let mut block = Vec::new();
    let mut cursor = 0usize;
    loop {
        if cancelled() {
            return Err("Mix analysis cancelled.".into());
        }
        block.clear();
        let done = engine.render_ticks(12, &mut block);
        let stems = engine.take_track_audio();
        let frames = block.len() / 2;
        let first = before.saturating_sub(cursor).min(frames) * 2;
        let last = end.saturating_sub(cursor).min(frames) * 2;
        if first < last {
            let audio = &block[first..last];
            if audio.iter().any(|sample| !sample.is_finite()) {
                return Err(
                    "The Master returned invalid audio. Check its effects and try again.".into(),
                );
            }
            excerpt.feed((cursor + first / 2 - before) * 2, audio);
            output.feed(audio)?;
            if let Some(audio) = stems.first() {
                let audio = window(audio, first, last);
                if audio.iter().any(|sample| !sample.is_finite()) {
                    return Err("An insert is sending invalid audio to the Master.".into());
                }
                master.add_frames_f32(audio).map_err(|e| e.to_string())?;
            }
            for (track, meter) in &mut meters {
                if let Some(audio) = stems.get(*track) {
                    meter.feed(window(audio, first, last))?;
                }
            }
        }
        cursor = cursor.saturating_add(frames);
        progress((cursor as u64 * 1000 / end as u64).min(999) as u32);
        if done || cursor >= end {
            break;
        }
    }
    // Flush the true-peak interpolator so a transient at the cut is included.
    master
        .add_frames_f32(&[0.0; 128])
        .map_err(|e| e.to_string())?;
    excerpt.complete();
    let peak = master
        .true_peak(0)
        .unwrap_or(0.0)
        .max(master.true_peak(1).unwrap_or(0.0));
    Ok(Render {
        output: output.finish(),
        levels: meters
            .into_iter()
            .filter_map(|(track, m)| Some((track as u16, m.finish()?)))
            .collect(),
        peak: db(peak),
        excerpt_at: excerpt.best_at,
        excerpt: excerpt.best,
        rate,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Hit {
    GenreMenu,
    SetGenre(Genre),
    Processing,
    Character,
    Space,
    Details(u16),
    ToggleFx(u16, Fx),
    Scope(bool),
    ConfirmRoles,
    Rescan,
    Range(bool),
    Strength(u8),
    Analyze,
    Preview,
    Listen(bool),
    ReviewMoment,
    Stop,
    Include(u16),
    Focus(u16),
    RoleMenu(u16),
    SetRole(u16, Role),
    Page(bool),
    Apply,
    Close,
}

#[derive(Clone)]
struct Track {
    id: u16,
    name: String,
    role: Role,
    level: Level,
    protected: Option<&'static str>,
    include: bool,
    downstream: Vec<u16>,
    route: String,
    confirmed: bool,
    fx_choices: BTreeMap<Fx, bool>,
}

#[derive(Clone, Copy)]
struct Move {
    track: u16,
    value: f64,
    delta: f64,
}

struct Analysis {
    tracks: Vec<Track>,
    baseline: Render,
    proposed: Vec<Move>,
    additions: Vec<Addition>,
    plans: Vec<(u16, Vec<processing::Recipe>)>,
    preview: Option<Compare>,
    notes: Vec<String>,
    rejected: BTreeMap<(u16, Fx), String>,
    review_at: usize,
}

struct Compare {
    samples: [Arc<Sample>; 2],
    peak: f64,
}

pub(crate) struct Form {
    genre: Genre,
    genre_menu: bool,
    references: reference::Library,
    processing: bool,
    character: bool,
    space: bool,
    details: Option<u16>,
    selected: Vec<u16>,
    selection: Option<(u64, u64)>,
    selection_only: bool,
    use_range: bool,
    strength: u8,
    focus: Option<u16>,
    rows: Vec<Track>,
    role_menu: Option<u16>,
    first: usize,
    visible: std::cell::Cell<usize>,
    working: Option<(&'static str, f32)>,
    message: Option<String>,
    audition: Option<bool>,
    analysis: Option<Analysis>,
}

impl Form {
    fn span(&self) -> Option<(u64, u64)> {
        self.selection.filter(|_| self.use_range)
    }

    pub(crate) fn scroll(&mut self, amount: isize) {
        let count = self.rows.len();
        self.first = self
            .first
            .saturating_add_signed(amount)
            .min(count.saturating_sub(self.visible.get().max(1)));
        self.role_menu = None;
        self.genre_menu = false;
    }
}

enum Stage {
    Analyze,
    ProcessSources(Vec<Addition>, bool),
    MeasureBuses(BusPass, Vec<u16>),
    ProcessBuses(BusPass, BTreeMap<u16, Level>, bool),
    Preview(Vec<Move>, Vec<Addition>, u8),
    ReviewBefore(Vec<Move>, Vec<Addition>, usize),
    ReviewAfter(Render),
}

struct BusPass {
    moves: Vec<Move>,
    additions: Vec<Addition>,
    remaining: VecDeque<Vec<u16>>,
}

struct Pending {
    cancel: Arc<AtomicBool>,
    receive: mpsc::Receiver<Result<Render, String>>,
    stage: Stage,
}

struct Source {
    project: Project,
    render: Project,
    plugin_revision: u64,
    tempo: f64,
    rate: u32,
}

#[derive(Default)]
pub(crate) struct Session {
    pending: Option<Pending>,
    scanning: Option<std::time::Instant>,
    setup_project: Option<Project>,
    source: Option<Source>,
    audition_started: Option<std::time::Instant>,
}

impl Session {
    pub(crate) fn busy(&self) -> bool {
        self.pending.is_some() || self.scanning.is_some()
    }
    pub(crate) fn auditioning(&self) -> bool {
        self.audition_started.is_some()
    }
}

fn form(app: &mut App) -> Option<&mut Form> {
    match &mut app.state.dialogs.last_mut()?.body {
        Body::MixAssistant(form) => Some(form),
        _ => None,
    }
}

pub(crate) fn open(app: &mut App) {
    if !crate::workspace_visibility::shown(VISIBILITY) {
        return;
    }
    if app.loading()
        || app.audio_rec.is_some()
        || app.state.rec.recording()
        || app.state.rec.count.is_some()
        || app.state.rec.waiting
    {
        app.state.message =
            Some("Finish the current take or render before opening Mix Assistant.".into());
        return;
    }
    let mut selected = mixer::selected(&app.state);
    selected.retain(|&track| {
        track != 0 && mixer::track(&app.state, track).is_some_and(|track| !track.clip_fx)
    });
    let selection = app
        .state
        .project
        .playlist_loop
        .filter(|(a, b)| a != b)
        .map(|(a, b)| (u64::from(a.min(b)), u64::from(a.max(b))));
    app.state.playlist.popup = None;
    app.show_form(Body::MixAssistant(Box::new(Form {
        genre: Genre::load(),
        genre_menu: false,
        references: reference::Library::load(),
        processing: true,
        character: false,
        space: false,
        details: None,
        selected,
        selection,
        selection_only: false,
        use_range: selection.is_some(),
        strength: 70,
        focus: None,
        rows: Vec::new(),
        role_menu: None,
        first: 0,
        visible: 8.into(),
        working: None,
        message: None,
        audition: None,
        analysis: None,
    })));
    start_scan(app);
}

pub(crate) fn is_open(app: &App) -> bool {
    matches!(
        app.state.dialogs.last().map(|modal| &modal.body),
        Some(Body::MixAssistant(_))
    )
}

pub(crate) fn closed(app: &mut App) {
    if let Some(job) = &app.mix_assistant.pending {
        job.cancel.store(true, Ordering::Release);
    }
    stop_audition(app);
    app.mix_assistant.source = None;
    app.mix_assistant.setup_project = None;
    app.mix_assistant.scanning = None;
}

fn fader_controlled(project: &Project, id: u16) -> bool {
    let target = Target::MixerVolume(id);
    project
        .automations
        .iter()
        .flat_map(|automation| &automation.targets)
        .chain(project.init_controls.iter().map(|control| &control.target))
        .chain(project.peak_links.iter().map(|link| &link.target))
        .any(|value| decode(*value) == target)
}

fn edges(project: &Project) -> BTreeMap<u16, Vec<u16>> {
    let enabled: BTreeSet<_> = project
        .mixer
        .iter()
        .filter(|m| m.enabled)
        .map(|m| m.index)
        .collect();
    let animated: BTreeSet<_> = project
        .automations
        .iter()
        .flat_map(|a| &a.targets)
        .chain(project.init_controls.iter().map(|control| &control.target))
        .chain(project.peak_links.iter().map(|link| &link.target))
        .filter_map(|&value| match decode(value) {
            Target::MixerSend(from, to) => Some((from, to)),
            _ => None,
        })
        .collect();
    project
        .mixer
        .iter()
        .filter(|m| m.enabled)
        .map(|m| {
            let routes = m
                .routes
                .iter()
                .copied()
                .filter(|to| {
                    enabled.contains(to)
                        && (animated.contains(&(m.index, *to))
                            || m.send_levels
                                .iter()
                                .find(|(dest, _)| dest == to)
                                .is_none_or(|&(_, gain)| gain > 0.0))
                })
                .collect();
            (m.index, routes)
        })
        .collect()
}

fn reach_master(edges: &BTreeMap<u16, Vec<u16>>) -> BTreeSet<u16> {
    let mut reached = BTreeSet::from([0]);
    loop {
        let next: Vec<_> = edges
            .iter()
            .filter(|(id, routes)| {
                !reached.contains(id) && routes.iter().any(|to| reached.contains(to))
            })
            .map(|(&id, _)| id)
            .collect();
        if next.is_empty() {
            return reached;
        }
        reached.extend(next);
    }
}

fn start_scan(app: &mut App) {
    stop_audition(app);
    app.mix_assistant.source = None;
    app.mix_assistant.setup_project = None;
    app.mix_assistant.scanning = Some(std::time::Instant::now());
    if let Some(form) = form(app) {
        form.working = Some(("Checking tracks and routing…", -1.0));
        form.analysis = None;
        form.details = None;
        form.role_menu = None;
        form.message = None;
    }
    app.invalidate();
}

fn finish_scan(app: &mut App) {
    let rows = setup::inventory(&app.state, &app.state.project);
    app.mix_assistant.setup_project = Some(app.state.project.clone());
    if let Some(form) = form(app) {
        form.rows = rows;
        form.focus = None;
        form.first = 0;
        form.working = None;
    }
    app.invalidate();
}

fn in_scope(track: &Track, selected: &[u16], selection: bool) -> bool {
    !selection || selected.contains(&track.id)
}

fn included(track: &Track, selected: &[u16], selection: bool) -> bool {
    track.include && track.protected.is_none() && in_scope(track, selected, selection)
}

fn processing_eligible(track: &Track, selected: &[u16], selection: bool) -> bool {
    included(track, selected, selection) && track.confirmed && track.role != Role::Unknown
}

fn ready(form: &Form) -> bool {
    let mut candidates = form
        .rows
        .iter()
        .filter(|track| included(track, &form.selected, form.selection_only))
        .peekable();
    candidates.peek().is_some()
        && candidates.all(|track| track.confirmed && track.role != Role::Unknown)
}

fn fader_change(project: &Project, id: u16, desired: f64) -> Option<Move> {
    let track = project.mixer.iter().find(|track| track.index == id)?;
    let old = fader_gain(track.volume.unwrap_or(0.8));
    if old <= 0.0 {
        return None;
    }
    let amount = old * gain(desired.clamp(-MAX_MOVE, MAX_MOVE));
    let value = (0.8 * (10.0 * amount).ln_1p() / 11_f64.ln()).clamp(0.0, 1.0);
    let delta = db(fader_gain(value) / old);
    (delta.abs() >= 0.09).then_some(Move {
        track: id,
        value,
        delta,
    })
}

fn effective_target(track: &Track, genre: Genre) -> f64 {
    // Peak-heavy material receives a small headroom allowance.
    track.role.target() + genre.balance_offset(track.role)
        - (track.level.crest - 13.0).clamp(0.0, 12.0) * 0.12
}

fn proposals(form: &Form, project: &Project) -> Vec<Move> {
    let Some(analysis) = &form.analysis else {
        return Vec::new();
    };
    let tracks = &analysis.tracks;
    let candidates = tracks.iter().filter(|track| {
        processing_eligible(track, &form.selected, form.selection_only)
            && !track.role.is_bus()
            && track.role != Role::Return
    });
    let anchor = form
        .focus
        .and_then(|id| tracks.iter().find(|track| track.id == id));
    let mut centres: Vec<_> = tracks
        .iter()
        .filter(|track| {
            track.protected != Some("Outside Master mix")
                && !track.role.is_bus()
                && track.role != Role::Return
        })
        .map(|track| track.level.typical - effective_target(track, form.genre))
        .collect();
    centres.sort_by(f64::total_cmp);
    let middle = centres.get(centres.len() / 2).copied();

    candidates
        .filter_map(|track| {
            let target = effective_target(track, form.genre);
            let correction = match (anchor, middle) {
                (Some(focus), _) if focus.id != track.id => {
                    target
                        - effective_target(focus, form.genre)
                        - track.level.difference(&focus.level)
                }
                (Some(_), _) => 0.0,
                (_, Some(middle)) => middle + target - track.level.typical,
                _ => 0.0,
            };
            let max_balance = if matches!(track.role, Role::Unknown | Role::General) {
                3.0
            } else {
                MAX_MOVE
            };
            let mut wanted = correction.clamp(-max_balance, max_balance) * f64::from(form.strength)
                / 100.0
                * form.genre.balance_scale();
            // A mostly quiet track can already have loud syllables. Never infer
            // peak headroom from its typical loudness or compensate compression blindly.
            if wanted > 0. && !track.level.frames.is_empty() {
                wanted = wanted.min((-1. - track.level.true_peak).max(0.));
                if track.level.dynamics() > 8. {
                    wanted = wanted.min(1.5);
                }
            }
            if correction.abs() < 1. {
                return None;
            }
            fader_change(project, track.id, wanted)
        })
        .collect()
}

fn with_moves(project: &Project, moves: &[Move]) -> Project {
    let mut result = project.clone();
    for change in moves {
        if let Some(track) = result
            .mixer
            .iter_mut()
            .find(|track| track.index == change.track)
        {
            track.volume = Some(change.value);
        }
    }
    result
}

fn stale(app: &App) -> bool {
    app.mix_assistant.source.as_ref().is_none_or(|source| {
        source.tempo != app.state.tempo
            || source.plugin_revision != app.plugins.edit_revision
            || source.project != app.state.project
    })
}

fn report(app: &mut App, message: &str) {
    if let Some(form) = form(app) {
        form.message = Some(message.into());
    }
    app.invalidate();
}

fn start_analysis(app: &mut App) {
    if app.mix_assistant.busy() || app.audio_rec.is_some() || app.state.rec.recording() {
        return;
    }
    if app.mix_assistant.setup_project.as_ref() != Some(&app.state.project) {
        return report(app, "The Mixer changed. Rescan tracks before mixing.");
    }
    if !form(app).is_some_and(|form| ready(form)) {
        return report(
            app,
            "Include at least one track and confirm every included role.",
        );
    }
    if let Err(message) = crate::clip_effects::normalize(app) {
        return report(app, &message);
    }
    let project = crate::rack::play_project(&app.state);
    if crate::audio_edits::pending(&project) {
        return report(
            app,
            "Wait for the current audio edits to finish, then analyze again.",
        );
    }
    if let Err(message) = crate::clip_effects::validate(&project) {
        return report(app, &message);
    }
    let Some(form) = form(app) else {
        return;
    };
    let request = RenderRequest {
        span: form.span(),
        tracks: form
            .rows
            .iter()
            .map(|track| usize::from(track.id))
            .collect(),
        excerpt_at: None,
        effects: Vec::new(),
    };
    form.analysis = None;
    form.first = 0;
    form.role_menu = None;
    let rate = crate::transport::Transport::device_rate().unwrap_or(44_100);
    app.mix_assistant.source = Some(Source {
        project: app.state.project.clone(),
        render: project.clone(),
        plugin_revision: app.plugins.edit_revision,
        tempo: app.state.tempo,
        rate,
    });
    queue(app, &project, rate, request, Stage::Analyze);
}

fn queue(app: &mut App, project: &Project, rate: u32, request: RenderRequest, stage: Stage) {
    stop_audition(app);
    crate::browser_preview::dismiss(app);
    app.state.playlist.picker_ui.stop_previews();
    app.plugins.set_offline(true);
    lemon_sampler::stream::set_exporting(true);
    lemon_parametric_eq_2::set_exporting(true);
    let status = match stage {
        Stage::Analyze => "Analyzing Mixer audio",
        Stage::ProcessSources(..) => "Measuring source track processing",
        Stage::MeasureBuses(..) => "Measuring included buses after source changes",
        Stage::ProcessBuses(..) => "Checking bus compression and recovery",
        Stage::Preview(..) => "Checking peaks across the full analysis range",
        Stage::ReviewBefore(..) | Stage::ReviewAfter(..) => "Preparing the review moment",
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let receive = app
        .transport
        .render_mix(project, rate, request, cancel.clone());
    app.mix_assistant.pending = Some(Pending {
        cancel,
        receive,
        stage,
    });
    if let Some(form) = form(app) {
        form.working = Some((status, 0.0));
        form.message = None;
    }
    app.invalidate();
}

fn preview(app: &mut App) {
    if app.state.mixer.all_fx_bypassed && form(app).is_some_and(|f| f.processing) {
        return report(
            app,
            "Mixer effects are globally bypassed. Enable them, or choose Levels only.",
        );
    }
    if stale(app) {
        return report(app, "The project changed. Rescan tracks before mixing.");
    }
    let source = app.mix_assistant.source.as_ref().unwrap();
    let project = source.render.clone();
    let rate = source.rate;
    let Some(form) = form(app) else { return };
    if !ready(form) {
        return report(
            app,
            "Include at least one track and confirm every included role.",
        );
    }
    let Some(analysis) = &mut form.analysis else {
        return;
    };
    for track in &mut analysis.tracks {
        if let Some(row) = form.rows.iter().find(|row| row.id == track.id) {
            let level = track.level.clone();
            *track = row.clone();
            track.level = level;
        }
    }
    analysis.preview = None;
    analysis.proposed.clear();
    analysis.additions.clear();
    analysis.plans.clear();
    analysis.notes.clear();
    analysis.rejected.clear();
    let analysis = form.analysis.as_ref().unwrap();
    let plans: Vec<_> = analysis
        .tracks
        .iter()
        .filter(|t| !t.role.is_bus())
        .map(|t| (t.id, processing::choices(form, &project, t)))
        .collect();
    let request = RenderRequest {
        span: form.span(),
        tracks: analysis
            .tracks
            .iter()
            .filter(|t| !t.role.is_bus())
            .map(|t| usize::from(t.id))
            .collect(),
        excerpt_at: Some(analysis.baseline.excerpt_at),
        effects: Vec::new(),
    };
    form.analysis.as_mut().unwrap().plans = plans.clone();
    let moves = proposals(form, &project);
    let additions = match processing::prepare(app, plans, &project, rate) {
        Ok(a) => a,
        Err(e) => return report(app, &e),
    };
    if additions.is_empty() {
        start_buses(app, moves, additions);
    } else {
        let processed = processing::with_processing(&project, &additions);
        let mut request = request;
        request.effects = preview_effects(&additions);
        queue(
            app,
            &processed,
            rate,
            request,
            Stage::ProcessSources(additions, false),
        );
    }
}

fn preview_effects(additions: &[Addition]) -> Vec<(usize, usize, os_engine::SharedEffect)> {
    additions
        .iter()
        .map(|a| {
            (
                usize::from(a.track),
                usize::from(a.slot),
                a.plugin.clone() as os_engine::SharedEffect,
            )
        })
        .collect()
}

fn start_buses(app: &mut App, moves: Vec<Move>, additions: Vec<Addition>) {
    let Some(form) = form(app) else { return };
    let Some(analysis) = form.analysis.as_ref() else {
        return;
    };
    let remaining = if form.processing {
        match setup::bus_layers(&analysis.tracks, form) {
            Ok(layers) => layers,
            Err(error) => return report(app, error),
        }
    } else {
        VecDeque::new()
    };
    queue_bus_or_final(
        app,
        BusPass {
            moves,
            additions,
            remaining,
        },
    );
}

fn queue_bus_or_final(app: &mut App, mut pass: BusPass) {
    let source = app.mix_assistant.source.as_ref().unwrap();
    let project =
        processing::with_processing(&with_moves(&source.render, &pass.moves), &pass.additions);
    let rate = source.rate;
    let form = form(app).unwrap();
    let excerpt_at = form.analysis.as_ref().unwrap().baseline.excerpt_at;
    let span = form.span();
    let effects = preview_effects(&pass.additions);
    if let Some(layer) = pass.remaining.pop_front() {
        let tracks = layer.iter().copied().map(usize::from).collect();
        queue(
            app,
            &project,
            rate,
            RenderRequest {
                span,
                tracks,
                excerpt_at: Some(excerpt_at),
                effects,
            },
            Stage::MeasureBuses(pass, layer),
        );
    } else if pass.moves.is_empty() && pass.additions.is_empty() {
        let a = form.analysis.as_mut().unwrap();
        a.preview = compare(&a.baseline, &a.baseline);
        report(
            app,
            "No changes needed with these choices. Before and After are identical; Apply is unnecessary.",
        );
    } else {
        let tracks = form
            .analysis
            .as_ref()
            .unwrap()
            .tracks
            .iter()
            .map(|t| usize::from(t.id))
            .collect();
        queue(
            app,
            &project,
            rate,
            RenderRequest {
                span,
                tracks,
                excerpt_at: Some(excerpt_at),
                effects,
            },
            Stage::Preview(pass.moves, pass.additions, 0),
        );
    }
}

fn compare(before: &Render, after: &Render) -> Option<Compare> {
    let len = before.excerpt.len().min(after.excerpt.len());
    if len == 0 {
        return None;
    }
    let measures = |samples: &[f32]| -> Option<(f64, f64)> {
        let mut meter = EbuR128::new(2, before.rate, Mode::I | Mode::TRUE_PEAK).ok()?;
        meter.add_frames_f32(samples).ok()?;
        let lufs = meter
            .loudness_global()
            .ok()
            .filter(|value| value.is_finite())
            .unwrap_or_else(|| {
                10.0 * (samples
                    .iter()
                    .map(|value| f64::from(*value).powi(2))
                    .sum::<f64>()
                    / samples.len() as f64)
                    .log10()
            });
        meter.add_frames_f32(&[0.0; 128]).ok()?;
        let peak = meter.true_peak(0).ok()?.max(meter.true_peak(1).ok()?);
        Some((lufs, db(peak)))
    };
    let (a, b) = (&before.excerpt[..len], &after.excerpt[..len]);
    let (al, ap) = measures(a)?;
    let (bl, bp) = measures(b)?;
    if !al.is_finite() || !bl.is_finite() {
        return None;
    }
    // Match down to the quieter version with enough headroom for the playback resampler.
    let target = al.min(bl).min(al - ap - 1.0).min(bl - bp - 1.0);
    let sample = |data: &[f32], loudness: f64| {
        let gain = gain(target - loudness) as f32;
        let frames = data.len() / 2;
        let (left, right) = data
            .chunks_exact(2)
            .enumerate()
            .map(|(i, frame)| {
                let fade = i.min(frames - 1 - i).min(128) as f32 / 128.0;
                (frame[0] * gain * fade, frame[1] * gain * fade)
            })
            .unzip();
        Arc::new(Sample {
            rate: before.rate,
            left,
            right,
        })
    };
    Some(Compare {
        samples: [sample(a, al), sample(b, bl)],
        peak: after.peak,
    })
}

pub(crate) fn tick(app: &mut App) {
    if let Some(started) = app.mix_assistant.scanning {
        if started.elapsed() >= std::time::Duration::from_millis(40) {
            app.mix_assistant.scanning = None;
            if is_open(app) {
                finish_scan(app);
            }
        }
        return;
    }
    let Some(job) = app.mix_assistant.pending.as_ref() else {
        if let Some(started) = app.mix_assistant.audition_started {
            if !is_open(app)
                || started.elapsed().as_millis() > 400 && !app.transport.preview_status().0
            {
                stop_audition(app);
            }
        }
        return;
    };
    let result = match job.receive.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Disconnected) => Err("The audio renderer stopped.".into()),
        Err(mpsc::TryRecvError::Empty) => {
            let progress = app.transport.offline_progress();
            if let Some(form) = form(app) {
                if let Some((_, shown)) = &mut form.working {
                    if (progress - *shown).abs() >= 0.009 {
                        *shown = progress;
                        app.invalidate();
                    }
                }
            }
            return;
        }
    };
    let job = app.mix_assistant.pending.take().unwrap();
    app.plugins.set_offline(false);
    lemon_sampler::stream::set_exporting(false);
    lemon_parametric_eq_2::set_exporting(false);
    if let Some(form) = form(app) {
        form.working = None;
    } else {
        return;
    }
    app.invalidate();
    if job.cancel.load(Ordering::Acquire) {
        return report(app, "Analysis or preview cancelled.");
    }
    if stale(app) {
        return report(app, "The project changed. Analyze again.");
    }
    let mut rendered = match result {
        Ok(rendered) => rendered,
        Err(message) => return report(app, &message),
    };
    match job.stage {
        Stage::Analyze => {
            let form = form(app).unwrap();
            let tracks: Vec<_> = form
                .rows
                .iter()
                .filter_map(|track| {
                    let mut track = track.clone();
                    track.level = rendered.levels.get(&track.id)?.clone();
                    Some(track)
                })
                .collect();
            if !tracks
                .iter()
                .any(|track| processing_eligible(track, &form.selected, form.selection_only))
            {
                return report(
                    app,
                    "No included tracks produced audio in this range. Try another range or change the selection.",
                );
            }
            form.analysis = Some(Analysis {
                tracks,
                baseline: rendered,
                proposed: Vec::new(),
                additions: Vec::new(),
                plans: Vec::new(),
                preview: None,
                notes: Vec::new(),
                rejected: BTreeMap::new(),
                review_at: 0,
            });
            preview(app);
        }
        Stage::ProcessSources(mut additions, tuned) => {
            if additions
                .iter()
                .any(|a| !rendered.levels.contains_key(&a.track))
            {
                return report(
                    app,
                    "A proposed processor made a track inaudible. Disable that effect and preview again.",
                );
            }
            let project = app.mix_assistant.source.as_ref().unwrap().render.clone();
            let rate = app.mix_assistant.source.as_ref().unwrap().rate;
            if !tuned {
                let before = form(app)
                    .unwrap()
                    .analysis
                    .as_ref()
                    .unwrap()
                    .tracks
                    .iter()
                    .map(|t| (t.id, t.level.clone()))
                    .collect();
                match processing::calibrate(
                    app,
                    &mut additions,
                    &before,
                    &rendered.levels,
                    &project,
                    rate,
                ) {
                    Ok(true) => {
                        queue_check(
                            app,
                            &project,
                            rate,
                            &[],
                            &additions,
                            Stage::ProcessSources(additions.clone(), true),
                            None,
                        );
                        return;
                    }
                    Err(e) => return report(app, &e),
                    _ => {}
                }
            }
            let form = form(app).unwrap();
            let analysis = form.analysis.as_mut().unwrap();
            let originals: Vec<_> = analysis.tracks.iter().map(|t| t.level.clone()).collect();
            for track in &mut analysis.tracks {
                if let Some(level) = rendered.levels.remove(&track.id) {
                    track.level = level;
                }
            }
            let moves = proposals(form, &project);
            for (track, level) in form
                .analysis
                .as_mut()
                .unwrap()
                .tracks
                .iter_mut()
                .zip(originals)
            {
                track.level = level;
            }
            start_buses(app, moves, additions);
        }
        Stage::MeasureBuses(mut pass, layer) => {
            let source = app.mix_assistant.source.as_ref().unwrap();
            let project = processing::with_processing(
                &with_moves(&source.render, &pass.moves),
                &pass.additions,
            );
            let rate = source.rate;
            let form = form(app).unwrap();
            let analysis = form.analysis.as_ref().unwrap();
            let plans: Vec<_> = layer
                .iter()
                .copied()
                .filter_map(|id| {
                    let mut track = analysis.tracks.iter().find(|track| track.id == id)?.clone();
                    track.level = rendered.levels.get(&id)?.clone();
                    Some((id, processing::choices(form, &project, &track)))
                })
                .collect();
            form.analysis.as_mut().unwrap().plans.extend(plans.clone());
            let count = pass.additions.len();
            match processing::prepare(app, plans, &project, rate) {
                Ok(additions) => pass.additions.extend(additions),
                Err(error) => return report(app, &error),
            }
            if pass.additions.len() == count {
                queue_bus_or_final(app, pass);
            } else {
                let source = app.mix_assistant.source.as_ref().unwrap().render.clone();
                queue_check(
                    app,
                    &source,
                    rate,
                    &pass.moves.clone(),
                    &pass.additions.clone(),
                    Stage::ProcessBuses(pass, rendered.levels, false),
                    None,
                );
            }
        }
        Stage::ProcessBuses(mut pass, before, tuned) => {
            let source = app.mix_assistant.source.as_ref().unwrap();
            let project = source.render.clone();
            let rate = source.rate;
            if !tuned {
                match processing::calibrate(
                    app,
                    &mut pass.additions,
                    &before,
                    &rendered.levels,
                    &project,
                    rate,
                ) {
                    Ok(true) => {
                        queue_check(
                            app,
                            &project,
                            rate,
                            &pass.moves.clone(),
                            &pass.additions.clone(),
                            Stage::ProcessBuses(pass, before, true),
                            None,
                        );
                        return;
                    }
                    Err(e) => return report(app, &e),
                    _ => {}
                }
            }
            queue_bus_or_final(app, pass);
        }
        Stage::Preview(mut moves, mut additions, attempt) => {
            let source = app.mix_assistant.source.as_ref().unwrap();
            let project = source.render.clone();
            let rate = source.rate;
            let f = form(app).unwrap();
            let a = f.analysis.as_mut().unwrap();
            let issues = quality::validate(&a.baseline, &rendered, &a.tracks);
            if !issues.is_empty() {
                let first = &issues[0];
                let label = a
                    .tracks
                    .iter()
                    .find(|t| t.id == first.track)
                    .map(|t| t.name.as_str())
                    .unwrap_or("Master");
                a.notes.push(format!(
                    "{label} at {} in range: {}. Backed off proposed changes.",
                    quality::timestamp(first.at),
                    first.reason
                ));
                a.review_at = first.at;
                if attempt >= 2 {
                    return report(
                        app,
                        "This proposal still changes peaks too much. Apply is withheld. Try a smaller selection or Levels only; review the flagged track's existing effects.",
                    );
                }
                if attempt == 0
                    && issues.iter().all(|i| {
                        i.track != 0 && a.tracks.iter().any(|t| t.id == i.track && !t.role.is_bus())
                    })
                {
                    let ids: BTreeSet<_> = issues.iter().map(|i| i.track).collect();
                    for p in additions.iter().filter(|p| ids.contains(&p.track)) {
                        a.rejected.insert(
                            (p.track, p.recipe.fx),
                            "Removed by peak/dynamics review; see the result message".into(),
                        );
                    }
                    additions.retain(|p| !ids.contains(&p.track));
                    moves.retain(|p| !ids.contains(&p.track) || p.delta <= 0.);
                } else {
                    for p in &additions {
                        a.rejected.insert(
                            (p.track, p.recipe.fx),
                            "Removed after a bus/Master or repeated safety check".into(),
                        );
                    }
                    additions.clear();
                    moves.retain(|m| m.delta < 0.);
                }
                queue_check(
                    app,
                    &project,
                    rate,
                    &moves.clone(),
                    &additions.clone(),
                    Stage::Preview(moves, additions, attempt + 1),
                    None,
                );
                return;
            }
            let Some(analysis) = form(app).and_then(|form| form.analysis.as_mut()) else {
                return;
            };
            if analysis.notes.is_empty() {
                // Offer the largest local change as an audition point, even when
                // it passes the guard. Coordinates are relative to the chosen range.
                analysis.review_at = analysis
                    .baseline
                    .levels
                    .iter()
                    .filter_map(|(id, b)| {
                        let p = rendered.levels.get(id)?;
                        let c = quality::change(b, p);
                        Some((c.at, c.surge))
                    })
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .map_or(0, |x| x.0);
            }
            analysis.plans = additions.iter().fold(
                Vec::new(),
                |mut plans: Vec<(u16, Vec<processing::Recipe>)>, a| {
                    if let Some((_, rs)) = plans.iter_mut().find(|(id, _)| *id == a.track) {
                        rs.push(a.recipe.clone());
                    } else {
                        plans.push((a.track, vec![a.recipe.clone()]));
                    }
                    plans
                },
            );
            analysis.preview = compare(&analysis.baseline, &rendered);
            analysis.proposed = moves;
            analysis.additions = additions;
            if analysis.preview.is_none() {
                report(
                    app,
                    "An audible A/B preview could not be made for this range. Try a different section.",
                );
            }
        }
        Stage::ReviewBefore(moves, additions, at) => {
            let source = app.mix_assistant.source.as_ref().unwrap();
            let project = source.render.clone();
            let rate = source.rate;
            queue_check(
                app,
                &project,
                rate,
                &moves,
                &additions,
                Stage::ReviewAfter(rendered),
                Some(at),
            );
        }
        Stage::ReviewAfter(before) => {
            let a = form(app).unwrap().analysis.as_mut().unwrap();
            if let Some(pair) = compare(&before, &rendered) {
                a.preview = Some(pair);
            }
        }
    }
}

fn queue_check(
    app: &mut App,
    project: &Project,
    rate: u32,
    moves: &[Move],
    additions: &[Addition],
    stage: Stage,
    at: Option<usize>,
) {
    let f = form(app).unwrap();
    let a = f.analysis.as_ref().unwrap();
    let request = RenderRequest {
        span: f.span(),
        tracks: a.tracks.iter().map(|t| usize::from(t.id)).collect(),
        excerpt_at: Some(at.unwrap_or(a.baseline.excerpt_at)),
        effects: preview_effects(additions),
    };
    let project = processing::with_processing(&with_moves(project, moves), additions);
    queue(app, &project, rate, request, stage);
}

fn review_moment(app: &mut App) {
    if stale(app) {
        return report(app, "The project changed. Mix again before reviewing.");
    }
    let source = app.mix_assistant.source.as_ref().unwrap();
    let project = source.render.clone();
    let rate = source.rate;
    let Some(a) = form(app).and_then(|f| f.analysis.as_ref()) else {
        return;
    };
    if a.preview.is_none() {
        return;
    }
    let at =
        ((a.review_at as f64 * quality::STEP - 2.).max(0.) * f64::from(rate)).round() as usize * 2;
    let stage = Stage::ReviewBefore(a.proposed.clone(), a.additions.clone(), at);
    queue_check(app, &project, rate, &[], &[], stage, Some(at));
}

fn stop_audition(app: &mut App) {
    if app.mix_assistant.audition_started.take().is_some() {
        app.transport.stop_preview();
        if let Some(form) = form(app) {
            form.audition = None;
        }
        app.invalidate();
    }
}

fn listen(app: &mut App, after: bool) {
    if stale(app) {
        return report(app, "The project changed. Analyze again before listening.");
    }
    let Some(sample) = form(app).and_then(|form| {
        form.analysis
            .as_ref()?
            .preview
            .as_ref()
            .map(|audio| audio.samples[usize::from(after)].clone())
    }) else {
        return;
    };
    let duration = sample.frames() as f64 / f64::from(sample.rate);
    let project =
        crate::browser_preview::project("Mix Assistant preview.wav", duration, 0.0, 120.0);
    app.transport.preview_for(project, None, sample);
    app.mix_assistant.audition_started = Some(std::time::Instant::now());
    if let Some(form) = form(app) {
        form.audition = Some(after);
    }
    app.invalidate();
}

fn apply(app: &mut App) {
    if stale(app) {
        return report(app, "The project changed. Analyze again before applying.");
    }
    let Some((moves, additions)) = form(app).and_then(|f| {
        let a = f.analysis.as_ref()?;
        a.preview.as_ref()?;
        Some((a.proposed.clone(), a.additions.clone()))
    }) else {
        return;
    };
    let ids: BTreeSet<_> = moves
        .iter()
        .map(|m| m.track)
        .chain(additions.iter().map(|a| a.track))
        .collect();
    if ids.is_empty() {
        return;
    }
    if ids.iter().any(|&id| mixer::locked(&app.state, id)) {
        return report(app, "A track is locked. Analyze again.");
    }
    let mut saved = app.state.project.clone();
    if let Err(e) = crate::project_plugins::store_mixer_slots(
        &mut saved,
        &app.plugins,
        &ids.iter().copied().collect::<Vec<_>>(),
    ) {
        return report(app, &e);
    }
    let snapshots = ids
        .iter()
        .filter_map(|&id| {
            let mut c = mixer::channel_copy(&app.state, id)?;
            c.track = saved.mixer.iter().find(|m| m.index == id)?.clone();
            Some(c)
        })
        .collect();
    history::record(
        &mut app.state,
        "Mix Assistant balance and processing",
        Snap::RackStrips(snapshots),
    );
    for change in &moves {
        mixer::track_mut(&mut app.state, change.track)
            .unwrap()
            .volume = Some(change.value);
    }
    for a in &additions {
        let m = mixer::track_mut(&mut app.state, a.track).unwrap();
        if m.slots.iter().all(Option::is_none) {
            m.slots_enabled = true;
        }
        m.slots[a.slot as usize] = Some(a.saved.clone());
        m.slot_on[a.slot as usize] = true;
        m.slot_mix[a.slot as usize] = a.recipe.mix;
        if let Ok(mut p) = a.plugin.lock() {
            p.reset_fx();
            p.set_offline(false);
        }
        app.plugins.hosted.insert(
            crate::plugins::fx_key(a.track, a.slot),
            crate::plugins::Hosted::new(a.plugin.clone()),
        );
        app.state
            .mixer
            .slot_names
            .insert((a.track, a.slot), format!("Mix · {}", a.recipe.fx.name()));
    }
    app.transport.set_effects(app.plugins.effects());
    app.state.piano.dirty = true;
    app.sync_live();
    app.state.message = Some(format!(
        "Mix Assistant: {} faders, {} stock effects. Undo restores the previous mix.",
        moves.len(),
        additions.len()
    ));
    app.answer(Reply::Button(Answer::Ok));
}

fn next_step(form: &Form) -> (&'static str, Hit) {
    match &form.analysis {
        Some(a) if a.preview.is_some() => ("Mix Again", Hit::Analyze),
        Some(_) => ("Update Preview", Hit::Preview),
        None => ("Mix It", Hit::Analyze),
    }
}

pub(crate) fn primary(app: &mut App) {
    if let Some(form) = form(app) {
        if form.working.is_none() {
            let (_, hit) = next_step(form);
            command(app, hit);
        }
    }
}

pub(crate) fn command(app: &mut App, hit: Hit) {
    if app.mix_assistant.scanning.is_some() {
        if hit == Hit::Close {
            app.answer(Reply::Button(Answer::Cancel));
        }
        return;
    }
    if let Some(job) = &app.mix_assistant.pending {
        if hit == Hit::Close {
            job.cancel.store(true, Ordering::Release);
            report(app, "Cancelling…");
        }
        return;
    }
    match hit {
        Hit::Analyze => return start_analysis(app),
        Hit::Rescan => return start_scan(app),
        Hit::Preview => return preview(app),
        Hit::Listen(after) => return listen(app, after),
        Hit::ReviewMoment => return review_moment(app),
        Hit::Stop => return stop_audition(app),
        Hit::Apply => return apply(app),
        Hit::Close => return app.answer(Reply::Button(Answer::Cancel)),
        _ => {}
    }
    let visible = form(app).map_or(1, |form| form.visible.get());
    if let Hit::Page(next) = hit {
        if let Some(form) = form(app) {
            form.scroll(if next {
                visible as isize
            } else {
                -(visible as isize)
            });
        }
        return;
    }
    if let Hit::RoleMenu(id) = hit {
        if let Some(form) = form(app) {
            form.genre_menu = false;
            form.role_menu = (form.role_menu != Some(id)).then_some(id);
        }
        return;
    }
    if let Hit::Details(id) = hit {
        if let Some(f) = form(app) {
            f.genre_menu = false;
            f.details = (f.details != Some(id)).then_some(id);
        }
        return;
    }
    if hit == Hit::GenreMenu {
        if let Some(f) = form(app) {
            f.genre_menu = !f.genre_menu;
            f.role_menu = None;
            f.details = None;
        }
        return;
    }
    let automatic = if let Hit::ToggleFx(id, fx) = hit {
        form(app)
            .and_then(|f| f.analysis.as_ref())
            .is_some_and(|a| {
                a.plans
                    .iter()
                    .any(|(t, rs)| *t == id && rs.iter().any(|r| r.fx == fx))
            })
    } else {
        false
    };
    stop_audition(app);
    let Some(form) = form(app) else {
        return;
    };
    form.role_menu = None;
    match hit {
        Hit::Processing => form.processing = !form.processing,
        Hit::Character => form.character = !form.character,
        Hit::Space => form.space = !form.space,
        Hit::SetGenre(genre) => {
            form.genre = genre;
            form.genre_menu = false;
            genre.save();
        }
        Hit::ToggleFx(id, fx) => {
            if let Some(t) = form.rows.iter_mut().find(|t| t.id == id) {
                t.fx_choices.insert(fx, !automatic);
                if !automatic {
                    let alternatives: &[Fx] = match fx {
                        Fx::Compressor | Fx::Press | Fx::Transient => {
                            &[Fx::Compressor, Fx::Press, Fx::Transient]
                        }
                        Fx::Hall | Fx::Reverb => &[Fx::Hall, Fx::Reverb],
                        Fx::Stereo | Fx::Matrix => &[Fx::Stereo, Fx::Matrix],
                        _ => &[],
                    };
                    for &other in alternatives {
                        if other != fx {
                            t.fx_choices.insert(other, false);
                        }
                    }
                }
            }
        }
        Hit::Scope(value) => {
            form.selection_only = value && !form.selected.is_empty();
            if form.selection_only {
                for track in &mut form.rows {
                    if form.selected.contains(&track.id) && track.protected.is_none() {
                        track.include = true;
                    }
                }
            }
        }
        Hit::ConfirmRoles => {
            for track in &mut form.rows {
                if included(track, &form.selected, form.selection_only)
                    && track.role != Role::Unknown
                {
                    track.confirmed = true;
                }
            }
        }
        Hit::Range(value) => {
            form.use_range = value && form.selection.is_some();
            form.analysis = None;
            form.first = 0;
        }
        Hit::Strength(value) => form.strength = value,
        Hit::Focus(track) => {
            if form.rows.iter().any(|row| {
                row.id == track
                    && row.protected.is_none()
                    && !row.role.is_bus()
                    && row.role != Role::Return
            }) {
                form.focus = (form.focus != Some(track)).then_some(track);
            }
        }
        Hit::Include(id) | Hit::SetRole(id, _) => {
            if let Some(track) = form.rows.iter_mut().find(|track| track.id == id) {
                match hit {
                    Hit::Include(_)
                        if track.protected.is_none()
                            && in_scope(track, &form.selected, form.selection_only) =>
                    {
                        track.include = !track.include
                    }
                    Hit::Include(_) => {}
                    Hit::SetRole(_, role) => {
                        track.role = role;
                        track.confirmed = true;
                        if (role.is_bus() || role == Role::Return) && form.focus == Some(id) {
                            form.focus = None;
                        }
                        track.fx_choices.clear();
                    }
                    _ => unreachable!(),
                }
            }
        }
        _ => {}
    }
    form.message = None;
    if let Some(analysis) = &mut form.analysis {
        analysis.preview = None;
        analysis.proposed.clear();
    }
    app.invalidate();
}

fn button(
    ui: &mut crate::app::Ui,
    r: os_ui::Rect,
    label: &str,
    hit: Hit,
    active: bool,
    enabled: bool,
) {
    use crate::{app::Action, dialog::DHit};
    if !enabled {
        ui.c.fill_rrect(r, 4.0, ui.t.button);
        ui.c.text_center(os_ui::Font::new(12), r, ui.t.text_dim, label);
        return;
    }
    let action = Action::Dlg(DHit::MixAssistant(hit));
    let pressed = ui.down(r, &action);
    let hover = ui.hit(r, action, label.to_owned());
    crate::views::dialog::button(ui, r, label, active, hover, pressed, false);
}

pub(crate) fn paint(
    ui: &mut crate::app::Ui,
    state: &crate::app::State,
    modal: &Modal,
    form: &Form,
) {
    use os_ui::{Font, Rect};
    let r = crate::views::dialog::place(
        state,
        modal,
        (state.screen.0 - 24.0).clamp(410.0, 900.0),
        (state.screen.1 - 24.0).clamp(490.0, 720.0),
    );
    crate::views::dialog::frame(ui, r, "Mix Assistant");
    let pad = 16.0;
    let left = r.x + pad;
    let width = r.w - pad * 2.0;
    let work = form.working.is_some();
    let form_row = |x, y, w| Rect::new(left + x, r.y + y, w, 26.0);
    let body = Rect::new(left, r.y + 184.0, width, r.h - 284.0);
    let footer = r.bottom() - 43.0;
    let ink = ui.t.text;
    let dim = ui.t.text_dim;
    let info = Font::new(12);
    let title = Font::new(12).bold();
    button(
        ui,
        Rect::new(r.right() - 118., r.y + 3., 84., 21.),
        "Rescan tracks",
        Hit::Rescan,
        false,
        !work,
    );
    let analysis = form.analysis.as_ref();
    let needs_confirmation = form.rows.iter().any(|t| {
        included(t, &form.selected, form.selection_only) && !t.confirmed && t.role != Role::Unknown
    });

    button(
        ui,
        form_row(0.0, 37.0, 91.0),
        "All inserts",
        Hit::Scope(false),
        !form.selection_only,
        !work,
    );
    button(
        ui,
        form_row(97.0, 37.0, 116.0),
        "Selected only",
        Hit::Scope(true),
        form.selection_only,
        !work && !form.selected.is_empty(),
    );
    let range = if form.use_range {
        "Playlist selection"
    } else {
        "Whole song"
    };
    button(
        ui,
        form_row(width - 130.0, 37.0, 130.0),
        range,
        Hit::Range(!form.use_range),
        false,
        !work && form.selection.is_some(),
    );

    ui.c.text(title, left, r.y + 81.0, ink, "Strength");
    for (index, (level, label)) in [(40, "40%"), (70, "70%"), (100, "100%")]
        .into_iter()
        .enumerate()
    {
        button(
            ui,
            form_row(60.0 + index as f32 * 57.0, 74.0, 51.0),
            label,
            Hit::Strength(level),
            form.strength == level,
            !work,
        );
    }
    let focus = form
        .focus
        .and_then(|id| form.rows.iter().find(|track| track.id == id));
    let focus_text = focus.map_or_else(
        || "Focus  None".into(),
        |track| format!("Focus  {}", track.name),
    );
    ui.c.text_clip(
        info,
        left + 239.0,
        r.y + 82.0,
        (width - 239.0).max(0.0),
        dim,
        &focus_text,
    );

    let w = (width - 12.) / 3.;
    button(
        ui,
        form_row(0., 111., w),
        if form.processing {
            "Stock FX: On"
        } else {
            "Levels only"
        },
        Hit::Processing,
        form.processing,
        !work,
    );
    button(
        ui,
        form_row(w + 6., 111., w),
        "Add warmth",
        Hit::Character,
        form.character,
        !work && form.processing,
    );
    button(
        ui,
        form_row(2. * w + 12., 111., w),
        "Add space",
        Hit::Space,
        form.space,
        !work && form.processing,
    );
    button(
        ui,
        form_row(0., 145., 202.),
        &format!("Genre: {}", form.genre.label()),
        Hit::GenreMenu,
        form.genre_menu,
        !work,
    );
    let genre_hint = format!(
        "{} · {}",
        form.references.status(),
        form.genre.description()
    );
    ui.c.text_clip(
        info,
        left + 212.,
        r.y + 152.,
        width - 212.,
        dim,
        &genre_hint,
    );
    ui.hit(
        form_row(208., 145., width - 208.),
        crate::app::Action::Dlg(crate::dialog::DHit::Body),
        genre_hint,
    );
    ui.c.fill_rrect(body, 5.0, ui.t.s(0x1b1e1f));
    if let Some((status, progress)) = form.working {
        let label = form.message.as_deref().unwrap_or(status);
        ui.c.text_center(
            title,
            Rect::new(body.x, body.cy() - 31.0, body.w, 24.0),
            ink,
            label,
        );
        if progress < 0.0 {
            let phase = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                / 90) as usize
                % 8;
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                ui.c.fill_circle(
                    body.cx() + angle.cos() * 10.,
                    body.cy() + 9. + angle.sin() * 10.,
                    1.8,
                    if i == phase { ui.t.accent } else { dim },
                );
            }
        } else {
            let bar = Rect::new(body.x + 24.0, body.cy() + 1.0, body.w - 48.0, 6.0);
            ui.c.fill_rrect(bar, 3.0, ui.t.button);
            ui.c.fill_rrect(
                Rect::new(bar.x, bar.y, bar.w * progress, bar.h),
                3.0,
                ui.t.accent,
            );
            ui.c.text_center(
                info,
                Rect::new(body.x, body.cy() + 12.0, body.w, 20.0),
                dim,
                &format!("{:.0}%", progress * 100.0),
            );
        }
    } else if !form.rows.is_empty() {
        let row_height = 48.0;
        let visible = ((body.h - 55.0) / row_height).floor().max(1.0) as usize;
        form.visible.set(visible);
        let start = form.first.min(form.rows.len().saturating_sub(visible));
        let role_x = (body.w * 0.43).max(152.0);
        let role_w = (body.w * 0.25).clamp(89.0, 146.0);
        let focus_x = role_x + role_w + 6.0;
        let delta_x = focus_x + 52.0;
        // Size to the natural label width: the shared button fitter otherwise
        // squeezes these words into very thin glyphs in the former 54 px cell.
        let include_w = ["Included", "Excluded"]
            .into_iter()
            .map(|label| {
                ui.c.text_width(
                    Font::new(13),
                    crate::prefs::language::Language::current().text(label),
                )
            })
            .fold(54.0_f32, |width, text| width.max(text.ceil() + 16.0));
        let name_x = 9.0 + include_w + 6.0;
        for (offset, label) in [
            (12.0, "Mixer track"),
            (role_x + 7.0, "Role"),
            (focus_x + 4.0, "Focus"),
            (
                delta_x + 4.0,
                if analysis.is_some() {
                    "Result"
                } else {
                    "Setup"
                },
            ),
        ] {
            ui.c.text(title, body.x + offset, body.y + 7.0, dim, label);
        }
        for (row, track) in form.rows.iter().skip(start).take(visible).enumerate() {
            let line = Rect::new(
                body.x + 5.0,
                body.y + 29.0 + row as f32 * row_height,
                body.w - 10.0,
                row_height - 1.0,
            );
            if row % 2 == 0 {
                ui.c.fill_rrect(line, 3.0, ui.t.s(0x272b2d));
            }
            let in_scope = in_scope(track, &form.selected, form.selection_only);
            let can_include = track.protected.is_none() && in_scope;
            let included = included(track, &form.selected, form.selection_only);
            button(
                ui,
                Rect::new(body.x + 9.0, line.y + 3.0, include_w, 22.0),
                if included { "Included" } else { "Excluded" },
                Hit::Include(track.id),
                included,
                can_include,
            );
            let label = format!("{}  {}", track.id, track.name);
            ui.c.text_clip(
                info,
                body.x + name_x,
                line.y + 7.0,
                (role_x - name_x - 6.0).max(0.0),
                if in_scope { ink } else { dim },
                &label,
            );
            button(
                ui,
                Rect::new(body.x + role_x, line.y + 3.0, role_w, 22.0),
                track.role.label(),
                Hit::RoleMenu(track.id),
                form.role_menu == Some(track.id),
                true,
            );
            let focused = form.focus == Some(track.id);
            button(
                ui,
                Rect::new(body.x + focus_x, line.y + 3.0, 43.0, 22.0),
                if focused { "On" } else { "Set" },
                Hit::Focus(track.id),
                focused,
                track.protected.is_none() && !track.role.is_bus() && track.role != Role::Return,
            );
            let status = if let Some(reason) = track.protected {
                reason.into()
            } else if !in_scope {
                "Outside selection".into()
            } else if !included {
                "Excluded".into()
            } else if track.role == Role::Unknown {
                "Role needed".into()
            } else if !track.confirmed {
                "Suggested".into()
            } else if let Some(analysis) = analysis {
                if !analysis.tracks.iter().any(|t| t.id == track.id) {
                    "No audio".into()
                } else if analysis.preview.is_none() {
                    "Ready to preview".into()
                } else if let Some(change) = analysis
                    .proposed
                    .iter()
                    .find(|change| change.track == track.id)
                {
                    format!("{:+.1} dB", change.delta)
                } else if track.role.is_bus() || track.role == Role::Return {
                    "FX only".into()
                } else {
                    "0.0 dB".into()
                }
            } else if track.role.is_bus() || track.role == Role::Return {
                "FX only".into()
            } else {
                "Confirmed".into()
            };
            ui.c.text_clip(
                info,
                body.x + delta_x + 4.0,
                line.y + 7.0,
                body.w - delta_x - 10.0,
                if included { ink } else { dim },
                &status,
            );
            let route = if track.route.is_empty() {
                "No audible output".into()
            } else {
                format!("To {}", track.route)
            };
            let detail = Rect::new(body.x + 12., line.y + 27., body.w - 24., 18.);
            if let Some(analysis) = analysis.filter(|a| {
                included && form.processing && a.tracks.iter().any(|t| t.id == track.id)
            }) {
                let recipes = analysis
                    .plans
                    .iter()
                    .find(|(id, _)| *id == track.id)
                    .map(|(_, recipes)| {
                        recipes
                            .iter()
                            .map(|r| r.fx.name())
                            .collect::<Vec<_>>()
                            .join(" > ")
                    })
                    .filter(|names| !names.is_empty())
                    .unwrap_or_else(|| "No stock effects".into());
                let fx_width = (detail.w * 0.5).min(310.);
                let fx_rect = Rect::new(detail.right() - fx_width, detail.y, fx_width, detail.h);
                button(
                    ui,
                    fx_rect,
                    &format!("{recipes} - review"),
                    Hit::Details(track.id),
                    false,
                    true,
                );
                ui.c.text_clip(
                    Font::new(11),
                    detail.x,
                    detail.y + 2.,
                    detail.w - fx_width - 7.,
                    dim,
                    &route,
                );
            } else {
                ui.c.text_clip(
                    Font::new(11),
                    detail.x,
                    detail.y + 2.,
                    detail.w,
                    dim,
                    &route,
                );
            }
        }
        let navigation = body.bottom() - 24.0;
        button(
            ui,
            Rect::new(body.x + 9., navigation, 142., 21.),
            "Confirm suggested roles",
            Hit::ConfirmRoles,
            false,
            needs_confirmation,
        );
        if form.rows.len() > visible {
            ui.c.text(
                info,
                body.x + 158.0,
                navigation + 4.0,
                dim,
                &format!(
                    "{}–{} of {}",
                    start + 1,
                    (start + visible).min(form.rows.len()),
                    form.rows.len()
                ),
            );
            button(
                ui,
                Rect::new(body.right() - 114.0, navigation, 50.0, 21.0),
                "Prev",
                Hit::Page(false),
                false,
                start > 0,
            );
            button(
                ui,
                Rect::new(body.right() - 58.0, navigation, 50.0, 21.0),
                "Next",
                Hit::Page(true),
                false,
                start + visible < form.rows.len(),
            );
        }
    } else {
        ui.c.text_center(
            title,
            Rect::new(body.x + 6.0, body.cy() - 24.0, body.w - 12.0, 22.0),
            ink,
            "No assigned or configured Mixer tracks found.",
        );
        ui.c.text_center(
            info,
            Rect::new(body.x + 6.0, body.cy() + 2.0, body.w - 12.0, 22.0),
            dim,
            "Route song sources to Mixer inserts, then rescan tracks.",
        );
    }

    let preview = form
        .analysis
        .as_ref()
        .and_then(|analysis| analysis.preview.as_ref());
    let audition_active = form.audition.is_some();
    let message = form
        .message
        .clone()
        .or_else(|| analysis.and_then(|a| a.notes.last().cloned()))
        .or_else(|| processing::existing_expansion(&state.project))
        .or_else(|| {
            preview.map(|preview| {
                let target = if preview.peak > 0.0 {
                    " · OVER 0: reduce gain before Master"
                } else if preview.peak > HEADROOM + 0.5 {
                    " · Headroom reported separately"
                } else {
                    ""
                };
                format!(
                    "Range checked · Pre-Master {:.1} dBTP{target} · Matched A/B ≤12 s",
                    preview.peak
                )
            })
        })
        .or_else(|| {
            analysis
                .filter(|a| a.preview.is_none())
                .map(|_| "Choices changed - update the preview before applying.".into())
        });
    let instruction = if preview.is_some() {
        let a = analysis.unwrap();
        format!(
            "{} level changes, {} stock effects. Compare Before / After, then Apply.",
            a.proposed.len(),
            a.additions.len()
        )
    } else if ready(form) {
        "Ready to mix. Included buses get stock effects after source balancing.".into()
    } else {
        "Include channels and confirm their roles. Focus is optional.".into()
    };
    let review_width = if preview.is_some() { 164. } else { 0. };
    ui.c.text_clip(
        info,
        left,
        footer - 52.,
        width - review_width,
        dim,
        &instruction,
    );
    if let Some(a) = analysis.filter(|_| preview.is_some()) {
        button(
            ui,
            Rect::new(r.right() - pad - 158., footer - 56., 158., 22.),
            &format!("Review {} in range", quality::timestamp(a.review_at)),
            Hit::ReviewMoment,
            false,
            !work,
        );
    }
    if let Some(message) = message {
        ui.c.text_clip(info, left, footer - 32.0, width, dim, &message);
        ui.hit(
            Rect::new(left, footer - 34., width, 20.),
            crate::app::Action::Dlg(crate::dialog::DHit::Body),
            message,
        );
    }
    if work {
        button(
            ui,
            Rect::new(r.right() - 104.0, footer, 88.0, 29.0),
            "Cancel",
            Hit::Close,
            false,
            true,
        );
    } else {
        let has_moves = form.analysis.as_ref().is_some_and(|analysis| {
            !analysis.proposed.is_empty() || !analysis.additions.is_empty()
        });
        let (first, target) = next_step(form);
        let small = ((width - 215.0) / 3.0).clamp(48.0, 69.0);
        button(
            ui,
            Rect::new(left, footer, 116.0, 29.0),
            first,
            target,
            preview.is_none(),
            ready(form),
        );
        button(
            ui,
            Rect::new(left + 122.0, footer, small, 29.0),
            "Before",
            Hit::Listen(false),
            form.audition == Some(false),
            preview.is_some(),
        );
        button(
            ui,
            Rect::new(left + 126.0 + small, footer, small, 29.0),
            "After",
            Hit::Listen(true),
            form.audition == Some(true),
            preview.is_some(),
        );
        button(
            ui,
            Rect::new(left + 130.0 + 2.0 * small, footer, small - 10.0, 29.0),
            "Stop",
            Hit::Stop,
            false,
            audition_active,
        );
        button(
            ui,
            Rect::new(r.right() - 92.0, footer, 76.0, 29.0),
            "Apply",
            Hit::Apply,
            true,
            preview.is_some() && has_moves,
        );
    }

    if let Some(id) = form.details.filter(|_| !work) {
        if let Some(analysis) = &form.analysis {
            if let Some(track) = form.rows.iter().find(|t| t.id == id) {
                let panel = body;
                ui.hit(
                    panel,
                    crate::app::Action::Dlg(crate::dialog::DHit::Body),
                    "",
                );
                ui.c.fill_rrect(panel, 5., ui.t.s(0x24292b));
                ui.c.text_clip(
                    title,
                    panel.x + 10.,
                    panel.y + 8.,
                    panel.w - 105.,
                    ink,
                    &format!("{} - {} - stock effects", track.name, track.role.label()),
                );
                button(
                    ui,
                    Rect::new(panel.right() - 85., panel.y + 4., 77., 23.),
                    "Back",
                    Hit::Details(id),
                    false,
                    true,
                );
                let row_h = ((panel.h - 55.) / 8.).max(22.);
                let col_w = (panel.w - 18.) / 2.;
                let proposed = analysis
                    .plans
                    .iter()
                    .find(|(t, _)| *t == id)
                    .map(|(_, r)| r.as_slice())
                    .unwrap_or(&[]);
                let existing = state.project.mixer.iter().find(|t| t.index == id);
                for (i, fx) in Fx::ALL.into_iter().enumerate() {
                    let x = panel.x + 7. + (i / 8) as f32 * (col_w + 4.);
                    let y = panel.y + 33. + (i % 8) as f32 * row_h;
                    let chosen = proposed.iter().find(|r| r.fx == fx);
                    let reason = chosen.map(|r| r.reason.clone()).unwrap_or_else(|| {
                        analysis
                            .rejected
                            .get(&(id, fx))
                            .cloned()
                            .unwrap_or_else(|| {
                                processing::skipped_reason(form, &state.project, track, fx)
                            })
                    });
                    let already = existing.is_some_and(|m| {
                        m.slots
                            .iter()
                            .flatten()
                            .any(|s| crate::project_plugins::slot_title(s) == fx.name())
                    });
                    let label = format!(
                        "{} {}",
                        if already {
                            "Kept"
                        } else if chosen.is_some() {
                            "On"
                        } else {
                            "+"
                        },
                        fx.name()
                    );
                    let enabled = fx != Fx::Tesla
                        && !already
                        && form.processing
                        && processing_eligible(track, &form.selected, form.selection_only)
                        && existing.is_some_and(|m| {
                            m.slots_enabled || m.slots.iter().all(Option::is_none)
                        });
                    button(
                        ui,
                        Rect::new(x, y, col_w, 22.),
                        &label,
                        Hit::ToggleFx(id, fx),
                        chosen.is_some(),
                        enabled,
                    );
                    if enabled {
                        ui.hit(
                            Rect::new(x, y, col_w, 22.),
                            crate::app::Action::Dlg(crate::dialog::DHit::MixAssistant(
                                Hit::ToggleFx(id, fx),
                            )),
                            reason.clone(),
                        );
                    }
                    if row_h >= 36. {
                        ui.c.text_clip(
                            Font::new(10),
                            x + 3.,
                            y + 24.,
                            col_w - 6.,
                            dim,
                            if already {
                                "Existing settings preserved"
                            } else {
                                &reason
                            },
                        );
                    }
                }
                ui.c.text_clip(Font::new(11),panel.x+9.,panel.bottom()-17.,panel.w-18.,dim,"Click to include/exclude. Role changes restore suggestions. Tesla is a manual editor.");
            }
        }
    }

    if let Some(id) = form.role_menu.filter(|_| !work) {
        let menu = Rect::new(body.cx() - 181.0, body.cy() - 78.0, 362.0, 162.0);
        ui.hit(menu, crate::app::Action::Dlg(crate::dialog::DHit::Body), "");
        ui.c.fill_rrect(menu.outset(2.0), 6.0, ui.t.s(0x141617));
        ui.c.fill_rrect(menu, 5.0, ui.t.s(0x292e30));
        let current = form.rows.iter().find(|track| track.id == id);
        for (i, role) in Role::ALL.into_iter().enumerate() {
            let x = menu.x + 7.0 + (i % 3) as f32 * 116.0;
            let y = menu.y + 7.0 + (i / 3) as f32 * 25.0;
            button(
                ui,
                Rect::new(x, y, 112.0, 22.0),
                role.label(),
                Hit::SetRole(id, role),
                current.is_some_and(|track| track.role == role),
                true,
            );
        }
    }
    if form.genre_menu && !work {
        let menu = Rect::new(left + 4., r.cy() - 120., width - 8., 240.);
        ui.hit(
            menu,
            crate::app::Action::Dlg(crate::dialog::DHit::Body),
            "Choose a mix starting point",
        );
        ui.c.fill_rrect(menu.outset(2.), 6., ui.t.s(0x141617));
        ui.c.fill_rrect(menu, 5., ui.t.s(0x292e30));
        ui.c.text(
            title,
            menu.x + 10.,
            menu.y + 9.,
            ink,
            "Genre - choose a starting point",
        );
        let cell = (menu.w - 24.) / 2.;
        for (i, genre) in Genre::ALL.into_iter().enumerate() {
            let b = Rect::new(
                menu.x + 8. + (i % 2) as f32 * (cell + 8.),
                menu.y + 33. + (i / 2) as f32 * 29.,
                cell,
                25.,
            );
            button(
                ui,
                b,
                genre.label(),
                Hit::SetGenre(genre),
                form.genre == genre,
                true,
            );
            ui.hit(
                b,
                crate::app::Action::Dlg(crate::dialog::DHit::MixAssistant(Hit::SetGenre(genre))),
                genre.description(),
            );
        }
        button(
            ui,
            Rect::new(menu.right() - 82., menu.bottom() - 29., 74., 23.),
            "Cancel",
            Hit::GenreMenu,
            false,
            true,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use os_model::{Arrangement, Clip, ClipSource, InitControl, target};
    use std::{
        path::PathBuf,
        time::{Duration, Instant},
    };

    fn song() -> (Project, Vec<(String, Arc<Sample>)>) {
        let mut project = crate::app::test_project();
        project.tempo = Some(120.0);
        let mut samples = Vec::new();
        let mut clips = Vec::new();
        for (i, (name, frequency, amplitude)) in [
            ("Kick 909", 65.0, 0.46),
            ("Snare", 200.0, 0.21),
            ("Bass", 85.0, 0.76),
            ("Lead Vocal", 397.0, 0.13),
            ("Keys", 650.0, 0.31),
        ]
        .into_iter()
        .enumerate()
        {
            let n = i as u16;
            let path = format!("mix-assistant-{i}.wav");
            let sound: Vec<f32> = (0..96_000)
                .map(|frame| {
                    (frame as f32 * std::f32::consts::TAU * frequency / 48_000.0).sin() * amplitude
                })
                .collect();
            samples.push((
                path.clone(),
                Arc::new(Sample {
                    rate: 48_000,
                    right: sound.clone(),
                    left: sound,
                }),
            ));
            let channel = &mut project.channels[i];
            channel.kind = os_model::ChannelKind::AudioClip;
            channel.sound.sampler_flags = Some(os_model::ChannelSound::AUDIO_CLIP_FLAGS);
            channel.sample_path = Some(path);
            channel.volume = 1.0;
            project.mixer[i + 1].name = Some(name.into());
            // Bass enters half a second later, so the short initial selection must exclude it.
            let position = if n == 2 { 96 } else { 0 };
            clips.push(Clip::new(
                position,
                384 - position,
                n,
                ClipSource::Channel(n),
            ));
        }
        project.mixer[1].routes = vec![6];
        project.mixer[2].routes = vec![6];
        project.mixer[6].name = Some("Drum Bus".into());
        project.mixer[5].edit_locked = true;
        project.mixer[7].name = Some("Empty insert".into());
        project.arrangements = vec![Arrangement {
            clips,
            ..Default::default()
        }];
        (project, samples)
    }

    fn engine(project: &Project, samples: &[(String, Arc<Sample>)]) -> Engine {
        let mut library = os_engine::SampleLibrary::new(None);
        for (path, sample) in samples {
            library.insert_buffer(path.clone(), sample.clone());
        }
        Engine::new(project, &mut library, 48_000, os_engine::PlayMode::Song)
    }

    fn capture_project(
        project: &Project,
        samples: &[(String, Arc<Sample>)],
        span: Option<(u64, u64)>,
        tracks: Vec<usize>,
    ) -> Render {
        render(
            engine(project, samples),
            48_000,
            RenderRequest {
                span,
                tracks,
                excerpt_at: None,
                effects: Vec::new(),
            },
            || false,
            |_| {},
        )
        .unwrap()
    }

    #[test]
    fn mix_assistant_reference_pack_renders_bounded_corrections_on_sources_and_bus() {
        use reference::{Kind, Library, Pack};
        let (mut project, mut samples) = song();
        project.mixer[5].edit_locked = false;
        project.mixer[5].name = Some("Acoustic Guitar".into());
        project.channels[3].name = Some("Lead vocal".into());
        for (i, (_, sample)) in samples.iter_mut().enumerate() {
            project.channels[i].volume = 0.78125;
            let frequencies: [f32; 3] = match i {
                0 => [90., 650., 1600.],
                1 => [250., 650., 1600.],
                2 => [100., 250., 1600.],
                _ => [250., 650., 1600.],
            };
            let wave: Vec<_> = (0..96_000)
                .map(|frame| {
                    let t = frame as f32 / 48_000.;
                    let phase = i as f32 * 0.23;
                    let envelope = if i == 3 && frame % 48_000 >= 32_000 {
                        1.7
                    } else {
                        1.
                    };
                    frequencies
                        .into_iter()
                        .zip([0.025, 0.034, 0.083])
                        .map(|(hz, gain)| gain * (t * hz * std::f32::consts::TAU + phase).sin())
                        .sum::<f32>()
                        * envelope
                })
                .collect();
            *sample = Arc::new(Sample {
                rate: 48_000,
                left: wave.clone(),
                right: wave,
            });
        }
        let baseline = capture_project(&project, &samples, None, vec![1, 2, 3, 4, 5, 6]);
        let mut app = App::new(None);
        app.plugins.effects = crate::plugins::builtin_effects();
        app.state.project = project.clone();
        open(&mut app);
        process(&mut app);
        let f = form(&mut app).unwrap();
        f.genre = Genre::Balanced;
        let fixture = serde_json::json!({
            "tonal": {"centre": [-10., -10., -10., -13., -16., -22.], "upper": [-9., -9., -9., -12., -15., -21.]},
            "dynamics": {"ratio": 2.8, "attack_ms": 12., "release_ms": 140.}
        });
        let data = serde_json::json!({"profiles": {
            "kick": fixture, "snare": fixture, "bass": fixture, "vocals": fixture,
            "acoustic_guitar": fixture, "drum_bus": fixture
        }});
        let mut file = b"MLMIXREF/1\n".to_vec();
        // Separate whitespace keeps the fixture from resembling a compiled pack.
        file.extend(serde_json::to_vec(&data).unwrap());
        let invalid = String::from_utf8(file.clone())
            .unwrap()
            .replace("2.8", "0.8");
        assert!(Pack::parse(invalid.as_bytes()).is_err());
        let mut tracks = f.rows.clone();
        tracks.retain(|t| t.id <= 6);
        for track in &mut tracks {
            track.confirmed = true;
            track.include = true;
            track.level = baseline.levels[&track.id].clone();
        }
        let bypass = tracks
            .iter()
            .map(|t| (t.id, processing::choices(f, &project, t)))
            .collect::<Vec<_>>();
        assert!(
            bypass
                .iter()
                .flat_map(|(_, effects)| effects)
                .all(|r| !r.reason.contains("reference"))
        );
        f.references = Library::Loaded(Pack::parse(&file).unwrap());
        let kinds = [
            Kind::Kick,
            Kind::Snare,
            Kind::Bass,
            Kind::Vocals,
            Kind::AcousticGuitar,
            Kind::DrumBus,
        ];
        let plans: Vec<_> = tracks
            .iter()
            .zip(kinds)
            .map(|(t, kind)| {
                assert_eq!(f.references.get(t, &project).unwrap().0, kind);
                let recipes = processing::choices(f, &project, t);
                assert!(
                    recipes
                        .iter()
                        .any(|r| r.fx == Fx::Eq && r.reason.contains("reference:")),
                    "{}",
                    t.name
                );
                assert!(
                    recipes
                        .iter()
                        .flat_map(|r| &r.values)
                        .filter(|(name, _)| *name == "Band 3 level")
                        .all(|(_, gain)| (-1.12..0.0).contains(gain))
                );
                (t.id, recipes)
            })
            .collect();
        let vocal = plans[3].1.iter().find(|r| r.fx == Fx::Compressor).unwrap();
        assert!(vocal.reason.contains("Vocals timing reference"));
        assert!(
            vocal
                .values
                .iter()
                .any(|(name, value)| *name == "Comp ratio" && *value > 0.)
        );
        // Keep the bus test to EQ here so parallel source and bus processing are
        // verified independently of the existing dynamics stacking guard.
        let additions = processing::prepare(&mut app, plans, &project, 48000).unwrap();
        let processed = processing::with_processing(&project, &additions);
        let after = render(
            engine(&processed, &samples),
            48000,
            RenderRequest {
                span: None,
                tracks: vec![1, 2, 3, 4, 5, 6],
                excerpt_at: None,
                effects: preview_effects(&additions),
            },
            || false,
            |_| {},
        )
        .unwrap();
        assert!(quality::validate(&baseline, &after, &tracks).is_empty());
        for id in [1, 2, 3, 4, 5, 6] {
            assert!(
                after.levels[&id].rms(0.5) < baseline.levels[&id].rms(0.5) - 0.1,
                "Expected measured attenuation on track {id}"
            );
        }
        if let Some(path) = std::env::var_os("MIX_ASSISTANT_REFERENCE_TEST_PACK") {
            let pack = Pack::parse(&std::fs::read(path).unwrap()).unwrap();
            let f = form(&mut app).unwrap();
            f.references = Library::Loaded(pack);
            assert_eq!(f.references.count(), 27);
            for (track, kind) in tracks.iter().zip(kinds) {
                assert_eq!(f.references.get(track, &project).unwrap().0, kind);
                assert!(
                    f.references
                        .get(track, &project)
                        .unwrap()
                        .1
                        .dynamics
                        .is_some()
                );
            }
            let witnessed = tracks
                .iter()
                .filter(|t| {
                    processing::choices(f, &project, t)
                        .iter()
                        .any(|r| r.fx == Fx::Eq && r.reason.contains("reference:"))
                })
                .count();
            assert!(
                witnessed >= 3,
                "The installed data should find the synthetic excess across several roles"
            );
        }
    }

    #[test]
    fn mix_assistant_audio_roles_range_and_headroom() {
        let (mut project, samples) = song();
        let run = capture_project(&project, &samples, None, vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(
            run.levels.keys().copied().collect::<Vec<_>>(),
            [1, 2, 3, 4, 5, 6]
        );
        assert!(run.levels[&3].low_fraction > 0.65);
        assert!(run.levels[&4].typical < run.levels[&3].typical);
        let selected = capture_project(&project, &samples, Some((0, 72)), vec![1, 3]);
        assert!(selected.levels.contains_key(&1));
        assert!(!selected.levels.contains_key(&3));
        let late = capture_project(&project, &samples, Some((120, 168)), vec![1, 3]);
        assert!(late.levels.contains_key(&1) && late.levels.contains_key(&3));
        let references = capture_project(&project, &samples, None, vec![1, 3, 6]);
        assert!(references.peak.is_finite() && references.excerpt.len() == run.excerpt.len());
        project.mixer[0].volume = Some(0.4);
        let quieter = capture_project(&project, &samples, None, Vec::new());
        assert!(
            (run.peak - quieter.peak).abs() < 0.01,
            "headroom measures the pre-Master signal"
        );
        assert_ne!(
            run.excerpt, quieter.excerpt,
            "preview includes the Master controls"
        );
        let pair = compare(&run, &quieter).unwrap();
        let peak = |i: usize| {
            pair.samples[i]
                .left
                .iter()
                .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
        };
        assert!(
            (peak(0) - peak(1)).abs() < 0.01,
            "loudness matched audition"
        );
        assert!(
            render(
                engine(&project, &samples),
                48_000,
                RenderRequest {
                    span: None,
                    tracks: Vec::new(),
                    excerpt_at: None,
                    effects: Vec::new()
                },
                || true,
                |_| {}
            )
            .is_err()
        );
    }

    fn process(app: &mut App) {
        let start = Instant::now();
        while app.mix_assistant.busy() && start.elapsed() < Duration::from_secs(45) {
            super::tick(app);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!app.mix_assistant.busy());
    }

    #[test]
    fn mix_assistant_genre_processes_bare_vocal_and_respects_preview_choices() {
        let saved_genre = Genre::load();
        Genre::Balanced.save();
        let (mut project, samples) = song();
        project.arrangements[0]
            .clips
            .retain(|c| c.source == ClipSource::Channel(3));
        let mut sample = (*samples[3].1).clone();
        for (i, (l, r)) in sample.left.iter_mut().zip(&mut sample.right).enumerate() {
            let envelope = if i % 48_000 >= 32_000 { 1.38 } else { 1.0 };
            *l *= envelope;
            *r *= envelope;
        }
        let folder = PathBuf::from(
            std::env::var_os("MIX_ASSISTANT_TEST_OUTPUT")
                .unwrap_or_else(|| std::env::temp_dir().into()),
        );
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join(format!("genre-vocal-{}.wav", std::process::id()));
        std::fs::write(&path, crate::export::pcm16_stereo(&sample).unwrap()).unwrap();
        project.channels[3].sample_path = Some(path.to_string_lossy().into_owned());
        project.channels[3].volume = 0.78125;
        let mut app = App::new(None);
        app.plugins.effects = crate::plugins::builtin_effects();
        app.state.project = project.clone();
        app.state.tempo = 120.;
        open(&mut app);
        process(&mut app);
        command(&mut app, Hit::ConfirmRoles);
        command(&mut app, Hit::SetGenre(Genre::Jazz));
        command(&mut app, Hit::Analyze);
        process(&mut app);
        {
            let f = form(&mut app).unwrap();
            let a = f.analysis.as_ref().unwrap();
            assert!(a.preview.is_some(), "{:?}", f.message);
            assert!(a.additions.is_empty());
            let vocal = a.tracks.iter().find(|t| t.id == 4).unwrap();
            let reason = processing::skipped_reason(f, &project, vocal, Fx::Compressor);
            assert!(reason.contains("RMS variation"), "{reason}");
        }
        command(&mut app, Hit::SetGenre(Genre::Pop));
        assert!(
            form(&mut app)
                .unwrap()
                .analysis
                .as_ref()
                .unwrap()
                .preview
                .is_none()
        );
        assert_eq!(
            app.state.project, project,
            "Choosing a genre never edits the song"
        );
        command(&mut app, Hit::Preview);
        process(&mut app);
        {
            let f = form(&mut app).unwrap();
            let a = f.analysis.as_ref().unwrap();
            assert!(a.preview.is_some(), "{:?}", f.message);
            assert!(a.notes.is_empty(), "{:?}", a.notes);
            let comp = a
                .additions
                .iter()
                .find(|p| p.track == 4 && p.recipe.fx == Fx::Compressor)
                .expect("Pop must keep measured compression on a bare, moderately varying vocal");
            assert!((comp.recipe.target_reduction.unwrap() - 2.45).abs() < 0.01);
            // Measure the actual stock DSP, before loudness matching or fader moves.
            let before = capture_project(&project, &[], None, vec![4]);
            let processed = processing::with_processing(&project, &a.additions);
            let after = render(
                engine(&processed, &[]),
                48000,
                RenderRequest {
                    span: None,
                    tracks: vec![4],
                    excerpt_at: None,
                    effects: preview_effects(&a.additions),
                },
                || false,
                |_| {},
            )
            .unwrap();
            let response = quality::change(&before.levels[&4], &after.levels[&4]);
            assert!(
                response.reduction > 0.5 && response.reduction < 5.,
                "{response:?}"
            );
            assert!(after.levels[&4].true_peak <= before.levels[&4].true_peak + 0.1);
        }
        command(&mut app, Hit::Strength(40));
        command(&mut app, Hit::Preview);
        process(&mut app);
        let comp = form(&mut app)
            .unwrap()
            .analysis
            .as_ref()
            .unwrap()
            .additions
            .iter()
            .find(|a| a.recipe.fx == Fx::Compressor)
            .unwrap();
        assert!(
            (comp.recipe.target_reduction.unwrap() - 1.4).abs() < 0.01,
            "Render calibration must retain the user's strength"
        );
        command(&mut app, Hit::Processing);
        command(&mut app, Hit::Preview);
        process(&mut app);
        assert!(
            form(&mut app)
                .unwrap()
                .analysis
                .as_ref()
                .unwrap()
                .additions
                .is_empty()
        );
        command(&mut app, Hit::Processing);
        command(&mut app, Hit::Preview);
        process(&mut app);
        command(&mut app, Hit::Apply);
        assert!(!is_open(&app));
        assert!(
            app.state.project.mixer[4]
                .slots
                .iter()
                .flatten()
                .any(|s| crate::project_plugins::slot_title(s) == "ML Limiter")
        );
        history::undo(&mut app.state).unwrap();
        assert_eq!(app.state.project, project);
        open(&mut app);
        process(&mut app);
        assert_eq!(form(&mut app).unwrap().genre, Genre::Pop);
        command(&mut app, Hit::GenreMenu);
        command(&mut app, Hit::GenreMenu);
        assert_eq!(
            form(&mut app).unwrap().genre,
            Genre::Pop,
            "Cancel leaves the choice alone"
        );
        command(&mut app, Hit::Close);
        saved_genre.save();
    }

    #[test]
    fn mix_assistant_render_review_apply_history_and_protections() {
        use crate::{app::Action, dialog::DHit, mixer::MHit};
        let (mut project, samples) = song();
        let home = std::env::var_os("MIX_ASSISTANT_TEST_OUTPUT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let folder = home.join(format!("mix-assistant-test-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        for (i, (_, sample)) in samples.iter().enumerate() {
            let path = folder.join(format!("{i}.wav"));
            std::fs::write(&path, crate::export::pcm16_stereo(sample).unwrap()).unwrap();
            project.channels[i].sample_path = Some(path.to_string_lossy().into_owned());
        }
        project.init_controls.push(InitControl {
            head: 0,
            target: target::mixer(3, target::MIXER_VOLUME),
            value: 12_800,
        });
        let mut app = App::new(None);
        app.plugins.effects = crate::plugins::builtin_effects();
        app.state.tempo = 120.0;
        app.state.project = project.clone();
        app.state.selected_mixer = 2;
        app.state.mixer.sel = vec![1, 2];
        app.state.raise(crate::app::Win::Mixer);
        let mut canvas = os_ui::Canvas::new(1280, 900, 1.0).unwrap();
        app.paint(&mut canvas, None);
        let bounds = |app: &App, hit| {
            app.hits
                .iter()
                .find(|h| h.action == Action::Mix(hit))
                .map(|h| h.rect)
        };
        let attach = bounds(&app, MHit::AttachPlaylist).unwrap();
        let assistant = bounds(&app, MHit::MixAssistant).unwrap();
        assert!(assistant.x >= attach.right() && assistant.y == attach.y);
        app.act(Action::Mix(MHit::MixAssistant));
        assert!(is_open(&app));
        assert!(app.mix_assistant.busy());
        process(&mut app);
        {
            let f = form(&mut app).unwrap();
            assert!(f.analysis.is_none());
            assert!(!ready(f));
            let bus = f.rows.iter().find(|track| track.id == 6).unwrap();
            assert_eq!(bus.role, Role::DrumBus);
            assert!(!bus.include);
            let kick = f.rows.iter().find(|track| track.id == 1).unwrap();
            assert!(kick.include && kick.route.contains("Drum Bus"));
        }
        command(&mut app, Hit::Analyze);
        assert!(
            !app.mix_assistant.busy(),
            "unconfirmed roles block the render"
        );
        command(&mut app, Hit::ConfirmRoles);
        command(&mut app, Hit::Focus(4));
        command(&mut app, Hit::Analyze);
        process(&mut app);
        {
            let form = form(&mut app).unwrap();
            let analysis = form.analysis.as_ref().unwrap();
            assert!(analysis.preview.is_some(), "{:?}", form.message);
            let changed: Vec<_> = analysis
                .proposed
                .iter()
                .map(|change| change.track)
                .collect();
            assert!(changed.contains(&1) || changed.contains(&2), "{changed:?}");
            assert!(
                !changed.iter().any(|id| [3, 4, 5, 6, 7].contains(id)),
                "{changed:?}"
            );
            assert!(analysis.additions.iter().all(|a| a.track != 6));
        }
        assert_eq!(
            app.state.project, project,
            "analysis and preview never mutate the project"
        );
        command(&mut app, Hit::SetRole(6, Role::DrumBus));
        command(&mut app, Hit::Include(6));
        command(&mut app, Hit::ToggleFx(6, Fx::Compressor));
        command(&mut app, Hit::Preview);
        process(&mut app);
        {
            let f = form(&mut app).unwrap();
            let result = f.analysis.as_ref().unwrap();
            assert!(result.preview.is_some(), "{:?}", f.message);
            assert!(
                result
                    .additions
                    .iter()
                    .any(|a| a.track == 6 && a.recipe.fx == Fx::Compressor)
            );
            assert!(result.proposed.iter().any(|m| [1, 2].contains(&m.track)));
            assert!(
                result.proposed.iter().all(|m| m.track != 6),
                "bus faders stay fixed"
            );
        }
        app.state.project.mixer[6].edit_locked = true;
        command(&mut app, Hit::Apply);
        assert!(
            is_open(&app),
            "stale or newly locked proposals cannot be applied"
        );
        app.state.project.mixer[6].edit_locked = false;
        command(&mut app, Hit::Include(6));
        command(&mut app, Hit::Scope(true));
        command(&mut app, Hit::SetRole(2, Role::Music));
        command(&mut app, Hit::Include(1));
        command(&mut app, Hit::Character);
        command(&mut app, Hit::Preview);
        process(&mut app);
        {
            let form = form(&mut app).unwrap();
            let analysis = form.analysis.as_ref().unwrap();
            assert!(analysis.preview.is_some(), "{:?}", form.message);
            assert_eq!(
                analysis
                    .proposed
                    .iter()
                    .map(|change| change.track)
                    .collect::<Vec<_>>(),
                [2]
            );
        }
        app.paint(&mut canvas, None);
        assert!(
            app.hits
                .iter()
                .any(|hit| hit.action == Action::Dlg(DHit::MixAssistant(Hit::Apply)))
        );
        if let Ok(out) = std::env::var("MIX_ASSISTANT_SCREENSHOTS") {
            canvas.pixmap.save_png(format!("{out}/review.png")).unwrap();
            command(&mut app, Hit::Details(2));
            app.paint(&mut canvas, None);
            canvas
                .pixmap
                .save_png(format!("{out}/effects.png"))
                .unwrap();
            command(&mut app, Hit::Details(2));
            command(&mut app, Hit::GenreMenu);
            app.paint(&mut canvas, None);
            canvas.pixmap.save_png(format!("{out}/genres.png")).unwrap();
            command(&mut app, Hit::GenreMenu);
            for scale in [1.25, 1.5, 2.] {
                let mut scaled =
                    os_ui::Canvas::new((1280. * scale) as u32, (900. * scale) as u32, scale)
                        .unwrap();
                app.paint(&mut scaled, None);
                scaled
                    .pixmap
                    .save_png(format!("{out}/review-{scale}.png"))
                    .unwrap();
            }
            let design = app.design();
            app.set_design(os_theme::design::Design::Lemon, false);
            app.paint(&mut canvas, None);
            canvas
                .pixmap
                .save_png(format!("{out}/review-studio.png"))
                .unwrap();
            app.set_design(design, false);
        }
        let before = app.state.history.len();
        command(&mut app, Hit::Apply);
        assert!(!app.modal_open());
        assert_eq!(app.state.history.len(), before + 1);
        assert_ne!(app.state.project.mixer[2].volume, project.mixer[2].volume);
        assert!(
            app.state.project.mixer[2]
                .slots
                .iter()
                .flatten()
                .any(|s| crate::project_plugins::slot_title(s) == "ML Tape")
        );
        assert!(app.state.project.mixer[2].slots_enabled);
        let applied = app.state.project.clone();
        let stored = os_flp::project::create(&applied)
            .unwrap()
            .to_bytes()
            .unwrap();
        let reopened = os_flp::project::load(&os_flp::FlpFile::parse(&stored).unwrap()).unwrap();
        assert_eq!(
            reopened.mixer[2].slots, applied.mixer[2].slots,
            "Applied processors survive project save/reopen"
        );
        assert_eq!(reopened.mixer[2].slot_on, applied.mixer[2].slot_on);
        history::undo(&mut app.state).unwrap();
        assert_eq!(app.state.project, project);
        history::redo(&mut app.state).unwrap();
        assert_eq!(app.state.project, applied);
        open(&mut app);
        process(&mut app);
        command(&mut app, Hit::ConfirmRoles);
        command(&mut app, Hit::Analyze);
        command(&mut app, Hit::Close);
        process(&mut app);
        assert!(form(&mut app).unwrap().analysis.is_none());
        command(&mut app, Hit::Close);
        assert_eq!(app.state.project, applied);
        assert!(!app.modal_open());
        crate::attached_mixer::toggle(&mut app);
        app.paint(&mut canvas, None);
        let attach = bounds(&app, MHit::AttachPlaylist).unwrap();
        let assistant = bounds(&app, MHit::MixAssistant).unwrap();
        assert!(assistant.x >= attach.right() && assistant.y == attach.y);
        crate::workspace_visibility::toggle(VISIBILITY);
        app.paint(&mut canvas, None);
        assert!(bounds(&app, MHit::MixAssistant).is_none());
        assert!(
            !mixer::options_menu(&app.state, 0.0, 0.0)
                .items
                .iter()
                .any(|item| item.label == "Mix Assistant…")
        );
        assert_eq!(app.state.project, applied);
        crate::workspace_visibility::toggle(VISIBILITY);
        drop(app);
        for i in 0..samples.len() {
            std::fs::remove_file(folder.join(format!("{i}.wav"))).unwrap();
        }
        std::fs::remove_dir(folder).unwrap();
    }
    #[test]
    fn mix_assistant_labels_and_nested_bus_order() {
        let (mut project, _) = song();
        project.mixer[1].name = Some("Main Vox".into());
        project.mixer[2].name = Some("BackMid".into());
        project.mixer[6].name = Some("VocalMix".into());
        project.mixer[6].routes = vec![8];
        project.mixer[8].name = Some("Sum".into());
        project.mixer[8].routes = vec![9];
        project.mixer[9].name = Some("MIX BUS".into());
        let mut app = App::new(None);
        app.state.project = project;
        open(&mut app);
        process(&mut app);
        let f = form(&mut app).unwrap();
        let row = |id| f.rows.iter().find(|r| r.id == id).unwrap();
        assert_eq!(row(1).role, Role::LeadVocal);
        assert_eq!(row(2).role, Role::BackingVocal);
        assert_eq!(row(6).role, Role::VocalBus);
        assert_eq!(row(9).role, Role::MixBus);
        assert_eq!(row(1).route, "VocalMix > Sum > MIX BUS > Master");
        assert!(!row(9).include);
        assert_eq!(Role::guess("Drums Mix", true), Role::DrumBus);
        assert_eq!(Role::guess("Instrument Mix", true), Role::InstrumentBus);
        assert_eq!(Role::guess("Bass Mix", true), Role::BassBus);
        for t in &mut f.rows {
            if [6, 9].contains(&t.id) {
                t.include = true;
                t.confirmed = true;
            }
        }
        assert_eq!(
            setup::bus_layers(&f.rows, f).unwrap(),
            VecDeque::from([vec![6], vec![9]])
        );
    }

    fn measured_level(loudness: f64) -> Level {
        Level {
            frames: Vec::new(),
            true_peak: loudness + 3.,
            typical: loudness,
            windows: vec![loudness as f32; 10],
            gate: -60.,
            crest: 8.,
            low_fraction: 0.1,
            sub_fraction: 0.001,
            high_fraction: 0.1,
            side_fraction: 0.,
        }
    }

    #[test]
    fn mix_assistant_loud_mix_keeps_individual_moves() {
        let mut app = App::new(None);
        app.plugins.effects = crate::plugins::builtin_effects();
        app.state.project = crate::app::test_project();
        open(&mut app);
        let project = app.state.project.clone();
        let f = form(&mut app).unwrap();
        f.processing = false;
        f.focus = Some(1);
        let tracks = [
            (Role::LeadVocal, 0.5),
            (Role::Music, 0.85),
            (Role::Pad, 0.17),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (role, amplitude))| Track {
            id: i as u16 + 1,
            name: role.label().into(),
            role,
            level: measured_level(db(amplitude) - 3.),
            protected: None,
            include: true,
            downstream: vec![0],
            route: "Master".into(),
            confirmed: true,
            fx_choices: BTreeMap::new(),
        })
        .collect();
        f.analysis = Some(Analysis {
            tracks,
            baseline: Render {
                output: None,
                levels: BTreeMap::new(),
                peak: db(1.52),
                excerpt_at: 0,
                excerpt: Vec::new(),
                rate: 48000,
            },
            proposed: Vec::new(),
            additions: Vec::new(),
            plans: Vec::new(),
            preview: None,
            notes: Vec::new(),
            rejected: BTreeMap::new(),
            review_at: 0,
        });
        let moves = proposals(f, &project);
        assert!(
            !moves.iter().any(|m| m.track == 1),
            "The focus stays at its original level"
        );
        assert!(moves.iter().find(|m| m.track == 2).unwrap().delta < -4.);
        assert!(moves.iter().find(|m| m.track == 3).unwrap().delta > 0.8);
        f.analysis.as_mut().unwrap().baseline.peak = -25.;
        let quiet = proposals(f, &project);
        assert!(
            moves
                .iter()
                .zip(quiet)
                .all(|(a, b)| (a.delta - b.delta).abs() < 1e-10),
            "Headroom never overwrites relative balance"
        );
        f.focus = Some(2);
        assert!(
            !proposals(f, &project).iter().any(|m| m.track == 2),
            "Non-lead focus is also anchored correctly"
        );
    }

    #[test]
    fn mix_assistant_stock_catalog_renders_and_reopens_exact_settings() {
        let (project, samples) = song();
        let mut app = App::new(None);
        app.plugins.effects = crate::plugins::builtin_effects();
        app.state.project = project.clone();
        let mut measured = capture_project(&project, &samples, None, vec![1, 2, 3, 4, 5, 6]);
        let mut tracks = setup::inventory(&app.state, &project);
        for track in &mut tracks {
            if let Some(level) = measured.levels.remove(&track.id) {
                track.level = level;
            }
        }
        let t = tracks.iter_mut().find(|t| t.id == 3).unwrap();
        // A dynamic, wide, bass-heavy source exercises conditional recipes and measured thresholds.
        t.level.crest = 19.;
        t.level.side_fraction = 0.3;
        let linked: BTreeSet<_> = crate::plugins::builtin_effects()
            .iter()
            .map(|a| a.desc.name.clone())
            .collect();
        assert_eq!(
            linked,
            Fx::ALL.into_iter().map(|f| f.name().to_owned()).collect()
        );
        for role in [Role::Bass, Role::DrumBus, Role::LeadVocal] {
            t.role = role;
            for fx in Fx::ALL.into_iter().filter(|f| *f != Fx::Tesla) {
                let recipe = processing::recipe(fx, t, &project, 70);
                let additions =
                    processing::prepare(&mut app, vec![(3, vec![recipe.clone()])], &project, 48000)
                        .unwrap_or_else(|e| panic!("{role:?} {fx:?}: {e}"));
                let a = &additions[0];
                let wrapped = crate::project_plugins::wrapped_slot(&a.saved).unwrap();
                let available = app
                    .plugins
                    .effects
                    .iter()
                    .find(|p| p.desc.name == fx.name())
                    .unwrap();
                let reopened = available.lib.clone().create(&available.desc.id).unwrap();
                {
                    let mut p = reopened.lock().unwrap();
                    assert!(p.load_state(&wrapped.state));
                    assert!(p.activate(48000));
                    for &(name, value) in &recipe.values {
                        let value = processing::clap_value(fx, name, value);
                        let info = (0..p.param_count())
                            .filter_map(|i| p.param_info(i))
                            .find(|p| p.name == name)
                            .unwrap();
                        let actual =
                            p.param_value(info.id).unwrap() * (info.max - info.min) + info.min;
                        let tolerance = if info.steps > 0 { 1.01 } else { 0.001 };
                        assert!(
                            (actual - value).abs() < tolerance,
                            "{fx:?} {name}: expected {value}, saved {actual}"
                        );
                    }
                }
                if role == Role::Bass {
                    let processed = processing::with_processing(&project, &additions);
                    let run = |plugin: os_host::SharedPlugin| {
                        render(
                            engine(&processed, &samples),
                            48000,
                            RenderRequest {
                                span: None,
                                tracks: vec![3],
                                excerpt_at: None,
                                effects: vec![(
                                    3,
                                    a.slot as usize,
                                    plugin as os_engine::SharedEffect,
                                )],
                            },
                            || false,
                            |_| {},
                        )
                        .unwrap()
                    };
                    let first = run(a.plugin.clone());
                    let second = run(reopened);
                    assert!(first.excerpt.iter().all(|x| x.is_finite()), "{fx:?}");
                    let error = first
                        .excerpt
                        .iter()
                        .zip(&second.excerpt)
                        .map(|(a, b)| (a - b).abs())
                        .fold(0., f32::max);
                    assert!(error < 0.005, "{fx:?} reopen audio error {error}");
                    assert!(first.levels.contains_key(&3), "{fx:?} silenced source");
                    assert!(
                        first
                            .excerpt
                            .iter()
                            .zip(&measured.excerpt)
                            .any(|(a, b)| (a - b).abs() > 1e-5),
                        "{fx:?} didn't process audio"
                    );
                }
            }
        }
    }

    #[test]
    fn mix_assistant_late_vocal_burst_is_not_boosted_into_overload() {
        let (mut project, _) = song();
        project.arrangements[0].clips.truncate(2);
        for (i, clip) in project.arrangements[0].clips.iter_mut().enumerate() {
            clip.position = 0;
            clip.length = 1536;
            project.mixer[i + 1].routes = vec![6];
            project.channels[i].volume = 0.78125;
        }
        project.mixer[1].name = Some("Main Vox".into());
        project.mixer[2].name = Some("Keys".into());
        project.mixer[6].name = Some("Vocal Bus".into());
        let samples: Vec<_> = (0..2)
            .map(|i| {
                let path = format!("vocal-quality-{i}.wav");
                project.channels[i].sample_path = Some(path.clone());
                let data: Vec<_> = (0..48000 * 8)
                    .map(|n| {
                        let t = n as f64 / 48000.;
                        let amplitude = if i == 1 {
                            0.07
                        } else if (6.0..6.04).contains(&t) {
                            0.80
                        } else if (2.0..2.7).contains(&t) || (4.0..4.7).contains(&t) {
                            0.
                        } else {
                            0.045 * (0.65 + 0.35 * (t * 13.).sin())
                        };
                        (amplitude
                            * (t * std::f64::consts::TAU * if i == 0 { 440. } else { 1600. }).sin())
                            as f32
                    })
                    .collect();
                (
                    path,
                    Arc::new(Sample {
                        rate: 48000,
                        left: data.clone(),
                        right: data,
                    }),
                )
            })
            .collect();
        let original = capture_project(&project, &samples, None, vec![1, 2, 6]);
        let legacy = with_moves(&project, &[fader_change(&project, 1, 4.2).unwrap()]);
        let bad = capture_project(&legacy, &samples, None, vec![1, 2, 6]);
        assert!(
            bad.levels[&1].true_peak > 1.,
            "fixture must reproduce the former up-gain hazard"
        );
        assert!(!quality::validate(&original, &bad, &[]).is_empty());

        let folder = PathBuf::from(
            std::env::var_os("MIX_ASSISTANT_TEST_OUTPUT")
                .unwrap_or_else(|| std::env::temp_dir().into_os_string()),
        )
        .join("vocal-burst");
        std::fs::create_dir_all(&folder).unwrap();
        for (i, (_, s)) in samples.iter().enumerate() {
            let path = folder.join(format!("{i}.wav"));
            std::fs::write(&path, crate::export::pcm16_stereo(s).unwrap()).unwrap();
            project.channels[i].sample_path = Some(path.to_string_lossy().into_owned());
        }
        let mut app = App::new(None);
        app.plugins.effects = crate::plugins::builtin_effects();
        app.state.project = project.clone();
        open(&mut app);
        process(&mut app);
        command(&mut app, Hit::ConfirmRoles);
        command(&mut app, Hit::Focus(2));
        command(&mut app, Hit::Include(6));
        command(&mut app, Hit::SetRole(6, Role::VocalBus));
        command(&mut app, Hit::Analyze);
        process(&mut app);
        let f = form(&mut app).unwrap();
        let a = f.analysis.as_ref().unwrap();
        assert!(a.preview.is_some(), "{:?} {:?}", f.message, a.notes);
        let applied = processing::with_processing(&with_moves(&project, &a.proposed), &a.additions);
        let effects = preview_effects(&a.additions);
        let paths: Vec<_> = samples
            .iter()
            .enumerate()
            .map(|(i, (_, s))| (project.channels[i].sample_path.clone().unwrap(), s.clone()))
            .collect();
        let after = render(
            engine(&applied, &paths),
            48000,
            RenderRequest {
                span: None,
                tracks: vec![1, 2, 6],
                excerpt_at: None,
                effects,
            },
            || false,
            |_| {},
        )
        .unwrap();
        let issues = quality::validate(&original, &after, &a.tracks);
        assert!(
            issues.is_empty(),
            "{:?}",
            issues.iter().map(|i| &i.reason).collect::<Vec<_>>()
        );
        assert!(
            after.levels[&1].true_peak < 0.,
            "vocal burst must retain headroom"
        );
        assert!(
            a.additions
                .iter()
                .filter(|a| a.recipe.fx == Fx::Compressor)
                .count()
                <= 1,
            "avoid automatic source/bus compressor stacking"
        );
        std::fs::write(folder.join("measurements.txt"),format!("original vocal {:.2} dBTP\nformer 4.2 dB boost {:.2} dBTP\nverified proposal {:.2} dBTP\nnotes {:?}\n",original.levels[&1].true_peak,bad.levels[&1].true_peak,after.levels[&1].true_peak,a.notes)).unwrap();
        assert!(
            a.baseline.levels.len() >= 3,
            "retain baseline stems for final verification"
        );
        let vocal = a.tracks.iter().find(|t| t.id == 1).unwrap().clone();
        let rate = a.baseline.rate;
        let mut unsafe_recipe = processing::recipe(Fx::Compressor, &vocal, &project, 70);
        for (name, value) in &mut unsafe_recipe.values {
            match *name {
                "Comp ratio" => *value = -700.,
                "Comp knee" => *value = -450.,
                "Comp threshold" => {
                    *value = f64::from(
                        lemon_limiter::params::parse_text(
                            lemon_limiter::params::DEFS[8].text,
                            "-35",
                            0,
                            1000,
                        )
                        .unwrap(),
                    )
                }
                _ => {}
            }
        }
        let unsafe_fx =
            processing::prepare(&mut app, vec![(1, vec![unsafe_recipe])], &project, rate).unwrap();
        assert!(
            processing::existing_expansion(&processing::with_processing(&project, &unsafe_fx))
                .is_some()
        );
        queue_check(
            &mut app,
            &project,
            rate,
            &[],
            &unsafe_fx,
            Stage::Preview(vec![], unsafe_fx.clone(), 0),
            None,
        );
        process(&mut app);
        let f = form(&mut app).unwrap();
        let a = f.analysis.as_ref().unwrap();
        assert!(a.preview.is_some(), "{:?}", f.message);
        assert!(
            !a.notes.is_empty(),
            "the real render pipeline must notice expansion"
        );
        assert!(
            a.additions.is_empty(),
            "unsafe processing must be removed before Apply"
        );
        command(&mut app, Hit::Preview);
        process(&mut app);
        assert_eq!(app.state.project, project);
        command(&mut app, Hit::ReviewMoment);
        process(&mut app);
        assert!(
            form(&mut app)
                .unwrap()
                .analysis
                .as_ref()
                .unwrap()
                .preview
                .is_some()
        );
        command(&mut app, Hit::Apply);
        assert!(!is_open(&app));
        history::undo(&mut app.state).unwrap();
        assert_eq!(app.state.project, project);
    }

    #[test]
    fn mix_assistant_eq_uses_overlap_and_dynamic_controls_roundtrip() {
        let (project, samples) = song();
        let mut app = App::new(None);
        app.plugins.effects = crate::plugins::builtin_effects();
        app.state.project = project.clone();
        let mut tracks = setup::inventory(&app.state, &project);
        let audio = |hz: f64, bright: bool| {
            (0..48000 * 3)
                .flat_map(|n| {
                    let t = n as f64 / 48000.;
                    let x = if bright && n % 48000 > 40000 {
                        0.2 * (t * std::f64::consts::TAU * 4000.).sin()
                    } else {
                        0.1 * (t * std::f64::consts::TAU * hz).sin()
                    };
                    [x as f32, x as f32]
                })
                .collect::<Vec<_>>()
        };
        let level = |audio: &[f32]| {
            let mut m = Meter::new(48000, Mode::M).unwrap();
            m.feed(audio).unwrap();
            m.finish().unwrap()
        };
        let mut focus = tracks.remove(0);
        focus.role = Role::LeadVocal;
        focus.level = level(&audio(1600., false));
        let mut music = tracks.remove(0);
        music.role = Role::Music;
        music.level = level(&audio(1600., false));
        assert!(quality::masking(&music, &focus).is_some());
        music.level.frames.rotate_left(40);
        music.level = level(&audio(85., false));
        assert!(
            quality::masking(&music, &focus).is_none(),
            "separate bands must not trigger masking EQ"
        );
        music.level = level(&audio(440., true));
        music.confirmed = true;
        music.include = true;
        open(&mut app);
        process(&mut app);
        let f = form(&mut app).unwrap();
        f.focus = Some(focus.id);
        f.analysis = Some(Analysis {
            tracks: vec![focus, music.clone()],
            baseline: capture_project(&project, &samples, None, vec![1, 2]),
            proposed: vec![],
            additions: vec![],
            plans: vec![],
            preview: None,
            notes: vec![],
            rejected: BTreeMap::new(),
            review_at: 0,
        });
        let recipes = processing::choices(f, &project, &music);
        let eq = recipes.iter().find(|r| r.fx == Fx::Eq).unwrap();
        assert!(eq.values.iter().any(|(name, _)| *name == "Band 6 Range"));
        let added = processing::prepare(
            &mut app,
            vec![(music.id, vec![eq.clone()])],
            &project,
            48000,
        )
        .unwrap();
        let wrapped = crate::project_plugins::wrapped_slot(&added[0].saved).unwrap();
        let available = app
            .plugins
            .effects
            .iter()
            .find(|a| a.desc.name == "ML EQ")
            .unwrap();
        let reopened = available.lib.clone().create(&available.desc.id).unwrap();
        let mut p = reopened.lock().unwrap();
        assert!(p.load_state(&wrapped.state));
        for &(name, value) in &eq.values {
            let info = (0..p.param_count())
                .filter_map(|i| p.param_info(i))
                .find(|p| p.name == name)
                .unwrap();
            let actual = p.param_value(info.id).unwrap() * (info.max - info.min) + info.min;
            assert!(
                (actual - processing::clap_value(Fx::Eq, name, value)).abs() < 0.002,
                "{name}: {actual}"
            );
        }
    }

    #[test]
    fn mix_assistant_signed_ratio_regression_compresses_instead_of_expanding() {
        let (mut project, samples) = song();
        project.channels[2].volume = 0.78125;
        let mut app = App::new(None);
        app.plugins.effects = crate::plugins::builtin_effects();
        let mut t = setup::inventory(&app.state, &project)
            .into_iter()
            .find(|t| t.id == 3)
            .unwrap();
        let before = capture_project(&project, &samples, None, vec![3]);
        t.level = before.levels[&3].clone();
        let mut r = processing::recipe(Fx::Compressor, &t, &project, 70);
        let raw = |i: usize, value: f64| {
            f64::from(
                lemon_limiter::params::parse_text(
                    lemon_limiter::params::DEFS[i].text,
                    &value.to_string(),
                    os_engine::effects::dynamics::RANGES[i].0,
                    os_engine::effects::dynamics::RANGES[i].1,
                )
                .unwrap(),
            )
        };
        for (name, value) in &mut r.values {
            match *name {
                "Comp threshold" => *value = raw(8, -24.),
                "Comp ratio" => *value = raw(9, 2.05),
                "Comp knee" => *value = 550.,
                _ => {}
            }
        }
        let fresh =
            processing::prepare(&mut app, vec![(3, vec![r.clone()])], &project, 48000).unwrap();
        let available = app
            .plugins
            .effects
            .iter()
            .find(|a| a.desc.name == "ML Limiter")
            .unwrap();
        let old = available.lib.clone().create(&available.desc.id).unwrap();
        {
            let mut p = old.lock().unwrap();
            // Reproduce the previous assistant's direct use of stored values in
            // the CLAP wire domain. Do NOT use the corrected converter here.
            for &(name, value) in &r.values {
                let info = (0..p.param_count())
                    .filter_map(|i| p.param_info(i))
                    .find(|i| i.name == name)
                    .unwrap();
                assert!(p.set_param(info.id, (value - info.min) / (info.max - info.min)));
            }
            p.idle();
            let bytes = p.save_state().unwrap();
            let state = lemon_limiter::params::fl_from_plugin_state(&bytes).unwrap();
            assert!(
                lemon_limiter::params::field(&state, 10) < 0,
                "old recipe really selects expansion"
            );
            assert!(p.load_state(&bytes) && p.activate(48000));
        }
        let saved = crate::project_plugins::wrapped_slot(&fresh[0].saved).unwrap();
        let state = lemon_limiter::params::fl_from_plugin_state(&saved.state).unwrap();
        assert!(
            lemon_limiter::params::field(&state, 10) > 0,
            "new recipe really selects compression"
        );
        assert_eq!(lemon_limiter::params::field(&state, 11), 550);
        let processed = processing::with_processing(&project, &fresh);
        let run = |p: os_host::SharedPlugin| {
            render(
                engine(&processed, &samples),
                48000,
                RenderRequest {
                    span: None,
                    tracks: vec![3],
                    excerpt_at: None,
                    effects: vec![(3, fresh[0].slot as usize, p as os_engine::SharedEffect)],
                },
                || false,
                |_| {},
            )
            .unwrap()
        };
        let legacy = run(old);
        let corrected = run(fresh[0].plugin.clone());
        assert!(
            legacy.levels[&3].typical > before.levels[&3].typical + 3.,
            "expansion must reproduce a large level rise"
        );
        assert!(
            corrected.levels[&3].typical < before.levels[&3].typical - 2.,
            "compression must lower loud material"
        );
        assert!(corrected.levels[&3].true_peak <= before.levels[&3].true_peak + 0.1);
        if let Ok(path) = std::env::var("MIX_ASSISTANT_TEST_OUTPUT") {
            std::fs::write(PathBuf::from(path).join("signed-ratio-regression.txt"),format!("before {:.2} LUFS / {:.2} dBTP\nold mapping {:.2} LUFS / {:.2} dBTP\ncorrect mapping {:.2} LUFS / {:.2} dBTP\n",before.levels[&3].typical,before.levels[&3].true_peak,legacy.levels[&3].typical,legacy.levels[&3].true_peak,corrected.levels[&3].typical,corrected.levels[&3].true_peak)).unwrap();
        }
    }
}
