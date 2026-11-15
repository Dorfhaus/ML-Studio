//! Reference video is decoded off-thread using a local FFmpeg executable.
use super::*;
use os_model::timeline::Video;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

pub const EXTENSIONS: &[&str] = &["mp4", "mov", "mkv", "avi", "webm", "m4v", "mpg", "mpeg"];
const WIDTH: usize = 416;
const HEIGHT: usize = 234;

type FrameResult = Result<Vec<u32>, String>;
type Decode = (u64, u64, mpsc::Receiver<FrameResult>);
type Import = (usize, Option<Video>, mpsc::Receiver<Result<Video, String>>);

pub struct Player {
    pub open: bool,
    source: Option<(String, f64, f64)>,
    generation: u64,
    requested: Option<u64>,
    image: Option<Vec<u32>>,
    decode: Option<Decode>,
    import: Option<Import>,
    last: Option<Instant>,
    error: Option<String>,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            open: true,
            source: None,
            generation: 0,
            requested: None,
            image: None,
            decode: None,
            import: None,
            last: None,
            error: None,
        }
    }
}

fn tool() -> Result<PathBuf, String> {
    let custom = std::env::var_os("ML_STUDIO_FFMPEG")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute());
    let name = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    let exe = std::env::current_exe().ok();
    let beside_app = exe.as_deref().and_then(Path::parent);
    let path = std::env::var_os("PATH");
    custom
        .into_iter()
        .chain(
            beside_app
                .into_iter()
                .flat_map(|dir| [dir.join(name), dir.join("Sample Exchange").join(name)]),
        )
        .chain(
            path.as_deref()
                .into_iter()
                .flat_map(std::env::split_paths)
                .filter(|dir| dir.is_absolute())
                .map(|dir| dir.join(name)),
        )
        .find(|path| path.is_file())
        .ok_or_else(|| {
            "FFmpeg is required for video preview. Install FFmpeg and put it on PATH, then retry."
                .into()
        })
}

fn decode_command(ffmpeg: &Path) -> Command {
    let mut cmd = Command::new(ffmpeg);
    cmd.args([
        "-hide_banner",
        "-nostdin",
        "-protocol_whitelist",
        "file,pipe",
        "-format_whitelist",
        "mov,matroska,webm,avi,mpeg,mpegts",
        "-threads",
        "2",
        "-filter_threads",
        "1",
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd
}

fn bounded_output(cmd: &mut Command, timeout: Duration) -> Result<Output, String> {
    use std::io::Read;
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start FFmpeg: {e}"))?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let read = |pipe: &mut dyn Read, limit: usize| {
        let mut buf = [0u8; 4096];
        let mut result = Vec::new();
        while let Ok(n) = pipe.read(&mut buf) {
            if n == 0 {
                break;
            }
            result.extend_from_slice(&buf[..n.min(limit.saturating_sub(result.len()))]);
        }
        result
    };
    let a = std::thread::spawn(move || read(&mut stdout, WIDTH * HEIGHT * 3 + 1));
    let b = std::thread::spawn(move || read(&mut stderr, 64 * 1024));
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(8)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break Err("FFmpeg did not finish decoding in time.".to_string());
            }
        }
    };
    let stdout = a.join().unwrap_or_default();
    let stderr = b.join().unwrap_or_default();
    Ok(Output {
        status: status?,
        stdout,
        stderr,
    })
}

