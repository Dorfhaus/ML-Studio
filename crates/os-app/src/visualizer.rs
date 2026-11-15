// SPDX-License-Identifier: GPL-3.0-or-later
//! First-use setup for the optional stock Mixer visualizer.
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    time::Duration,
};

use os_host::{Descriptor, Format, Kind};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    app::App,
    dialog::{Answer, Body, Kind as DialogKind, Msg, Then, spans},
    mixer as mx,
    plugin_downloads::{Download, copy_verified},
};

pub const LABEL: &str = "ML Visualizer";
pub const VISIBILITY: &str = "visible_ml-visualizer";
const ID: &str = "org.audiovisualizer.plugin";
const VERSION: &str = "0.77";
const MODULE: &str = "AudioVisualizer.clap";
const SOURCE: &str = "https://github.com/mtyas/AudioVisualizer/releases/tag/v0.77";

#[derive(Deserialize)]
struct Package {
    url: String,
    size: u64,
    sha256: String,
    prefix: String,
    files: BTreeMap<String, Member>,
}

#[derive(Deserialize)]
struct Member {
    size: u64,
    sha256: String,
    executable: bool,
}

fn package() -> Option<&'static Package> {
    static CATALOG: std::sync::OnceLock<BTreeMap<String, Package>> = std::sync::OnceLock::new();
    let platform = if cfg!(all(windows, target_arch = "x86_64")) {
        "windows"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        return None;
    };
    CATALOG.get_or_init(|| serde_json::from_str(include_str!("../../../packaging/visualizer/catalog.json"))
        .expect("valid visualizer catalog")).get(platform)
}

pub fn visible() -> bool {
    package().is_some() && crate::workspace_visibility::shown(VISIBILITY)
}

fn root() -> Result<PathBuf, String> {
    #[cfg(windows)]
    if let Ok(exe) = std::env::current_exe() {
        if exe.file_name().is_some_and(|name| name == "lemonstudio-development.exe") {
            if let Some(product) = exe.parent().and_then(Path::parent) {
                return Ok(product.join(LABEL));
            }
        }
    }
    Ok(os_paths::user_dir().ok_or("Cannot find the ML Studio user folder")?.join(LABEL))
}

fn plain(path: &Path) -> Result<fs::Metadata, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err(format!("Visualizer package path is a junction or link: {}", path.display()));
        }
    }
    if meta.file_type().is_symlink() {
        return Err(format!("Visualizer package path is a link: {}", path.display()));
    }
    Ok(meta)
}

fn verified(root: &Path, package: &Package) -> bool {
    if !plain(root).is_ok_and(|m| m.is_dir()) {
        return false;
    }
    package.files.iter().all(|(relative, member)| {
        let path = root.join(relative);
        if !path.ancestors().take_while(|p| *p != root).all(|p| plain(p).is_ok()) {
            return false;
        }
        if !plain(&path).is_ok_and(|m| m.is_file() && m.len() == member.size) {
            return false;
        }
        let Ok(bytes) = fs::read(&path) else { return false };
        format!("{:x}", Sha256::digest(bytes)) == member.sha256
    })
}

fn ready_path() -> Option<PathBuf> {
    let package = package()?;
    let root = root().ok()?;
    let installed = root.join(VERSION);
    let marker = installed.join("installed.sha256");
    (plain(&root).is_ok_and(|m| m.is_dir())
        && plain(&installed).is_ok_and(|m| m.is_dir())
        && plain(&marker).is_ok_and(|m| m.is_file() && m.len() == 64)
        && fs::read_to_string(marker).is_ok_and(|s| s == package.sha256))
        .then(|| installed.join(MODULE))
}

pub fn available() -> Option<crate::plugins::Available> {
    let path = ready_path()?;
    if let Some((root, revision)) = root().ok().map(|root| (root.clone(), root.join(VERSION))) {
        let _ = crate::job_scratch::keep_revision(&root, &revision);
    }
    let desc = Descriptor {
        format: Format::Clap,
        id: ID.into(),
        name: "AudioVisualizer".into(),
        vendor: "mtyas".into(),
        version: "0.77.0".into(),
        features: ["audio-effect", "analyzer", "utility"].map(str::to_owned).to_vec(),
        kind: Kind::Effect,
        path,
    };
    let lib = Arc::new(InstalledLibrary { desc: desc.clone(), inner: Default::default() });
    Some(crate::plugins::Available { desc, lib })
}

