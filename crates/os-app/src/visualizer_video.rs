// SPDX-License-Identifier: GPL-3.0-or-later
//! Offline video export for saved AudioVisualizer scenes.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

use crate::{
    app::App,
    dialog::{Answer, Body, Kind, Msg, Then, spans},
    export::{Job as AudioJob, Options, Tail},
    filedlg::{FileDlg, Mode},
    mixer as mx,
    plugin_downloads::{Download, copy_verified},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const VISIBILITY: &str = "visible_visualizer-video-export";
const TITLE: &str = "Export visualizer video";
const WORKER: &str = "MLVisualizerRender.exe";

#[derive(Deserialize)]
struct Catalog {
    renderer_sha256: String,
    renderer_size: u64,
    encoder_url: String,
    archive_sha256: String,
    archive_size: u64,
    encoder_member: String,
    encoder_sha256: String,
    encoder_size: u64,
}

fn catalog() -> &'static Catalog {
    static VALUE: OnceLock<Catalog> = OnceLock::new();
    VALUE.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../packaging/visualizer/video-tools.json"
        ))
        .expect("validated video tools catalog")
    })
}

const EMBEDDED_WORKER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/visualizer-worker.exe"));

/// Unbundled developer checkouts can still use their local renderer.
fn development_renderer() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    if exe.file_name()? != "lemonstudio-development.exe" {
        return None;
    }
    Some(exe.parent()?.parent()?.join("ML Visualizer Video Tools").join(WORKER))
}

pub(crate) fn hosted_supported() -> bool {
    supported() && (!EMBEDDED_WORKER.is_empty() || development_renderer().is_some_and(|p| p.is_file()))
}

pub fn supported() -> bool {
    cfg!(all(windows, target_arch = "x86_64"))
}

pub fn available() -> bool {
    hosted_supported() && crate::workspace_visibility::shown(VISIBILITY)
}

fn tools_dir() -> Result<PathBuf, String> {
    if EMBEDDED_WORKER.is_empty()
        && let Some(dir) = development_renderer().and_then(|p| p.parent().map(Path::to_path_buf))
    {
        return Ok(dir);
    }
    if !supported() {
        return Err("Visualizer video export requires 64-bit Windows".into());
    }
    os_paths::user_dir().map(|p| p.join("ML Visualizer Video Tools"))
        .ok_or_else(|| "The ML Studio profile is unavailable".into())
}

fn error(app: &mut App, message: &str) {
    app.show_msg(Msg::new(
        Kind::Error,
        TITLE,
        spans(&[message]),
        &[Answer::Ok],
        Then::Nothing,
    ));
}

pub fn options(app: &mut App, slot: u8) {
    if !available() {
        return;
    }
    if let Some(run) = &app.visualizer_video {
        let (label, amount) = run.status.clone();
        progress(app, &label, amount);
        return;
    }
    let (x, y) = app.mouse;
    use crate::playlist::{Cmd, PItem, Popup};
    let range = app.state.playlist.range.is_some_and(|(a, b)| a != b);
    let mut items = Vec::new();
    for (vertical, label) in [(false, "Landscape"), (true, "Vertical")] {
        let choice = |selection| {
            Cmd::Mixer(mx::MCmd::VisualVideo {
                slot,
                vertical,
                selection,
            })
        };
        let mut choices = vec![PItem::new("Whole song", choice(false))];
        if range {
            choices.push(PItem::new("Playlist selection", choice(true)));
        }
        items.push(PItem::sub(format!("{label} at 1080p / 30 fps"), choices));
    }
    app.state.playlist.popup = Some(Popup {
        x,
        y,
        items,
        sub: None,
        scroll: [0; 2],
        cascade: Vec::new(),
    });
    app.invalidate();
}

#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    slot: Option<(u16, u8)>,
    vertical: bool,
    span: Option<(u64, u64)>,
    replace: bool,
}

pub fn choose_file(app: &mut App, slot: u8, vertical: bool, selection: bool) {
    if !available() {
        return;
    }
    choose_file_for(
        app,
        Some((app.state.selected_mixer, slot)),
        vertical,
        selection,
    );
}

pub fn choose(app: &mut App, vertical: bool, selection: bool) {
    choose_file_for(app, None, vertical, selection);
}

fn choose_file_for(app: &mut App, slot: Option<(u16, u8)>, vertical: bool, selection: bool) {
    let span = if selection {
        let Some(range) = selection_span(app) else {
            return;
        };
        Some(range)
    } else {
        None
    };
    let dlg = FileDlg::new(
        Mode::Save,
        TITLE,
        "visualizer video",
        Then::VisualizerVideoFile(Request {
            slot,
            vertical,
            span,
            replace: false,
        }),
    )
    .filter("MP4 video", &["mp4"])
    .start(app.state.path.as_deref().and_then(Path::parent))
    .name(&format!(
        "{}.mp4",
        crate::backup::project_name(app.state.path.as_deref())
    ));
    app.file_dialog(dlg);
}

pub fn workspace_options(app: &mut App) {
    if app.visualizer_video.is_some() {
        let (label, amount) = app.visualizer_video.as_ref().unwrap().status.clone();
        progress(app, &label, amount);
        return;
    }
    use crate::playlist::{Cmd, PItem, Popup};
    let (x, y) = app.mouse;
    let has_range = app.state.playlist.range.is_some_and(|(a, b)| a != b);
    let items = [(false, "Landscape 1080p"), (true, "Vertical 1080p")]
        .into_iter()
        .map(|(vertical, label)| {
            let command = |selection| {
                Cmd::Visual(crate::visual_scenes::Command::Save {
                    vertical,
                    selection,
                })
            };
            let mut options = vec![PItem::new("Whole song", command(false))];
            if has_range {
                options.push(PItem::new("Playlist selection", command(true)));
            }
            PItem::sub(label, options)
        })
        .collect();
    app.state.playlist.popup = Some(Popup {
        x,
        y,
        items,
        sub: None,
        scroll: [0; 2],
        cascade: Vec::new(),
    });
    app.invalidate();
}

fn mp4_path(path: &Path) -> bool {
    path.is_absolute()
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
}

fn encoder_present() -> bool {
    tools_dir().is_ok_and(|root| {
        fs::symlink_metadata(root.join("ffmpeg.exe"))
            .is_ok_and(|m| m.is_file() && m.len() == catalog().encoder_size)
    })
}

pub(crate) fn setup_download_size() -> u64 {
    if hosted_supported() && !encoder_present() { catalog().archive_size } else { 0 }
}

pub(crate) fn setup_hosted_export(
    download: bool, cancel: &AtomicBool, report: impl Fn(u64, u64),
) -> Result<(), String> {
    ensure_tools(&tools_dir()?, true, download, cancel, report)
}

fn download_prompt() -> String {
    let size = catalog().archive_size as f64 / 1_000_000.0;
    format!(
        "Download the optional FFmpeg video encoder from GitHub? The download is {size:.1} MB. Future exports can run offline."
    )
}

pub fn accept_file(app: &mut App, mut request: Request, file: PathBuf) {
    if !mp4_path(&file) {
        error(app, "Choose an MP4 filename for the video.");
        return;
    }
    request.replace = file.is_file();
    if encoder_present() {
        start(app, request, file, false);
    } else {
        app.show_msg(
            Msg::new(
                Kind::Question,
                TITLE,
                spans(&[&download_prompt()]),
                &[Answer::Yes, Answer::Cancel],
                Then::VisualizerVideoInstall(request, file),
            )
            .labelled(&["Download and export", "Cancel"]),
        );
    }
}

