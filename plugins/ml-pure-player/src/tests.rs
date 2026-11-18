use super::*;
use std::path::Path;
fn font() -> PathBuf {
    PathBuf::from(
        std::env::var("ML_PURE_TEST_FONT").expect("Path to the official GeneralUser GS test bank"),
    )
}
fn energy(l: &[f32]) -> f64 {
    l.iter().map(|v| (*v as f64).powi(2)).sum::<f64>()
}
#[test]
fn native_general_midi_instruments_and_unicode_path() {
    for rate in [8000, 44100, 48000, 96000, 192000] {
        let mut synth = native::Synth::new(&font(), rate).unwrap();
        assert!(
            (0..128).all(|program| synth
                .presets
                .iter()
                .any(|p| p.bank == 0 && p.program == program)),
            "complete General MIDI bank"
        );
        assert!(synth.presets.iter().filter(|p| p.bank == 128).count() >= 13);
        for (bank, program, key) in [
            (0, 0, 60),
            (0, 24, 60),
            (0, 33, 40),
            (0, 48, 60),
            (0, 56, 60),
            (0, 73, 72),
            (0, 89, 60),
            (128, 0, 36),
        ] {
            synth.reset();
            synth.select(bank, program);
            synth.controls(0.16, 0., 0.);
            synth.note(key, 0.8, true);
            let mut l = vec![0.; rate as usize / 4];
            let mut r = l.clone();
            synth.render(&mut l, &mut r);
            assert!(
                energy(&l) + energy(&r) > 1e-5,
                "silent {rate} {bank}:{program}"
            );
            assert!(l.iter().chain(&r).all(|x| x.is_finite()));
            synth.note(key, 0., false);
        }
    }
    let root = PathBuf::from(std::env::var("ML_TOOLS_RENDER_DIR").unwrap());
    let unicode = root.join("Pure Player é 音.sf2");
    std::fs::copy(font(), &unicode).unwrap();
    assert!(native::Synth::new(&unicode, 48000).is_ok());
    std::fs::remove_file(unicode).unwrap();
    assert!(native::Synth::new(Path::new("does-not-exist.sf2"), 48000).is_err());
    let mut synth = native::Synth::new(&font(), 192000).unwrap();
    synth.select(0, 80);
    synth.controls(0.1, 0., 0.);
    synth.note(69, 0.8, true);
    let mut l = vec![0.; 96000];
    let mut r = l.clone();
    synth.render(&mut l, &mut r);
    let power = |hz: f64| {
        let (mut re, mut im) = (0., 0.);
        for (i, x) in l[48000..].iter().enumerate() {
            let phase = std::f64::consts::TAU * hz * i as f64 / 192000.;
            re += *x as f64 * phase.cos();
            im += *x as f64 * phase.sin();
        }
        re * re + im * im
    };
    assert!(
        power(440.) > power(220.) * 20. && power(440.) > power(880.) * 10.,
        "192 kHz playback retains A4 pitch"
    );
}
#[test]
fn download_integrity_cancellation_and_existing_install() {
    let progress = AtomicU64::new(0);
    let bytes = std::fs::read(font()).unwrap();
    let mut writer = std::io::sink();
    library::transfer(
        &mut std::io::Cursor::new(&bytes),
        &mut writer,
        &progress,
        &|| false,
    )
    .unwrap();
    assert_eq!(progress.load(Ordering::Relaxed), library::SIZE);
    assert!(
        library::transfer(
            &mut std::io::Cursor::new(b"not a soundfont"),
            &mut writer,
            &progress,
            &|| false
        )
        .is_err()
    );
    assert!(
        library::transfer(
            &mut std::io::Cursor::new(&bytes),
            &mut writer,
            &progress,
            &|| true
        )
        .unwrap_err()
        .contains("cancelled")
    );
    let mut damaged = bytes.clone();
    damaged[1024] ^= 1;
    assert!(
        library::transfer(
            &mut std::io::Cursor::new(damaged),
            &mut writer,
            &progress,
            &|| false
        )
        .unwrap_err()
        .contains("verification")
    );
    assert!(
        library::transfer(
            &mut std::io::Cursor::new(&bytes),
            &mut writer,
            &progress,
            &|| progress.load(Ordering::Relaxed) > 0
        )
        .unwrap_err()
        .contains("cancelled")
    );
    let root =
        PathBuf::from(std::env::var("ML_TOOLS_RENDER_DIR").unwrap()).join("cancelled-download");
    assert!(library::download(&root, &progress, || true).is_err());
    assert!(!root.join("GeneralUser-GS-2.0.3.sf2.part").exists());
}
#[test]
fn actual_clap_midi_state_reload_and_no_network_on_first_use() {
    use os_engine::Instrument as _;
    let lib = unsafe {
        os_clap::Library::from_entry(
            &LINKED_CLAP_ENTRY as *const _ as *const std::ffi::c_void,
            Path::new("ML Pure Player.clap"),
        )
    }
    .unwrap();
    let mut first = os_clap::Plugin::new(&lib, &lib.plugins[0].id).unwrap();
    let mut state = PurePlayer::default().params.serialize_fields();
    state.insert(
        "pure_soundfont".into(),
        serde_json::to_string(&Asset {
            path: font().to_string_lossy().into_owned(),
        })
        .unwrap(),
    );
    let serialized = serde_json::json!({"version":"0.0.1","params":{},"fields":state}).to_string();
    let mut blob = (serialized.len() as u64).to_le_bytes().to_vec();
    blob.extend_from_slice(serialized.as_bytes());
    assert!(first.load_state(&blob));
    assert!(first.activate(48000));
    first.note_on(0, 60, 0.8);
    let mut l = [0.; 4096];
    let mut r = l;
    first.process(48000, &mut l, &mut r);
    assert!(energy(&l) > 1e-4);
    first.note_off(0, 60);
    let saved = first.save_state().unwrap();
    let mut second = os_clap::Plugin::new(&lib, &lib.plugins[0].id).unwrap();
    assert!(second.load_state(&saved));
    assert!(second.activate(48000));
    assert_eq!(second.save_state().unwrap(), saved);
    let empty = PurePlayer::default();
    assert!(!empty.params.shared.download.load(Ordering::Acquire));
    assert!(empty.params.shared.prompt.load(Ordering::Acquire));
}
#[test]
fn editor_setup_list_selection_and_scaling() {
    use lemon_small_fx::editor::{Btn, Extension, Logic, Mods};
    let player = PurePlayer::default();
    let params = player.params.clone();
    let panel = Arc::new(ui::Panel::new(params.clone()));
    let mut shared = lemon_small_fx::Shared::default();
    shared.extension = Some(panel.clone());
    let mut logic = Logic::new(&spec::SPEC, params.fx.clone(), Arc::new(shared), None);
    let out = PathBuf::from(std::env::var("ML_TOOLS_RENDER_DIR").unwrap());
    for scale in [1., 1.5, 2.] {
        let mut canvas =
            os_ui::Canvas::new((780. * scale) as u32, (530. * scale) as u32, scale).unwrap();
        logic.paint(&mut canvas, &os_theme::Palette::default());
        canvas
            .pixmap
            .save_png(out.join(format!("ML Pure Player setup {scale}.png")))
            .unwrap();
    }
    assert!(panel.click(550., 330., None));
    assert!(!params.shared.prompt.load(Ordering::Acquire));
    let synth = native::Synth::new(&font(), 48000).unwrap();
    params.shared.prepared(&synth);
    for scale in [1., 1.5, 2.] {
        let mut canvas =
            os_ui::Canvas::new((780. * scale) as u32, (530. * scale) as u32, scale).unwrap();
        logic.paint(&mut canvas, &os_theme::Palette::default());
        canvas
            .pixmap
            .save_png(out.join(format!("ML Pure Player ready {scale}.png")))
            .unwrap();
    }
    assert!(panel.click(30., 290., None));
    assert_ne!(params.fx.v(1), 0);
    assert!(panel.click(30., 220., None));
    let mut canvas = os_ui::Canvas::new(780, 530, 1.).unwrap();
    logic.paint(&mut canvas, &os_theme::Palette::default());
    canvas
        .pixmap
        .save_png(out.join("ML Pure Player families.png"))
        .unwrap();
    assert!(panel.click(30., 418., None)); // Bass family
    assert!(panel.click(30., 260., None));
    assert_eq!(params.fx.v(1), 32);
    panel.file_dropped(&font());
    assert_eq!(params.shared.asset().path, font().to_string_lossy());
    let edits = panel.take_state_edits();
    assert_eq!(edits.len(), 1);
    let old: serde_json::Value = serde_json::from_str(&edits[0]).unwrap();
    params.deserialize_fields(
        &[(
            "pure_soundfont".to_string(),
            old["fields"]["pure_soundfont"]
                .as_str()
                .unwrap()
                .to_string(),
        )]
        .into(),
    );
    assert!(
        params.shared.asset().path.is_empty(),
        "file-drop Undo restores the previous library token"
    );
    logic.down(64., 68., Btn::Left, Mods::default(), false);
    logic.delta(0., -20., Mods::default());
    logic.up(Btn::Left);
    assert!(params.fx.v(2) > 80);
    let original = params.serialize_fields();
    let settings = os_paths::user_file("settings.txt").unwrap();
    let existing = std::fs::read(&settings).ok();
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    std::fs::write(&settings, "visible_stock-dsp-controls=0\n").unwrap();
    let p = params.clone();
    std::thread::spawn(move || {
        let mut shared = lemon_small_fx::Shared::default();
        shared.extension = Some(Arc::new(ui::Panel::new(p.clone())));
        let mut logic = Logic::new(&spec::SPEC, p.fx.clone(), Arc::new(shared), None);
        let mut canvas = os_ui::Canvas::new(780, 530, 1.).unwrap();
        logic.paint(&mut canvas, &os_theme::Palette::default());
        assert!(!logic.hits.iter().any(|hit| hit.t.id() == "gain"));
        assert!(!ui::Panel::new(p).click(30., 290., None));
    })
    .join()
    .unwrap();
    if let Some(bytes) = existing {
        std::fs::write(settings, bytes).unwrap()
    } else {
        std::fs::remove_file(settings).unwrap()
    }
    assert_eq!(params.serialize_fields(), original);
}

