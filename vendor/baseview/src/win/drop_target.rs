use std::ffi::OsString;
use std::mem::transmute;
use std::os::windows::prelude::OsStringExt;
use std::ptr::null_mut;
use std::rc::{Rc, Weak};

use winapi::shared::guiddef::{IsEqualIID, REFIID};
use winapi::shared::minwindef::{DWORD, WPARAM};
use winapi::shared::ntdef::{HRESULT, ULONG};
use winapi::shared::windef::{POINT, POINTL};
use winapi::shared::winerror::{E_NOINTERFACE, E_UNEXPECTED, S_OK};
use winapi::um::objidl::IDataObject;
use winapi::um::oleidl::{
    IDropTarget, IDropTargetVtbl, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE,
    DROPEFFECT_NONE, DROPEFFECT_SCROLL,
};
use winapi::um::unknwnbase::{IUnknown, IUnknownVtbl};
use winapi::um::winuser::{ScreenToClient, CF_HDROP};
use windows::core::Interface as WindowsInterface;
use windows::Win32::System::Com::{
    IDataObject as ShellDataObject, DVASPECT_CONTENT, FORMATETC, STGMEDIUM, TYMED_HGLOBAL,
};
use windows::Win32::System::Ole::ReleaseStgMedium;
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
use winapi::Interface;

use crate::{DropData, DropEffect, Event, EventStatus, MouseEvent, PhyPoint, Point};

use super::WindowState;

// These function pointers have to be stored in a (const) variable before they can be transmuted
// Transmuting is needed because winapi has a bug where the pt parameter has an incorrect
// type `*const POINTL`
const DRAG_ENTER_PTR: unsafe extern "system" fn(
    this: *mut IDropTarget,
    pDataObj: *const IDataObject,
    grfKeyState: DWORD,
    pt: POINTL,
    pdwEffect: *mut DWORD,
) -> HRESULT = DropTarget::drag_enter;
const DRAG_OVER_PTR: unsafe extern "system" fn(
    this: *mut IDropTarget,
    grfKeyState: DWORD,
    pt: POINTL,
    pdwEffect: *mut DWORD,
) -> HRESULT = DropTarget::drag_over;
const DROP_PTR: unsafe extern "system" fn(
    this: *mut IDropTarget,
    pDataObj: *const IDataObject,
    grfKeyState: DWORD,
    pt: POINTL,
    pdwEffect: *mut DWORD,
) -> HRESULT = DropTarget::drop;

#[allow(clippy::missing_transmute_annotations)]
const DROP_TARGET_VTBL: IDropTargetVtbl = IDropTargetVtbl {
    parent: IUnknownVtbl {
        QueryInterface: DropTarget::query_interface,
        AddRef: DropTarget::add_ref,
        Release: DropTarget::release,
    },
    DragEnter: unsafe { transmute(DRAG_ENTER_PTR) },
    DragOver: unsafe { transmute(DRAG_OVER_PTR) },
    DragLeave: DropTarget::drag_leave,
    Drop: unsafe { transmute(DROP_PTR) },
};

#[repr(C)]
pub(super) struct DropTarget {
    base: IDropTarget,

    window_state: Weak<WindowState>,

    // These are cached since DragOver and DragLeave callbacks don't provide them,
    // and handling drag move events gets awkward on the client end otherwise
    drag_position: Point,
    drop_data: DropData,
}

impl DropTarget {
    pub(super) fn new(window_state: Weak<WindowState>) -> Self {
        Self {
            base: IDropTarget { lpVtbl: &DROP_TARGET_VTBL },

            window_state,

            drag_position: Point::new(0.0, 0.0),
            drop_data: DropData::None,
        }
    }