const SCRATCH_FILES: &[&str] = &[
    "scene.json", "master.wav", "render.mp4", "progress.txt", "renderer.log",
    "encoder.log", "encoder.zip", "ffmpeg.exe", WORKER,
];
const EXPORT_FOLDERS: &str = "visualizer_export_folders";

fn export_folders() -> Vec<PathBuf> {
    crate::prefs::text(EXPORT_FOLDERS)
        .and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

fn remember_export_folder(parent: &Path) {
    let mut paths = export_folders();
    paths.retain(|p| p != parent);
    paths.insert(0, parent.to_owned());
    paths.truncate(16);
    if let Ok(text) = serde_json::to_string(&paths) {
        crate::prefs::set_text(EXPORT_FOLDERS, &text);
    }
}

pub(crate) fn reclaim_on_startup() {
    if !supported() {
        return;
    }
    let mut paths = export_folders();
    paths.truncate(16);
    paths.extend(tools_dir().ok());
    std::thread::spawn(move || {
        for parent in paths.into_iter().filter(|p| p.is_absolute()) {
            crate::job_scratch::reclaim(&parent, "visualizer", SCRATCH_FILES);
        }
    });
}

struct WorkDir(tempfile::TempDir);
impl WorkDir {
    fn create(parent: &Path) -> Result<Self, String> {
        crate::job_scratch::create(parent, "visualizer", SCRATCH_FILES).map(Self).map_err(io_error)
    }

    fn path(&self) -> &Path {
        self.0.path()
    }
}

enum Event {
    Progress(String, u64, u64),
    Tools(Result<(), String>),
    Finished(Result<PathBuf, String>),
}

pub struct Run {
    target: PathBuf,
    request: Request,
    dir: Option<WorkDir>,
    cancel: Arc<AtomicBool>,
    rx: Option<mpsc::Receiver<Event>>,
    audio: bool,
    tempo: f64,
    beat: f64,
    status: (String, (u64, u64)),
    show_progress: bool,
    scenes: Option<SceneRender>,
}
impl Drop for Run {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

fn io_error(e: io::Error) -> String {
    e.to_string()
}

pub fn start(app: &mut App, request: Request, path: PathBuf, download: bool) {
    if app.visualizer_video.is_some() || app.loading() || app.audio_rec.is_some() {
        error(app, "Finish the current operation before exporting video.");
        return;
    }
    if !(if request.slot.is_some() {
        available()
    } else {
        crate::visual_scenes::available()
    }) {
        return;
    }
    let legacy = request.slot.is_some();
    let Some(parent) = path.parent() else { return };
    let dir = match WorkDir::create(parent) {
        Ok(dir) => dir,
        Err(e) => {
            error(app, &e);
            return;
        }
    };
    remember_export_folder(parent);
    let scene = request.slot.map(|(track, slot)| {
        let key = crate::plugins::fx_key(track, slot);
        let Some(scene) = app
            .plugins
            .descriptor(key)
            .filter(|desc| crate::visualizer::is_managed(desc))
            .and_then(|_| app.plugins.state(key))
        else {
            return Err(
                "The ML Visualizer instance has changed. Open its editor and try exporting again."
                    .to_string(),
            );
        };
        fs::write(dir.path().join("scene.json"), scene).map_err(io_error)
    });
    if let Some(Err(e)) = scene {
        error(app, &e);
        return;
    }
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = tools_dir().and_then(|root| {
            ensure_tools(&root, legacy, download, &worker_cancel, |done, total| {
                let _ = tx.send(Event::Progress("Downloading FFmpeg...".into(), done, total));
            })
        });
        let _ = tx.send(Event::Tools(result));
    });
    let text = "Checking video tools...";
    app.visualizer_video = Some(Run {
        target: path,
        request,
        dir: Some(dir),
        cancel,
        rx: Some(rx),
        audio: false,
        tempo: 0.,
        beat: 0.,
        status: (text.into(), (0, 0)),
        show_progress: true,
        scenes: None,
    });
    progress(app, text, (0, 0));
}

fn check_cancel(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        Err("Video export cancelled".into())
    } else {
        Ok(())
    }
}

fn check_file(path: &Path, size: u64, sha: &str) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(io_error)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() != size {
        return Err(format!(
            "Video tool failed verification: {}",
            path.display()
        ));
    }
    let mut f = File::open(path).map_err(io_error)?;
    let mut digest = Sha256::new();
    io::copy(&mut f, &mut digest).map_err(io_error)?;
    if format!("{:x}", digest.finalize()) != sha {
        return Err(format!(
            "Video tool failed checksum verification: {}",
            path.display()
        ));
    }
    Ok(())
}

fn ensure_tools(
    root: &Path,
    legacy: bool,
    download: bool,
    cancel: &AtomicBool,
    report: impl Fn(u64, u64),
) -> Result<(), String> {
    let c = catalog();
    check_cancel(cancel)?;
    fs::create_dir_all(root).map_err(io_error)?;
    let encoder = root.join("ffmpeg.exe");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(root.join("encoder.lock"))
        .map_err(io_error)?;
    lock.try_lock()
        .map_err(|_| "Another ML Studio instance is setting up the video encoder")?;
    if legacy {
        let worker = root.join(WORKER);
        if check_file(&worker, c.renderer_size, &c.renderer_sha256).is_err() {
            if EMBEDDED_WORKER.is_empty() {
                return Err("The Visualizer renderer is unavailable in this build.".into());
            }
            let dir = WorkDir::create(root)?;
            let staged = dir.path().join(WORKER);
            fs::write(&staged, EMBEDDED_WORKER).map_err(io_error)?;
            check_file(&staged, c.renderer_size, &c.renderer_sha256)?;
            check_cancel(cancel)?;
            fs::rename(staged, worker).map_err(io_error)?;
        }
    }
    if check_file(&encoder, c.encoder_size, &c.encoder_sha256).is_ok() {
        return Ok(());
    }
    if !download {
        return Err("Open Export visualizer video to download the encoder".into());
    }
    let dir = WorkDir::create(root)?;
    let zip = dir.path().join("encoder.zip");
    let request = Download {
        platform: "windows-x86_64".into(),
        url: c.encoder_url.clone(),
        filename: "encoder.zip".into(),
        version: "9.0.2".into(),
        note: String::new(),
        sha256: c.archive_sha256.clone(),
        size: c.archive_size,
        install: Vec::new(),
    };
    let reply = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_secs(20))
        .build()
        .get(&request.url)
        .set("User-Agent", "MLStudio-Visualizer/1.0")
        .call()
        .map_err(|e| e.to_string())?;
    let mut last = u64::MAX;
    copy_verified(
        reply.into_reader(),
        File::create(&zip).map_err(io_error)?,
        &request,
        |bytes| {
            check_cancel(cancel)?;
            let percent = bytes * 100 / c.archive_size;
            if percent != last {
                report(bytes, c.archive_size);
                last = percent;
            }
            Ok(())
        },
    )?;
    let mut zip =
        zip::ZipArchive::new(File::open(zip).map_err(io_error)?).map_err(|e| e.to_string())?;
    let mut member = zip.by_name(&c.encoder_member).map_err(|e| e.to_string())?;
    if member.size() != c.encoder_size || member.is_dir() {
        return Err("Invalid encoder archive member".into());
    }
    let stage = dir.path().join("ffmpeg.exe");
    io::copy(
        &mut (&mut member).take(c.encoder_size + 1),
        &mut File::create(&stage).map_err(io_error)?,
    )
    .map_err(io_error)?;
    check_file(&stage, c.encoder_size, &c.encoder_sha256)?;
    check_cancel(cancel)?;
    fs::rename(stage, encoder).map_err(io_error)
}