pub fn is_managed(desc: &Descriptor) -> bool {
    desc.id == ID && desc.format == Format::Clap
        && root().is_ok_and(|root| desc.path == root.join(VERSION).join(MODULE))
}

struct InstalledLibrary {
    desc: Descriptor,
    inner: std::sync::OnceLock<Arc<dyn os_host::Library>>,
}

impl os_host::Library for InstalledLibrary {
    fn path(&self) -> &Path { &self.desc.path }
    fn plugins(&self) -> &[Descriptor] { std::slice::from_ref(&self.desc) }
    fn create(self: Arc<Self>, id: &str) -> Result<os_host::SharedPlugin, String> {
        let inner = if let Some(inner) = self.inner.get() {
            inner.clone()
        } else {
            let folder = self.desc.path.parent().ok_or("Invalid Visualizer path")?;
            if !package().is_some_and(|package| verified(folder, package)) {
                return Err("ML Visualizer files are missing or damaged".into());
            }
            let inner = crate::plugins::load_file(&self.desc.path, Format::Clap)?;
            self.inner.get_or_init(|| inner).clone()
        };
        inner.create(id)
    }
}

fn extract(archive: File, stage: &Path, package: &Package) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(archive).map_err(|e| e.to_string())?;
    for (relative, member) in &package.files {
        let mut entry = zip.by_name(&format!("{}{relative}", package.prefix)).map_err(|e| e.to_string())?;
        if entry.size() != member.size || entry.is_dir()
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err("Visualizer archive contains an unexpected file".into());
        }
        if !crate::helper_install::safe_name(relative) {
            return Err("Invalid visualizer package path".into());
        }
        let path = stage.join(relative);
        fs::create_dir_all(path.parent().ok_or("Invalid archive path")?).map_err(|e| e.to_string())?;
        let mut output = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| e.to_string())?;
        if std::io::copy(&mut (&mut entry).take(member.size + 1), &mut output).map_err(|e| e.to_string())? != member.size {
            return Err("Visualizer archive file is incomplete".into());
        }
        output.flush().map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(if member.executable { 0o755 } else { 0o644 }))
                .map_err(|e| e.to_string())?;
        }
        #[cfg(not(unix))]
        let _ = member.executable;
    }
    if !verified(stage, package) {
        return Err("Visualizer extracted files failed verification".into());
    }
    Ok(())
}

enum Update {
    Progress(u64),
    Checking,
    ExportTools(u64, u64),
    Done(Result<(), String>),
}

