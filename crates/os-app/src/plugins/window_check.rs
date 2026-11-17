// Copyright (C) 2026 ItsAlphaNeon
// Copyright (C) 2026 ariymarkowitz
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! `lemonstudio --plugin-window-check <dir> <plugin file>...` (macOS): opens each plugin's editor in a real plugin
//! window, as the app does, at the display's scale, then at 1x and 2x (a window moved between monitors), and checks at
//! each that the frame's editor area fits the editor's view (neither clipped nor left with a gap). A picture of the
//! window at each scale goes to `<dir>`. The format is the file's (`.vst`, `.vst3`, `.clap`).
//!
//! Plugin windows need the main thread on macOS, which tests never get, so this runs from the program itself (the
//! Windows checks are ignored tests in `plugins.rs`). The app is not activated: the windows open behind the one in use.
//! Exits 1 when an editor did not fit or a plugin did not open.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use objc2::{MainThreadMarker, MainThreadOnly};
use objc2::rc::Retained;
use objc2_app_kit::{NSBackingStoreType, NSView, NSWindow, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::window::WindowId;

use super::{load_file, Available, Plugins};
use crate::plugin_window::ChannelInfo;

/// Nothing to handle: the frames are not winit windows.
struct Idle;

impl ApplicationHandler for Idle {
    fn resumed(&mut self, _: &ActiveEventLoop) {}
    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

/// Runs the windows and the plugins' idle calls for `ms`.
fn pump(el: &mut EventLoop<()>, pl: &mut Plugins, ms: u64) {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        el.pump_app_events(Some(Duration::from_millis(10)), &mut Idle);
        pl.idle(|_| None);
    }
}

/// A view's frame in its superview, in points: x, y, width, height.
fn frame_of(view: &NSView) -> (f64, f64, f64, f64) {
    let f = view.frame();
    (f.origin.x, f.origin.y, f.size.width, f.size.height)
}

/// What one look at an open window found: the editor area and the editor's own view's frame in it, in points; the
/// window's number.
struct Look {
    area: (f64, f64),
    editor: Option<(f64, f64, f64, f64)>,
    window: Option<isize>,
}

fn look(pl: &Plugins, row: usize) -> Option<Look> {
    let w = pl.hosted.get(&row)?.window.as_ref()?;
    // SAFETY: the frame's views live as long as its window, which `pl` holds.
    let (frame, area) = unsafe { (&*(w.hwnd() as *const NSView), &*(w.area() as *const NSView)) };
    let editor = area.subviews().firstObject().map(|v| frame_of(&v));
    let size = area.bounds().size;
    Some(Look { area: (size.width, size.height), editor, window: frame.window().map(|win| win.windowNumber()) })
}

/// The window's picture (no shadow), with `screencapture`.
fn shoot(window: isize, path: &Path) -> bool {
    std::process::Command::new("screencapture").arg("-x").arg("-o").arg(format!("-l{window}")).arg(path).status().is_ok_and(|s| s.success())
}

/// Resolve built-in editors and registered Audio Units through the same factories as the picker.
fn resolve(spec: &str) -> Result<Available, String> {
    if let Some(name) = spec.strip_prefix("stock:") {
        return super::builtin().into_iter().chain(super::builtin_effects()).find(|a| a.desc.name == name)
            .ok_or_else(|| format!("Unknown stock plugin {name}"));
    }
    let (lib, name) = if let Some(name) = spec.strip_prefix("au:") {
        (load_file(Path::new(os_host::audio_unit::REGISTRY_PATH), os_host::Format::AudioUnit)?, Some(name))
    } else {
        let file = Path::new(spec);
        let format = match file.extension().and_then(|s| s.to_str()).map(str::to_ascii_lowercase).as_deref() {
            Some("clap") => os_host::Format::Clap,
            Some("vst3") => os_host::Format::Vst3,
            Some("vst") => os_host::Format::Vst2,
            _ => return Err(format!("Unknown plugin file format {spec}")),
        };
        (load_file(file, format)?, None)
    };
    let desc = lib.plugins().iter().find(|d| name.is_none_or(|name| d.name == name))
        .cloned().ok_or_else(|| format!("No matching plugin for {spec}"))?;
    Ok(Available { lib, desc })
}

pub fn run(args: &[String]) -> i32 {
    // Geometry-only checks exercise the real detached frame without ordering it
    // onto the user's screen or capturing any windows.
    let offscreen = args.first().is_some_and(|arg| arg == "--offscreen");
    let args = if offscreen { &args[1..] } else { args };
    let docked = offscreen && args.first().is_some_and(|arg| arg == "--docked");
    let args = if docked { &args[1..] } else { args };
    let Some((dir, files)) = args.split_first() else {
        eprintln!("usage: lemonstudio --plugin-window-check [--offscreen [--docked]] <dir> <plugin file | stock:name | au:name>...");
        return 2;
    };
    let dir = PathBuf::from(dir);
    let _ = std::fs::create_dir_all(&dir);
    let mut builder = EventLoop::<()>::with_user_event();
    builder.with_activation_policy(ActivationPolicy::Accessory).with_activate_ignoring_other_apps(false).with_default_menu(false);
    let mut el = builder.build().expect("event loop");
    let owner = docked.then(|| {
        let mtm = MainThreadMarker::new().unwrap();
        // This test window is never ordered on screen.
        unsafe { NSWindow::initWithContentRect_styleMask_backing_defer(NSWindow::alloc(mtm),
            NSRect::new(NSPoint::ZERO, NSSize::new(1500.0, 1000.0)), NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered, false) }
    });
    let mut failed = false;
    for spec in files {
        let entry = match resolve(spec) {
            Ok(entry) => entry,
            Err(e) => { eprintln!("{spec}: {e}"); failed = true; continue; }
        };
        let name = entry.desc.name.clone();
        let effect = entry.desc.kind == os_host::Kind::Effect;
        let row = if effect { super::fx_key(1, 0) } else { 0 };
        let mut pl = Plugins { scale: 2.0, rate: 48000, ..Plugins::default() };
        if let Some(view) = owner.as_ref().and_then(|w| w.contentView()) {
            pl.owner = Retained::as_ptr(&view) as isize;
        }
        let created = if effect {
            pl.effects.push(entry);
            pl.create_effect(1, 0, 0, None)
        } else {
            pl.available.push(entry);
            pl.create(0, 0)
        };
        if let Err(e) = created {
            println!("{name}: {e}");
            failed = true;
            continue;
        }
        if !pl.hosted[&row].plugin.lock().is_ok_and(|p| p.has_editor()) {
            println!("{name}: no editor");
            continue;
        }
        let info = ChannelInfo { name: name.clone(), target: "Insert 1".into(), enabled: true, volume: 0.78, pan: 0.0, pitch_cents: 0, range: 2, track: 1, max_track: 125, sidechain: None, misc: None };
        pl.hosted.get_mut(&row).unwrap().detached = !docked;
        let opened = pl.open_window_with_visibility(row, info, !offscreen);
        if let Err(e) = opened {
            println!("{name}: the window did not open: {e}");
            failed = true;
            continue;
        }
        if offscreen && name == ml_plasma::NAME {
            // Check immediately, before a size mismatch can compound on every idle.
            if let Some(l) = look(&pl, row) {
                if l.editor.is_some_and(|(_, _, w, h)| (w-l.area.0).abs() >= 0.5 || (h-l.area.1).abs() >= 0.5) {
                    eprintln!("{name}: initial editor {:?} does not fit {:?}", l.editor, l.area);
                    failed = true;
                    pl.drop_key(row);
                    continue;
                }
            }
        }
        pump(&mut el, &mut pl, 900);
        let stem = name.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
        let display = pl.hosted[&row].window.as_ref().map_or(2.0, |w| w.scale());
        for (k, tag) in [(display, "display"), (1.0, "1x"), (2.0, "2x")] {
            if tag != "display" {
                pl.rescale_window(row, k);
                pump(&mut el, &mut pl, 700);
            }
            let reported = pl.hosted[&row].plugin.lock().ok().and_then(|p| p.editor_size());
            let Some(l) = look(&pl, row) else {
                println!("{name} at {k}: the window is gone");
                failed = true;
                break;
            };
            // The editor's view fills the area: its size, and at the area's origin (not left where a smaller area had it).
            let fits = l.editor.is_some_and(|(x, y, w, h)| x.abs() <= 0.5 / f64::from(k) && y.abs() <= 0.5 / f64::from(k) && (w - l.area.0).abs() <= 0.5 / f64::from(k) && (h - l.area.1).abs() <= 0.5 / f64::from(k));
            failed |= !fits;
            let shot = dir.join(format!("{stem}_{tag}.png"));
            let pictured = !offscreen && l.window.is_some_and(|n| shoot(n, &shot));
            println!(
                "{name} at {k} ({tag}): editor view (x, y, w, h) {:?} pt, editor area {:?} pt, reported {reported:?} px: {}{}",
                l.editor,
                l.area,
                if fits { "fits" } else { "DOES NOT FIT" },
                if pictured { format!(", {}", shot.display()) } else { String::new() }
            );
        }
        if offscreen && name == ml_plasma::NAME {
            if let Some(initial) = look(&pl, row) {
                // A resize drag takes this same queue. Shrink, enlarge and
                // return to the original size across several host idle polls.
                for factor in [0.75, 1.25, 1.0] {
                    let requested = ((initial.area.0 * factor).round() as u32,
                                     (initial.area.1 * factor).round() as u32);
                    pl.hosted.get_mut(&row).unwrap().pending_editor_resize = Some(requested);
                    pump(&mut el, &mut pl, 250);
                    let fits = look(&pl, row).is_some_and(|l| {
                        l.editor.is_some_and(|(x, y, w, h)| x.abs() < 0.5 && y.abs() < 0.5
                            && (l.area.0-w).abs() < 0.5 && (l.area.1-h).abs() < 0.5
                            && (w-f64::from(requested.0)).abs() <= 2.0 && (h-f64::from(requested.1)).abs() <= 2.0)
                    });
                    println!("{name} resize to {requested:?} points: {}", if fits { "fits" } else { "DOES NOT FIT" });
                    failed |= !fits;
                }
            }
        }
        pl.drop_key(row);
        pump(&mut el, &mut pl, 200);
    }
    i32::from(failed)
}