fn progress(app: &mut App, text: &str, amount: (u64, u64)) {
    if let Some(run) = &mut app.visualizer_video {
        run.show_progress = true;
    }
    let mut msg = Msg::new(
        Kind::Info,
        TITLE,
        spans(&[text]),
        &[Answer::Yes, Answer::Cancel],
        Then::VisualizerVideoProgress,
    )
    .labelled(&["Cancel export", "Run in background"]);
    msg.progress = Some(amount);
    msg.focus = 1;
    app.show_msg(msg);
}

fn status(app: &mut App, text: &str, amount: (u64, u64)) {
    if let Some(run) = &mut app.visualizer_video {
        if run.status.0 == text && run.status.1 == amount {
            return;
        }
        run.status = (text.into(), amount);
    }
    app.state.message = Some(text.into());
    for modal in &mut app.state.dialogs {
        if let Body::Msg(msg) = &mut modal.body {
            if msg.then == Then::VisualizerVideoProgress {
                msg.text = spans(&[text]);
                msg.progress = Some(amount);
            }
        }
    }
    app.invalidate();
}

pub fn background(app: &mut App) {
    if let Some(run) = &mut app.visualizer_video {
        run.show_progress = false;
    }
}

pub fn cancel(app: &mut App) {
    if let Some(run) = &app.visualizer_video {
        run.cancel.store(true, Ordering::Relaxed);
        status(app, "Cancelling visualizer video export...", (0, 0));
    }
}

pub fn project_changed(app: &mut App) {
    // The temporary files remain owned by this job until its worker has stopped.
    cancel(app);
}

pub fn tick(app: &mut App) {
    loop {
        let Some(rx) = app.visualizer_video.as_ref().and_then(|r| r.rx.as_ref()) else {
            return;
        };
        match rx.try_recv() {
            Ok(Event::Progress(msg, a, b)) => status(app, &msg, (a, b)),
            Ok(Event::Tools(Ok(()))) => {
                begin_audio(app);
                return;
            }
            Ok(Event::Tools(Err(e)) | Event::Finished(Err(e))) => {
                complete(app, Err(e));
                return;
            }
            Ok(Event::Finished(Ok(p))) => {
                complete(app, Ok(p));
                return;
            }
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                complete(app, Err("The visualizer worker stopped".into()));
                return;
            }
        }
    }
}

fn begin_audio(app: &mut App) {
    let run = app.visualizer_video.as_ref().unwrap();
    if run.cancel.load(Ordering::Relaxed)
        || app.loading()
        || app.export.is_some()
        || app.audio_rec.is_some()
    {
        complete(
            app,
            Err("Video export cancelled before rendering audio".into()),
        );
        return;
    }
    let request = run.request.clone();
    let folder = run.dir.as_ref().unwrap().path().to_path_buf();
    if let Err(e) = crate::clip_effects::normalize(app) {
        complete(app, Err(e));
        return;
    }
    let project = crate::rack::play_project(&app.state);
    if crate::audio_edits::pending(&project) {
        complete(
            app,
            Err("Wait for the current audio edits to finish before exporting video.".into()),
        );
        return;
    }
    let tempo_lane = project
        .arrangement()
        .is_some_and(|a| !a.timeline.tempo.is_empty());
    if (tempo_lane && request.slot.is_some())
        || (!tempo_lane
            && project.automations.iter().any(|automation| {
                automation.targets.contains(&os_model::target::TEMPO)
                    && project.arrangement().is_some_and(|a| {
                        a.clips.iter().any(|clip| {
                            !clip.muted
                                && clip.source == os_model::ClipSource::Channel(automation.channel)
                        })
                    })
            }))
    {
        complete(
            app,
            Err("Use the Playlist Tempo track for scene video with tempo changes.".into()),
        );
        return;
    }
    if request.span.is_none()
        && project.arrangement().is_some_and(|arrangement| {
            arrangement
                .markers
                .iter()
                .any(|marker| marker.kind == os_model::TimeMarker::SKIP)
        })
    {
        complete(
            app,
            Err(
                "Use Playlist selection to export a continuous range in a song with Skip markers."
                    .into(),
            ),
        );
        return;
    }
    // Give a visual-only intro or outro space in the audio export, without changing playback or project data.
    let mut span = request.span;
    if request.slot.is_none()
        && span.is_none()
        && let Some(arr) = project.arrangement()
    {
        let engine = os_engine::Engine::new(
            &project,
            &mut os_engine::SampleLibrary::new(None),
            48_000,
            os_engine::PlayMode::Song,
        );
        let audio_end = engine.end_tick();
        let visual_end = arr
            .timeline
            .visuals
            .last()
            .map_or(0, |scene| u64::from(scene.end));
        if visual_end > audio_end {
            let start = arr
                .markers
                .iter()
                .filter(|marker| marker.kind == os_model::TimeMarker::START)
                .map(|marker| u64::from(marker.position))
                .max()
                .filter(|&tick| tick < visual_end)
                .unwrap_or(0);
            span = Some((start, visual_end));
        }
        // Drop the planning engine before the live plugin-backed render starts.
        drop(engine);
    }
    let run = app.visualizer_video.as_mut().unwrap();
    run.tempo = project.tick_tempo(app.state.tempo);
    let start = span.map(|(start, _)| start).unwrap_or_else(|| {
        project
            .arrangement()
            .and_then(|a| {
                let end = a.clips.iter().map(|clip| clip.end()).max().unwrap_or(0);
                a.markers
                    .iter()
                    .filter(|m| m.kind == os_model::TimeMarker::START)
                    .map(|m| m.position)
                    .max()
                    .filter(|start| *start < end)
            })
            .map_or(0, u64::from)
    });
    run.beat = start as f64 / f64::from(project.ppq.max(1));
    if request.slot.is_none() {
        if project
            .arrangement()
            .is_none_or(|arr| arr.timeline.visuals.is_empty())
        {
            complete(
                app,
                Err("Add a scene to the Visuals lane before exporting.".into()),
            );
            return;
        };
        run.scenes = Some(SceneRender {
            project: project.clone(),
            map: project.tempo_map(app.state.tempo),
            ppq: project.ppq.max(1),
            start,
        });
    }
    let job = AudioJob {
        playlist_solos: Vec::new(),
        instrument_rows: None,
        capture_tracks: None,
        options: Options {
            tail: Tail::Cut,
            markers: false,
            loops: false,
            rate: 48_000,
            ..Options::default()
        },
        folder,
        name: "master".into(),
        auto_number: false,
        approved_replacements: Vec::new(),
        span,
        markers: Vec::new(),
        loop_frames: None,
        tempo: (run.tempo, 0.),
        track_names: Vec::new(),
    };
    run.rx = None;
    run.audio = true;
    app.state.dialogs.retain(
        |modal| !matches!(&modal.body, Body::Msg(msg) if msg.then == Then::VisualizerVideoProgress),
    );
    status(app, "Rendering the Master mix...", (0, 0));
    app.start_project_export(project, job, true);
    if app.export.is_none() {
        complete(app, Err("Could not start the audio export".into()));
    }
}