fn install(root: &Path, package: &Package, cancel: &AtomicBool, report: impl Fn(Update), scan: impl Fn(&Path) -> crate::plugin_db::Scanned) -> Result<(), String> {
    let check = || if cancel.load(Ordering::Relaxed) { Err("Visualizer download cancelled".to_owned()) } else { Ok(()) };
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    plain(root)?;
    let lock = OpenOptions::new().write(true).create(true).truncate(false)
        .open(root.join("install.lock")).map_err(|e| e.to_string())?;
    lock.try_lock().map_err(|_| "Another ML Studio window is installing the visualizer")?;

    let dest = root.join(VERSION);
    if verified(&dest, package) {
        check()?;
        let marker = dest.join("installed.sha256");
        if fs::symlink_metadata(&marker).is_ok() && !plain(&marker)?.is_file() {
            return Err("Invalid visualizer installation marker".into());
        }
        fs::write(marker, &package.sha256).map_err(|e| e.to_string())?;
        return Ok(());
    }
    if fs::symlink_metadata(&dest).is_ok() {
        return Err(format!("Existing visualizer files failed verification. Move this folder aside and retry: {}", dest.display()));
    }
    check()?;
    let mut files: Vec<_> = package.files.keys().map(|name| format!("payload/{name}")).collect();
    files.extend(["download.zip".into(), "payload/installed.sha256".into(), "payload/.ml-revision-files.json".into()]);
    let files: Vec<_> = files.iter().map(String::as_str).collect();
    let stage = crate::job_scratch::create(root, "visualizer-install", &files).map_err(|e| e.to_string())?;
    let stage = stage.path();
    let result = (|| {
        let archive = stage.join("download.zip");
        let response = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(20)).timeout_read(Duration::from_secs(20))
            .build().get(&package.url).set("User-Agent", "MLStudio-Visualizer/1.0")
            .call().map_err(|e| e.to_string())?;
        let download = Download {
            filename: "AudioVisualizer.zip".into(), size: package.size, sha256: package.sha256.clone(),
            url: package.url.clone(), platform: String::new(), version: VERSION.into(),
            note: String::new(), install: vec![],
        };
        let file = OpenOptions::new().create_new(true).write(true).open(&archive).map_err(|e| e.to_string())?;
        let mut percent = u64::MAX;
        copy_verified(response.into_reader(), file, &download, |count| {
            check()?;
            let next = count * 100 / package.size;
            if next != percent {
                report(Update::Progress(count));
                percent = next;
            }
            Ok(())
        })?;
        report(Update::Checking);
        let payload = stage.join("payload");
        fs::create_dir(&payload).map_err(|e| e.to_string())?;
        extract(File::open(&archive).map_err(|e| e.to_string())?, &payload, package)?;
        check()?;
        let scan = scan(&payload.join(MODULE));
        if !scan.plugins.iter().any(|p| p.id == ID) {
            return Err(format!("ML Studio could not load the visualizer: {}", scan.error));
        }
        check()?;
        fs::write(payload.join("installed.sha256"), &package.sha256).map_err(|e| e.to_string())?;
        let files = package.files.keys().cloned().chain(["installed.sha256".into()]).collect::<Vec<_>>();
        crate::job_scratch::mark_revision(&payload, &files).map_err(|e| e.to_string())?;
        fs::rename(payload, &dest).map_err(|e| e.to_string())
    })();
    result
}

#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub track: u16,
    pub slot: u8,
    runtime_id: u64,
    saved: Option<os_model::SlotPlugin>,
    encoder_download: u64,
}

pub fn saved_slot(slot: &os_model::SlotPlugin) -> bool {
    crate::project_plugins::wrapped_slot(slot).is_some_and(|r| {
        r.format == "clap" && r.name == "AudioVisualizer"
            && (r.vendor == "mtyas" || r.path.rsplit(['/', '\\']).next() == Some(MODULE))
    })
}

pub fn open(app: &mut App, track: u16, slot: u8) {
    let Some(mixer) = mx::track(&app.state, track) else { return };
    let Some(saved) = mixer.slots.get(slot as usize) else { return };
    if slot == 10 || mx::locked(&app.state, track) || saved.as_ref().is_some_and(|s| !saved_slot(s)) {
        return;
    }
    let plugin_ready = ready_path().is_some();
    let encoder_download = crate::visualizer_video::setup_download_size();
    let target = Target { track, slot, runtime_id: mixer.runtime_id, saved: saved.clone(), encoder_download };
    if plugin_ready && encoder_download == 0 {
        finish(app, target);
        return;
    }
    let Some(package) = package() else {
        app.state.message = Some("The visualizer download is unavailable for this platform".into());
        return;
    };
    if let Some(job) = &app.visualizer_job {
        app.show_msg(progress(job));
        return;
    }
    let size = package.size as f64 / 1_000_000.;
    let installed = package.files.values().map(|m| m.size).sum::<u64>() as f64 / 1_000_000.;
    let note = if cfg!(target_os = "macos") { "\n\nThe upstream Mac package is unsigned. If macOS blocks it, ML Studio will report the loading error." } else { "" };
    let mut prompt = if plugin_ready {
        "AudioVisualizer is already installed. Set up MP4 export?".to_owned()
    } else {
        format!("Download AudioVisualizer by mtyas from GitHub ({size:.1} MB download, {installed:.1} MB installed)?{note}")
    };
    if encoder_download > 0 {
        prompt.push_str(&format!(" Includes the FFmpeg MP4 encoder ({:.1} MB download from GitHub).",
            encoder_download as f64 / 1_000_000.0));
    }
    prompt.push_str(" You can use it offline after setup.");
    app.show_msg(Msg::new(DialogKind::Question, LABEL, spans(&[&prompt]), &[Answer::Yes, Answer::Cancel],
        Then::VisualizerInstall(target)).labelled(&["Download and install", "Cancel"]));
}

