// SPDX-License-Identifier: GPL-3.0-or-later
//! ML Patch's native map and inspector; one ordinary mixer slot owns the graph.
pub mod core;
use crate::{
    app::{Action, App, State, Ui},
    dialog::{Body, DHit, Modal},
    plugins,
};
use core::{Edge, Graph};
use os_ui::{Color, Font, Rect};
use std::sync::{Arc, Mutex};
use winit::keyboard::{Key, NamedKey};
pub const VISIBILITY: &str = "visible_ml-patch";
pub fn restore(app: &mut App, key: usize, bytes: &[u8]) -> Option<Result<(), String>> {
    let g = graph(app, key)?;
    core::set_catalog(&app.plugins.effects);
    let result = (|| {
        let mut g = g.lock().map_err(|_| "Patch is unavailable")?;
        let old = g.snapshot()?;
        g.replace(Graph::from_bytes(bytes)?)?;
        Ok(old)
    })();
    Some(match result {
        Ok(old) => {
            record(app, key, old);
            Ok(())
        }
        Err(e) => Err(e),
    })
}
pub fn sync(app: &mut App) {
    let Some(index) = app.plugins.effect_index(core::NAME) else {
        return;
    };
    let todo: Vec<_> = app
        .state
        .project
        .mixer
        .iter()
        .flat_map(|m| {
            m.slots.iter().enumerate().filter_map(move |(k, s)| {
                s.as_ref()
                    .filter(|s| s.name == core::NAME)
                    .map(|s| (m.index, k as u8, s))
            })
        })
        .filter(|(t, k, _)| {
            !app.plugins.hosted.contains_key(&plugins::fx_key(*t, *k))
                && !app.fx_failed.contains(&plugins::fx_key(*t, *k))
        })
        .map(|(t, k, s)| (t, k, s.state.clone()))
        .collect();
    let mut added = false;
    for (t, k, state) in todo {
        match app.plugins.create_effect(
            t,
            k,
            index,
            (!state.is_empty()).then_some(state.as_slice()),
        ) {
            Ok(_) => added = true,
            Err(e) => {
                app.fx_failed.push(plugins::fx_key(t, k));
                app.state.message = Some(e);
            }
        }
    }
    if added {
        app.transport.set_effects(app.plugins.effects());
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Close,
    Play,
    Add,
    Choose(usize),
    ListPage(i8),
    Select(u32),
    Out(u32),
    In(u32, bool),
    Cable(usize),
    Remove,
    Bypass,
    Gain(i8),
    Open,
    FocusParam(u32),
    ParamPage(i8),
    Parameter(u32, u16),
    Map(u8),
    Macro(u8, u16),
    Save,
    Load,
    Zoom(i8),
    Arrange,
    Undo,
    Redo,
}
pub struct Form {
    pub key: usize,
    pub graph: Arc<Mutex<Graph>>,
    pub selected: u32,
    pub wire: Option<u32>,
    pub add: bool,
    pub query: String,
    pub first: usize,
    choices: Vec<(usize, String)>,
    pub page: usize,
    pub param: Option<u32>,
    last_select: Option<(u32, (f32, f32), std::time::Instant)>,
    pub zoom: f32,
    drag: Option<(u32, f32, f32, f32, f32)>,
    before: Option<core::Document>,
    pub mouse: (f32, f32),
    pub error: Option<String>,
    #[cfg(plugin_windows)]
    editor: Option<ChildEditor>,
}
#[cfg(plugin_windows)]
struct ChildEditor {
    graph: Arc<Mutex<Graph>>,
    id: u32,
    plugin: os_host::SharedPlugin,
    window: crate::plugin_window::FrameWindow,
    link: Option<os_hostlink::Link>,
    before: core::Document,
    info: crate::plugin_window::ChannelInfo,
}
#[cfg(plugin_windows)]
impl Drop for ChildEditor {
    fn drop(&mut self) {
        let _ = plugins::editor_action(&self.plugin, os_host::EditorAction::Close);
        if let Ok(mut g) = self.graph.lock() {
            if g.plugin(self.id)
                .is_some_and(|p| Arc::ptr_eq(&p, &self.plugin))
                && g.snapshot().is_ok_and(|d| d != self.before)
            {
                g.edits
                    .push(serde_json::json!({"plugin":{"patch":self.before}}).to_string());
            }
        }
    }
}
pub fn graph(app: &App, key: usize) -> Option<Arc<Mutex<Graph>>> {
    app.plugins
        .hosted
        .get(&key)?
        .plugin
        .lock()
        .ok()?
        .host_extension()?
        .downcast::<Mutex<Graph>>()
        .ok()
}
pub fn open(app: &mut App, key: usize) -> bool {
    let Some(g) = graph(app, key) else {
        return false;
    };
    core::set_catalog(&app.plugins.effects);
    let choices = app
        .plugins
        .effects
        .iter()
        .enumerate()
        .filter(|(_, a)| a.desc.id != core::ID)
        .map(|(i, a)| {
            (
                i,
                if crate::project_plugins::is_builtin(&a.desc) {
                    a.desc.name.clone()
                } else {
                    format!("{} ({})", a.desc.name, plugins::format_name(a.desc.format))
                },
            )
        })
        .collect();
    app.state.dialogs.push(Modal::new(Body::Patch(Form {
        key,
        graph: g,
        selected: 0,
        wire: None,
        add: false,
        query: String::new(),
        first: 0,
        choices,
        page: 0,
        param: None,
        last_select: None,
        zoom: ((app.state.screen.0 - 290.) / 830.).clamp(0.4, 1.),
        drag: None,
        before: None,
        mouse: app.mouse,
        error: None,
        #[cfg(plugin_windows)]
        editor: None,
    })));
    app.invalidate();
    true
}
fn form(s: &State) -> Option<&Form> {
    match &s.dialogs.last()?.body {
        Body::Patch(f) => Some(f),
        _ => None,
    }
}
fn form_mut(s: &mut State) -> Option<&mut Form> {
    match &mut s.dialogs.last_mut()?.body {
        Body::Patch(f) => Some(f),
        _ => None,
    }
}
fn record(app: &mut App, key: usize, old: core::Document) {
    app.params_tick();
    app.record_plugin_state_edit(
        key,
        &serde_json::json!({"plugin":{"patch":old}}).to_string(),
    );
    app.state.piano.dirty = true;
}
pub fn command(app: &mut App, c: Command) {
    let Some(f) = form(&app.state) else {
        return;
    };
    let (key, g, id) = (f.key, f.graph.clone(), f.selected);
    if matches!(c, Command::Close) {
        app.state.dialogs.pop();
        app.invalidate();
        return;
    }
    if matches!(
        c,
        Command::Save | Command::Load | Command::Undo | Command::Redo
    ) {
        #[cfg(plugin_windows)]
        if let Some(f) = form_mut(&mut app.state) {
            f.editor = None;
        }
        app.params_tick();
    }
    if matches!(c, Command::Save | Command::Load) {
        let cmd = if c == Command::Save {
            crate::presets::PCmd::SaveAs(key)
        } else {
            crate::presets::PCmd::Import(key)
        };
        app.state.message = crate::presets::run(app, cmd);
        return;
    }
    if matches!(c, Command::Undo | Command::Redo) {
        if c == Command::Undo {
            crate::history::undo(&mut app.state);
        } else {
            crate::history::redo(&mut app.state);
        }
        app.plugin_tick();
        app.invalidate();
        return;
    }
    if c == Command::Play {
        app.act(if app.transport.playing() {
            Action::Stop
        } else {
            Action::Play
        });
        return;
    }
    if let Command::Select(n) = c {
        let now = std::time::Instant::now();
        let mut double = false;
        if let Some(f) = form_mut(&mut app.state) {
            double = n > 1
                && f.last_select.is_some_and(|(id, pos, at)| {
                    id == n
                        && now.duration_since(at).as_millis() < 400
                        && (pos.0 - f.mouse.0).hypot(pos.1 - f.mouse.1) < 4.
                });
            f.last_select = Some((n, f.mouse, now));
            if double {
                f.drag = None;
                f.before = None;
            }
        }
        if double {
            open_child(app);
            return;
        }
    }
    if c == Command::Open {
        open_child(app);
        return;
    }
    #[cfg(plugin_windows)]
    if matches!(
        c,
        Command::Select(_)
            | Command::Parameter(..)
            | Command::Macro(..)
            | Command::Choose(_)
            | Command::In(..)
            | Command::Cable(_)
            | Command::Remove
            | Command::Bypass
            | Command::Gain(_)
            | Command::Map(_)
            | Command::Arrange
    ) {
        if let Some(f) = form_mut(&mut app.state) {
            f.editor = None;
        }
        app.params_tick();
    }
    if matches!(c, Command::Bypass | Command::Gain(_)) {
        let changed = (|| {
            let mut g = g.lock().ok()?;
            let old = g.snapshot().ok()?;
            let n = g.doc.nodes.iter_mut().find(|n| n.id == id)?;
            match c {
                Command::Bypass => n.bypass = !n.bypass,
                Command::Gain(d) => n.gain = (n.gain + d as f32 * 0.05).clamp(0., 2.),
                _ => {}
            }
            Some(old)
        })();
        if let Some(old) = changed {
            record(app, key, old);
        }
        app.invalidate();
        return;
    }
    let Some(f) = form_mut(&mut app.state) else {
        return;
    };
    f.error = None;
    match c {
        Command::Add => {
            f.add = !f.add;
            f.query.clear();
            f.first = 0;
        }
        Command::ListPage(d) => f.first = f.first.saturating_add_signed(d as isize * 10),
        Command::Select(n) => {
            f.selected = n;
            f.page = 0;
            f.param = None;
            if n > 1 {
                if let Ok(g) = g.lock() {
                    if let Some(node) = g.doc.nodes.iter().find(|x| x.id == n) {
                        f.drag = Some((n, f.mouse.0, f.mouse.1, node.x, node.y));
                        f.before = g.snapshot().ok();
                    }
                }
            }
        }
        Command::Out(n) => f.wire = Some(n),
        Command::FocusParam(id) => {
            f.param = Some(id);
        }
        Command::ParamPage(d) => f.page = f.page.saturating_add_signed(d as isize),
        Command::Zoom(d) => f.zoom = (f.zoom + d as f32 * 0.1).clamp(0.4, 1.5),
        Command::Parameter(p, v) => {
            f.param = Some(p);
            let before = g.lock().ok().and_then(|g| g.snapshot().ok());
            if let Some(p) = g.lock().ok().and_then(|g| g.plugin(id)) {
                if let Ok(mut p) = p.lock() {
                    p.set_param(p_id(c), v as f64 / 1000.);
                }
            }
            if let Some(old) = before {
                record(app, key, old);
            }
        }
        Command::Macro(i, v) => {
            let old = g.lock().ok().and_then(|g| g.snapshot().ok());
            if let Ok(mut g) = g.lock() {
                g.set_macro(i as usize, v as f64 / 1000.);
            }
            if let Some(old) = old {
                record(app, key, old);
            }
        }
        _ => {
            let Ok(mut graph) = g.lock() else {
                return;
            };
            let old = match graph.snapshot() {
                Ok(v) => v,
                Err(e) => {
                    f.error = Some(e);
                    return;
                }
            };
            let mut next = old.clone();
            match c {
                Command::Choose(index) => {
                    let Some(a) = app.plugins.effects.get(index) else {
                        return;
                    };
                    if a.desc.id == core::ID {
                        return;
                    }
                    let n = next.next_id;
                    next.next_id += 1;
                    next.nodes.push(core::Node {
                        id: n,
                        uid: a.desc.uid(),
                        name: a.desc.name.clone(),
                        state: vec![],
                        x: 190. + ((n - 2) % 3) as f32 * 180.,
                        y: 65. + ((n - 2) / 3) as f32 * 95.,
                        bypass: false,
                        gain: 1.,
                    });
                    // First effect replaces the initial through cable. Further effects are
                    // inserted after the selected node, preserving its outgoing branches.
                    let from = if id == 1 { 0 } else { id };
                    let outgoing: Vec<_> = next
                        .edges
                        .iter()
                        .filter(|e| e.from == from && !e.key)
                        .copied()
                        .collect();
                    if outgoing.is_empty() {
                        next.edges.push(Edge {
                            from: n,
                            to: 1,
                            key: false,
                        });
                    } else {
                        next.edges.retain(|e| e.from != from || e.key);
                        for mut e in outgoing {
                            e.from = n;
                            next.edges.push(e);
                        }
                    }
                    next.edges.push(Edge {
                        from,
                        to: n,
                        key: false,
                    });
                    f.selected = n;
                    f.add = false;
                }
                Command::In(to, key) => {
                    let Some(from) = f.wire.take() else {
                        return;
                    };
                    let e = Edge { from, to, key };
                    if next.edges.contains(&e) {
                        return;
                    }
                    next.edges.push(e);
                }
                Command::Cable(i) => {
                    if i >= next.edges.len() {
                        return;
                    }
                    next.edges.remove(i);
                }
                Command::Remove => {
                    if id < 2 {
                        return;
                    }
                    next.nodes.retain(|n| n.id != id);
                    next.edges.retain(|e| e.from != id && e.to != id);
                    for m in &mut next.macros {
                        if m.node == id {
                            m.node = 0;
                        }
                    }
                    f.selected = 0;
                }
                Command::Bypass => {
                    if let Some(n) = next.nodes.iter_mut().find(|n| n.id == id) {
                        n.bypass = !n.bypass;
                    }
                }
                Command::Gain(d) => {
                    if let Some(n) = next.nodes.iter_mut().find(|n| n.id == id) {
                        n.gain = (n.gain + d as f32 * 0.05).clamp(0., 2.);
                    }
                }
                Command::Map(m) => {
                    let Some(param) = f.param else {
                        f.error = Some("Select a parameter in the inspector first".into());
                        return;
                    };
                    let Some(p) = graph.plugin(id) else {
                        return;
                    };
                    let p = p.lock().unwrap();
                    let value = p.param_value(param).unwrap_or(0.5);
                    let name = (0..p.param_count())
                        .filter_map(|i| p.param_info(i))
                        .find(|p| p.id == param)
                        .map(|p| p.name)
                        .unwrap_or_else(|| format!("Macro {}", m + 1));
                    next.macros[m as usize] = core::Macro {
                        name,
                        node: id,
                        param,
                        value,
                        low: 0.,
                        high: 1.,
                    };
                }
                Command::Arrange => {
                    for (i, n) in next.nodes.iter_mut().enumerate() {
                        n.x = 180. + (i % 3) as f32 * 180.;
                        n.y = 45. + (i / 3) as f32 * 90.;
                    }
                    f.zoom = if next.nodes.len() > 12 {
                        0.4
                    } else {
                        f.zoom.min(0.8)
                    };
                }
                _ => return,
            }
            // Compare against the current snapshot, so adding a cable does not
            // reload plugins whose editor has changed parameters since opening.
            for n in &old.nodes {
                if let Some(o) = graph.doc.nodes.iter_mut().find(|o| o.id == n.id) {
                    o.state = n.state.clone();
                }
            }
            match graph.replace(next) {
                Ok(()) => {
                    drop(graph);
                    record(app, key, old);
                }
                Err(e) => f.error = Some(e),
            }
        }
    }
    app.invalidate();
}
fn p_id(c: Command) -> u32 {
    if let Command::Parameter(p, _) = c {
        p
    } else {
        0
    }
}
pub fn motion(app: &mut App, x: f32, y: f32) -> bool {
    let Some(f) = form_mut(&mut app.state) else {
        return false;
    };
    f.mouse = (x, y);
    if let Some((id, x0, y0, nx, ny)) = f.drag {
        if let Ok(mut g) = f.graph.lock() {
            if let Some(n) = g.doc.nodes.iter_mut().find(|n| n.id == id) {
                n.x = (nx + (x - x0) / f.zoom).clamp(0., 1800.);
                n.y = (ny + (y - y0) / f.zoom).clamp(0., 1000.);
            }
        }
        app.mouse = (x, y);
        app.invalidate();
        return true;
    }
    if f.wire.is_some() {
        app.invalidate();
    }
    false
}
pub fn release(app: &mut App) -> bool {
    let Some(f) = form_mut(&mut app.state) else {
        return false;
    };
    let dragged = f.drag.take().is_some();
    let before = f.before.take();
    let key = f.key;
    if dragged {
        if let Some(old) = before {
            let changed = f.graph.lock().is_ok_and(|g| {
                g.doc
                    .nodes
                    .iter()
                    .zip(&old.nodes)
                    .any(|(n, o)| n.x != o.x || n.y != o.y)
            });
            if changed {
                record(app, key, old);
            }
        }
        app.invalidate();
        return true;
    }
    let target = app
        .hits
        .iter()
        .rev()
        .find(|h| h.rect.contains(app.mouse.0, app.mouse.1))
        .and_then(|h| {
            if let Action::Dlg(DHit::Patch(c @ Command::In(..))) = h.action {
                Some(c)
            } else {
                None
            }
        });
    if let Some(c) = target {
        command(app, c);
    }
    false
}
pub fn key(app: &mut App, k: &Key, ctrl: bool, alt: bool) {
    if ctrl && matches!(k,Key::Character(c) if c.eq_ignore_ascii_case("z")) {
        command(
            app,
            if app.shift {
                Command::Redo
            } else {
                Command::Undo
            },
        );
        return;
    }
    let Some(f) = form_mut(&mut app.state) else {
        return;
    };
    match k {
        Key::Named(NamedKey::Escape) => {
            if f.wire.take().is_some() || f.add {
                f.add = false;
            } else if let Some(old) = f.before.take() {
                if let Ok(mut g) = f.graph.lock() {
                    for n in &mut g.doc.nodes {
                        if let Some(o) = old.nodes.iter().find(|o| o.id == n.id) {
                            n.x = o.x;
                            n.y = o.y;
                        }
                    }
                }
                f.drag = None;
            } else {
                command(app, Command::Close);
            }
        }
        Key::Named(NamedKey::Space) if !f.add => command(app, Command::Play),
        Key::Named(NamedKey::Delete) if !f.add => command(app, Command::Remove),
        Key::Named(NamedKey::Backspace) if f.add => {
            f.query.pop();
            f.first = 0;
        }
        Key::Character(s) if f.add && !ctrl && !alt => {
            f.query.push_str(s);
            f.first = 0;
        }
        _ => {}
    }
    app.invalidate();
}
#[cfg(plugin_windows)]
fn open_child(app: &mut App) {
    let (owner, scale) = (app.plugins.owner, app.plugins.scale);
    let Some(f) = form_mut(&mut app.state) else {
        return;
    };
    f.editor = None;
    let Ok(g) = f.graph.lock() else {
        return;
    };
    let Some(p) = g.plugin(f.selected) else {
        return;
    };
    let Ok(before) = g.snapshot() else {
        return;
    };
    drop(g);
    let info = crate::plugin_window::ChannelInfo {
        name: p.lock().unwrap().desc().name.clone(),
        target: "ML Patch".into(),
        sidechain: None,
        enabled: true,
        volume: 1.,
        pan: 0.,
        pitch_cents: 0,
        range: 12,
        track: 0,
        max_track: 0,
        misc: None,
    };
    let pos = crate::plugin_window::first_position(owner, scale, true);
    #[cfg(target_os = "linux")]
    let w = crate::plugin_window::FrameWindow::with_zoom(
        owner,
        info.clone(),
        (560., 437.),
        pos,
        true,
        1.,
    );
    #[cfg(not(target_os = "linux"))]
    let w =
        crate::plugin_window::FrameWindow::new(owner, info.clone(), (560., 437.), scale, pos, true);
    match w.and_then(|w| {
        w.set_strip(false);
        let size = plugins::editor_action(
            &p,
            os_host::EditorAction::Open {
                parent: w.area(),
                scale: w.scale(),
            },
        )?;
        let link = plugins::fit_editor(&w, size, w.scale());
        w.raise();
        Ok((w, link))
    }) {
        Ok((window, link)) => {
            f.editor = Some(ChildEditor {
                graph: f.graph.clone(),
                id: f.selected,
                plugin: p,
                window,
                link,
                before,
                info,
            })
        }
        Err(e) => {
            f.error = Some(format!(
                "{e}. Use the parameter controls in this inspector."
            ))
        }
    }
}
#[cfg(not(plugin_windows))]
fn open_child(app: &mut App) {
    if let Some(f) = form_mut(&mut app.state) {
        f.error = Some("Use the parameter controls in this inspector on this platform.".into());
    }
}
pub fn tick(app: &mut App) {
    #[cfg(plugin_windows)]
    let owner = app.plugins.owner;
    let key = match form(&app.state) {
        Some(f) => f.key,
        None => return,
    };
    let current = graph(app, key);
    let Some(f) = form_mut(&mut app.state) else {
        return;
    };
    let Some(current) = current else {
        app.state.dialogs.pop();
        return;
    };
    if !Arc::ptr_eq(&current, &f.graph) {
        #[cfg(plugin_windows)]
        {
            f.editor = None;
        }
        f.graph = current;
    }
    #[cfg(plugin_windows)]
    {
        let mut close = false;
        if let Some(e) = &mut f.editor {
            if !f
                .graph
                .lock()
                .ok()
                .and_then(|g| g.plugin(e.id))
                .is_some_and(|p| Arc::ptr_eq(&p, &e.plugin))
            {
                close = true;
            }
            let mut size = None;
            let mut job = None;
            let mut resizable = None;
            if let Ok(mut p) = e.plugin.try_lock() {
                size = p.editor_size();
                resizable = p.editor_resizable().then_some((1., 1.));
                job = p.take_ui_job();
            }
            if let Some(job) = job {
                job();
            }
            if let Some(l) = &mut e.link {
                e.window.set_page(l.page());
                e.window.set_hint(l.hint());
                resizable = l.resizable().map(|(w, h)| (w as f32, h as f32));
                if let Some((w, h)) = l.size() {
                    let k = e.window.scale();
                    size = Some(((w as f32 * k).round() as u32, (h as f32 * k).round() as u32));
                }
            }
            e.window.set_resizable(resizable);
            if let Some(size) = size {
                let k = e.window.scale();
                e.window
                    .set_editor_size((size.0 as f32 / k, size.1 as f32 / k));
            }
            for event in e.window.take_events() {
                match event {
                    crate::plugin_window::FrameEvent::Close => close = true,
                    crate::plugin_window::FrameEvent::ToggleOn => {
                        if let Ok(mut g) = f.graph.lock() {
                            if let Some(n) = g.doc.nodes.iter_mut().find(|n| n.id == e.id) {
                                n.bypass = !n.bypass;
                                e.info.enabled = !n.bypass;
                                e.window.set_info(e.info.clone());
                            }
                        }
                    }
                    crate::plugin_window::FrameEvent::Detach(on) => {
                        e.window.set_detached(owner, on)
                    }
                    crate::plugin_window::FrameEvent::Page(n) => {
                        if let Some(l) = &mut e.link {
                            l.set_page(n);
                        }
                    }
                    crate::plugin_window::FrameEvent::Resize(w, h) => {
                        if let Some(l) = &mut e.link {
                            l.set_size(w, h);
                        } else if let Ok(mut p) = e.plugin.lock() {
                            let k = e.window.scale();
                            p.resize_editor((w as f32 * k) as u32, (h as f32 * k) as u32);
                        }
                    }
                    _ => {}
                }
            }
        }
        if close {
            f.editor = None;
        }
    }
}
fn button(ui: &mut Ui, r: Rect, label: &str, c: Command, on: bool) {
    if matches!(c, Command::Out(_) | Command::In(..)) {
        let (hint, color) = match c {
            Command::Out(_) => ("Audio output: drag or click to connect", ui.t.accent),
            Command::In(_, true) => (
                "Sidechain input (if supported by this effect)",
                Color::rgb(0xb395ee),
            ),
            _ => ("Stereo audio input", ui.t.accent),
        };
        ui.hit(r, Action::Dlg(DHit::Patch(c)), hint);
        ui.c.fill_rrect(r, 7., color);
        return;
    }

    let hot = ui.hit(r, Action::Dlg(DHit::Patch(c)), label.to_string());
    ui.c.fill_rrect(
        r,
        4.,
        if on {
            ui.t.accent
        } else if hot {
            ui.t.button_hi
        } else {
            ui.t.sunken
        },
    );
    ui.c.text_clip(
        Font::new(12),
        r.x + 7.,
        r.y + 6.,
        r.w - 14.,
        if on { ui.t.sunken } else { ui.t.text },
        label,
    );
}
fn label(ui: &mut Ui, x: f32, y: f32, w: f32, text: &str) {
    ui.c.text_clip(Font::new(12), x, y, w, ui.t.text, text);
}
pub fn paint(ui: &mut Ui, r: Rect, f: &Form) {
    let Ok(g) = f.graph.try_lock() else {
        return;
    };
    for (i, (s, c)) in [
        ("Play / Stop", Command::Play),
        ("Add plugin", Command::Add),
        ("Load", Command::Load),
        ("Save", Command::Save),
        ("Arrange", Command::Arrange),
        ("Undo", Command::Undo),
        ("Redo", Command::Redo),
        ("−", Command::Zoom(-1)),
        ("+", Command::Zoom(1)),
    ]
    .into_iter()
    .enumerate()
    {
        button(
            ui,
            Rect::new(
                r.x + 12. + i as f32 * ((r.w - 24.) / 9.),
                r.y + 36.,
                (r.w - 24.) / 9. - 6.,
                28.,
            ),
            s,
            c,
            false,
        );
    }
    let map = Rect::new(r.x + 12., r.y + 76., r.w - 254., r.h - 154.);
    ui.c.fill_rrect(map, 5., ui.t.sunken);
    let node_rect = |id: u32| {
        let (x, y) = if id == 0 {
            (20., 120.)
        } else if id == 1 {
            (660., 120.)
        } else {
            g.doc
                .nodes
                .iter()
                .find(|n| n.id == id)
                .map(|n| (n.x, n.y))
                .unwrap_or((0., 0.))
        };
        Rect::new(
            map.x + x * f.zoom,
            map.y + y * f.zoom,
            144. * f.zoom,
            60. * f.zoom,
        )
    };
    ui.c.push_clip(map);
    ui.clip_hit_rect(map, |ui| {
        for (i, e) in g.doc.edges.iter().enumerate() {
            let a = node_rect(e.from);
            let b = node_rect(e.to);
            let (x0, y0, x1, y1) = (
                a.x + a.w,
                a.y + a.h * 0.5,
                b.x,
                b.y + b.h * if e.key { 0.8 } else { 0.5 },
            );
            let color = if e.key {
                Color::rgb(0xb395ee)
            } else {
                ui.t.accent
            };
            let mid = (x0 + x1) * 0.5;
            ui.c.line(x0, y0, mid, y0, 2., color);
            ui.c.line(mid, y0, mid, y1, 2., color);
            ui.c.line(mid, y1, x1, y1, 2., color);
            let hit = Rect::new(mid - 7., (y0 + y1) * 0.5 - 7., 14., 14.);
            ui.hit(
                hit,
                Action::Dlg(DHit::Patch(Command::Cable(i))),
                "Disconnect cable (Undo restores it)",
            );
            ui.c.fill_rrect(Rect::new(mid - 3., (y0 + y1) * 0.5 - 3., 6., 6.), 3., color);
        }
        for id in [0, 1].into_iter().chain(g.doc.nodes.iter().map(|n| n.id)) {
            let n = node_rect(id);
            let spec = g.doc.nodes.iter().find(|n| n.id == id);
            let name = if id == 0 {
                "From ML Studio"
            } else if id == 1 {
                "To ML Studio"
            } else {
                &spec.unwrap().name
            };
            ui.hit(
                n,
                Action::Dlg(DHit::Patch(Command::Select(id))),
                "Select or drag; double-click to open plugin",
            );
            ui.c.fill_rrect(n, 5., ui.t.button_hi);
            ui.c.stroke_rrect(
                n,
                5.,
                if f.selected == id { 2. } else { 1. },
                if f.selected == id {
                    ui.t.accent
                } else {
                    ui.t.text_dim
                },
            );
            label(ui, n.x + 9., n.y + 8., n.w - 18., name);
            if id != 0 {
                button(
                    ui,
                    Rect::new(n.x - 7., n.y + n.h * 0.5 - 7., 14., 14.),
                    "",
                    Command::In(id, false),
                    true,
                );
            }
            if id != 1 {
                button(
                    ui,
                    Rect::new(n.x + n.w - 7., n.y + n.h * 0.5 - 7., 14., 14.),
                    "",
                    Command::Out(id),
                    true,
                );
            }
            if id > 1 {
                button(
                    ui,
                    Rect::new(n.x - 7., n.y + n.h * 0.8 - 7., 14., 14.),
                    "",
                    Command::In(id, true),
                    false,
                );
                let status = if g.error(id).is_some() {
                    "Missing plugin"
                } else if spec.unwrap().bypass {
                    "Bypassed"
                } else {
                    "Stereo · Key input below"
                };
                if f.zoom >= 0.75 {
                    label(ui, n.x + 8., n.y + n.h - 18., n.w - 16., status);
                }
            }
        }
        if let Some(id) = f.wire {
            let a = node_rect(id);
            ui.c.line(
                a.x + a.w,
                a.y + a.h * 0.5,
                f.mouse.0,
                f.mouse.1,
                2.,
                ui.t.accent,
            );
        }
    });
    ui.c.pop_clip();
    let x = map.x + map.w + 12.;
    let w = r.x + r.w - x - 12.;
    let y = map.y;
    if f.add {
        label(ui, x, y, w, "ADD EFFECT");
        label(
            ui,
            x,
            y + 24.,
            w,
            &format!(
                "Search: {}",
                if f.query.is_empty() {
                    "type a name"
                } else {
                    &f.query
                }
            ),
        );
        let q = f.query.to_lowercase();
        let results: Vec<_> = f
            .choices
            .iter()
            .filter(|(_, n)| n.to_lowercase().contains(&q))
            .collect();
        let first = f.first.min(results.len().saturating_sub(1) / 10 * 10);
        for (i, (index, name)) in results.iter().skip(first).take(10).enumerate() {
            button(
                ui,
                Rect::new(x, y + 50. + i as f32 * 29., w, 26.),
                name,
                Command::Choose(*index),
                false,
            );
        }
        button(
            ui,
            Rect::new(x, y + 346., w / 2. - 3., 26.),
            "Previous",
            Command::ListPage(-1),
            false,
        );
        button(
            ui,
            Rect::new(x + w / 2. + 3., y + 346., w / 2. - 3., 26.),
            "Next",
            Command::ListPage(1),
            false,
        );
    } else {
        label(ui, x, y, w, "PLUGIN CONTROLS");
        if let Some(n) = g.doc.nodes.iter().find(|n| n.id == f.selected) {
            label(ui, x, y + 24., w, &n.name);
            button(
                ui,
                Rect::new(x, y + 48., w, 27.),
                "Open plugin editor",
                Command::Open,
                false,
            );
            button(
                ui,
                Rect::new(x, y + 81., w / 2. - 3., 27.),
                if n.bypass { "Enable" } else { "Bypass" },
                Command::Bypass,
                n.bypass,
            );
            button(
                ui,
                Rect::new(x + w / 2. + 3., y + 81., w / 2. - 3., 27.),
                "Remove",
                Command::Remove,
                false,
            );
            button(
                ui,
                Rect::new(x, y + 114., 28., 24.),
                "−",
                Command::Gain(-1),
                false,
            );
            label(
                ui,
                x + 35.,
                y + 119.,
                w - 70.,
                &format!("Output {:.0}%", n.gain * 100.),
            );
            button(
                ui,
                Rect::new(x + w - 28., y + 114., 28., 24.),
                "+",
                Command::Gain(1),
                false,
            );
            if let Some(p) = g.plugin(n.id) {
                if let Ok(p) = p.try_lock() {
                    let first = f.page * 4;
                    for (row, i) in (first..p.param_count()).take(4).enumerate() {
                        if let Some(param) = p.param_info(i) {
                            let yy = y + 151. + row as f32 * 45.;
                            ui.hit(
                                Rect::new(x, yy, w, 17.),
                                Action::Dlg(DHit::Patch(Command::FocusParam(param.id))),
                                "Select this parameter for a macro",
                            );
                            if f.param == Some(param.id) {
                                ui.c.fill_rrect(Rect::new(x, yy, w, 17.), 2., ui.t.button_hi);
                            }
                            label(ui, x, yy, w, &param.name);
                            let v = p.param_value(param.id).unwrap_or(0.);
                            slider(ui, Rect::new(x, yy + 18., w, 16.), v, |v| {
                                Command::Parameter(param.id, v)
                            });
                        }
                    }
                    button(
                        ui,
                        Rect::new(x, y + 334., w / 2. - 3., 24.),
                        "Previous",
                        Command::ParamPage(-1),
                        false,
                    );
                    button(
                        ui,
                        Rect::new(x + w / 2. + 3., y + 334., w / 2. - 3., 24.),
                        "Next",
                        Command::ParamPage(1),
                        false,
                    );
                }
            }
            label(ui, x, y + 368., w, "Select a parameter, then map:");
            for i in 0..8 {
                button(
                    ui,
                    Rect::new(x + i as f32 * (w / 8.), y + 389., w / 8. - 2., 23.),
                    &(i + 1).to_string(),
                    Command::Map(i),
                    false,
                );
            }
        } else {
            label(ui, x, y + 25., w, "Select a plugin to edit it.");
            label(ui, x, y + 50., w, "Drag nodes to arrange them.");
            label(ui, x, y + 75., w, "Connect an output to an input.");
            label(ui, x, y + 100., w, "Cable dots disconnect paths.");
            label(ui, x, y + 125., w, "Lower inputs are sidechains.");
        }
    }
    let my = r.y + r.h - 66.;
    for (i, m) in g.doc.macros.iter().enumerate() {
        let mw = (r.w - 24.) / 8.;
        label(ui, r.x + 12. + i as f32 * mw, my, mw - 8., &m.name);
        slider(
            ui,
            Rect::new(r.x + 12. + i as f32 * mw, my + 21., mw - 8., 15.),
            m.value,
            |v| Command::Macro(i as u8, v),
        );
    }
    let message=f.error.as_deref().or(g.message.as_deref()).unwrap_or("Add plugins, then connect their ports. Parallel paths are summed; use Output to balance them. Escape cancels a cable.");
    label(ui, r.x + 12., r.y + r.h - 23., r.w - 24., message);
}
fn slider(ui: &mut Ui, r: Rect, value: f64, cmd: impl Fn(u16) -> Command) {
    ui.c.fill_rrect(r, 3., ui.t.button_hi);
    ui.c.fill_rrect(
        Rect::new(r.x, r.y, r.w * value as f32, r.h),
        3.,
        ui.t.accent,
    );
    for i in 0..100 {
        ui.hit(
            Rect::new(r.x + r.w * i as f32 / 100., r.y, r.w / 100. + 0.1, r.h),
            Action::Dlg(DHit::Patch(cmd((i * 1000 / 99) as u16))),
            "Set value",
        );
    }
}

#[cfg(test)]
mod tests;
