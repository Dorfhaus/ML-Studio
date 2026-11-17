//! Xdnd file transfer to the editor under the release point. The plugin and host
//! share a UI thread: never wait synchronously for a plugin's Status or Finished.
use super::*;
use x11rb::protocol::xproto::{ConnectionExt as _, SelectionNotifyEvent};
use x11rb::wrapper::ConnectionExt as _;
use std::time::{Duration, Instant};

x11rb::atom_manager! {
    Names: NamesCookie {
        XdndAware, XdndEnter, XdndPosition, XdndStatus, XdndLeave, XdndDrop,
        XdndFinished, XdndSelection, XdndActionCopy, TARGETS,
        uri: b"text/uri-list",
    }
}

struct Transfer {
    names: Names,
    source: u32,
    target: u32,
    payload: Vec<u8>,
    deadline: Instant,
    dropped: bool,
}
thread_local! { static ACTIVE: RefCell<Option<Transfer>> = const { RefCell::new(None) }; }

fn send(c: &RustConnection, target: u32, kind: u32, data: [u32; 5]) {
    let event = ClientMessageEvent::new(32, target, kind, data);
    let _ = c.send_event(false, target, EventMask::NO_EVENT, event);
    let _ = c.flush();
}

pub(crate) fn begin(frames: &[(usize, u32)], files: &[String]) -> Option<(usize, bool)> {
    let conn = CONN.with(|s| s.borrow().clone())?;
    let c = &conn.c;
    let names = Names::new(c).ok()?.reply().ok()?;
    let mut window = conn.root;
    let mut row = None;
    let mut recipient = None;
    let point = loop {
        if let Some((r, _)) = frames.iter().find(|(_, id)| *id == window) { row = Some(*r); }
        if row.is_some() {
            let version = c.get_property(false,window,names.XdndAware,AtomEnum::ATOM,0,1).ok()?.reply().ok()?;
            if let Some(v) = version.value32().and_then(|mut i| i.next()).filter(|v| *v >= 3) {
                recipient = Some((window,v.min(5)));
            }
        }
        let p = c.query_pointer(window).ok()?.reply().ok()?;
        if !p.same_screen { return None; }
        if p.child == 0 { break ((p.root_x as u16 as u32) << 16) | p.root_y as u16 as u32; }
        window = p.child;
    };
    let row = row?;
    let Some((target,version)) = recipient else { return Some((row,false)) };
    let mut payload = String::new();
    for file in files {
        let path = std::fs::canonicalize(crate::plugins::resolve_sample(file)).ok()?;
        payload.push_str(url::Url::from_file_path(path).ok()?.as_str());
        payload.push_str("\r\n");
    }
    let source = c.generate_id().ok()?;
    c.create_window(x11rb::COPY_FROM_PARENT as u8,source,conn.root,0,0,1,1,0,WindowClass::INPUT_ONLY,0,&CreateWindowAux::new()).ok()?.check().ok()?;
    let result = c.set_selection_owner(source,names.XdndSelection,x11rb::CURRENT_TIME).ok().and_then(|s| s.check().ok());
    if result.is_none() { let _ = c.destroy_window(source); return Some((row,false)); }
    send(c,target,names.XdndEnter,[source,version<<24, names.uri,0,0]);
    send(c,target,names.XdndPosition,[source,0,point,x11rb::CURRENT_TIME,names.XdndActionCopy]);
    ACTIVE.with(|s| {
        if let Some(old) = s.borrow_mut().replace(Transfer { names,source,target,payload:payload.into_bytes(),deadline:Instant::now()+Duration::from_secs(10), dropped:false }) {
            if !old.dropped { send(c,old.target,old.names.XdndLeave,[old.source,0,0,0,0]); }
            let _ = c.destroy_window(old.source);
        }
    });
    Some((row,true))
}

pub(super) fn tick(c: &RustConnection, event: Option<&Event>) {
    ACTIVE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(t) = slot.as_mut() else { return };
        let n = &t.names;
        let mut done = false;
        match event {
            Some(Event::ClientMessage(m)) if m.window == t.source && m.data.as_data32()[0] == t.target => {
                if m.type_ == n.XdndStatus && !t.dropped {
                    t.dropped = m.data.as_data32()[1] & 1 != 0 && m.data.as_data32()[4] == n.XdndActionCopy;
                    if t.dropped { send(c,t.target,n.XdndDrop,[t.source,0,x11rb::CURRENT_TIME,0,0]); }
                    else { done = true; }
                } else if m.type_ == n.XdndFinished { done = true; }
            }
            Some(Event::SelectionRequest(r)) if r.owner == t.source && r.selection == n.XdndSelection => {
                let property = if r.property == 0 { r.target } else { r.property };
                let ok = if r.target == n.uri {
                    c.change_property8(PropMode::REPLACE,r.requestor,property,n.uri,&t.payload).is_ok()
                } else if r.target == n.TARGETS {
                    c.change_property32(PropMode::REPLACE,r.requestor,property,AtomEnum::ATOM,&[n.TARGETS,n.uri]).is_ok()
                } else { false };
                let reply = SelectionNotifyEvent { response_type:xproto::SELECTION_NOTIFY_EVENT,sequence:0,time:r.time,requestor:r.requestor,selection:r.selection,target:r.target,property:if ok {property} else {0} };
                let _ = c.send_event(false,r.requestor,EventMask::NO_EVENT,reply);
                let _ = c.flush();
            }
            Some(Event::SelectionClear(e)) if e.owner == t.source => done = true,
            _ => {}
        }
        if done || Instant::now() >= t.deadline {
            if !t.dropped { send(c,t.target,n.XdndLeave,[t.source,0,0,0,0]); }
            let _ = c.destroy_window(t.source);
            let _ = c.flush();
            *slot = None;
        }
    });
}