pub struct Job {
    target: Option<Target>,
    rx: Receiver<Update>,
    cancel: Arc<AtomicBool>,
    bytes: u64,
    checking: bool,
    export_tools: Option<(u64, u64)>,
}

fn progress(job: &Job) -> Msg {
    let (bytes, total) = job.export_tools.unwrap_or_else(|| (job.bytes, package().map_or(0, |p| p.size)));
    let text = if job.cancel.load(Ordering::Relaxed) {
        "Cancelling visualizer setup...".into()
    } else if let Some((done, total)) = job.export_tools {
        if total == 0 {
            "Preparing MP4 export tools…".into()
        } else {
            format!("Downloading FFmpeg for MP4 export
{:.1} / {:.1} MB",
                done as f64 / 1_000_000.0, total as f64 / 1_000_000.0)
        }
    } else if job.checking {
        "Verifying and installing the visualizer...".into()
    } else {
        format!("Downloading AudioVisualizer from GitHub\n{:.1} / {:.1} MB",
            job.bytes as f64 / 1_000_000., total as f64 / 1_000_000.)
    };
    let mut msg = Msg::new(DialogKind::Info, "Installing ML Visualizer", spans(&[&text]),
        &[Answer::Yes, Answer::Cancel], Then::VisualizerProgress).labelled(&["Cancel download", "Run in background"]);
    msg.progress = Some((bytes, total));
    msg.focus = 1;
    msg
}

pub fn start(app: &mut App, target: Target) {
    if app.visualizer_job.is_some() { return }
    let Some(package) = package() else { return };
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let encoder_download = target.encoder_download;
    std::thread::spawn(move || {
        let result = root().and_then(|root| install_with_export(&root, package, encoder_download, &worker_cancel,
            |msg| { let _ = tx.send(msg); },
            |path| crate::plugin_scan::scan_in_child(path, Format::Clap)));
        let _ = tx.send(Update::Done(result));
    });
    let job = Job { target: Some(target), rx, cancel, bytes: 0, checking: false, export_tools: None };
    app.show_msg(progress(&job));
    app.visualizer_job = Some(job);
}

fn install_with_export(
    root: &Path, package: &Package, encoder_download: u64, cancel: &AtomicBool,
    report: impl Fn(Update), scan: impl Fn(&Path) -> crate::plugin_db::Scanned,
) -> Result<(), String> {
    install(root, package, cancel, &report, scan)?;
    if !crate::visualizer_video::hosted_supported() {
        return Ok(());
    }
    report(Update::ExportTools(0, encoder_download));
    crate::visualizer_video::setup_hosted_export(encoder_download > 0, cancel, |done, total| {
        report(Update::ExportTools(done, total));
    }).map_err(|error| format!("AudioVisualizer is installed. MP4 export setup could not finish: {error}"))
}

pub fn cancel(app: &mut App) {
    if let Some(job) = &app.visualizer_job {
        job.cancel.store(true, Ordering::Relaxed);
    }
}

pub fn project_changed(app: &mut App) {
    if let Some(job) = &mut app.visualizer_job {
        job.target = None;
    }
}