pub fn owns_audio(app: &App) -> bool {
    app.visualizer_video.as_ref().is_some_and(|run| run.audio)
}

pub fn audio_cancel(app: &App) -> Option<Arc<AtomicBool>> {
    app.visualizer_video
        .as_ref()
        .filter(|run| run.audio)
        .map(|run| run.cancel.clone())
}

pub fn audio_finished(app: &mut App, result: Result<(f64, Vec<PathBuf>), String>) {
    let run = app.visualizer_video.as_mut().unwrap();
    run.audio = false;
    if let Err(e) = check_cancel(&run.cancel) {
        complete(app, Err(e));
        return;
    }
    let wav = match result.and_then(|(_, files)| {
        files
            .into_iter()
            .next()
            .ok_or("Missing rendered mix".into())
    }) {
        Ok(path) => path,
        Err(error) => {
            complete(app, Err(error));
            return;
        }
    };
    let dir = run.dir.take().unwrap();
    let path = run.target.clone();
    let vertical = run.request.vertical;
    let replace = run.request.replace;
    let tempo = run.tempo;
    let beat = run.beat;
    let scenes = run.scenes.take();
    let cancel = run.cancel.clone();
    let (tx, rx) = mpsc::channel();
    run.rx = Some(rx);
    std::thread::spawn(move || {
        let result = tools_dir().and_then(|tools| {
            let report = |done, total| {
                let _ = tx.send(Event::Progress(
                    "Rendering and encoding video...".into(),
                    done,
                    total,
                ));
            };
            if let Some(scenes) = scenes {
                encode_scenes(
                    &tools, dir.path(), &wav, &path, vertical, replace, scenes, &cancel, report,
                )
            } else {
                encode(
                    &tools, dir.path(), &wav, &path, vertical, replace, tempo, beat, &cancel, report,
                )
            }
        });
        drop(dir);
        let _ = tx.send(Event::Finished(result));
    });
    status(app, "Rendering and encoding video...", (0, 0));
    if run_dialog(app) {
        progress(app, "Rendering and encoding video...", (0, 0));
    }
}

fn run_dialog(app: &App) -> bool {
    app.visualizer_video
        .as_ref()
        .is_some_and(|run| run.show_progress)
        && app.state.dialogs.is_empty()
}

fn complete(app: &mut App, result: Result<PathBuf, String>) {
    app.visualizer_video = None;
    app.state.dialogs.retain(
        |modal| !matches!(&modal.body,Body::Msg(msg) if msg.then==Then::VisualizerVideoProgress),
    );
    match result {
        Ok(p) => { app.state.message = Some(format!("Exported {}", p.display())); crate::donate::prompt(app); },
        Err(e) if e.starts_with("Video export cancelled") => app.state.message = Some(e),
        Err(e) => error(app, &e),
    }
    app.invalidate();
}

#[cfg(test)]
#[test]
fn supporter_video_reminder_only_follows_successful_completion() {
    let mut app=App::new(None);app.state.dialogs.clear();
    for message in ["Video export cancelled", "Worker failed"] {
        complete(&mut app,Err(message.into()));
        assert!(!app.state.dialogs.iter().any(|m|matches!(&m.body,Body::Msg(msg) if msg.then==Then::SupporterPrompt)));app.state.dialogs.clear();
    }
    complete(&mut app,Ok(PathBuf::from("export.mp4")));
    assert!(app.state.dialogs.iter().any(|m|matches!(&m.body,Body::Msg(msg) if msg.then==Then::SupporterPrompt)));
}