    #[allow(non_snake_case)]
    fn on_event(&self, pdwEffect: Option<*mut DWORD>, event: MouseEvent) {
        let Some(window_state) = self.window_state.upgrade() else {
            return;
        };

        unsafe {
            let mut window = crate::Window::new(window_state.create_window());

            let event = Event::Mouse(event);
            let event_status =
                window_state.handler_mut().as_mut().unwrap().on_event(&mut window, event);

            if let Some(pdwEffect) = pdwEffect {
                match event_status {
                    EventStatus::AcceptDrop(DropEffect::Copy) => *pdwEffect = DROPEFFECT_COPY,
                    EventStatus::AcceptDrop(DropEffect::Move) => *pdwEffect = DROPEFFECT_MOVE,
                    EventStatus::AcceptDrop(DropEffect::Link) => *pdwEffect = DROPEFFECT_LINK,
                    EventStatus::AcceptDrop(DropEffect::Scroll) => *pdwEffect = DROPEFFECT_SCROLL,
                    _ => *pdwEffect = DROPEFFECT_NONE,
                }
            }
        }
    }

    fn parse_coordinates(&mut self, pt: POINTL) {
        let Some(window_state) = self.window_state.upgrade() else {
            return;
        };

        // OLE reports physical screen pixels; the editor lays its pads out in client points.
        let mut pt = POINT { x: pt.x, y: pt.y };
        unsafe { ScreenToClient(window_state.hwnd, &mut pt) };
        let phy_point = PhyPoint::new(pt.x, pt.y);
        self.drag_position = phy_point.to_logical(&window_state.window_info());
    }

    #[allow(non_snake_case)]
    unsafe extern "system" fn query_interface(
        this: *mut IUnknown, riid: REFIID, ppvObject: *mut *mut winapi::ctypes::c_void,
    ) -> HRESULT {
        if IsEqualIID(&*riid, &IUnknown::uuidof()) || IsEqualIID(&*riid, &IDropTarget::uuidof()) {
            Self::add_ref(this);
            *ppvObject = this as *mut winapi::ctypes::c_void;
            return S_OK;
        }

        E_NOINTERFACE
    }

    unsafe extern "system" fn add_ref(this: *mut IUnknown) -> ULONG {
        let arc = Rc::from_raw(this);
        let result = Rc::strong_count(&arc) + 1;
        let _ = Rc::into_raw(arc);

        Rc::increment_strong_count(this);

        result as ULONG
    }

    unsafe extern "system" fn release(this: *mut IUnknown) -> ULONG {
        let arc = Rc::from_raw(this);
        let result = Rc::strong_count(&arc) - 1;
        let _ = Rc::into_raw(arc);

        Rc::decrement_strong_count(this);

        result as ULONG
    }

    #[allow(non_snake_case)]
    unsafe extern "system" fn drag_enter(
        this: *mut IDropTarget, pDataObj: *const IDataObject, grfKeyState: DWORD, pt: POINTL,
        pdwEffect: *mut DWORD,
    ) -> HRESULT {
        let drop_target = &mut *(this as *mut DropTarget);
        let Some(window_state) = drop_target.window_state.upgrade() else {
            return E_UNEXPECTED;
        };

        let modifiers =
            window_state.keyboard_state().get_modifiers_from_mouse_wparam(grfKeyState as WPARAM);

        drop_target.parse_coordinates(pt);
        drop_target.drop_data = file_drop_data(pDataObj);

        let event = MouseEvent::DragEntered {
            position: drop_target.drag_position,
            modifiers,
            data: drop_target.drop_data.clone(),
        };

        drop_target.on_event(Some(pdwEffect), event);
        S_OK
    }

    #[allow(non_snake_case)]
    unsafe extern "system" fn drag_over(
        this: *mut IDropTarget, grfKeyState: DWORD, pt: POINTL, pdwEffect: *mut DWORD,
    ) -> HRESULT {
        let drop_target = &mut *(this as *mut DropTarget);
        let Some(window_state) = drop_target.window_state.upgrade() else {
            return E_UNEXPECTED;
        };

        let modifiers =
            window_state.keyboard_state().get_modifiers_from_mouse_wparam(grfKeyState as WPARAM);

        drop_target.parse_coordinates(pt);

        let event = MouseEvent::DragMoved {
            position: drop_target.drag_position,
            modifiers,
            data: drop_target.drop_data.clone(),
        };

        drop_target.on_event(Some(pdwEffect), event);
        S_OK
    }