pub fn tick(app: &mut App) {
    let Some(job) = &mut app.visualizer_job else { return };
    let mut changed = false;
    let mut done = None;
    loop {
        match job.rx.try_recv() {
            Ok(Update::Progress(bytes)) => job.bytes = bytes,
            Ok(Update::Checking) => job.checking = true,
            Ok(Update::ExportTools(done, total)) => job.export_tools = Some((done, total)),
            Ok(Update::Done(result)) => { done = Some(result); break }
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => {
                done = Some(Err("Visualizer setup stopped. Open ML Visualizer to retry.".into()));
                break;
            }
        }
        changed = true;
    }
    if let Some(result) = done {
        let job = app.visualizer_job.take().unwrap();
        app.state.dialogs.retain(|modal| !matches!(&modal.body, Body::Msg(msg) if msg.then == Then::VisualizerProgress));
        match result {
            Ok(()) if !job.cancel.load(Ordering::Relaxed) => {
                if let Some(target) = job.target { finish(app, target); }
                else { app.state.message = Some("ML Visualizer is installed".into()); }
            }
            Ok(()) => app.state.message = Some("ML Visualizer is installed".into()),
            Err(error) if job.cancel.load(Ordering::Relaxed) => app.state.message = Some(error),
            Err(error) => app.show_msg(Msg::new(DialogKind::Error, LABEL, spans(&[&error]), &[Answer::Ok], Then::Nothing)),
        }
        app.invalidate();
    } else if changed {
        let update = progress(job);
        for modal in &mut app.state.dialogs {
            if let Body::Msg(msg) = &mut modal.body {
                if msg.then == Then::VisualizerProgress {
                    msg.text = update.text.clone();
                    msg.progress = update.progress;
                }
            }
        }
        app.state.message = Some(match job.export_tools {
            Some((done, total)) if total > 0 => format!("MP4 encoder download: {}%", done * 100 / total),
            Some(_) => "Preparing MP4 export tools…".into(),
            None if job.checking => "Installing ML Visualizer…".into(),
            None => format!("ML Visualizer download: {}%", job.bytes * 100 / package().unwrap().size),
        });
        app.invalidate();
    }
}

