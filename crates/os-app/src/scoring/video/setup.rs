// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;
use crate::dialog::{Answer, Body, Kind, Msg, Then, spans};
use crate::visualizer_video as tools;
use std::sync::{Arc, OnceLock, atomic::{AtomicBool, Ordering}};

const TITLE: &str = "Set up Playlist video";

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ImportFile {
    pub(super) arrangement: usize,
    pub(super) path: PathBuf,
    pub(super) start: f64,
    pub(super) relink: Option<Video>,
}

enum Event {
    Progress(u64, u64),
    Finished(Result<(), String>),
}

pub(super) struct Run {
    pending: Option<ImportFile>,
    cancel: Arc<AtomicBool>,
    events: mpsc::Receiver<Event>,
}

impl Drop for Run {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

fn platform() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "x86_64") => Some("macos-x86_64"),
        ("macos", "aarch64") => Some("macos-aarch64"),
        ("linux", "x86_64") => Some("linux-x86_64"),
        _ => None,
    }
}

fn package() -> Option<&'static tools::EncoderPackage> {
    static PACKAGES: OnceLock<std::collections::HashMap<String, tools::EncoderPackage>> = OnceLock::new();
    PACKAGES.get_or_init(|| {
        serde_json::from_str(include_str!("../../../../../packaging/visualizer/reference-video-tools.json"))
            .expect("validated Playlist video tools catalog")
    }).get(platform()?)
}

fn root() -> Option<PathBuf> {
    Some(os_paths::user_dir()?.join("Components").join("Reference Video").join(platform()?))
}

pub(super) fn installed() -> Option<PathBuf> {
    if tools::supported() {
        return tools::installed_encoder();
    }
    let p = root()?.join("ffmpeg");
    let meta = std::fs::symlink_metadata(&p).ok()?;
    if !meta.is_file() || meta.len() != package()?.encoder_size {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }
    Some(p)
}

fn size() -> Option<u64> {
    if tools::supported() { Some(tools::encoder_download_size()) } else { Some(package()?.archive_size) }
}

fn install(cancel: &AtomicBool, report: impl Fn(u64, u64)) -> Result<(), String> {
    if tools::supported() {
        return tools::setup_reference_video(cancel, report);
    }
    if installed().is_some() {
        return Ok(());
    }
    let root = root().ok_or("The ML Studio profile is unavailable")?;
    let c = package().ok_or("No FFmpeg download is available for this platform")?;
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let lock = std::fs::OpenOptions::new().create(true).truncate(false).write(true)
        .open(root.join("encoder.lock")).map_err(|e| e.to_string())?;
    lock.try_lock().map_err(|_| "Another ML Studio instance is setting up Playlist video")?;
    if installed().is_some() {
        return Ok(());
    }
    tools::download_encoder(&root, "ffmpeg", c, cancel, report)
}

pub(crate) fn offer(app: &mut App, pending: Option<ImportFile>) {
    if tool().is_ok() {
        resume(app, pending);
        return;
    }
    if app.state.playlist.scoring.video.setup.is_some() {
        app.state.message = Some("Wait for Playlist video setup to finish.".into());
        app.invalidate();
        return;
    }
    let Some(bytes) = size() else {
        app.tell(Kind::Info, TITLE, spans(&["Install FFmpeg on your system PATH to enable Playlist video on this platform."]));
        return;
    };
    let text = format!(
        "Download FFmpeg for video import and preview? The download is {:.1} MB. Once installed, Playlist video works offline.",
        bytes as f64 / 1_000_000.0
    );
    let mut prompt = Msg::new(Kind::Question, TITLE, spans(&[&text]),
        &[Answer::Yes, Answer::Cancel], Then::ReferenceVideoSetup(pending))
        .labelled(&["Download", "Cancel"]);
    prompt.focus = 1;
    app.show_msg(prompt);
}

pub(crate) fn start(app: &mut App, pending: Option<ImportFile>) {
    if tool().is_ok() {
        resume(app, pending);
        return;
    }
    if app.state.playlist.scoring.video.setup.is_some() {
        return;
    }
    let Some(total) = size() else { return };
    let cancel = Arc::new(AtomicBool::new(false));
    let worker = cancel.clone();
    let (tx, events) = mpsc::channel();
    app.state.playlist.scoring.video.setup = Some(Run { pending, cancel, events });
    let mut msg = Msg::new(Kind::Info, TITLE, spans(&["Downloading FFmpegâ€¦"]),
        &[Answer::Cancel], Then::ReferenceVideoProgress);
    msg.progress = Some((0, total));
    app.show_msg(msg);
    std::thread::spawn(move || {
        let result = install(&worker, |done, total| {
            let _ = tx.send(Event::Progress(done, total));
            os_host::wake();
        });
        let _ = tx.send(Event::Finished(result));
        os_host::wake();
    });
}

