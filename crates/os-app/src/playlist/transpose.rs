//! Transpose selected Playlist placements in one edit, isolating shared sources.
use crate::{
    app::App,
    history::{Snap, Tail},
    playlist as pl,
};
use os_model::{ClipSource, stretch_mode};
use std::collections::{HashMap, HashSet};

enum Pitch {
    Audio(usize, [u32; 4]),
    Pattern(u16),
}

pub(super) fn run(app: &mut App, semitones: i32) -> Option<String> {
    let s = &mut app.state;
    let mut selected = s.playlist.sel.clone();
    selected.sort_unstable();
    selected.dedup();
    let mut groups = Vec::<(ClipSource, Vec<usize>)>::new();
    let mut by_source = HashMap::new();
    let mut skipped = 0;
    for i in selected {
        let Some(c) = pl::clips(s).get(i) else {
            continue;
        };
        let usable = match c.source {
            ClipSource::Pattern(n) => s.project.pattern(n).is_some_and(|p| !p.notes.is_empty()),
            ClipSource::Channel(id) => s
                .project
                .channel(id)
                .is_some_and(|ch| ch.kind.is_sampler() && !crate::ara::is(ch)),
        };
        if !usable || pl::track_info(s, c.track).is_some_and(|t| t.locked || t.frozen.is_some()) {
            skipped += 1;
            continue;
        }
        let group = *by_source.entry(c.source).or_insert_with(|| {
            groups.push((c.source, Vec::new()));
            groups.len() - 1
        });
        groups[group].1.push(i);
    }
    if groups.is_empty() {
        return Some("Select an editable audio clip or MIDI pattern to transpose".into());
    }

    let mut uses = HashMap::<ClipSource, usize>::new();
    for c in s.project.arrangements.iter().flat_map(|a| {
        a.clips
            .iter()
            .chain(a.tracks.iter().filter_map(|t| t.frozen.as_ref()).flatten())
    }) {
        *uses.entry(c.source).or_default() += 1;
    }
    let mut channel_id = s
        .project
        .channels
        .iter()
        .map(|ch| ch.index)
        .max()
        .unwrap_or(0);
    let mut pattern_id = s
        .project
        .patterns
        .iter()
        .map(|p| p.number)
        .max()
        .unwrap_or(0);
    let mut plan = Vec::new();
    let mut count = 0;
    for (source, indices) in groups {
        let shared = uses.get(&source).copied().unwrap_or(0) > indices.len();
        let (pitch, copy) = match source {
            ClipSource::Pattern(n) => {
                let p = s.project.pattern(n).unwrap();
                if p.notes
                    .iter()
                    .any(|note| !(0..=131).contains(&(i32::from(note.key) + semitones)))
                {
                    return Some(
                        "Transpose would move a selected note outside the supported pitch range"
                            .into(),
                    );
                }
                let copy = if shared {
                    if pattern_id >= u16::MAX - 0x5000 {
                        return Some("No free pattern number for a transposed copy".into());
                    }
                    pattern_id += 1;
                    Some(ClipSource::Pattern(pattern_id))
                } else {
                    None
                };
                (Pitch::Pattern(n), copy)
            }
            ClipSource::Channel(id) => {
                let row = s
                    .project
                    .channels
                    .iter()
                    .position(|ch| ch.index == id)
                    .unwrap();
                let ch = &s.project.channels[row];
                let mut stretch = ch.sound.stretch;
                let mut cents = stretch[1] as i32;
                // Auto preserves the clip's duration. Carry over any pitch
                // that was already coming from the old resampling rate.
                if ch.sound.stretch_mode() == stretch_mode::RESAMPLE {
                    let duration = if ch.sound.stretch_time().is_some() {
                        let seconds = pl::sample_secs(s, ch);
                        if seconds <= 0.0 {
                            return Some("Load the fitted audio clip before transposing it".into());
                        }
                        seconds
                    } else {
                        0.0
                    };
                    let factor = ch
                        .sound
                        .stretch_factor(duration, s.project.tick_tempo(s.tempo));
                    stretch[3] = stretch_mode::AUTO as u32;
                    cents = (-1200.0 * factor.log2()).round() as i32;
                }
                let cents = cents.saturating_add(semitones * 100);
                if !(-1200..=1200).contains(&cents) {
                    return Some(
                        "Transpose would exceed the audio clip's pitch shift range (±12 semitones)"
                            .into(),
                    );
                }
                stretch[1] = cents as u32;
                let notes_use_source = s
                    .project
                    .patterns
                    .iter()
                    .any(|p| p.notes.iter().any(|n| n.channel == id));
                let copy = if shared || notes_use_source {
                    use os_model::target::{Target, decode};
                    let targets_audio = |raw| {
                        matches!(decode(raw),
                        Target::ChannelVolume(ch) | Target::ChannelPan(ch) | Target::ChannelPitch(ch) if ch == id)
                    };
                    let automated =
                        s.project
                            .automations
                            .iter()
                            .any(|a| a.targets.iter().copied().any(targets_audio))
                            || s.project
                                .init_controls
                                .iter()
                                .any(|c| targets_audio(c.target))
                            || s.project.peak_links.iter().any(|l| targets_audio(l.target))
                            || s.project.patterns.iter().any(|p| {
                                p.controls.iter().any(|c| usize::from(c.channel()) == row)
                            });
                    if automated {
                        return Some("This shared audio source has channel automation. Make an independent clip before transposing it.".into());
                    }
                    if channel_id >= 0x4fff {
                        return Some("No free audio channel number for a transposed copy".into());
                    }
                    channel_id += 1;
                    Some(ClipSource::Channel(channel_id))
                } else {
                    None
                };
                (Pitch::Audio(row, stretch), copy)
            }
        };
        count += indices.len();
        plan.push((indices, pitch, copy));
    }

    let mut undo = Vec::new();
    if plan
        .iter()
        .any(|(_, pitch, _)| matches!(pitch, Pitch::Pattern(_)))
    {
        undo.push(Snap::Patterns {
            patterns: s.project.patterns.clone(),
            current: s.pattern,
        });
    }
    if plan
        .iter()
        .any(|(_, _, copy)| matches!(copy, Some(ClipSource::Channel(_))))
    {
        undo.push(Snap::Tail(Box::new(Tail::now(s, s.project.channels.len()))));
        undo.push(Snap::Vocals(s.project.vocal_edits.clone()));
        undo.push(Snap::AudioEdits(s.project.audio_edits.clone()));
    }
    for (_, pitch, copy) in &plan {
        if let Pitch::Audio(row, _) = pitch
            && copy.is_none()
            && let Some(sample) = Snap::sample(s, *row)
        {
            // Restore the hosted sampler's stretch settings alongside the project data.
            undo.push(sample);
        }
    }
    pl::snapshot_with(s, "Transpose Playlist clips", Snap::Several(undo));
    for (indices, pitch, copy) in plan {
        match (pitch, copy) {
            (Pitch::Pattern(n), copy) => {
                let target = if let Some(ClipSource::Pattern(new)) = copy {
                    let mut p = s.project.pattern(n).unwrap().clone();
                    p.name = Some(crate::patterns::clone_name(s, n));
                    p.number = new;
                    s.project.patterns.push(p);
                    new
                } else {
                    n
                };
                crate::patterns::transpose(s, target, semitones);
            }
            (Pitch::Audio(row, stretch), copy) => {
                if let Some(ClipSource::Channel(id)) = copy {
                    let mut ch = s.project.channels[row].clone();
                    let old = ch.index;
                    ch.index = id;
                    ch.name = Some(crate::rack::clone_name(s, row));
                    ch.sound.stretch = stretch;
                    s.project.channels.push(ch);
                    crate::vocal::clone_source(s, old, id);
                } else {
                    let sound = &mut s.project.channels[row].sound;
                    sound.stretch = stretch;
                    app.plugins.set_sampler_stretch(row, sound);
                }
            }
        }
        if let Some(copy) = copy {
            for i in indices {
                pl::arr_mut(s).clips[i].source = copy;
            }
        }
    }
    s.piano.dirty = true;
    app.invalidate();
    let unit = if semitones.abs() == 1 {
        "semitone"
    } else {
        "semitones"
    };
    let suffix = if skipped == 0 {
        String::new()
    } else {
        format!(" ({skipped} skipped)")
    };
    Some(format!(
        "Transposed {count} {} {semitones:+} {unit}{suffix}",
        if count == 1 { "clip" } else { "clips" }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use os_model::{ChannelKind, Clip, Note};
    use winit::keyboard::{Key, NamedKey};

    #[test]
    fn playlist_transpose_shortcuts_isolate_shared_sources_and_undo_together() {
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state.z = vec![crate::app::Win::Playlist];
        let p = &mut app.state.project;
        p.patterns[0].notes = vec![Note {
            key: 60,
            length: 24,
            ..Note::step(1, 4)
        }];
        p.channels[0].kind = ChannelKind::AudioClip;
        let (audio, midi) = (ClipSource::Channel(0), ClipSource::Pattern(1));
        let mut trimmed = Clip::new(0, 96, 0, audio);
        trimmed.window = Some((250.0, 750.0));
        trimmed.set_fade_in(Some((12.0, 0.2)));
        pl::arr_mut(&mut app.state).clips = vec![
            trimmed,
            Clip::new(96, 96, 0, audio),
            Clip::new(192, 96, 0, audio),
            Clip::new(0, 384, 1, midi),
            Clip::new(384, 384, 1, midi),
        ];
        let before = app.state.project.clone();
        app.state.playlist.sel = vec![0, 1, 3];
        app.key(&Key::Named(NamedKey::ArrowUp), false, true);
        let selected = pl::clips(&app.state);
        let (ClipSource::Channel(new_audio), ClipSource::Pattern(new_midi)) =
            (selected[0].source, selected[3].source)
        else {
            panic!("selected sources")
        };
        assert_ne!(new_audio, 0);
        assert_ne!(new_midi, 1);
        assert_eq!(selected[0].source, selected[1].source);
        for (i, (old, new)) in before
            .arrangement()
            .unwrap()
            .clips
            .iter()
            .zip(selected)
            .enumerate()
        {
            let mut expected = old.clone();
            if [0, 1, 3].contains(&i) {
                expected.source = new.source;
            }
            assert_eq!(&expected, new, "only the selected source can change");
        }
        let pitch = |a: &App| {
            (
                a.state
                    .project
                    .channel(new_audio)
                    .unwrap()
                    .sound
                    .stretch_cents(),
                a.state.project.pattern(new_midi).unwrap().notes[0].key,
            )
        };
        assert_eq!(pitch(&app), (100.0, 61));
        assert_eq!(
            app.state.project.channel(new_audio).unwrap().mixer_track,
            before.channels[0].mixer_track
        );
        assert_eq!(app.state.project.pattern(1), before.pattern(1));
        assert_eq!(app.state.project.channel(0), before.channel(0));

        let transposed = app.state.project.clone();
        assert_eq!(
            crate::history::undo(&mut app.state),
            Some("Transpose Playlist clips")
        );
        assert_eq!(app.state.project, before);
        assert_eq!(
            crate::history::redo(&mut app.state),
            Some("Transpose Playlist clips")
        );
        assert_eq!(app.state.project, transposed);
        app.state.playlist.sel = vec![0, 1, 3];
        for (key, ctrl, expected) in [
            (NamedKey::ArrowDown, false, (0.0, 60)),
            (NamedKey::ArrowUp, true, (1200.0, 72)),
            (NamedKey::ArrowDown, true, (0.0, 60)),
            (NamedKey::ArrowDown, true, (-1200.0, 48)),
        ] {
            app.key(&Key::Named(key), ctrl, true);
            assert_eq!(pitch(&app), expected);
        }
        let limited = app.state.project.clone();
        app.key(&Key::Named(NamedKey::ArrowDown), false, true);
        assert_eq!(
            app.state.project, limited,
            "an out-of-range audio step cannot partially transpose MIDI"
        );
        let saved = os_flp::project::create(&app.state.project)
            .unwrap()
            .to_bytes()
            .unwrap();
        let reopened = os_flp::project::load(&os_flp::FlpFile::parse(&saved).unwrap()).unwrap();
        assert_eq!(reopened.pattern(new_midi).unwrap().notes[0].key, 48);
        assert_eq!(
            reopened.channel(new_audio).unwrap().sound.stretch_cents(),
            -1200.0
        );

        app.state.playlist.sel = vec![3];
        app.key(&Key::Named(NamedKey::ArrowUp), true, true);
        assert_eq!(
            pitch(&app),
            (-1200.0, 60),
            "MIDI alone does not transpose audio"
        );
        let snapshot = app.state.project.clone();
        app.key(&Key::Named(NamedKey::ArrowDown), true, false);
        assert_eq!(
            app.state.project, snapshot,
            "Ctrl without Alt retains track navigation"
        );
        crate::settings_file::set_flag(pl::TRANSPOSE_VISIBILITY, false);
        let options = pl::default_options_menu(&app.state, 0., 0.);
        assert!(
            !options.items[0]
                .children
                .iter()
                .any(|p| p.label == "Transpose selected clips")
        );
        app.key(&Key::Named(NamedKey::ArrowDown), false, true);
        assert_eq!(pitch(&app), (-1200.0, 59));
    }

    #[test]
    fn playlist_transpose_changes_hosted_audio_pitch_without_changing_timing() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("transpose-tone.wav");
        let rate = 48_000;
        let tone: Vec<f32> = (0..rate * 2)
            .flat_map(|i| {
                let sample = (std::f32::consts::TAU * 220.0 * i as f32 / rate as f32).sin() * 0.2;
                [sample, sample]
            })
            .collect();
        os_engine::wav::write_f32_stereo(&mut std::fs::File::create(&path).unwrap(), rate, &tone)
            .unwrap();
        let mut app = App::new(None);
        app.state.project = crate::app::empty_project();
        app.plugins = crate::plugins::Plugins::builtins();
        app.state.tempo = 120.0;
        app.state.project.tempo = Some(120.0);
        pl::add_audio_clip(&mut app.state, path.to_str().unwrap(), 0, 0).unwrap();
        app.sync_samplers(None);
        app.plugins.set_rate(rate);
        app.plugins.set_offline(true);
        assert!(
            !app.plugins.instruments().is_empty(),
            "{:?}",
            app.state.message
        );
        let clip = pl::clips(&app.state)[0].clone();
        let render = |a: &App| {
            os_engine::Engine::with_instruments(
                &a.state.project,
                &mut crate::transport::host_library(None),
                rate,
                os_engine::PlayMode::Song,
                &a.plugins.instruments(),
            )
            .render_all(0)
        };
        let freq = |frames: &[f32]| {
            let start = rate as usize / 4;
            let end = rate as usize * 5 / 4;
            frames[start * 2..end * 2]
                .chunks_exact(2)
                .map(|lr| lr[0])
                .collect::<Vec<_>>()
                .windows(2)
                .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
                .count() as f64
        };
        let before = render(&app);
        app.state.playlist.sel = vec![0];
        assert!(run(&mut app, 12).unwrap().starts_with("Transposed 1 clip"));
        let after = render(&app);
        assert_eq!(&pl::clips(&app.state)[0], &clip);
        assert_eq!(after.len(), before.len());
        assert!((freq(&before) - 220.0).abs() < 3.0);
        assert!(
            (freq(&after) - 440.0).abs() < 5.0,
            "heard {} Hz",
            freq(&after)
        );

        let sound = &mut app.state.project.channels[0].sound;
        sound.stretch = [0, 0, 10000, stretch_mode::RESAMPLE as u32];
        assert!(app.plugins.set_sampler_stretch(0, sound));
        let before = render(&app);
        assert!((freq(&before) - 110.0).abs() < 3.0);
        assert!(run(&mut app, 12).unwrap().starts_with("Transposed 1 clip"));
        let after = render(&app);
        assert_eq!(after.len(), before.len());
        assert!(
            (freq(&after) - 220.0).abs() < 5.0,
            "heard {} Hz",
            freq(&after)
        );
    }
}