fn timestamp(s: &str) -> Option<f64> {
    let mut fields = s.split(':');
    let h = fields.next()?.parse::<f64>().ok()?;
    let m = fields.next()?.parse::<f64>().ok()?;
    let s = fields.next()?.parse::<f64>().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

fn probe(path: PathBuf, start: f64) -> Result<Video, String> {
    let ffmpeg = tool()?;
    let path = path
        .canonicalize()
        .map_err(|e| format!("Video file could not be opened: {e}"))?;
    let output = bounded_output(
        decode_command(&ffmpeg).arg("-i").arg(&path),
        Duration::from_secs(12),
    )?;
    let info = String::from_utf8_lossy(&output.stderr);
    let duration = info
        .lines()
        .find_map(|line| {
            let tail = line.trim_start().strip_prefix("Duration: ")?;
            timestamp(tail.split(',').next()?.trim())
        })
        .filter(|t| (0.001..=86400.0).contains(t))
        .ok_or("Could not read the video's duration.")?;
    let fps = info
        .lines()
        .filter(|l| l.contains("Video:") && !l.contains("attached pic"))
        .find_map(|line| {
            line.split(',')
                .find_map(|item| item.trim().strip_suffix(" fps")?.parse::<f64>().ok())
        })
        .filter(|t| (1.0..=240.0).contains(t))
        .ok_or("Could not read a supported video frame rate.")?;
    // FFmpeg's human-readable probe rounds common fractional frame rates.
    let fps = [
        (23.98, 24_000.0 / 1001.0),
        (29.97, 30_000.0 / 1001.0),
        (47.95, 48_000.0 / 1001.0),
        (59.94, 60_000.0 / 1001.0),
        (119.88, 120_000.0 / 1001.0),
    ]
    .into_iter()
    .find(|(rounded, _)| (fps - rounded).abs() < 1e-6)
    .map_or(fps, |(_, exact)| exact);
    let path = path
        .to_str()
        .ok_or("The video path cannot be saved as text.")?
        .to_owned();
    let video = Video {
        path,
        start: (start * fps).round() / fps,
        duration,
        fps,
        source_in: 0.0,
        source_duration: Some(duration),
    };
    frame(&video.path, 0.0)?;
    Ok(video)
}

fn frame(path: &str, sec: f64) -> FrameResult {
    let ffmpeg = tool()?;
    let path = Path::new(path);
    if !path.is_absolute() || !path.is_file() {
        return Err("Reference video is missing. Right-click its clip to relink it.".into());
    }
    let output = bounded_output(decode_command(&ffmpeg)
        .args(["-loglevel", "error", "-ss", &format!("{sec:.6}"), "-i"]).arg(path)
        .args(["-map", "0:V:0", "-frames:v", "1", "-an", "-sn", "-dn", "-vf",
            "scale=416:234:force_original_aspect_ratio=decrease,pad=416:234:(ow-iw)/2:(oh-ih)/2,setsar=1",
            "-pix_fmt", "rgb24", "-c:v", "rawvideo", "-threads", "1", "-f", "rawvideo", "pipe:1"]),
        Duration::from_secs(12))?;
    if !output.status.success() || output.stdout.len() != WIDTH * HEIGHT * 3 {
        return Err("FFmpeg could not decode a frame here.".into());
    }
    Ok(output
        .stdout
        .chunks_exact(3)
        .map(|p| u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2]))
        .collect())
}

pub fn snap(seconds: f64, fps: f64) -> f64 {
    (seconds * fps).round() / fps
}

fn fits(clips: &[Video], clip: &Video, except: Option<usize>) -> bool {
    clips.iter().enumerate().all(|(i, other)| {
        Some(i) == except || clip.end() <= other.start + 1e-9 || clip.start >= other.end() - 1e-9
    })
}

pub fn adjust(clips: &mut [Video], index: usize, edge: VideoEdge, delta: f64) -> bool {
    let Some(mut clip) = clips.get(index).cloned() else {
        return false;
    };
    let length = clip.source_length();
    let frame = (1.0 / clip.fps).min(clip.duration);
    match edge {
        VideoEdge::Body => clip.start = snap(clip.start + delta, clip.fps).clamp(0.0, 86400.0),
        VideoEdge::Left => {
            let previous_end = clips.get(index.wrapping_sub(1)).map_or(0.0, Video::end);
            let lower = (-clip.source_in).max(previous_end - clip.start).min(0.0);
            let offset = snap(clip.source_in + delta, clip.fps) - clip.source_in;
            let offset = offset.clamp(lower, (clip.duration - frame).max(0.0));
            clip.start += offset;
            clip.source_in += offset;
            clip.duration -= offset;
        }
        VideoEdge::Right => {
            let room = clips
                .get(index + 1)
                .map_or(length, |next| next.start - clip.start);
            let extent = (length - clip.source_in).min(room).max(frame);
            let out = snap(clip.source_in + clip.duration + delta, clip.fps);
            clip.duration = (out - clip.source_in).clamp(frame, extent);
        }
    }
    clip.source_duration = Some(length);
    if !fits(clips, &clip, Some(index)) || clip.start > 86400.0 {
        return false;
    }
    clips[index] = clip;
    true
}