#[test]
#[ignore = "Explicit live download check; uses the author's pinned public GitHub file"]
fn official_download_install_and_upgrade_reuse() {
    let root =
        PathBuf::from(std::env::var("ML_TOOLS_RENDER_DIR").unwrap()).join("downloaded-library");
    let progress = AtomicU64::new(0);
    let first = library::download(&root, &progress, || false).unwrap();
    assert!(library::installed(&first));
    let modified = std::fs::metadata(&first).unwrap().modified().unwrap();
    let second = library::download(&root, &progress, || false).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        modified,
        std::fs::metadata(second).unwrap().modified().unwrap(),
        "upgrade reuses verified library"
    );
    assert!(root.join("GeneralUser-GS-LICENSE.txt").is_file());
    assert!(!root.join("GeneralUser-GS-2.0.3.sf2.part").exists());
    // Exercise the actual first-use button -> loader worker -> live instrument path.
    use lemon_small_fx::editor::Extension;
    let worker_root=PathBuf::from(std::env::var("ML_TOOLS_RENDER_DIR").unwrap()).join("first-use-worker-library");
    let previous=std::env::var_os("ML_PURE_LIBRARY_DIR");
    // This explicitly invoked integration test runs alone (--test-threads=1), before spawning its worker.
    unsafe{std::env::set_var("ML_PURE_LIBRARY_DIR",&worker_root);}
    let mut player=PurePlayer::default();let shared=player.params.shared.clone();
    // Wait for explicit consent even if a previous test run left this library cached.
    player.start_worker(1,true);
    assert!(!shared.download.load(Ordering::Acquire));
    let panel=ui::Panel::new(player.params.clone());assert!(panel.click(100.,330.,None));
    let deadline=std::time::Instant::now()+Duration::from_secs(60);
    while shared.pending.load(Ordering::Acquire).is_null()&&std::time::Instant::now()<deadline{std::thread::sleep(Duration::from_millis(20));}
    player.take_pending();assert!(!player.active.is_null(),"{}",shared.status.lock().unwrap());
    assert_eq!(shared.progress.load(Ordering::Relaxed),library::SIZE);
    assert!(!shared.prompt.load(Ordering::Acquire));
    unsafe{(*player.active).select(0,0);(*player.active).note(60,0.8,true);}
    let mut l=[0.;4096];let mut r=l;player.render(&mut l,&mut r);assert!(energy(&l)>1e-4);
    player.stop_worker();drop(player);
    unsafe{if let Some(p)=previous{std::env::set_var("ML_PURE_LIBRARY_DIR",p)}else{std::env::remove_var("ML_PURE_LIBRARY_DIR")}}
}
