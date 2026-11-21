// SPDX-License-Identifier: GPL-3.0-or-later
//! Use the installed Windows cursor silhouette and its real hotspot for the original family.
use super::{Canvas, Cursor};
use std::{mem, ptr, slice};
use windows_sys::Win32::{Graphics::Gdi::*, UI::WindowsAndMessaging::*};

pub(super) fn mask(cursor: Cursor, scale: f32) -> Option<(Canvas, u16, u16)> {
    let resource = match cursor {
        Cursor::Default => IDC_ARROW,
        Cursor::Text => IDC_IBEAM,
        Cursor::EwResize => IDC_SIZEWE,
        Cursor::NsResize => IDC_SIZENS,
        Cursor::NwseResize => IDC_SIZENWSE,
        Cursor::NeswResize => IDC_SIZENESW,
        Cursor::Playlist(..) => return None,
    };
    let size = (32.0 * scale).round() as i32;
    let pad = (3.0 * scale).ceil() as i32;
    let mut mask = Canvas::new((size + 2 * pad) as u32, (size + 2 * pad) as u32, scale).ok()?;
    unsafe {
        // LoadCursor returns a shared handle. GetIconInfo returns owned bitmap copies.
        let handle = LoadCursorW(ptr::null_mut(), resource);
        let mut icon: ICONINFO = mem::zeroed();
        if handle.is_null() || GetIconInfo(handle, &mut icon) == 0 { return None; }
        let mut bitmap: BITMAP = mem::zeroed();
        let mono = icon.hbmColor.is_null();
        let ok = GetObjectW(icon.hbmMask, mem::size_of::<BITMAP>() as i32, (&mut bitmap as *mut BITMAP).cast());
        DeleteObject(icon.hbmMask);
        if !mono { DeleteObject(icon.hbmColor); }
        let height = if mono { bitmap.bmHeight / 2 } else { bitmap.bmHeight };
        if ok == 0 || bitmap.bmWidth <= 0 || height <= 0 { return None; }
        let hotspot = |pos: u32, extent: i32| (pad + (pos as f32 * size as f32 / extent as f32).round() as i32) as u16;

        let dc = CreateCompatibleDC(ptr::null_mut());
        if dc.is_null() { return None; }
        let mut info: BITMAPINFO = mem::zeroed();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: size,
            biHeight: -size, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB,
            ..mem::zeroed()
        };
        let mut bits = ptr::null_mut();
        let dib = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, ptr::null_mut(), 0);
        if dib.is_null() { DeleteDC(dc); return None; }
        let previous = SelectObject(dc, dib);
        let capture = |background: u32| -> Option<Vec<u8>> {
            if previous.is_null() || previous as isize == -1 || bits.is_null() { return None; }
            let count = (size * size) as usize;
            slice::from_raw_parts_mut(bits.cast::<u32>(), count).fill(background);
            if DrawIconEx(dc, 0, 0, handle, size, size, 0, ptr::null_mut(), DI_NORMAL) == 0 { return None; }
            GdiFlush();
            let pixels = slice::from_raw_parts(bits.cast::<u32>(), count);
            Some(pixels.iter().map(|p| (p & 255) as u8).collect())
        };
        let black = capture(0);
        let white = capture(0x00ff_ffff);
        SelectObject(dc, previous);
        DeleteObject(dib);
        DeleteDC(dc);
        // Two backgrounds recover coverage for both ARGB and monochrome AND/XOR cursors.
        let stride = mask.pixmap.width() as usize;
        let dest = mask.pixmap.data_mut();
        for (i, (b, w)) in black?.into_iter().zip(white?).enumerate() {
            let alpha = 255 - w.saturating_sub(b);
            // Keep dark outlines distinguishable from the fill of the stock pointer.
            let intensity = ((u16::from(alpha) + u16::from(b.min(alpha))) / 2) as u8;
            let offset = 4 * ((pad as usize + i / size as usize) * stride + pad as usize + i % size as usize);
            dest[offset..offset+4].copy_from_slice(&[intensity, intensity, intensity, 255]);
        }
        Some((mask, hotspot(icon.xHotspot, bitmap.bmWidth), hotspot(icon.yHotspot, height)))
    }
}