pub fn split(s: &mut State, index: usize, seconds: f64) {
    let Some(clip) = data(s).videos.get(index) else {
        return;
    };
    let cut = snap(clip.source_in + seconds - clip.start, clip.fps) - clip.source_in;
    let frame = 1.0 / clip.fps;
    if cut < frame - 1e-9 || clip.duration - cut < frame - 1e-9 {
        return;
    }
    let mut right = clip.clone();
    let length = Some(clip.source_length());
    right.start += cut;
    right.source_in += cut;
    right.duration -= cut;
    right.source_duration = length;
    update(s, "Split video clip", |timeline| {
        let left = &mut timeline.videos[index];
        left.duration = cut;
        left.source_duration = length;
        timeline.videos.insert(index + 1, right);
    });
}

pub fn import(app: &mut App, path: PathBuf, start: f64, relink: Option<Video>) {
    let s = &mut app.state;
    if !(0.0..=86400.0).contains(&start) {
        s.message = Some("Choose a video start within the first 24 hours.".into());
        app.invalidate();
        return;
    }
    if s.playlist.scoring.video.import.is_some() {
        s.message = Some("Wait for the current video import to finish.".into());
        app.invalidate();
        return;
    }
    let (tx, rx) = mpsc::sync_channel(1);
    s.playlist.scoring.video.import = Some((s.project.arrangement_index(), relink, rx));
    s.message = Some("Reading reference video...".into());
    std::thread::spawn(move || {
        let _ = tx.send(probe(path, start));
    });
    app.invalidate();
}

fn finish_import(
    s: &mut State,
    mut video: Video,
    relink: Option<Video>,
) -> Result<&'static str, &'static str> {
    let mut timeline = data(s).clone();
    let label = if let Some(original) = relink {
        let Some(i) = timeline.videos.iter().position(|v| *v == original) else {
            return Err("The video clip changed while the replacement was loading.");
        };
        if video.source_length() + 1e-9 < original.source_in + original.duration {
            return Err("The replacement video is too short for this clip.");
        }
        video.start = original.start;
        video.source_in = original.source_in;
        video.duration = original.duration;
        timeline.videos[i] = video;
        "Relink video clip"
    } else {
        if !fits(&timeline.videos, &video, None) {
            return Err("The video would overlap another clip. Import it at an open position.");
        }
        timeline.videos.push(video);
        "Import video"
    };
    timeline.normalize();
    apply(s, label, timeline);
    s.playlist.scoring.video.open = true;
    Lane::Video.show(true);
    Ok(if label == "Relink video clip" {
        "Video clip relinked."
    } else {
        "Reference video added to the Playlist."
    })
}