    unsafe extern "system" fn drag_leave(this: *mut IDropTarget) -> HRESULT {
        let drop_target = &mut *(this as *mut DropTarget);
        drop_target.on_event(None, MouseEvent::DragLeft);
        S_OK
    }

    #[allow(non_snake_case)]
    unsafe extern "system" fn drop(
        this: *mut IDropTarget, pDataObj: *const IDataObject, grfKeyState: DWORD, pt: POINTL,
        pdwEffect: *mut DWORD,
    ) -> HRESULT {
        let drop_target = &mut *(this as *mut DropTarget);
        let Some(window_state) = drop_target.window_state.upgrade() else {
            return E_UNEXPECTED;
        };

        let modifiers =
            window_state.keyboard_state().get_modifiers_from_mouse_wparam(grfKeyState as WPARAM);

        drop_target.parse_coordinates(pt);
        drop_target.drop_data = file_drop_data(pDataObj);

        let event = MouseEvent::DragDropped {
            position: drop_target.drag_position,
            modifiers,
            data: drop_target.drop_data.clone(),
        };

        drop_target.on_event(Some(pdwEffect), event);
        S_OK
    }
}

// GetData transfers ownership of the whole medium, including any custom releaser.
struct FileMedium(STGMEDIUM);
impl Drop for FileMedium {
    fn drop(&mut self) {
        unsafe { ReleaseStgMedium(&mut self.0) };
    }
}

unsafe fn file_drop_data(raw: *const IDataObject) -> DropData {
    let ptr = raw as *mut std::ffi::c_void;
    let Some(object) = ShellDataObject::from_raw_borrowed(&ptr) else { return DropData::None };
    let format = FORMATETC {
        cfFormat: CF_HDROP as u16,
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
        ..Default::default()
    };
    let Ok(medium) = object.GetData(&format) else { return DropData::None };
    let medium = FileMedium(medium);
    if medium.0.tymed != format.tymed { return DropData::None; }
    let handle = HDROP(medium.0.u.hGlobal.0);
    if handle.is_invalid() { return DropData::None; }

    let count = DragQueryFileW(handle, u32::MAX, None);
    let mut paths = Vec::new();
    for i in 0..count {
        let len = DragQueryFileW(handle, i, None);
        if len == 0 { continue; }
        let mut buffer = vec![0; len as usize + 1];
        let copied = DragQueryFileW(handle, i, Some(&mut buffer)) as usize;
        if copied > 0 {
            paths.push(OsString::from_wide(&buffer[..copied]).into());
        }
    }
    if paths.is_empty() { DropData::None } else { DropData::Files(paths) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::HSTRING;
    use windows::Win32::System::Ole::{OleInitialize, OleUninitialize};
    use windows::Win32::UI::Shell::{SHCreateItemFromParsingName, SHCreateShellItemArrayFromShellItem, IShellItem, IShellItemArray, BHID_DataObject};

    #[test]
    fn shell_sample_drag_reads_the_path_on_enter_and_drop() {
        let path = std::env::temp_dir().join(format!("ML Drums snare ü-{}.wav", std::process::id()));
        std::fs::write(&path, include_bytes!("../../../../plugins/ml-drums/resources/default_samples/sd_1.wav")).unwrap();
        unsafe {
            OleInitialize(None).unwrap();
            {
                let item: IShellItem = SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None).unwrap();
                let items: IShellItemArray = SHCreateShellItemArrayFromShellItem(&item).unwrap();
                let data: ShellDataObject = items.BindToHandler(None, &BHID_DataObject).unwrap();
                for _ in 0..2 {
                    assert_eq!(file_drop_data(data.as_raw().cast()), DropData::Files(vec![path.clone()]));
                }
            }
            OleUninitialize();
        }
        std::fs::remove_file(path).unwrap();
    }
}
