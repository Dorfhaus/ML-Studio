use super::*;
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
    assert_eq!(n, 0, "audio callback allocation/free");
}
fn set(p: &FxParams, i: usize, v: i32) {
    p.live.set(p.p[i].as_ptr(), p.p[i].preview_normalized(v));
}
fn tone(n: usize, rate: f64, f: f64, a: f32) -> Vec<f32> {
    (0..n)
        .map(|i| (std::f64::consts::TAU * f * i as f64 / rate).sin() as f32 * a)
        .collect()
}
#[test]
fn vocal_reuses_mixer_engine_and_fixed_dry_latency() {
    for rate in [8000, 44100, 48000, 96000, 192000] {
        for voice in [0, 1] {
            let p = FxParams::new(&specs::VOCAL);
            set(&p, 0, 1000);
            set(&p, 3, voice);
            let mut d = vocal::Vocal::default();
            d.prepare(rate as f32, &p);
            let delay = d.latency() as usize;
            assert!(delay > 0 && delay < rate as usize / 10);
            let input = tone(
                rate as usize * 2,
                rate as f64,
                if voice == 0 { 224. } else { 450. },
                0.2,
            );
            let mut expected: Vec<_> = input.iter().flat_map(|&x| [x, x * 0.5]).collect();
            let mut native = os_dsp::snaptune::SnapTune::new(rate, [65535, 0, 0, voice as u16], 0);
            native.process(&mut expected);
            let mut l = input.clone();
            let mut r: Vec<_> = input.iter().map(|v| v * 0.5).collect();
            no_alloc(|| d.audio(&mut l, &mut r, &p));
            assert_eq!(native.latency() as usize, delay);
            assert!(
                l.iter()
                    .zip(expected.chunks_exact(2))
                    .all(|(x, y)| (x - y[0]).abs() < 1e-6)
            );
            assert!(
                r.iter()
                    .zip(expected.chunks_exact(2))
                    .all(|(x, y)| (x - y[1]).abs() < 1e-6)
            );
            if rate >= 44100 {
                let z: Vec<_> = l[rate as usize..]
                    .windows(2)
                    .enumerate()
                    .filter_map(|(i, w)| {
                        if w[0] <= 0. && w[1] > 0. {
                            Some(i as f64)
                        } else {
                            None
                        }
                    })
                    .collect();
                let hz = (z.len() - 1) as f64 * rate as f64 / (z.last().unwrap() - z[0]);
                assert!(
                    (hz - if voice == 0 { 220. } else { 440. }).abs() < 1.5,
                    "{rate} {voice} {hz}"
                );
            }
            for (index, value) in [(0, 0), (4, 0)] {
                set(&p, index, value);
                no_alloc(|| d.reset(&p));
                l.copy_from_slice(&input);
                r.copy_from_slice(&input);
                no_alloc(|| d.audio(&mut l, &mut r, &p));
                assert!(l[..delay].iter().all(|v| *v == 0.));
                assert_eq!(&l[delay..], &input[..input.len() - delay]);
                assert_eq!(l, r);
                assert_eq!(d.latency() as usize, delay);
            }
        }
    }
}
#[test]
fn time_gate_repeat_dry_transport_and_editor_cancel() {
    let p = Arc::new(FxParams::new(&specs::TIME));
    set(&p, 3, 0);
    let mut d = time::Time::default();
    d.prepare(8000., &p);
    let mut l = vec![0.2; 16000];
    let mut r = vec![0.1; 16000];
    no_alloc(|| d.audio(&mut l, &mut r, &p));
    for step in 0..16 {
        assert!((l[step * 1000 + 200] - if step % 2 == 0 { 0.2 } else { 0. }).abs() < 1e-6);
    }
    let panel = d.panel(p.clone());
    assert!(panel.click(24., 332., None));
    for i in 0..16 {
        assert_eq!(p.v(i + 6), 1000);
    }
    assert!(panel.click(25., 500., None));
    assert!(p.v(6) < 1000);
    assert!(panel.character('\u{1b}'));
    assert_eq!(p.v(6), 1000);
    assert!(!panel.dragging());
    set(&p, 1, 1);
    set(&p, 2, 4);
    d.reset(&p);
    let original: Vec<f32> = (0..16000).map(|i| i as f32 / 16000.).collect();
    l.copy_from_slice(&original);
    r.fill(0.);
    no_alloc(|| d.audio(&mut l, &mut r, &p));
    for i in 0..16000 {
        assert!(
            (l[i] - original[i % 4000]).abs() < 1e-5,
            "repeat sample {i}: {}",
            l[i]
        );
    }
    assert!(r.iter().all(|v| *v == 0.));
    set(&p, 4, 0);
    d.reset(&p);
    l.copy_from_slice(&original);
    r.copy_from_slice(&original);
    no_alloc(|| d.audio(&mut l, &mut r, &p));
    assert_eq!(l, original);
    assert_eq!(r, original);
    // Host beat position, tempo and odd-sized blocks must produce the same result.
    set(&p, 4, 1000);
    set(&p, 1, 0);
    for i in 0..16 {
        set(&p, 6 + i, if i % 2 == 0 { 1000 } else { 0 });
    }
    let render = |block: usize| {
        use os_engine::HostedEffect as _;
        use os_host::Plugin as _;
        let lib = unsafe {
            os_clap::Library::from_entry(
                &a::LINKED_CLAP_ENTRY as *const _ as *const std::ffi::c_void,
                std::path::Path::new("Time.clap"),
            )
        }
        .unwrap();
        let mut d = os_clap::Plugin::new(&lib, &lib.plugins[0].id).unwrap();
        assert!(d.activate(8000));
        d.set_param(d.param_info(3).unwrap().id, 0.);
        let mut l = vec![0.2; 24000];
        let mut r = l.clone();
        for start in (0..l.len()).step_by(block) {
            let n = block.min(l.len() - start);
            d.transport_fx(&os_engine::TransportInfo {
                playing: true,
                tempo: 80.,
                frame: start as u64,
                beats: start as f64 / 8000. * 80. / 60.,
                bar_start: 0.,
                bar: 0,
                numerator: 4,
                denominator: 4,
                looping: None,
            });
            d.process_fx(8000, &mut l[start..start + n], &mut r[start..start + n]);
        }
        l
    };
    let a = render(24000);
    let b = render(137);
    assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-5));
}
pub(crate) fn linear(rate: u32, gain: f32) -> amp::Asset {
    amp::Asset{name:"Synthetic linear test".into(),model:serde_json::json!({"version":"0.5.4","architecture":"Linear","config":{"receptive_field":1,"bias":false},"weights":[gain],"sample_rate":rate}).to_string(),..Default::default()}
}
#[test]
fn amp_latency_resampling_stereo_reset_cabinet_and_embedded_state() {
    for rate in [8000, 44100, 48000, 96000, 192000] {
        for model_rate in [32000, 48000, 192000] {
            let mut a = amp::Amp::default();
            a.params.shared.replace(linear(model_rate, 0.5));
            assert!(a.prepare(rate));
            let source = tone(rate as usize / 2, rate as f64, 300., 0.1);
            let mut l = source.clone();
            let mut r = vec![0.; l.len()];
            no_alloc(|| a.audio(&mut l, &mut r));
            assert!(r.iter().all(|v| v.abs() < 1e-6));
            let start = (rate as usize / 4).max(256);
            let expected = &source[start - 256..source.len() - 256];
            let err = l[start..]
                .iter()
                .zip(expected)
                .map(|(a, b)| (*a - *b * 0.5).abs())
                .fold(0f32, f32::max);
            assert!(err < 0.003, "{rate}/{model_rate}: {err}");
            let first = l.clone();
            no_alloc(|| a.reset());
            l.copy_from_slice(&source);
            no_alloc(|| a.audio(&mut l, &mut r));
            assert!(
                l.iter().zip(first).all(|(x, y)| (x - y).abs() < 1e-6),
                "repeatable reset {rate}/{model_rate}"
            );
            set(&a.params.fx, 2, 0);
            a.reset();
            l.copy_from_slice(&source);
            no_alloc(|| a.audio(&mut l, &mut r));
            assert_eq!(&l[256..], &source[..source.len() - 256]);
            assert!(l[..256].iter().all(|v| *v == 0.));
        }
    }
    let mut a = amp::Amp::default();
    let mut asset = linear(48000, 0.5);
    asset.cab = ml_space_core::impulse::Impulse {
        name: "Synthetic cab".into(),
        rate: 48000,
        left: vec![0.25],
        right: vec![0.75],
    }
    .save();
    asset.cab_name = "Synthetic cab".into();
    a.params.shared.replace(asset);
    a.prepare(48000);
    let state = a.params.serialize_fields();
    let mut b = amp::Amp::default();
    b.params.deserialize_fields(&state);
    b.prepare(48000);
    assert_eq!(state, b.params.serialize_fields());
    for effect in [&mut a, &mut b] {
        let mut l = [0.; 512];
        let mut r = l;
        l[0] = 0.4;
        r[0] = 0.4;
        no_alloc(|| effect.audio(&mut l, &mut r));
        assert!((l[256] - 0.05).abs() < 1e-6);
        assert!((r[256] - 0.15).abs() < 1e-6);
    }
}
mod a {
    type Effect = super::time::TimeShaper;
    os_plugkit::linked_clap_entry!(Effect);
}
mod b {
    type Effect = super::vocal::VocalTune;
    os_plugkit::linked_clap_entry!(Effect);
}
mod c {
    type Effect = super::amp::Amp;
    os_plugkit::linked_clap_entry!(Effect);
}
#[test]
fn actual_host_state_latency_and_synced_echo() {
    use os_engine::HostedEffect as _;
    use os_host::Plugin as _;
    for (entry, name) in [
        (&a::LINKED_CLAP_ENTRY, "ML Time Shaper"),
        (&b::LINKED_CLAP_ENTRY, "ML Vocal Tune"),
        (&c::LINKED_CLAP_ENTRY, "ML Amp"),
    ] {
        let lib = unsafe {
            os_clap::Library::from_entry(
                entry as *const _ as *const std::ffi::c_void,
                std::path::Path::new("ML utility.clap"),
            )
        }
        .unwrap();
        assert_eq!(lib.plugins[0].name, name);
        let mut p = os_clap::Plugin::new(&lib, &lib.plugins[0].id).unwrap();
        assert!(p.activate(48000));
        assert_eq!(
            p.latency(),
            if name == "ML Amp" {
                256
            } else if name == "ML Vocal Tune" {
                os_dsp::snaptune::SnapTune::new(48000, [65535, 0, 0, 0], 0).latency()
            } else {
                0
            }
        );
        for i in 0..p.param_count() {
            let id = p.param_info(i).unwrap().id;
            p.set_param(id, 0.37);
        }
        let mut l = [0.; 512];
        let mut r = l;
        p.process_fx(48000, &mut l, &mut r);
        p.idle();
        let saved = p.save_state().unwrap();
        let mut restored = os_clap::Plugin::new(&lib, &lib.plugins[0].id).unwrap();
        assert!(restored.load_state(&saved));
        assert!(restored.activate(48000));
        for i in 0..p.param_count() {
            let id = p.param_info(i).unwrap().id;
            assert_eq!(p.param_value(id), restored.param_value(id));
        }
    }
}
#[test]
fn parameter_text_and_stock_editor_renders() {
    struct Model(&'static Spec);
    impl lemon_small_fx::view::Model for Model {
        fn norm(&self, id: &str) -> f32 {
            let p = &self.0.settings[self.0.index(id).unwrap()];
            (p.start - p.range.0) as f32 / (p.range.1 - p.range.0) as f32
        }
        fn plain(&self, id: &str) -> f32 {
            self.0.settings[self.0.index(id).unwrap()].start as f32
        }
        fn text(&self, id: &str) -> String {
            let i = self.0.index(id).unwrap();
            (self.0.text)(i, self.0.settings[i].start)
        }
    }
    for spec in [&specs::TIME, &specs::VOCAL, &specs::AMP] {
        for (i, p) in spec.settings.iter().enumerate() {
            for v in [p.range.0, p.start, p.range.1] {
                assert_eq!(
                    (spec.parse)(i, &(spec.text)(i, v)),
                    Some(v),
                    "{} {}",
                    spec.name,
                    p.id
                );
            }
        }
        let p = Arc::new(FxParams::new(spec));
        let t = time::Time::default();
        let v = vocal::Vocal::default();
        let a = amp::Amp::default();
        let panel: Arc<dyn Extension> = match spec.name {
            "ML Time Shaper" => t.panel(p),
            "ML Vocal Tune" => v.panel(p),
            _ => Arc::new(amp::Panel {
                shared: a.params.shared.clone(),
            }),
        };
        for scale in [1., 1.5, 2.] {
            let mut canvas = Canvas::new(
                (spec.size.0 as f32 * scale) as u32,
                (spec.size.1 as f32 * scale) as u32,
                scale,
            )
            .unwrap();
            let model = Model(spec);
            lemon_small_fx::view::paint(
                spec,
                &mut canvas,
                &os_theme::Palette::default(),
                &model,
                &Default::default(),
                "",
            );
            panel.paint(&mut canvas, &model);
            if let Ok(dir) = std::env::var("ML_TOOLS_RENDER_DIR") {
                canvas
                    .pixmap
                    .save_png(std::path::Path::new(&dir).join(format!("{} {scale}.png", spec.name)))
                    .unwrap();
            }
        }
    }
}