fn command(exe: &Path) -> Command {
    let mut cmd = Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[allow(clippy::too_many_arguments)]
fn encode(
    tools: &Path,
    dir: &Path,
    wav: &Path,
    dest: &Path,
    vertical: bool,
    replace: bool,
    tempo: f64,
    beat: f64,
    cancel: &AtomicBool,
    report: impl Fn(u64, u64),
) -> Result<PathBuf, String> {
    check_cancel(cancel)?;
    let (w, h) = if vertical { (1080, 1920) } else { (1920, 1080) };
    let dimensions = format!("{w}x{h}");
    let stage = dir.join("render.mp4");
    let marker = dir.join("progress.txt");
    let render_log = dir.join("renderer.log");
    let encoder_log = dir.join("encoder.log");
    let mut renderer = Process(
        command(&tools.join(WORKER))
            .arg(dir.join("scene.json"))
            .arg(wav)
            .arg(&marker)
            .args([
                w.to_string(),
                h.to_string(),
                "30".into(),
                tempo.to_string(),
                beat.to_string(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(File::create(&render_log).map_err(io_error)?)
            .spawn()
            .map_err(io_error)?,
    );
    let pipe = renderer.0.stdout.take().unwrap();
    let mut encoder = Process(
        command(&tools.join("ffmpeg.exe"))
            .args([
                "-nostdin",
                "-v",
                "error",
                "-n",
                "-f",
                "rawvideo",
                "-pixel_format",
                "rgba",
                "-video_size",
                &dimensions,
                "-framerate",
                "30",
                "-i",
                "pipe:0",
                "-i",
            ])
            .arg(wav)
            .args([
                "-vf",
                "vflip",
                "-c:v",
                "libx264",
                "-preset",
                "fast",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-b:a",
                "320k",
                "-shortest",
                "-movflags",
                "+faststart",
            ])
            .arg(&stage)
            .stdin(Stdio::from(pipe))
            .stdout(Stdio::null())
            .stderr(File::create(&encoder_log).map_err(io_error)?)
            .spawn()
            .map_err(io_error)?,
    );
    let fail = |path: &Path, name: &str| {
        let mut message = String::new();
        if let Ok(file) = File::open(path) {
            let _ = file.take(1024).read_to_string(&mut message);
        }
        format!("{name} stopped while exporting. {}", message.trim())
    };
    let mut previous = (0, 0);
    loop {
        check_cancel(cancel)?;
        let rendering = renderer.0.try_wait().map_err(io_error)?;
        let encoding = encoder.0.try_wait().map_err(io_error)?;
        if rendering.is_some_and(|s| !s.success()) {
            return Err(fail(&render_log, "The visualizer renderer"));
        }
        if encoding.is_some_and(|s| !s.success()) {
            return Err(fail(&encoder_log, "FFmpeg"));
        }
        if rendering.is_some() && encoding.is_some() {
            break;
        }
        if let Ok(text) = fs::read_to_string(&marker) {
            if let Some((a, b)) = text.trim().split_once('/') {
                if let (Ok(a), Ok(b)) = (a.parse::<u64>(), b.parse::<u64>()) {
                    if b > 0 && a <= b && previous != (a, b) {
                        previous = (a, b);
                        report(a, b);
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    check_cancel(cancel)?;
    if stage.metadata().map_err(io_error)?.len() == 0 {
        return Err("The video file is empty".into());
    }
    save_video(&stage, dest, replace)
        .map_err(|e| format!("Could not save {}: {e}", dest.display()))?;
    Ok(dest.to_owned())
}

#[cfg(windows)]
fn save_video(stage: &Path, dest: &Path, replace: bool) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MoveFileExW};
    let wide = |path: &Path| {
        path.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let (stage, dest) = (wide(stage), wide(dest));
    let flags = if replace {
        MOVEFILE_REPLACE_EXISTING
    } else {
        0
    };
    if unsafe { MoveFileExW(stage.as_ptr(), dest.as_ptr(), flags) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(windows))]
fn save_video(_stage: &Path, _dest: &Path, _replace: bool) -> io::Result<()> {
    Err(io::Error::other("Visualizer video export requires Windows"))
}

#[cfg(all(test, windows, target_arch = "x86_64"))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires the pinned CLAP, renderer and encoder on a machine with OpenGL"]
    fn visualizer_video_roundtrip() {
        let tools = tools_dir().unwrap();
        let fixtures = PathBuf::from(std::env::var_os("ML_VISUALIZER_VIDEO_TEST_FIXTURES").expect("fixture path"));
        let source = fixtures.join("test.wav");
        assert!(hosted_supported(), "Stock-named build must supply a renderer");
        assert!(!tools.join(WORKER).exists(), "start with a clean renderer directory");
        fs::create_dir_all(&tools).unwrap();
        let scratch = WorkDir::create(&tools).unwrap();
        fs::copy(fixtures.join("ffmpeg.exe"), tools.join("ffmpeg.exe")).unwrap();
        setup_hosted_export(false, &AtomicBool::new(false), |_, _| panic!("encoder was already installed")).unwrap();
        check_file(&tools.join(WORKER), catalog().renderer_size, &catalog().renderer_sha256).unwrap();
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state.tempo = 120.;
        app.state.selected_mixer = 3;
        let entry = crate::visualizer::available().expect("verified installed CLAP");
        let key = crate::plugins::fx_key(3, 0);
        let reference = crate::project_plugins::plugin_ref(&entry.desc, Vec::new());
        app.plugins.effects.push(entry);
        let index = app.plugins.effects.len() - 1;
        let hosted = app.plugins.prepare_effect(index, None, true).unwrap();
        app.plugins.hosted.insert(key, hosted);
        app.plugins.added(key);
        app.transport.set_effects(app.plugins.effects());
        let track = mx::track_mut(&mut app.state, 3).unwrap();
        track.slots[0] = Some(os_model::SlotPlugin {
            name: os_flp::wrapper::WRAPPER.into(),
            state: os_flp::wrapper::encode(None, &reference),
        });
        track.slots_enabled = true;
        let channel = &mut app.state.project.channels[0];
        channel.kind = os_model::ChannelKind::AudioClip;
        channel.sample_path = Some(source.to_string_lossy().into());
        channel.sound.sampler_flags = Some(os_model::ChannelSound::AUDIO_CLIP_FLAGS);
        crate::playlist::arr_mut(&mut app.state)
            .clips
            .push(os_model::Clip::new(
                96,
                192,
                0,
                os_model::ClipSource::Channel(0),
            ));
        app.state.playlist.range = Some((96, 288));
        use crate::settings_file::set_flag;
        set_flag(VISIBILITY, false);
        assert!(
            !mx::default_slot_menu(&app.state, 0, 0., 0.)
                .items
                .iter()
                .any(|item| item.label.starts_with(TITLE))
        );
        set_flag(VISIBILITY, true);
        assert!(
            mx::default_slot_menu(&app.state, 0, 0., 0.)
                .items
                .iter()
                .any(|item| item.label.starts_with(TITLE))
        );

        // The editor's instance is independent of the current Mixer selection.
        app.state.selected_mixer = 0;
        for (vertical, span, dimensions, seconds, frames) in [
            (false, None, (1920, 1080), 1.5, 45),
            (true, Some((96, 288)), (1080, 1920), 1.0, 30),
        ] {
            let output = scratch.path().join(if vertical {
                "vertical.mp4"
            } else {
                "landscape.mp4"
            });
            start(
                &mut app,
                Request {
                    slot: Some((3, 0)),
                    vertical,
                    span,
                    replace: false,
                },
                output.clone(),
                false,
            );
            let started = std::time::Instant::now();
            while app.visualizer_video.is_some() {
                assert!(started.elapsed() < Duration::from_secs(120));
                app.export_tick();
                tick(&mut app);
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(
                output.is_file(),
                "{:?}",
                app.state.dialogs.last().and_then(|d| match &d.body {
                    Body::Msg(m) => Some(&m.text),
                    _ => None,
                })
            );
            let probe = command(&fixtures.join("ffprobe.exe"))
                .args(["-v", "error", "-show_streams", "-of", "json"])
                .arg(&output)
                .output()
                .unwrap();
            assert!(probe.status.success());
            let json: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
            let streams = json["streams"].as_array().unwrap();
            let video = streams.iter().find(|s| s["codec_name"] == "h264").unwrap();
            let audio = streams.iter().find(|s| s["codec_name"] == "aac").unwrap();
            assert_eq!(
                (
                    video["width"].as_u64().unwrap(),
                    video["height"].as_u64().unwrap()
                ),
                dimensions
            );
            assert_eq!(
                video["nb_frames"].as_str().unwrap().parse::<u64>().unwrap(),
                frames
            );
            for stream in [video, audio] {
                let duration = stream["duration"].as_str().unwrap().parse::<f64>().unwrap();
                assert!(
                    (duration - seconds).abs() < 0.034,
                    "stream duration {duration}"
                );
            }
            let pcm = command(&tools.join("ffmpeg.exe"))
                .args(["-v", "error", "-i"])
                .arg(&output)
                .args(["-map", "0:a", "-f", "s16le", "-ac", "1", "pipe:1"])
                .output()
                .unwrap();
            assert!(pcm.status.success());
            assert!(
                pcm.stdout
                    .chunks_exact(2)
                    .any(|sample| i16::from_le_bytes([sample[0], sample[1]]).unsigned_abs() > 1000)
            );
            let hashes = command(&tools.join("ffmpeg.exe"))
                .args(["-v", "error", "-i"])
                .arg(&output)
                .args(["-an", "-f", "framemd5", "pipe:1"])
                .output()
                .unwrap();
            assert!(hashes.status.success());
            let hashes = String::from_utf8(hashes.stdout).unwrap();
            let first_last: Vec<_> = hashes
                .lines()
                .filter_map(|line| {
                    (!line.starts_with('#')).then(|| line.rsplit_once(',').unwrap().1)
                })
                .collect();
            assert_ne!(first_last.first(), first_last.last());
            println!(
                "{}×{}, {frames} frames, {seconds}s, audio and motion verified",
                dimensions.0, dimensions.1
            );
            let staged = scratch.path().join("temporary.mp4");
            fs::write(&staged, b"pending").unwrap();
            assert!(
                save_video(&staged, &output, false).is_err(),
                "a new destination must not replace an existing video"
            );
            assert!(staged.is_file());
        }
        let cancelled = scratch.path().join("cancelled.mp4");
        start(
            &mut app,
            Request {
                slot: Some((3, 0)),
                vertical: false,
                span: None,
                replace: false,
            },
            cancelled.clone(),
            false,
        );
        cancel(&mut app);
        let started = std::time::Instant::now();
        while app.visualizer_video.is_some() {
            assert!(started.elapsed() < Duration::from_secs(30));
            app.export_tick();
            tick(&mut app);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!cancelled.exists());
    }
}
#[derive(Clone)]
struct SceneRender {
    project: os_model::Project,
    map: os_model::timeline::TempoMap,
    ppq: u16,
    start: u64,
}

impl SceneRender {
    fn position(&self, elapsed: f64) -> f64 {
        self.map.tick(self.map.seconds(self.start as f64) + elapsed)
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_scenes(
    tools: &Path,
    dir: &Path,
    wav: &Path,
    dest: &Path,
    vertical: bool,
    replace: bool,
    scenes: SceneRender,
    cancel: &AtomicBool,
    report: impl Fn(u64, u64),
) -> Result<PathBuf, String> {
    check_cancel(cancel)?;
    let (w, h) = if vertical { (1080, 1920) } else { (1920, 1080) };
    let dimensions = format!("{w}x{h}");
    let stage = dir.join("render.mp4");
    let encoder_log = dir.join("encoder.log");
    let audio = os_engine::sample::load(wav).map_err(|error| error.to_string())?;
    let rate = u64::from(audio.rate);
    let frames = (audio.frames() as u64 * 30).div_ceil(rate);
    if frames == 0 {
        return Err("The Master render contains no audio frames".into());
    }
    let mut artwork = crate::visual_scenes::Artwork::default();
    let mut compositor = crate::visual_scenes::Compositor::default();
    let clips = &scenes.project.arrangement().unwrap().timeline.visuals;
    let mut canvas = os_ui::Canvas::new(w, h, 1.)?;
    let rect = os_ui::Rect::new(0., 0., w as f32, h as f32);
    let mut encoder = Process(
        command(&tools.join("ffmpeg.exe"))
            .args([
                "-nostdin",
                "-v",
                "error",
                "-n",
                "-f",
                "rawvideo",
                "-pixel_format",
                "rgba",
                "-video_size",
                &dimensions,
                "-framerate",
                "30",
                "-i",
                "pipe:0",
                "-i",
            ])
            .arg(wav)
            .args([
                "-c:v",
                "libx264",
                "-preset",
                "fast",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-b:a",
                "320k",
                "-movflags",
                "+faststart",
            ])
            .arg(&stage)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(File::create(&encoder_log).map_err(io_error)?)
            .spawn()
            .map_err(io_error)?,
    );
    let mut input = BufWriter::new(encoder.0.stdin.take().unwrap());
    let paint = (|| {
        for frame in 0..frames {
            check_cancel(cancel)?;
            let seconds = frame as f64 / 30.;
            let position = scenes.position(seconds);
            let layers = crate::visual_scenes::layers(&scenes.project, clips, None, position);
            // Take the same trailing analysis window as the live Master tap.
            let end = (frame * rate / 30).min(audio.frames() as u64) as usize;
            let mut mono = [0.; 1024];
            let start = end.saturating_sub(mono.len());
            let padding = mono.len() - (end - start);
            for (sample, (&l, &r)) in mono[padding..]
                .iter_mut()
                .zip(audio.left[start..end].iter().zip(&audio.right[start..end]))
            {
                *sample = (l + r) * 0.5;
            }
            let signal = crate::visual_scenes::Signal::from_audio(&mono);
            let art = [
                artwork.for_scene(layers[0].map(|layer| layer.scene))?,
                artwork.for_scene(layers[1].map(|layer| layer.scene))?,
            ];
            compositor.paint(
                &mut canvas,
                rect,
                layers,
                art.each_ref().map(|image| image.as_deref()),
                position / f64::from(scenes.ppq),
                signal,
            );
            input.write_all(canvas.pixmap.data()).map_err(io_error)?;
            if frame % 5 == 0 || frame + 1 == frames {
                report(frame + 1, frames);
            }
        }
        input.flush().map_err(io_error)
    })();
    drop(input);
    if let Err(error) = paint {
        if cancel.load(Ordering::Relaxed) {
            return Err(error);
        }
        let log = fs::read_to_string(&encoder_log).unwrap_or_default();
        return Err(if log.trim().is_empty() {
            error
        } else {
            format!("FFmpeg: {}", log.trim())
        });
    }
    let status = loop {
        check_cancel(cancel)?;
        if let Some(status) = encoder.0.try_wait().map_err(io_error)? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    if !status.success() || stage.metadata().map_err(io_error)?.len() == 0 {
        let log = fs::read_to_string(&encoder_log).unwrap_or_default();
        return Err(format!("FFmpeg did not finish the video. {}", log.trim()));
    }
    check_cancel(cancel)?;
    save_video(&stage, dest, replace)
        .map_err(|error| format!("Could not save {}: {error}", dest.display()))?;
    Ok(dest.to_owned())
}
#[cfg(all(test, windows, target_arch = "x86_64"))]
mod workspace_tests {
    use super::*;
    use crate::visual_scenes::Command;
    use os_model::{
        ChannelKind, ChannelSound, Clip, ClipSource,
        timeline::{TempoPoint, VisualTemplate},
    };

    #[test]
    #[ignore = "requires the pinned encoder and ffprobe"]
    fn visual_workspace_tempo_export() {
        let folder = tools_dir().unwrap();
        let scratch = WorkDir::create(&folder).unwrap();
        let source = scratch.path().join("source.wav");
        let mix: Vec<_> = (0..180_000)
            .flat_map(|i| {
                let v = (i as f32 * 0.057).sin() * 0.22;
                [v, v * 0.8]
            })
            .collect();
        os_engine::wav::write_f32_stereo(&mut File::create(&source).unwrap(), 48_000, &mix)
            .unwrap();
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state.tempo = 60.;
        let channel = &mut app.state.project.channels[0];
        channel.kind = ChannelKind::AudioClip;
        channel.sample_path = Some(source.to_string_lossy().into());
        channel.sound.sampler_flags = Some(ChannelSound::AUDIO_CLIP_FLAGS);
        let arrangement = crate::playlist::arr_mut(&mut app.state);
        arrangement
            .clips
            .push(Clip::new(0, 384, 0, ClipSource::Channel(0)));
        arrangement.timeline.tempo = vec![
            TempoPoint {
                tick: 0,
                bpm: 60.,
                ramp: true,
                ramp_end: None,
            },
            TempoPoint {
                tick: 192,
                bpm: 120.,
                ramp: false,
                ramp_end: None,
            },
        ];
        app.state.playlist.range = Some((0, 192));
        crate::visual_scenes::run(&mut app, Command::Add(0));
        app.state.playlist.range = Some((192, 384));
        crate::visual_scenes::run(&mut app, Command::Add(192));
        crate::visual_scenes::run(&mut app, Command::Template(VisualTemplate::Pulse));
        crate::visual_scenes::run(&mut app, Command::Text(false));
        app.state.playlist.edit.as_mut().unwrap().text = "Second scene".into();
        crate::playlist::commit_edit(&mut app.state);

        let orig = app
            .state
            .project
            .arrangement()
            .unwrap()
            .timeline
            .visuals
            .clone();
        assert_eq!((orig[0].end, orig[1].tick, orig[1].end), (192, 192, 384));
        assert_eq!(orig[1].title, "Second scene");
        assert_eq!(orig[1].template, VisualTemplate::Pulse);
        crate::history::undo(&mut app.state).unwrap();
        assert_ne!(
            app.state.project.arrangement().unwrap().timeline.visuals[1].title,
            "Second scene"
        );
        crate::history::redo(&mut app.state).unwrap();

        let reopened = os_flp::project::load(
            &os_flp::FlpFile::parse(
                &os_flp::project::create(&app.state.project)
                    .unwrap()
                    .to_bytes()
                    .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(reopened.arrangement().unwrap().timeline.visuals, orig);
        assert!(app.state.shown(crate::app::Win::Visualizer));
        let mut canvas = os_ui::Canvas::new(1100, 650, 1.).unwrap();
        app.state.native_view = Some(crate::app::Win::Visualizer);
        app.state.native_input = app.state.native_view;
        app.paint(&mut canvas, None);
        assert!(app.hits.iter().any(|hit| hit.action == crate::app::Action::DetachWin(crate::app::Win::Visualizer)));
        assert!(
            app.hits
                .iter()
                .any(|hit| hit.action == crate::app::Action::Visual(Command::Export))
        );
        app.state.native_view = None;
        app.state.native_input = None;
        let cut = app.state.project.tempo_map(60.).seconds(192.);
        let end = app.state.project.tempo_map(60.).seconds(384.);
        assert!((cut - 2. * 2_f64.ln()).abs() < 0.001);
        let output = scratch.path().join("tempo.mp4");
        start(
            &mut app,
            Request {
                slot: None,
                vertical: false,
                span: None,
                replace: false,
            },
            output.clone(),
            false,
        );
        let started = std::time::Instant::now();
        while app.visualizer_video.is_some() {
            assert!(started.elapsed() < Duration::from_secs(90));
            app.export_tick();
            tick(&mut app);
            std::thread::sleep(Duration::from_millis(8));
        }
        assert!(
            output.is_file(),
            "{:?}",
            app.state.dialogs.last().and_then(|d| match &d.body {
                Body::Msg(m) => Some(&m.text),
                _ => None,
            })
        );
        let decode = command(&folder.join("ffmpeg.exe"))
            .args(["-v", "error", "-i"])
            .arg(&output)
            .args([
                "-an",
                "-vf",
                "crop=2:2:960:540",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "pipe:1",
            ])
            .output()
            .unwrap();
        assert!(decode.status.success());
        let pixels: Vec<_> = decode
            .stdout
            .chunks_exact(12)
            .map(|v| [v[0], v[1], v[2]])
            .collect();
        let first = pixels[0];
        let first_pulse = pixels
            .iter()
            .position(|pixel| pixel[0] > first[0] + 10)
            .unwrap_or_else(|| {
                panic!(
                    "no scene cut in decoded pixels: {first:?}, {:?}",
                    pixels.last()
                )
            });
        assert_eq!(
            first_pulse,
            (cut * 30.).ceil() as usize,
            "cut frame with a tempo ramp"
        );
        assert_eq!(
            pixels.len(),
            (end * 30.).ceil() as usize,
            "last video frame covers the end of audio"
        );
        let probe = command(&folder.join("ffprobe.exe"))
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "stream=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(output)
            .output()
            .unwrap();
        assert!(probe.status.success());
        let audio_duration: f64 = String::from_utf8(probe.stdout)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert!(
            (audio_duration - end).abs() < 0.034,
            "audio was {audio_duration}s"
        );
        println!(
            "Scene cut at frame {first_pulse} / {}, audio {audio_duration:.3}s",
            pixels.len()
        );

        use crate::visual_scenes::{self as visuals, Edge, Parameter};
        crate::scoring::update(&mut app.state, "Fade test", |timeline| {
            assert_eq!(
                visuals::drag(&mut timeline.visuals, 0, 24, Edge::FadeIn),
                Some(0)
            );
            assert_eq!(
                visuals::drag(&mut timeline.visuals, 192, 96, Edge::FadeIn),
                Some(192)
            );
            assert_eq!(
                visuals::drag(&mut timeline.visuals, 192, -96, Edge::FadeOut),
                Some(192)
            );
        });
        visuals::run(&mut app, Command::Crossfade);
        visuals::run(&mut app, Command::Automate(Parameter::Opacity));
        let automation_id = app.state.project.automations.last().unwrap().channel;
        let scene_id = app.state.project.arrangement().unwrap().timeline.visuals[1].automation_id;
        assert_ne!(scene_id, 0);
        crate::history::undo(&mut app.state).unwrap();
        assert!(app.state.project.automation(automation_id).is_none());
        assert_eq!(
            app.state.project.arrangement().unwrap().timeline.visuals[1].automation_id,
            0
        );
        crate::history::redo(&mut app.state).unwrap();
        let curve = &mut app
            .state
            .project
            .automation_mut(automation_id)
            .unwrap()
            .points;
        for point in curve {
            point.value = 0.5;
        }
        for param in [Parameter::Zoom, Parameter::Intensity] {
            visuals::run(&mut app, Command::Automate(param));
            let curve = &mut app.state.project.automations.last_mut().unwrap().points;
            for point in curve {
                point.value = 0.25;
            }
        }
        let reopened =
            os_flp::project::load(&os_flp::project::create(&app.state.project).unwrap()).unwrap();
        let model = reopened.arrangement().unwrap();
        assert!(model.timeline.visuals[1].style.crossfade);
        assert_eq!(model.timeline.visuals[1].style.fade_in, 96);
        assert_eq!(reopened.automations.len(), 3);
        for a in &reopened.automations {
            assert!(a.targets.iter().any(|t| t & 0xffff == u32::from(scene_id)));
        }
        let project = &app.state.project;
        let scenes = &project.arrangement().unwrap().timeline.visuals;
        let at = visuals::layers(project, scenes, None, 144.);
        assert!((at[0].unwrap().opacity_for_test() - 0.5).abs() < 1e-5);
        assert!((at[1].unwrap().opacity_for_test() - 0.25).abs() < 1e-5);
        assert_eq!(at[1].unwrap().controls_for_test(), (0.75, 0.25));
        assert!(visuals::layers(project, scenes, None, 80.)[1].is_none());

        // Go through the same workspace paint as a detached pane while scrubbing in the Playlist.
        app.state.native_view = Some(crate::app::Win::Visualizer);
        app.state.native_input = app.state.native_view;
        let mut preview = |at, app: &mut App| {
            app.state.playlist.pos = at;
            visuals::tick(app);
            app.paint(&mut canvas, None);
            let offset = (300 * canvas.pixmap.width() as usize + 632) * 4;
            canvas.pixmap.data()[offset..offset + 3]
                .iter()
                .map(|&x| u16::from(x))
                .sum::<u16>()
        };
        let middle = preview(144, &mut app);
        std::thread::sleep(Duration::from_millis(55));
        let next = preview(195, &mut app);
        assert!(
            middle > next,
            "scrub preview follows the fade and opacity: {middle} to {next}"
        );
        app.state.native_view = None;
        app.state.native_input = None;

        for (vertical, crop) in [(false, "crop=2:2:960:540"), (true, "crop=2:2:540:960")] {
            let output = scratch.path().join(if vertical {
                "fade-portrait.mp4"
            } else {
                "fade-landscape.mp4"
            });
            start(
                &mut app,
                Request {
                    slot: None,
                    vertical,
                    span: Some((0, 384)),
                    replace: false,
                },
                output.clone(),
                false,
            );
            let started = std::time::Instant::now();
            while app.visualizer_video.is_some() {
                assert!(started.elapsed() < Duration::from_secs(90));
                app.export_tick();
                tick(&mut app);
                std::thread::sleep(Duration::from_millis(8));
            }
            assert!(
                output.is_file(),
                "{:?}",
                app.state.dialogs.last().and_then(|d| match &d.body {
                    Body::Msg(m) => Some(&m.text),
                    _ => None,
                })
            );
            let decode = command(&folder.join("ffmpeg.exe"))
                .args(["-v", "error", "-i"])
                .arg(&output)
                .args([
                    "-an", "-vf", crop, "-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1",
                ])
                .output()
                .unwrap();
            assert!(decode.status.success());
            let pixels = decode
                .stdout
                .chunks_exact(12)
                .map(|p| [p[0], p[1], p[2]])
                .collect::<Vec<_>>();
            assert_eq!(pixels.len(), (end * 30.).ceil() as usize);
            let map = app.state.project.tempo_map(60.);
            let frame = |tick| (map.seconds(tick) * 30.).ceil() as usize;
            let brightness = |i: usize| pixels[i].into_iter().map(u16::from).sum::<u16>();
            assert!(brightness(0) < 8, "fade opens in black");
            assert!(
                brightness(frame(60.)) > 45,
                "first scene reaches full brightness"
            );
            assert!(
                brightness(frame(144.)) > brightness(frame(195.)),
                "crossfade moves into lower automated opacity"
            );
            assert!(brightness(pixels.len() - 1) < 14, "outro fades to black");
            println!(
                "{} fades and automation verified across {} decoded frames",
                if vertical { "Vertical" } else { "Landscape" },
                pixels.len()
            );
        }
    }
}
#[cfg(windows)]
pub fn from_plugin_window(app: &mut App, key: usize, x: i32, y: i32) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
    use windows_sys::Win32::UI::WindowsAndMessaging::{IsWindow, SetForegroundWindow};
    if app.modal_open() || app.loading() {
        return;
    }
    let Some(slot) = crate::plugins::fx_of(key) else {
        return;
    };
    if !crate::workspace_visibility::shown(VISIBILITY)
        || !app
            .plugins
            .descriptor(key)
            .is_some_and(|desc| crate::visualizer::is_managed(&desc))
    {
        return;
    }
    let Some((owner, _)) = app.plugins.window_of(key) else {
        return;
    };
    let owner = owner as windows_sys::Win32::Foundation::HWND;
    let activate_main = |app: &mut App| {
        app.state.native_input = None;
        let main = app.plugins.owner as windows_sys::Win32::Foundation::HWND;
        unsafe {
            if IsWindow(main) != 0 {
                SetForegroundWindow(main);
                SetFocus(main);
            }
        }
    };
    if !available() {
        activate_main(app);
        error(
            app,
            "Visualizer MP4 export is unavailable in this build or has been hidden in Workspace Visibility.",
        );
        return;
    }
    if let Some(run) = &app.visualizer_video {
        let (label, amount) = run.status.clone();
        activate_main(app);
        progress(app, &label, amount);
        return;
    }

    let range = selection_span(app);
    let Some((vertical, selection)) = window_choice(owner, x, y, range.is_some()) else {
        return;
    };
    let project = app.state.path.as_deref();
    let default = format!("{}.mp4", crate::backup::project_name(project));
    let path = match window_save(owner, project.and_then(Path::parent), &default) {
        Ok(Some(path)) => path,
        Ok(None) => return,
        Err(e) => {
            activate_main(app);
            error(app, &e);
            return;
        }
    };
    if !mp4_path(&path) {
        activate_main(app);
        error(app, "Choose an MP4 filename for the video.");
        return;
    }
    let download = !encoder_present();
    if download && !confirm_encoder_download(owner) {
        return;
    }
    let request = Request {
        slot: Some(slot),
        vertical,
        span: range.filter(|_| selection),
        replace: path.is_file(),
    };
    // The editor owns the format menu, filename chooser and first-run consent.
    // After the final confirmation, activate the existing render progress.
    activate_main(app);
    start(app, request, path, download);
}

fn selection_span(app: &App) -> Option<(u64, u64)> {
    app.state
        .playlist
        .range
        .filter(|(a, b)| a != b)
        .map(|(a, b)| (u64::from(a.min(b)), u64::from(a.max(b))))
}

#[cfg(windows)]
fn window_choice(
    owner: windows_sys::Win32::Foundation::HWND,
    x: i32,
    y: i32,
    range: bool,
) -> Option<(bool, bool)> {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let menu = unsafe { CreatePopupMenu() };
    if menu.is_null() {
        return None;
    }
    let entries = [
        (1, "Landscape 1080p · Whole song"),
        (2, "Vertical 1080p · Whole song"),
        (3, "Landscape 1080p · Playlist selection"),
        (4, "Vertical 1080p · Playlist selection"),
    ];
    unsafe {
        AppendMenuW(
            menu,
            MF_STRING | MF_DISABLED,
            0,
            wide("MP4 · 30 fps · Master mix").as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        for (id, label) in entries.into_iter().take(if range { 4 } else { 2 }) {
            AppendMenuW(menu, MF_STRING, id, wide(label).as_ptr());
        }
    }
    let choice = unsafe {
        TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY,
            x,
            y,
            0,
            owner,
            std::ptr::null(),
        )
    };
    unsafe { DestroyMenu(menu) };
    match choice {
        1 => Some((false, false)),
        2 => Some((true, false)),
        3 if range => Some((false, true)),
        4 if range => Some((true, true)),
        _ => None,
    }
}

#[cfg(windows)]
fn confirm_encoder_download(owner: windows_sys::Win32::Foundation::HWND) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IDCANCEL, MB_ICONQUESTION, MB_OKCANCEL, MessageBoxW,
    };
    let text: Vec<_> = download_prompt().encode_utf16().chain(Some(0)).collect();
    let caption: Vec<_> = "Export MP4".encode_utf16().chain(Some(0)).collect();
    let answer = unsafe {
        MessageBoxW(
            owner,
            text.as_ptr(),
            caption.as_ptr(),
            MB_OKCANCEL | MB_ICONQUESTION,
        )
    };
    answer != 0 && answer != IDCANCEL
}

#[cfg(windows)]
fn window_save(
    owner: windows_sys::Win32::Foundation::HWND,
    folder: Option<&Path>,
    name: &str,
) -> Result<Option<PathBuf>, String> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let mut file = vec![0u16; 32_768];
    let name = wide(name);
    let name_len = name.len().min(file.len() - 1);
    file[..name_len].copy_from_slice(&name[..name_len]);
    let folder = folder.map(|p| {
        p.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    });
    let filter = wide("MP4 video (*.mp4)\0*.mp4\0");
    let extension = wide("mp4");
    let caption = wide("Export MP4");
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: owner,
        lpstrFilter: filter.as_ptr(),
        nFilterIndex: 1,
        lpstrFile: file.as_mut_ptr(),
        nMaxFile: file.len() as u32,
        lpstrInitialDir: folder.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()),
        lpstrTitle: caption.as_ptr(),
        Flags: OFN_EXPLORER | OFN_NOCHANGEDIR | OFN_PATHMUSTEXIST | OFN_OVERWRITEPROMPT,
        lpstrDefExt: extension.as_ptr(),
        ..Default::default()
    };
    if unsafe { GetSaveFileNameW(&mut dialog) } != 0 {
        let len = file.iter().position(|&c| c == 0).unwrap_or(file.len());
        return Ok(Some(std::ffi::OsString::from_wide(&file[..len]).into()));
    }
    let code = unsafe { CommDlgExtendedError() };
    if code == 0 {
        Ok(None)
    } else {
        Err(format!(
            "Could not open the MP4 save dialog (Windows error {code:#x})."
        ))
    }
}
