use super::*;
use lemon_small_fx::{editor::Extension, params::FxParams};
use nih_plug::prelude::*;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
struct Alloc;
thread_local! {static COUNT:Cell<Option<usize>>=const{Cell::new(None)};}
#[global_allocator]
static ALLOC: Alloc = Alloc;
unsafe impl GlobalAlloc for Alloc {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        COUNT.with(|c| {
            if let Some(n) = c.get() {
                c.set(Some(n + 1))
            }
        });
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        COUNT.with(|c| {
            if let Some(n) = c.get() {
                c.set(Some(n + 1))
            }
        });
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        COUNT.with(|c| {
            if let Some(n) = c.get() {
                c.set(Some(n + 1))
            }
        });
        unsafe { System.realloc(p, l, n) }
    }
}
fn no_alloc(f: impl FnOnce()) {
    COUNT.with(|c| c.set(Some(0)));
    f();
    let n = COUNT.with(|c| c.replace(None).unwrap());
    assert_eq!(n, 0, "audio thread allocation or free");
}
fn set(p: &FxParams, i: usize, v: i32) {
    p.live.set(p.p[i].as_ptr(), p.p[i].preview_normalized(v));
}
fn energy(a: &[f32]) -> f64 {
    a.iter().map(|x| f64::from(*x).powi(2)).sum()
}
fn signal(n: usize) -> Vec<f32> {
    let mut seed = 17u32;
    (0..n)
        .map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            seed as i32 as f32 / i32::MAX as f32 * 0.1
        })
        .collect()
}
fn ir(rate: u32, n: usize) -> impulse::Impulse {
    let mut l = vec![0.; n];
    let mut r = l.clone();
    l[0] = 0.5;
    r[0] = 0.25;
    if n > 300 {
        l[237] = 0.2;
        r[257] = -0.15;
    }
    if n > 5000 {
        l[4901] = -0.1;
        r[4888] = 0.08;
    }
    impulse::Impulse {
        name: "Test IR".into(),
        rate,
        left: l,
        right: r,
    }
}
#[test]
fn convolution_matches_direct_reference_across_partitions_and_reset() {
    for rate in [44100, 48000, 96000, 192000] {
        let source = ir(rate, 6000);
        let mut c = convolver::Convolver::default();
        c.params.shared.replace(Some(source.clone()), false);
        set(&c.params.fx, 0, 1000);
        assert!(c.prepare(rate));
        let input = signal(8000);
        let mut left = input.clone();
        let mut right = input.clone();
        for (l, r) in left.chunks_mut(73).zip(right.chunks_mut(73)) {
            no_alloc(|| c.audio(l, r));
        }
        for (out, impulse) in [(&left, &source.left), (&right, &source.right)] {
            for i in 0..out.len() {
                let expected: f32 = impulse
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| **v != 0.)
                    .map(|(j, h)| if i >= j { input[i - j] * h } else { 0. })
                    .sum();
                assert!(
                    (out[i] - expected).abs() < 2e-6,
                    "{rate} {i} {} {expected}",
                    out[i]
                );
            }
        }
        no_alloc(|| c.reset());
        left.fill(0.);
        right.fill(0.);
        no_alloc(|| c.audio(&mut left, &mut right));
        assert!(left.iter().chain(&right).all(|x| x.abs() < 1e-7));
        set(&c.params.fx, 0, 0);
        c.reset();
        left.copy_from_slice(&input);
        right.copy_from_slice(&input);
        c.audio(&mut left, &mut right);
        assert_eq!(left, input);
    }
}
#[test]
fn convolution_predelay_resample_and_tone() {
    let mut c = convolver::Convolver::default();
    let mut source = ir(48000, 4800);
    source.left.fill(0.);
    source.right.fill(0.);
    source.left[2400] = 0.5;
    source.right[2400] = 0.25;
    let resampled = source.at_rate(96000);
    let peak = resampled[0]
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .unwrap()
        .0;
    assert_eq!(peak, 4800);
    assert!((resampled[0].iter().sum::<f32>() - 0.5).abs() < 0.005);
    c.params.shared.replace(Some(source), false);
    set(&c.params.fx, 0, 1000);
    set(&c.params.fx, 1, 100);
    assert!(c.prepare(48000));
    let mut l = vec![0.; 4096];
    let mut r = l.clone();
    l[0] = 1.;
    r[0] = 1.;
    c.audio(&mut l, &mut r);
    assert!((l[2880] - 0.5).abs() < 1e-6);
    assert!((r[2880] - 0.25).abs() < 1e-6);
    set(&c.params.fx, 1, 0);
    set(&c.params.fx, 2, 200);
    set(&c.params.fx, 3, 2000);
    c.reset();
    l = signal(8192);
    r = l.clone();
    no_alloc(|| c.audio(&mut l, &mut r));
    assert!(l.iter().all(|v| v.is_finite()));
    assert!(energy(&l) > 0.);
}
fn wav() -> Vec<u8> {
    let mut b = b"RIFF".to_vec();
    b.extend(40u32.to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(48000u32.to_le_bytes());
    b.extend(192000u32.to_le_bytes());
    b.extend(4u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(4u32.to_le_bytes());
    b.extend(16384i16.to_le_bytes());
    b.extend((-8192i16).to_le_bytes());
    b
}
#[test]
fn impulse_import_embedded_reload_undo_and_invalid_files() {
    let bytes = wav();
    let impulse = impulse::Impulse::decode(&bytes, "Stereo test.wav".into()).unwrap();
    assert_eq!(impulse.left, [0.5]);
    assert_eq!(impulse.right, [-0.25]);
    assert!(impulse::Impulse::decode(&bytes[..30], "bad".into()).is_err());
    let mut bad = bytes.clone();
    bad[32] = 0;
    assert!(impulse::Impulse::decode(&bad, "bad".into()).is_err());
    let saved = impulse.save();
    let restore = impulse::Impulse::load(&saved).unwrap();
    assert_eq!(restore.left, impulse.left);
    let mut c = convolver::Convolver::default();
    let panel = convolver::Panel {
        shared: c.params.shared.clone(),
    };
    let before = c.params.serialize_fields();
    c.params.shared.replace(Some(impulse), true);
    let changes = panel.take_state_edits();
    assert_eq!(changes.len(), 1);
    let patch: serde_json::Value = serde_json::from_str(&changes[0]).unwrap();
    assert_eq!(patch["fields"]["impulse"], "null");
    let state = c.params.serialize_fields();
    c.params.deserialize_fields(&before);
    assert!(panel.take_state_edits().is_empty());
    assert_eq!(c.params.serialize_fields(), before);
    c.params.deserialize_fields(&state);
    assert_eq!(c.params.serialize_fields(), state);
    set(&c.params.fx, 0, 1000);
    assert!(c.prepare(48000));
    let mut l = [1., 0., 0.];
    let mut r = l;
    no_alloc(|| c.audio(&mut l, &mut r));
    assert!((l[0] - 0.5).abs() < 1e-6);
    assert!((r[0] + 0.25).abs() < 1e-6);
}
#[test]
fn live_ir_replacement_does_not_allocate_or_destroy_on_audio_thread() {
    let mut c = convolver::Convolver::default();
    set(&c.params.fx, 0, 1000);
    c.prepare(48000);
    for generation in 0..5 {
        let mut source = ir(48000, 10);
        source.left[0] = generation as f32 * 0.1;
        source.right[0] = source.left[0];
        c.params.shared.replace(Some(source), true);
        std::thread::sleep(std::time::Duration::from_millis(60));
        let mut l = [0.1; 2048];
        let mut r = l;
        no_alloc(|| c.audio(&mut l, &mut r));
        assert!(l.iter().all(|v| v.is_finite() && v.abs() <= 0.101));
    }
    std::thread::sleep(std::time::Duration::from_millis(40));
    let mut l = [0.1; 2048];
    let mut r = l;
    no_alloc(|| c.audio(&mut l, &mut r));
    assert!((l[2047] - 0.04).abs() < 1e-5);
}
#[test]
fn vocoder_carrier_modulator_routing_and_automation() {
    for rate in [8000., 44100., 48000., 96000., 192000.] {
        let mut v = vocoder::Vocoder::default();
        v.prepare(rate);
        let input = signal(8192);
        let mut l = input.clone();
        let mut r = l.clone();
        no_alloc(|| v.audio(&mut l, &mut r, None));
        assert!(energy(&l) > 1e-6);
        assert_eq!(l, r);
        no_alloc(|| v.reset());
        l.fill(0.);
        r.fill(0.);
        v.audio(&mut l, &mut r, None);
        assert_eq!(energy(&l), 0.);
        set(&v.params, 0, 1);
        set(&v.params, 6, 0);
        v.reset();
        l.copy_from_slice(&input);
        r.copy_from_slice(&input);
        v.audio(&mut l, &mut r, None);
        assert_eq!(
            energy(&l),
            0.,
            "no carrier without sibilance must be silent"
        );
        let carrier = signal(8192);
        let silent = vec![0.; 8192];
        v.reset();
        l.copy_from_slice(&input);
        r.copy_from_slice(&input);
        no_alloc(|| v.audio(&mut l, &mut r, Some((&carrier, &silent))));
        assert!(energy(&l) > 1e-6);
        assert_eq!(energy(&r), 0.);
        for step in 0..24 {
            set(&v.params, 0, step % 2);
            set(&v.params, 2, step % 2);
            set(&v.params, 3, if step % 2 == 0 { 50 } else { 1000 });
            set(&v.params, 4, if step % 2 == 0 { 0 } else { 1000 });
            set(&v.params, 5, if step % 2 == 0 { 0 } else { 1000 });
            l.copy_from_slice(&input);
            r.copy_from_slice(&input);
            no_alloc(|| v.audio(&mut l, &mut r, Some((&carrier, &carrier))));
            assert!(l.iter().all(|x| x.is_finite() && x.abs() <= 1.001));
        }
        set(&v.params, 7, 0);
        v.reset();
        l.copy_from_slice(&input);
        r.copy_from_slice(&input);
        v.audio(&mut l, &mut r, None);
        assert_eq!(l, input);
    }
}
mod a {
    type Effect = super::convolver::Convolver;
    os_plugkit::linked_clap_entry!(Effect);
}
mod b {
    type Effect = super::vocoder::Vocoder;
    os_plugkit::linked_clap_entry!(Effect);
}
#[test]
fn actual_host_state_and_sidechain() {
    use os_engine::HostedEffect as _;
    use os_host::Plugin as _;
    for (entry, name) in [
        (&a::LINKED_CLAP_ENTRY, "ML Convolver"),
        (&b::LINKED_CLAP_ENTRY, "ML Vocoder"),
    ] {
        let lib = unsafe {
            os_clap::Library::from_entry(
                entry as *const _ as *const std::ffi::c_void,
                std::path::Path::new("ML space.clap"),
            )
        }
        .unwrap();
        assert_eq!(lib.plugins[0].name, name);
        let mut p = os_clap::Plugin::new(&lib, &lib.plugins[0].id).unwrap();
        assert!(p.activate(48000));
        assert_eq!(p.latency(), 0);
        let id = p.param_info(0).unwrap().id;
        p.set_param(id, 1.);
        if name == "ML Vocoder" {
            p.set_param(p.param_info(6).unwrap().id, 0.);
        }
        let mut l = signal(4096);
        let mut r = l.clone();
        let key = signal(4096);
        let silent = vec![0.; 4096];
        p.process_fx_keyed(48000, &mut l, &mut r, (&key, &silent));
        p.idle();
        assert!(energy(&l) > 1e-6);
        if name == "ML Vocoder" {
            assert_eq!(energy(&r), 0., "host must preserve stereo carrier routing");
        }
        let saved = p.save_state().unwrap();
        let mut restored = os_clap::Plugin::new(&lib, &lib.plugins[0].id).unwrap();
        assert!(restored.load_state(&saved));
        assert!(restored.activate(48000));
        assert_eq!(restored.param_value(id), p.param_value(id));
    }
}
#[test]
fn editor_controls_and_actual_renders() {
    struct Model(&'static lemon_small_fx::Spec);
    impl lemon_small_fx::view::Model for Model {
        fn norm(&self, id: &str) -> f32 {
            let s = &self.0.settings[self.0.index(id).unwrap()];
            (s.start - s.range.0) as f32 / (s.range.1 - s.range.0) as f32
        }
        fn plain(&self, id: &str) -> f32 {
            self.0.settings[self.0.index(id).unwrap()].start as f32
        }
        fn text(&self, id: &str) -> String {
            let i = self.0.index(id).unwrap();
            (self.0.text)(i, self.0.settings[i].start)
        }
    }
    for spec in [&specs::CONVOLVER, &specs::VOCODER] {
        for (i, s) in spec.settings.iter().enumerate() {
            for v in [s.range.0, s.start, s.range.1] {
                assert_eq!(
                    (spec.parse)(i, &(spec.text)(i, v)),
                    Some(v),
                    "{} {}",
                    spec.name,
                    s.id
                );
            }
        }
        let c = convolver::Convolver::default();
        let v = vocoder::Vocoder::default();
        let panel: Box<dyn Extension> = if spec.name == "ML Convolver" {
            Box::new(convolver::Panel {
                shared: c.params.shared.clone(),
            })
        } else {
            Box::new(vocoder::Panel {
                readings: v.readings.clone(),
                params: v.params.clone(),
            })
        };
        if let Ok(dir) = std::env::var("ML_TOOLS_RENDER_DIR") {
            for scale in [1., 1.5, 2.] {
                let mut canvas = os_ui::Canvas::new(
                    (spec.size.0 as f32 * scale) as u32,
                    (spec.size.1 as f32 * scale) as u32,
                    scale,
                )
                .unwrap();
                let model = Model(spec);
                let hits = lemon_small_fx::view::paint(
                    spec,
                    &mut canvas,
                    &os_theme::Palette::default(),
                    &model,
                    &Default::default(),
                    "",
                );
                assert!(hits.len() >= spec.settings.len());
                panel.paint(&mut canvas, &model);
                canvas
                    .pixmap
                    .save_png(std::path::Path::new(&dir).join(format!("{} {scale}.png", spec.name)))
                    .unwrap();
            }
        }
    }
}