pub(crate) fn cancel(app: &mut App) {
    if let Some(run) = &app.state.playlist.scoring.video.setup {
        run.cancel.store(true, Ordering::Relaxed);
        app.state.message = Some("Cancelling Playlist video setupâ€¦".into());
        app.invalidate();
    }
}

fn resume(app: &mut App, pending: Option<ImportFile>) {
    let s = &mut app.state;
    s.playlist.scoring.video.error = None;
    s.playlist.scoring.video.requested = None;
    s.playlist.scoring.video.last = None;
    if let Some(file) = pending && s.project.arrangement_index() == file.arrangement {
        import(app, file.path, file.start, file.relink);
    } else {
        app.state.message = Some("Playlist video is ready. Use Tracks > Import video to add a file.".into());
        app.invalidate();
    }
}

pub(super) fn poll(app: &mut App) {
    loop {
        let Some(run) = &app.state.playlist.scoring.video.setup else { return };
        let result = match run.events.try_recv() {
            Ok(Event::Progress(done, total)) => {
                if !run.cancel.load(Ordering::Relaxed) {
                    for modal in &mut app.state.dialogs {
                        if let Body::Msg(msg) = &mut modal.body && msg.then == Then::ReferenceVideoProgress {
                            msg.progress = Some((done, total));
                        }
                    }
                    app.invalidate();
                }
                continue;
            }
            Ok(Event::Finished(result)) => result,
            Err(mpsc::TryRecvError::Disconnected) => Err("The FFmpeg download stopped unexpectedly.".into()),
            Err(mpsc::TryRecvError::Empty) => return,
        };
        let mut run = app.state.playlist.scoring.video.setup.take().expect("active video setup");
        app.state.dialogs.retain(|modal| !matches!(&modal.body, Body::Msg(msg) if msg.then == Then::ReferenceVideoProgress));
        if run.cancel.load(Ordering::Relaxed) {
            app.state.message = Some("Playlist video setup cancelled.".into());
        } else if let Err(error) = result {
            app.tell(Kind::Error, TITLE, spans(&[&error, "\n\nUse Set up video playback in the Video track menu to retry."]));
        } else if tool().is_ok() {
            resume(app, run.pending.take());
        } else {
            app.tell(Kind::Error, TITLE, spans(&["FFmpeg could not be found after setup. Use Set up video playback in the Video track menu to retry."]));
        }
        app.invalidate();
        return;
    }
}

#[cfg(all(test, windows, target_arch = "x86_64"))]
mod tests {
    use super::*;
    use crate::dialog::Reply;

    #[test]
    fn playlist_video_downloads_and_resumes_import_when_requested() {
        let Some(file) = std::env::var_os("ML_TEST_REFERENCE_VIDEO_INSTALL") else { return };
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        assert!(tool().is_err(), "use an isolated profile without FFmpeg on PATH");
        crate::scoring::run(&mut app, crate::scoring::Command::Show(Lane::Video, true));
        assert!(matches!(app.state.dialogs.last().map(|d| &d.body),
            Some(Body::Msg(msg)) if msg.then == Then::ReferenceVideoSetup(None)));
        app.answer(Reply::Button(Answer::Cancel));
        assert!(app.state.playlist.scoring.video.setup.is_none());
        assert!(tool().is_err());

        import(&mut app, file.into(), 2.0, None);
        assert!(matches!(app.state.dialogs.last().map(|d| &d.body),
            Some(Body::Msg(msg)) if matches!(msg.then, Then::ReferenceVideoSetup(Some(_)))));
        app.answer(Reply::Button(Answer::Yes));
        let deadline = Instant::now() + Duration::from_secs(150);
        loop {
            super::super::poll(&mut app);
            if !data(&app.state).videos.is_empty() { break; }
            assert!(Instant::now() < deadline, "{}", app.state.message.as_deref().unwrap_or("waiting for video"));
            if let Some(Body::Msg(msg)) = app.state.dialogs.last().map(|d| &d.body)
                && msg.kind == Kind::Error
            {
                panic!("video setup failed: {}", msg.plain());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let video = &data(&app.state).videos[0];
        assert_eq!(video.start, 2.0);
        assert_eq!(video.fps, 24.0);
        assert_ne!(frame(&video.path, 0.0).unwrap(), frame(&video.path, 1.25).unwrap());
        assert_eq!(tool().ok(), tools::installed_encoder(), "the Playlist shares the Visualizer dependency");
        crate::scoring::run(&mut app, crate::scoring::Command::Show(Lane::Video, true));
        assert!(app.state.dialogs.is_empty(), "an existing dependency is reused");
        assert!(app.state.playlist.scoring.video.setup.is_none());
    }
}
