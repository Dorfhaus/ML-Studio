//! Upgrade a browser gesture to an OS file drag once it reaches a hosted editor.
use crate::app::{App, Drag};
use crate::views::browser::Item;
use std::path::PathBuf;
use winit::{raw_window_handle::{HasWindowHandle, RawWindowHandle}, window::Window};

#[cfg(windows)]
fn over(area: *mut std::ffi::c_void) -> bool {
    use windows_sys::Win32::{Foundation::POINT, UI::WindowsAndMessaging::*};
    unsafe {
        let mut p = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut p) == 0 { return false; }
        let hit = WindowFromPoint(p);
        hit == area || IsChild(area, hit) != 0
    }
}

#[cfg(target_os = "macos")]
fn over(area: *mut std::ffi::c_void) -> bool {
    unsafe extern "C" { fn ml_drag_over_view(view: *mut std::ffi::c_void) -> i32; }
    unsafe { ml_drag_over_view(area) != 0 }
}

fn editor_at(app: &App, hit: impl Fn(*mut std::ffi::c_void) -> bool) -> Option<usize> {
    app.plugins.hosted.iter().find_map(|(&row, h)| {
        let area = h.window.as_ref().map(|w| w.area());
        let strip = h.rack_editor.as_ref().map(|s| s.area());
        area.into_iter().chain(strip).any(&hit).then_some(row)
    })
}

#[cfg(target_os = "macos")]
pub(crate) fn finish(app: &mut App) {
    use std::ffi::c_void;
    unsafe extern "C" {
        fn ml_completed_sample_drag() -> *mut c_void;
        fn ml_view_contains(area: *mut c_void, hit: *mut c_void) -> i32;
    }
    let hit = unsafe { ml_completed_sample_drag() };
    if !hit.is_null() && let Some(row) = editor_at(app, |area| unsafe { ml_view_contains(area, hit) != 0 }) {
        app.plugins.mark_edited(row);
    }
}

pub(crate) fn handoff(app: &mut App, source: &Window) -> bool {
    let Some(Drag::Browser { item, .. }) = &app.drag else { return false };
    let files = match item {
        Item::Sample(path) => std::slice::from_ref(path),
        Item::Samples(paths) => paths.as_slice(),
        _ => return false,
    };
    let Some(target) = editor_at(app, over) else { return false };
    let Some(desc) = app.plugins.descriptor(target) else { return false };
    // Keep direct channel imports and their project metadata for the built-ins.
    if (desc.format == os_host::Format::Native && files.len() == 1) ||
        (crate::project_plugins::is_builtin(&desc) && (desc.name == "ML Chop" ||
            app.state.project.channels.get(target).is_some_and(|c| c.kind.is_sampler()))) { return false; }
    let paths: Vec<_> = files.iter().map(|p| PathBuf::from(crate::plugins::resolve_sample(p))).collect();
    if paths.is_empty() || paths.iter().any(|p| !p.is_file()) { return false; }
    let Ok(handle) = source.window_handle() else { return false };
    let result = match handle.as_raw() {
        #[cfg(windows)]
        RawWindowHandle::Win32(w) => {
            use std::os::windows::ffi::OsStrExt;
            unsafe extern "C" { fn ml_drag_sample_files(hwnd: isize, paths: *const *const u16, n: u32) -> i32; }
            let names: Vec<Vec<_>> = paths.iter().map(|p| p.as_os_str().encode_wide().chain([0]).collect()).collect();
            let ptrs: Vec<_> = names.iter().map(|p| p.as_ptr()).collect();
            unsafe { ml_drag_sample_files(w.hwnd.get(), ptrs.as_ptr(), ptrs.len() as u32) }
        }
        #[cfg(target_os = "macos")]
        RawWindowHandle::AppKit(w) => {
            use std::{ffi::{CString, c_char, c_void}, os::unix::ffi::OsStrExt};
            unsafe extern "C" { fn ml_drag_sample_files(view: *mut c_void, paths: *const *const c_char, n: u32) -> i32; }
            let Ok(names) = paths.iter().map(|p| CString::new(p.as_os_str().as_bytes())).collect::<Result<Vec<_>, _>>() else { return false };
            let ptrs: Vec<_> = names.iter().map(|p| p.as_ptr()).collect();
            unsafe { ml_drag_sample_files(w.ns_view.as_ptr(), ptrs.as_ptr(), ptrs.len() as u32) }
        }
        _ => return false,
    };
    if result == 0 { return false; }
    #[cfg(windows)]
    if result == 2 && let Some(row) = editor_at(app, over) { app.plugins.mark_edited(row); }
    app.drag = None;
    app.pressed = None;
    app.state.browser_drag = None;
    app.state.sample_drop.clear();
    app.invalidate();
    true
}