pub fn poll(app: &mut App) {
    let s = &mut app.state;
    let player = &mut s.playlist.scoring.video;
    let result = player
        .import
        .as_ref()
        .and_then(|(_, _, rx)| match rx.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Video import stopped.".into())),
        });
    if let Some(result) = result {
        let (arrangement, relink, _) = player.import.take().unwrap();
        if s.project.arrangement_index() == arrangement {
            let message = match result {
                Ok(video) => match finish_import(s, video, relink) {
                    Ok(ok) | Err(ok) => ok.to_owned(),
                },
                Err(error) => error,
            };
            s.message = Some(message);
        }
        app.invalidate();
    }
    let s = &mut app.state;
    let visible = overlay_visible(s) && s.shown(Win::Playlist);
    let playing = app.transport.playing() && app.transport.song();
    let active = visible
        .then(|| {
            let seconds = if playing {
                app.transport.seconds()
            } else {
                s.project
                    .tempo_map(s.tempo)
                    .seconds(f64::from(s.playlist.pos))
            };
            let clips = &s.project.arrangement()?.timeline.videos;
            clips.iter().rev().find_map(|clip| {
                clip.source_time(seconds)
                    .map(|source_time| (clip, source_time))
            })
        })
        .flatten();
    let player = &mut s.playlist.scoring.video;
    let same = match (&player.source, &active) {
        (None, None) => true,
        (Some((path, start, offset)), Some((v, _))) => {
            *path == v.path && *start == v.start && *offset == v.source_in
        }
        _ => false,
    };
    let mut repaint = !same;
    if !same {
        player.source = active
            .as_ref()
            .map(|(v, _)| (v.path.clone(), v.start, v.source_in));
        player.generation = player.generation.wrapping_add(1);
        player.requested = None;
        player.image = None;
        player.error = None;
        player.last = None;
    }
    let target = active
        .as_ref()
        .map(|(v, local)| (*local * v.fps + 0.00001).floor() as u64);
    if let Some((generation, requested, rx)) = &player.decode {
        let relevant = *generation == player.generation && (playing || target == Some(*requested));
        match rx.try_recv() {
            Ok(result) => {
                if relevant {
                    match result {
                        Ok(image) => {
                            player.image = Some(image);
                            player.error = None;
                        }
                        Err(err) => player.error = Some(err),
                    }
                    repaint = true;
                }
                player.decode = None;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                player.decode = None;
                player.requested = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    if let (Some((video, _)), Some(target)) = (active, target) {
        let interval = if player.error.is_some() {
            Duration::from_secs(2)
        } else {
            Duration::from_millis(100)
        };
        if player.decode.is_none()
            && player.requested != Some(target)
            && player.last.is_none_or(|t| t.elapsed() >= interval)
        {
            let (tx, rx) = mpsc::sync_channel(1);
            player.requested = Some(target);
            player.last = Some(Instant::now());
            player.decode = Some((player.generation, target, rx));
            let path = video.path.clone();
            let seconds = (target as f64 / video.fps - 0.000002).max(0.0);
            std::thread::spawn(move || {
                let _ = tx.send(frame(&path, seconds));
            });
        }
    }
    if repaint {
        app.damage_win(Win::Playlist);
    }
}

pub fn overlay_visible(s: &State) -> bool {
    Lane::Video.shown(s) && s.playlist.scoring.video.open && !data(s).videos.is_empty()
}

pub fn paint(ui: &mut Ui, s: &State, grid: Rect) {
    let player = &s.playlist.scoring.video;
    if !overlay_visible(s) || grid.w < 190.0 || grid.h < 150.0 {
        return;
    }
    let w = (grid.w * 0.36)
        .clamp(178.0, 416.0)
        .min((grid.h - 45.0) * 16.0 / 9.0);
    let h = w * 9.0 / 16.0;
    let image = Rect::new(grid.right() - w - 12.0, grid.bottom() - h - 12.0, w, h);
    let title = Rect::new(image.x, image.y - 23.0, w, 23.0);
    ui.hit(
        Rect::new(title.x, title.y, w, h + 23.0),
        Action::Swallow,
        "Reference video preview",
    );
    ui.c.fill_rect(title, ui.t.track_header);
    ui.c.fill_rect(image, os_ui::Color::rgb(0x121419));
    ui.c.text_clip(
        Font::new(12),
        title.x + 7.0,
        title.y + 4.0,
        title.w - 34.0,
        ui.t.text,
        "Reference video",
    );
    let close = Rect::new(title.right() - 23.0, title.y, 23.0, 23.0);
    ui.hit(close, Action::ScoringVideoClose, "Hide video preview");
    ui.c.text_center(Font::new(13), close, ui.t.text, "x");
    if let Some(pixels) = &player.image {
        ui.c.image(image, WIDTH, HEIGHT, pixels);
    } else if let Some(error) = &player.error {
        ui.c.text_clip(
            Font::new(12),
            image.x + 8.0,
            image.cy(),
            image.w - 16.0,
            ui.t.text,
            error,
        );
    } else {
        ui.c.text_center(
            Font::new(12),
            image,
            ui.t.text_dim,
            "Seek inside the video clip",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_reference_frame_when_fixture_is_supplied() {
        let Some(path) = std::env::var_os("ML_TEST_REFERENCE_VIDEO") else {
            return;
        };
        let video = probe(path.into(), 2.0).unwrap();
        assert_eq!(video.start, 2.0);
        assert_eq!(video.fps, 24.0);
        let a = frame(&video.path, 0.0).unwrap();
        let b = frame(&video.path, 1.25).unwrap();
        assert_ne!(a, b);
    }
}