fn finish(app: &mut App, target: Target) {
    if !mx::track(&app.state, target.track).is_some_and(|m| m.runtime_id == target.runtime_id
        && m.slots.get(target.slot as usize) == Some(&target.saved)) || mx::locked(&app.state, target.track)
    {
        app.state.message = Some("ML Visualizer installed. Open an available Mixer slot to add it.".into());
        app.invalidate();
        return;
    }
    let Some(entry) = available() else {
        app.state.message = Some(format!("ML Visualizer files are missing. Try the download again. {SOURCE}"));
        app.invalidate();
        return;
    };
    let index = app.plugins.effects.iter().position(|a| a.desc.uid() == entry.desc.uid())
        .unwrap_or(app.plugins.effects.len());
    if index == app.plugins.effects.len() {
        app.plugins.effects.push(entry);
    } else {
        app.plugins.effects[index] = entry;
    }
    app.plugins.missing.remove(&format!("clap:{ID}"));
    let key = crate::plugins::fx_key(target.track, target.slot);
    if target.saved.is_some() {
        app.retry_scanned_wrappers();
        app.sync_wrapped();
        if app.plugins.hosted.contains_key(&key) {
            app.open_fx(target.track, target.slot);
        }
    } else {
        let prepared = app.plugins.prepare_effect(index, None, true);
        match prepared {
            Ok(hosted) => {
                if let Err(error) = crate::project_plugins::store_hosted(&mut app.state.project, &app.plugins) {
                    app.state.message = Some(error);
                    app.invalidate();
                    return;
                }
                let state = &mut app.state;
                if let Some(copy) = mx::channel_copy(state, target.track) {
                    crate::history::record(state, "Add ML Visualizer", crate::history::Snap::MixerChannel(Box::new(copy)));
                }
                let slot = target.slot as usize;
                let track = mx::track_mut(state, target.track).unwrap();
                let reference = crate::project_plugins::plugin_ref(&app.plugins.effects[index].desc, Vec::new());
                track.slots[slot] = Some(os_model::SlotPlugin {
                    name: os_flp::wrapper::WRAPPER.into(),
                    state: os_flp::wrapper::encode(None, &reference),
                });
                track.slot_on[slot] = true;
                track.slot_mix[slot] = 1.;
                track.slots_enabled = true;
                state.mixer.slot_names.insert((target.track, target.slot), LABEL.into());
                app.wrap_failed.retain(|&failed| failed != key);
                app.plugins.hosted.insert(key, hosted);
                app.plugins.added(key);
                app.transport.set_effects(app.plugins.effects());
                app.state.piano.dirty = true;
                app.open_fx(target.track, target.slot);
            }
            Err(error) => app.show_msg(Msg::new(DialogKind::Error, LABEL, spans(&[&error]), &[Answer::Ok], Then::Nothing)),
        }
    }
    app.state.plugdb.rebuild(&app.plugins);
    app.invalidate();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visualizer_menu_requires_consent_and_honors_visibility() {
        if package().is_none() { return; }
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        crate::settings_file::set_flag(VISIBILITY, true);
        assert!(mx::effect_menu(&app.state, 0, 0., 0.).items.iter().any(|i| i.label == LABEL));
        open(&mut app, 0, 0);
        assert!(app.visualizer_job.is_none());
        assert!(mx::track(&app.state, 0).unwrap().slots[0].is_none());
        let modal = app.state.dialogs.last().expect("first use asks for consent");
        assert!(matches!(&modal.body, Body::Msg(msg) if matches!(msg.then, Then::VisualizerInstall(_))));
        crate::settings_file::set_flag(VISIBILITY, false);
        assert!(!mx::effect_menu(&app.state, 0, 0., 0.).items.iter().any(|i| i.label == LABEL));
        crate::settings_file::set_flag(VISIBILITY, true);
    }

    #[test]
    #[ignore = "downloads the pinned upstream package and optional encoder in a clean profile"]
    fn visualizer_verified_download_and_effect_roundtrip() {
        let scratch = root().unwrap();
        let package = package().unwrap();
        assert!(!scratch.exists(), "use a clean profile");
        let progressed = std::cell::Cell::new(false);
        let encoder_download = crate::visualizer_video::setup_download_size();
        if crate::visualizer_video::hosted_supported() {
            assert!(encoder_download > 0, "clean setup must include the encoder");
        }
        install_with_export(&scratch, package, encoder_download, &AtomicBool::new(false), |update| {
            if matches!(update, Update::Progress(_)) {
                progressed.set(true);
            }
            if let Update::ExportTools(done, total) = update
                && total > 0 && done == total
            {
                println!("MP4 encoder transfer completed");
            }
        }, |p| crate::plugin_scan::scan_here(p, Format::Clap)).unwrap();
        assert!(progressed.replace(false), "the initial plugin transfer reports progress");
        assert_eq!(crate::visualizer_video::setup_download_size(), 0);
        let installed = scratch.join(VERSION);
        assert!(verified(&installed, package));
        let library = crate::plugins::load_file(&installed.join(MODULE), Format::Clap).unwrap();
        let desc = library.plugins().iter().find(|d| d.id == ID).unwrap().clone();
        let plugin = library.clone().create(ID).unwrap();
        let mut plugin = plugin.lock().unwrap();
        assert!(plugin.has_editor());
        assert!(plugin.activate(48_000));
        let before_l: Vec<f32> = (0..256).map(|n| (n as f32 * 0.11).sin() * 0.2).collect();
        let before_r: Vec<f32> = before_l.iter().map(|v| -v * 0.5).collect();
        let (mut left, mut right) = (before_l.clone(), before_r.clone());
        plugin.process_fx(48_000, &mut left, &mut right);
        assert_eq!((left, right), (before_l, before_r));
        let saved = plugin.save_state().filter(|state| !state.is_empty()).unwrap();
        let reference = crate::project_plugins::plugin_ref(&desc, saved.clone());
        let slot = os_model::SlotPlugin {
            name: os_flp::wrapper::WRAPPER.into(),
            state: os_flp::wrapper::encode(None, &reference),
        };
        assert!(saved_slot(&slot));
        let restored = library.create(ID).unwrap();
        assert!(restored.lock().unwrap().load_state(&crate::project_plugins::wrapped_slot(&slot).unwrap().state));
        install(&scratch, package, &AtomicBool::new(false), |_| progressed.set(true), |_| panic!("already installed")).unwrap();
        assert!(!progressed.get(), "second use needs no transfer");
    }
}
