// SPDX-License-Identifier: GPL-3.0-or-later
//! The docked and detached Browser share one library, tree and pointer layout.
use super::*;
use crate::{
    app::App,
    browser::BCmd,
    playlist::{Cmd, PItem, Popup},
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;

pub const VISIBILITY: &str = "visible_browser-layout-setting";
pub const ALL: usize = 10;
const PLUGINS: usize = 11;
const EFFECTS: usize = 12;
const RECENT: usize = 13;
const COLLECTION: usize = 100;
const EDGE: f32 = 12.;
const PITCH: f32 = 30.;
const MAX_RECENT: usize = 72;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Control {
    Edge,
    Star(usize),
    New,
    Rename(usize),
    Delete(usize),
    Add(usize),
    Remove(usize),
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
enum Bookmark {
    Plugin { uid: String, effect: bool },
    Preset(String),
    Sample(String),
    File(String),
}

#[derive(Default, Serialize, Deserialize)]
struct Collection {
    name: String,
    items: Vec<Bookmark>,
}

#[derive(Default, Serialize, Deserialize)]
struct Saved {
    favorites: Vec<Bookmark>,
    recent: Vec<Bookmark>,
    collections: Vec<Collection>,
}

#[derive(Default)]
pub struct Library {
    saved: Saved,
    pub open: bool,
    scroll: f32,
    pub edit: Option<(Option<usize>, crate::dialog::Field)>,
    pub(crate) presets: RefCell<HashMap<String, Rc<Vec<(String, PathBuf)>>>>,
}

impl Library {
    pub fn load() -> Self {
        Self {
            saved: crate::prefs::text("browser_library_v1")
                .and_then(|v| serde_json::from_str(&v).ok())
                .unwrap_or_default(),
            ..Self::default()
        }
    }
    fn save(&self) {
        if let Ok(data) = serde_json::to_string(&self.saved) {
            crate::prefs::set_text("browser_library_v1", &data);
        }
    }
}

pub fn enabled() -> bool {
    !crate::prefs::check(&crate::prefs::LEGACY_BROWSER)
}
pub fn library_tab(id: usize) -> bool {
    matches!(id, ALL | PLUGINS | EFFECTS | RECENT) || id >= COLLECTION
}
pub fn valid_tab(s: &State, id: usize) -> bool {
    matches!(id, ALL | PLUGINS | EFFECTS | RECENT)
        || id
            .checked_sub(COLLECTION)
            .is_some_and(|i| i < s.browser_library.saved.collections.len())
}

fn plugins(s: &State, effect: bool) -> &[crate::plugin_db::Entry] {
    if effect {
        &s.plugdb.lists.fx
    } else {
        &s.plugdb.lists.gens
    }
}
fn bookmark(s: &State, row: &Row) -> Option<Bookmark> {
    Some(match &row.item {
        Item::Generator(i) | Item::Effect(i) => {
            let effect = matches!(row.item, Item::Effect(_));
            Bookmark::Plugin {
                effect,
                uid: plugins(s, effect)
                    .iter()
                    .find(|e| e.index == *i)?
                    .uid
                    .clone(),
            }
        }
        Item::Sample(p) => Bookmark::Sample(crate::plugins::resolve_sample(p)),
        Item::Preset(p) => Bookmark::Preset(p.clone()),
        Item::File(p) => Bookmark::File(p.clone()),
        _ => return None,
    })
}

fn resolve(s: &State, b: &Bookmark) -> Option<Row> {
    let (key, label, icon, item, path) = match b {
        Bookmark::Plugin { uid, effect } => {
            let e = plugins(s, *effect).iter().find(|e| &e.uid == uid)?;
            (
                format!("plugin/{effect}/{uid}"),
                e.label.clone(),
                Icon::Plug,
                if *effect {
                    Item::Effect(e.index)
                } else {
                    Item::Generator(e.index)
                },
                None,
            )
        }
        Bookmark::Preset(p) => (
            format!("preset/{p}"),
            crate::presets::shown_name(Path::new(p)),
            Icon::FilePreset,
            Item::Preset(p.clone()),
            (!crate::presets::is_factory(Path::new(p))).then(|| p.into()),
        ),
        Bookmark::Sample(p) | Bookmark::File(p) => {
            let path = PathBuf::from(p);
            (
                format!("file/{p}"),
                folder_name(&path),
                file_icon(p),
                if matches!(b, Bookmark::Sample(_)) {
                    Item::Sample(p.clone())
                } else {
                    Item::File(p.clone())
                },
                Some(path),
            )
        }
    };
    Some(Row {
        key: format!("{}:{key}", s.browser_tab),
        depth: 0,
        label,
        icon,
        item,
        path,
        cat: FILE,
        bright: false,
        open: None,
    })
}

pub fn remember(s: &mut State, row: &Row) {
    let Some(b) = bookmark(s, row) else { return };
    if s.browser_tab == RECENT || s.browser_library.saved.recent.first() == Some(&b) {
        return;
    }
    let recent = &mut s.browser_library.saved.recent;
    recent.retain(|other| other != &b);
    recent.insert(0, b);
    recent.truncate(MAX_RECENT);
    s.browser_library.save();
}

pub(super) fn saved_rows(s: &State, favorites: bool) -> Vec<Row> {
    let entries = if favorites {
        &s.browser_library.saved.favorites
    } else if let Some(c) = s
        .browser_tab
        .checked_sub(COLLECTION)
        .and_then(|i| s.browser_library.saved.collections.get(i))
    {
        &c.items
    } else {
        &s.browser_library.saved.recent
    };
    let q = s.browser_search.as_str().to_lowercase();
    entries
        .iter()
        .filter_map(|b| resolve(s, b))
        .filter(|r| q.is_empty() || matches(s, r, &q))
        .collect()
}

fn file_favorite(b: &Bookmark) -> Option<&Path> {
    match b {
        Bookmark::Sample(p) | Bookmark::File(p) => Some(p.as_ref()),
        Bookmark::Preset(p) if !crate::presets::is_factory(Path::new(p)) => Some(p.as_ref()),
        _ => None,
    }
}

fn star(s: &mut State, index: usize) {
    let Some(b) = super::rows(s).get(index).and_then(|r| bookmark(s, r)) else {
        return;
    };
    if let Some(path) = file_favorite(&b) {
        let mut paths = crate::browser::favorites();
        if paths.iter().any(|p| p == path) {
            paths.retain(|p| p != path);
        } else {
            paths.push(path.into());
        }
        if let Ok(data) = serde_json::to_string(&paths) {
            crate::prefs::set_text("browser_favorites", &data);
        }
        *s.browser_index.borrow_mut() = Default::default();
    } else {
        let favorites = &mut s.browser_library.saved.favorites;
        if favorites.contains(&b) {
            favorites.retain(|entry| entry != &b);
        } else {
            favorites.push(b);
        }
        s.browser_library.save();
    }
    s.browser_scroll = s.browser_scroll.min(max_scroll(s));
}

pub(super) fn paint_star(
    ui: &mut Ui,
    s: &State,
    row: &Row,
    index: usize,
    r: Rect,
    paths: &[PathBuf],
) -> bool {
    if !crate::workspace_visibility::shown("visible_browser-favorites") {
        return false;
    }
    let Some(b) = bookmark(s, row) else {
        return false;
    };
    let on = file_favorite(&b).map_or_else(
        || s.browser_library.saved.favorites.contains(&b),
        |p| paths.iter().any(|f| f == p),
    );
    let hover = ui.hit(
        r,
        Action::BrowserLibrary(Control::Star(index)),
        if on {
            "Remove from Favorites"
        } else {
            "Add to Favorites"
        },
    );
    icons::draw(
        ui.c,
        Icon::Star,
        r.inset(3.),
        if on {
            ui.t.accent
        } else if hover {
            ui.t.glyph
        } else {
            ui.t.text_faint
        },
    );
    true
}

fn action(label: impl Into<String>, control: Control) -> PItem {
    PItem::new(label.into(), Cmd::Browser(BCmd::Library(control)))
}

pub fn row_menu(s: &State, row: &Row) -> Vec<PItem> {
    if !enabled() {
        return Vec::new();
    }
    let Some(b) = bookmark(s, row) else {
        return Vec::new();
    };
    let mut choices = Vec::new();
    for (i, c) in s.browser_library.saved.collections.iter().enumerate() {
        if !c.items.contains(&b) {
            choices.push(action(c.name.clone(), Control::Add(i)));
        }
    }
    let mut items = Vec::new();
    if !choices.is_empty() {
        items.push(PItem::sub("Add to collection", choices));
    }
    if let Some(i) = s.browser_tab.checked_sub(COLLECTION) {
        items.push(action("Remove from this collection", Control::Remove(i)));
    }
    items
}

pub fn collection_menu(app: &mut App, tab: usize, x: f32, y: f32) -> bool {
    let Some(i) = tab.checked_sub(COLLECTION) else {
        return false;
    };
    let items = vec![
        action("Rename collection...", Control::Rename(i)),
        action("Remove collection...", Control::Delete(i)),
    ];
    app.state.playlist.popup = Some(Popup {
        x,
        y,
        items,
        sub: None,
        scroll: [0; 2],
        cascade: Vec::new(),
    });
    app.invalidate();
    true
}

pub fn act(app: &mut App, control: Control) {
    let s = &mut app.state;
    match control {
        Control::Edge => {}
        Control::Star(index) => star(s, index),
        Control::New | Control::Rename(_) => {
            let i = if let Control::Rename(i) = control {
                Some(i)
            } else {
                None
            };
            let name = i
                .and_then(|i| s.browser_library.saved.collections.get(i))
                .map(|c| c.name.clone())
                .unwrap_or_default();
            s.browser_library.edit = Some((i, name.as_str().into()));
            s.browser_search_focus = true;
            s.browser_library.open = false;
        }
        Control::Delete(i) => {
            let Some(c) = s.browser_library.saved.collections.get(i) else {
                return;
            };
            use crate::dialog::{Answer, Kind, Msg, Then};
            let prompt = format!(
                "Remove the collection “{}”? Its original files stay in place.",
                c.name
            );
            app.show_msg(Msg::new(
                Kind::Question,
                "Remove collection",
                vec![(prompt, false)],
                &[Answer::Yes, Answer::Cancel],
                Then::RemoveBrowserCollection(i),
            ));
        }
        Control::Add(i) | Control::Remove(i) => {
            let Some(b) = super::rows(s)
                .iter()
                .find(|r| Some(&r.key) == s.browser_sel.as_ref())
                .and_then(|r| bookmark(s, r))
            else {
                return;
            };
            let Some(c) = s.browser_library.saved.collections.get_mut(i) else {
                return;
            };
            if matches!(control, Control::Add(_)) {
                if !c.items.contains(&b) {
                    c.items.push(b);
                }
            } else {
                c.items.retain(|other| other != &b);
            }
            s.browser_library.save();
        }
    }
    app.invalidate();
}

pub fn remove_collection(app: &mut App, i: usize) {
    if i >= app.state.browser_library.saved.collections.len() {
        return;
    }
    app.state.browser_library.saved.collections.remove(i);
    app.state.browser_library.save();
    if app.state.browser_tab >= COLLECTION + i {
        let id = if app.state.browser_tab == COLLECTION + i {
            0
        } else {
            app.state.browser_tab - 1
        };
        app.browser_act(&Action::BrowserTab(id));
    }
    app.invalidate();
}

pub fn edit_key(app: &mut App, key: &winit::keyboard::Key, ctrl: bool, alt: bool) -> bool {
    use winit::keyboard::{Key, NamedKey};
    let s = &mut app.state;
    let Some((index, name)) = &mut s.browser_library.edit else {
        return false;
    };
    match key {
        Key::Named(NamedKey::Escape) => {}
        Key::Named(NamedKey::Enter) => {
            let name = name.as_str().trim().to_owned();
            if name.is_empty()
                || s.browser_library
                    .saved
                    .collections
                    .iter()
                    .enumerate()
                    .any(|(i, c)| Some(i) != *index && c.name.eq_ignore_ascii_case(&name))
            {
                s.message = Some("Choose a unique collection name".into());
                app.invalidate();
                return true;
            }
            let i = if let Some(i) = *index {
                s.browser_library.saved.collections[i].name = name;
                i
            } else {
                let list = &mut s.browser_library.saved.collections;
                list.push(Collection {
                    name,
                    items: Vec::new(),
                });
                list.len() - 1
            };
            s.browser_library.save();
            app.state.browser_library.edit = None;
            app.state.browser_search_focus = false;
            app.browser_act(&Action::BrowserTab(COLLECTION + i));
            return true;
        }
        _ => {
            name.key_limited(key, ctrl, alt, app.shift, 96);
            app.invalidate();
            return true;
        }
    }
    s.browser_library.edit = None;
    s.browser_search_focus = false;
    app.invalidate();
    true
}

fn group(s: &State, out: &mut Vec<Row>, key: String, label: &str) -> bool {
    let open = is_open(s, &key);
    out.push(Row {
        key,
        label: label.into(),
        depth: 0,
        icon: if open { Icon::FolderOpen } else { Icon::Folder },
        cat: FOLDER,
        bright: true,
        open: Some(open),
        item: Item::None,
        path: None,
    });
    open
}

fn plugin_presets(s: &State, name: &str) -> Rc<Vec<(String, PathBuf)>> {
    if let Some(rows) = s.browser_library.presets.borrow().get(name) {
        return rows.clone();
    }
    let mut items = Vec::new();
    if let Some(root) = crate::presets::root() {
        let dir = root.join(crate::presets::folder_name(name));
        for entry in list_dir(s, &dir).iter() {
            if entry.dir {
                for sub in list_dir(s, &entry.path)
                    .iter()
                    .filter(|e| !e.dir && crate::presets::is_preset_file(&e.path))
                {
                    items.push((
                        format!("{}/{}", entry.name, crate::presets::shown_name(&sub.path)),
                        sub.path.clone(),
                    ));
                }
            } else if crate::presets::is_preset_file(&entry.path) {
                items.push((crate::presets::shown_name(&entry.path), entry.path.clone()));
            }
        }
    }
    items.extend(
        crate::presets::factory(name)
            .into_iter()
            .map(|p| (p.name, p.path)),
    );
    let items = Rc::new(items);
    s.browser_library
        .presets
        .borrow_mut()
        .insert(name.into(), items.clone());
    items
}

fn source_group(s: &State, e: &crate::plugin_db::Entry) -> usize {
    if e.builtin {
        return 0;
    }
    let catalog = crate::plugin_downloads::catalog();
    if s.plugdb.installed_downloads.iter().any(|&i| {
        let p = &catalog[i];
        p.scan_vendor
            .as_ref()
            .is_none_or(|vendor| vendor.eq_ignore_ascii_case(&e.vendor))
            && std::iter::once(&p.name)
                .chain(&p.scan_aliases)
                .any(|name| name.eq_ignore_ascii_case(&e.name))
    }) {
        2
    } else {
        1
    }
}

fn local_plugins(s: &State, out: &mut Vec<Row>) {
    let mut groups: [Vec<_>; 3] = Default::default();
    for effect in [false, true] {
        if !effect && s.browser_tab == EFFECTS {
            continue;
        }
        for e in plugins(s, effect) {
            groups[source_group(s, e)].push((e, effect));
        }
    }
    for entries in &mut groups {
        entries.sort_by_cached_key(|(entry, _)| entry.label.to_lowercase());
    }
    for (index, (label, entries)) in [
        "ML Studio Plugins",
        "Third-party Plugins",
        "ML Studio Community Plugins",
    ]
    .into_iter()
    .zip(groups)
    .enumerate()
    {
        let root = format!("{}:source/{index}", s.browser_tab);
        if !group(s, out, root.clone(), label) {
            continue;
        }
        for (e, effect) in entries {
            let key = format!("{root}/{effect}/{}", e.uid);
            let open = is_open(s, &key);
            out.push(Row {
                key: key.clone(),
                label: e.label.clone(),
                depth: 1,
                icon: if open { Icon::FolderOpen } else { Icon::Folder },
                cat: FOLDER,
                bright: false,
                open: Some(open),
                item: if effect {
                    Item::Effect(e.index)
                } else {
                    Item::Generator(e.index)
                },
                path: None,
            });
            if open {
                for (label, path) in plugin_presets(s, &e.name).iter() {
                    out.push(Row {
                        key: format!("{key}/{}", path.display()),
                        depth: 2,
                        label: label.clone(),
                        icon: Icon::FilePreset,
                        cat: FILE,
                        open: None,
                        bright: false,
                        item: Item::Preset(path.to_string_lossy().into_owned()),
                        path: (!crate::presets::is_factory(path)).then(|| path.clone()),
                    });
                }
            }
        }
    }
}

pub(super) fn rows(s: &State) -> Vec<Row> {
    if s.browser_tab == RECENT || s.browser_tab >= COLLECTION {
        return saved_rows(s, false);
    }
    let mut out = Vec::new();
    local_plugins(s, &mut out);
    if s.browser_tab == ALL {
        let key = format!("{ALL}:project");
        if group(s, &mut out, key.clone(), "Current project") {
            project_rows(s, &key, 0, &mut out);
        }
        if group(s, &mut out, format!("{ALL}:samples"), "Samples") {
            if s.browser_search.is_empty() {
                out.extend(tree(s).into_iter().map(|mut r| {
                    r.depth = r.depth.saturating_add(1);
                    r
                }));
            } else {
                let mut index = s.browser_index.borrow_mut();
                let mut roots = library_roots(s);
                if let Some(dir) = crate::data_folder::folder(s)
                    && !roots.contains(&dir)
                {
                    roots.push(dir);
                }
                ensure_index(&mut index, roots);
                let q = s.browser_search.as_str().to_lowercase();
                out.extend(
                    index
                        .data
                        .lock()
                        .unwrap()
                        .rows
                        .iter()
                        .filter(|r| matches(s, r, &q))
                        .cloned(),
                );
            }
        }
    }
    if !s.browser_search.is_empty() {
        let q = s.browser_search.as_str().to_lowercase();
        out.retain(|r| matches(s, r, &q));
        for r in &mut out {
            r.depth = 0;
        }
    }
    out
}

fn panel(s: &State, open: bool) -> Rect {
    let content = crate::views::browser_content(s);
    let top = list_top(s);
    let width = if open {
        (content.w * 0.42)
            .clamp(156., 210.)
            .min((content.w - 24.).max(0.))
    } else {
        EDGE
    };
    Rect::new(content.x, top, width, (content.bottom() - top).max(0.))
}

pub fn content_inset(s: &State) -> f32 {
    if s.browser_library.open && crate::views::browser_content(s).w >= 365. {
        panel(s, true).w
    } else {
        0.
    }
}

pub fn leave(app: &mut App) {
    if app.state.browser_library.open {
        app.state.browser_library.open = false;
        app.invalidate();
    }
}

pub fn hover(app: &mut App, x: f32, y: f32) {
    let s = &app.state;
    if s.native_view != Some(crate::app::Win::Browser)
        && s.native_detached.contains(&crate::app::Win::Browser)
    {
        return;
    }
    let available = enabled()
        && s.show_browser
        && !s.browser_auto_hidden
        && matches!(s.native_view, None | Some(crate::app::Win::Browser))
        && !app.modal_open()
        && s.menu_open.is_none()
        && s.playlist.popup.is_none()
        && app.drag.is_none()
        && s.browser_library.edit.is_none();
    let open = available && panel(s, s.browser_library.open).contains(x, y);
    if s.browser_library.open != open {
        app.state.browser_library.open = open;
        app.invalidate();
    }
}

fn sections(s: &State) -> Vec<(usize, Icon, &'static str)> {
    let mut sections = vec![
        (ALL, Icon::Browser, "All"),
        (PLUGINS, Icon::Plug, "Plugins"),
        (EFFECTS, Icon::Speaker, "Effects"),
        (0, Icon::Wave, "Samples"),
        (1, Icon::Page, "Current project"),
    ];
    let tabs = tabs(s);
    if tabs.iter().any(|t| t.0 == 5) {
        sections.push((5, Icon::Download, "Download"));
    }
    if tabs.iter().any(|t| t.0 == 3) {
        sections.push((3, Icon::Star, "Favorites"));
    }
    sections.push((RECENT, Icon::Undo, "Recent"));
    sections.extend(
        tabs.into_iter()
            .filter(|t| matches!(t.0, 2 | 4 | PARAMETERS_TAB)),
    );
    sections
}
fn max_nav_scroll(s: &State) -> f32 {
    let rows = sections(s).len() + 1 + s.browser_library.saved.collections.len();
    (rows as f32 * PITCH + 20. - panel(s, true).h).max(0.)
}

pub fn wheel(app: &mut App, x: f32, y: f32, dy: f32) -> bool {
    if !enabled() || !app.state.browser_library.open || !panel(&app.state, true).contains(x, y) {
        return false;
    }
    app.state.browser_library.scroll =
        (app.state.browser_library.scroll - dy * PITCH).clamp(0., max_nav_scroll(&app.state));
    app.invalidate();
    true
}

fn project_count(s: &State) -> usize {
    let p = &s.project;
    p.patterns.len()
        + p.channels.len()
        + p.mixer
            .iter()
            .map(|m| m.slots.iter().flatten().count())
            .sum::<usize>()
        + p.channels
            .iter()
            .filter(|c| c.sample_path.as_ref().is_some_and(|p| !p.is_empty()))
            .count()
        + p.init_controls.len()
}

pub fn title(s: &State) -> String {
    if let Some(c) = s
        .browser_tab
        .checked_sub(COLLECTION)
        .and_then(|i| s.browser_library.saved.collections.get(i))
    {
        return c.name.clone();
    }
    sections(s)
        .into_iter()
        .find(|t| t.0 == s.browser_tab)
        .map_or("Browser", |t| t.2)
        .into()
}

pub fn paint(ui: &mut Ui, s: &State) {
    let r = panel(s, s.browser_library.open);
    let t = ui.t;
    ui.c.fill_rect(r, t.browser_tabs);
    ui.hit(
        r,
        Action::BrowserLibrary(Control::Edge),
        "Hover to browse categories and collections",
    );
    ui.c.fill_rect(Rect::new(r.right() - 1., r.y, 1., r.h), t.splitter);
    if !s.browser_library.open {
        let y = r.y + (r.h * 0.35).min(115.);
        ui.c.fill_rrect(Rect::new(r.x + 3., y - 11., 2., 22.), 1., t.text_dim);
        icons::draw(
            ui.c,
            Icon::ArrowRight,
            Rect::new(r.x + 4., y - 5., 7., 10.),
            t.text_dim,
        );
        return;
    }
    // Share the background file index with search. Counts never recursively scan on the UI thread.
    let samples = {
        let mut index = s.browser_index.borrow_mut();
        let roots = library_roots(s);
        if !matches!(s.browser_tab, 3 | ALL) || s.browser_search.is_empty() {
            ensure_index(&mut index, roots.clone());
        }
        let data = index.data.lock().unwrap();
        (data.done && index.roots == roots).then_some(data.samples)
    };
    let plugin_count = s.plugdb.lists.gens.len() + s.plugdb.lists.fx.len();
    let mut y = r.y + 4. - s.browser_library.scroll.min(max_nav_scroll(s));
    ui.c.push_clip(r);
    ui.clip_hit_rect(r, |ui| {
        let draw =
            |ui: &mut Ui, y: f32, id: usize, icon: Icon, label: &str, count: Option<usize>| {
                let rect = Rect::new(r.x + 2., y, r.w - 4., PITCH - 1.);
                if rect.bottom() <= r.y || rect.y >= r.bottom() {
                    return;
                }
                let hover = ui.hit(rect, Action::BrowserTab(id), label.to_owned());
                let active = s.browser_tab == id;
                if active || hover {
                    ui.c.fill_rrect(
                        rect,
                        3.,
                        if active {
                            t.accent.shade(-0.4)
                        } else {
                            t.browser_row_hi
                        },
                    );
                }
                icons::draw(
                    ui.c,
                    icon,
                    Rect::new(rect.x + 7., rect.cy() - 8., 16., 16.),
                    if active { t.glyph } else { t.text_dim },
                );
                let count = count.map_or_else(|| "…".into(), |n| format!("({n})"));
                let f = os_ui::Font::new(13);
                let w = ui.c.text_width(f, &count);
                ui.c.text_clip(
                    f,
                    rect.x + 30.,
                    rect.cy() - 8.,
                    (rect.w - 40. - w).max(0.),
                    t.text,
                    label,
                );
                ui.c.text(f, rect.right() - 6. - w, rect.cy() - 8., t.text_dim, &count);
            };
        for (id, icon, label) in sections(s) {
            if id == 3 {
                y += 5.;
                ui.c.fill_rect(Rect::new(r.x + 8., y - 3., r.w - 16., 1.), t.splitter);
            }
            let count = match id {
                ALL => samples.map(|n| n + plugin_count + project_count(s)),
                PLUGINS | 2 => Some(plugin_count),
                EFFECTS => Some(s.plugdb.lists.fx.len()),
                0 => samples,
                1 => Some(project_count(s)),
                5 => Some(
                    crate::plugin_downloads::catalog()
                        .iter()
                        .filter(|p| crate::plugin_downloads::download(p).is_some())
                        .count(),
                ),
                3 => Some(
                    s.browser_library.saved.favorites.len() + crate::browser::favorites().len(),
                ),
                RECENT => Some(s.browser_library.saved.recent.len()),
                4 => Some(crate::presets::factory_plugins().len()),
                PARAMETERS_TAB => Some(s.params.lists.len()),
                _ => None,
            };
            draw(ui, y, id, icon, label, count);
            y += PITCH;
        }
        y += 7.;
        ui.c.fill_rect(Rect::new(r.x + 8., y - 4., r.w - 16., 1.), t.splitter);
        ui.c.text(
            os_ui::Font::new(13).bold(),
            r.x + 9.,
            y + 6.,
            t.text,
            "Collections",
        );
        let plus = Rect::new(r.right() - 31., y, 29., PITCH);
        let hot = ui.hit(
            plus,
            Action::BrowserLibrary(Control::New),
            "Create a collection",
        );
        icons::draw(
            ui.c,
            Icon::Plus,
            plus.inset(7.),
            if hot { t.accent } else { t.text },
        );
        y += PITCH;
        for (i, c) in s.browser_library.saved.collections.iter().enumerate() {
            draw(
                ui,
                y,
                COLLECTION + i,
                Icon::Folder,
                &c.name,
                Some(c.items.len()),
            );
            y += PITCH;
        }
    });
    ui.c.pop_clip();
}

#[cfg(test)]
mod tests {
    use super::*;
    use os_ui::Canvas;
    use winit::keyboard::{Key, NamedKey};

    fn click(app: &mut App, c: &mut Canvas, target: Action) {
        let r = app
            .hits
            .iter()
            .rev()
            .find(|h| h.action == target && !h.rect.is_empty())
            .unwrap_or_else(|| panic!("missing {target:?}"))
            .rect;
        app.mouse_move(r.cx(), r.cy());
        app.paint(c, None);
        assert_eq!(
            app.top_hit(r.cx(), r.cy()).map(|h| &h.action),
            Some(&target)
        );
        app.mouse_down();
        app.mouse_up();
        app.paint(c, None);
    }

    #[test]
    fn library_sidebar_hover_navigation_and_profile() {
        let fixture = tempfile::tempdir().unwrap();
        crate::prefs::set_text("browser_samples_root", &fixture.path().to_string_lossy());
        std::fs::write(fixture.path().join("sample.wav"), []).unwrap();
        let mut app = App::new(None);
        app.state.browser_w = 505.;
        app.plugins = crate::plugins::Plugins::builtins();
        app.state.plugdb.rebuild(&app.plugins);
        let mut c = Canvas::new(1360, 850, 1.).unwrap();
        app.paint(&mut c, None);
        assert!(
            !app.state.browser_library.open
                && app
                    .hits
                    .iter()
                    .all(|h| !matches!(h.action, Action::BrowserTab(_)))
        );
        let top = list_top(&app.state);
        app.mouse_move(5., top + 40.);
        app.paint(&mut c, None);
        assert!(app.state.browser_library.open);
        click(&mut app, &mut c, Action::BrowserTab(PLUGINS));
        let index = crate::views::browser::rows(&app.state)
            .iter()
            .position(|r| r.label == "ML Pure Player")
            .unwrap();
        app.mouse_move(480., top + 7.);
        app.paint(&mut c, None);
        assert!(!app.state.browser_library.open);
        // Select a plugin and expand its preset children through the normal browser action.
        app.browser_act(&Action::BrowserRow(index));
        app.mouse_up();
        app.paint(&mut c, None);
        let rows = crate::views::browser::rows(&app.state);
        let preset = rows
            .iter()
            .position(|r| matches!(r.item, Item::Preset(_)) && r.key.starts_with(&rows[index].key))
            .unwrap();
        let item = rows[preset].item.clone();
        let label = rows[preset].label.clone();
        app.browser_act(&Action::BrowserLibrary(Control::Star(preset)));
        app.browser_act(&Action::BrowserRow(preset));
        app.mouse_up();
        assert!(matches!(&item,Item::Preset(p) if crate::presets::is_factory(Path::new(p))));
        app.mouse_move(5., top + 10.);
        app.paint(&mut c, None);
        click(&mut app, &mut c, Action::BrowserTab(3));
        assert!(
            crate::views::browser::rows(&app.state)
                .iter()
                .any(|r| r.item == item)
        );
        click(&mut app, &mut c, Action::BrowserTab(RECENT));
        assert_eq!(crate::views::browser::rows(&app.state)[0].item, item);
        click(&mut app, &mut c, Action::BrowserLibrary(Control::New));
        assert!(app.browser_key(&Key::Character("Session".into()), false, false));
        assert!(app.browser_key(&Key::Named(NamedKey::Enter), false, false));
        assert_eq!(app.state.browser_tab, COLLECTION);
        app.browser_act(&Action::BrowserTab(3));
        app.browser_row_menu(0, 250., 300.);
        let cmd = app
            .state
            .playlist
            .popup
            .take()
            .unwrap()
            .items
            .into_iter()
            .find(|p| p.label == "Add to collection")
            .unwrap()
            .children[0]
            .cmd
            .clone();
        if let Cmd::Browser(cmd) = cmd {
            app.browser_cmd(cmd);
        } else {
            panic!("collection command");
        }
        app.browser_act(&Action::BrowserTab(COLLECTION));
        assert_eq!(crate::views::browser::rows(&app.state)[0].item, item);
        app.browser_act(&Action::BrowserLibrary(Control::Rename(0)));
        app.browser_key(&Key::Character(" A".into()), false, false);
        app.browser_key(&Key::Named(NamedKey::Enter), false, false);
        assert_eq!(title(&app.state), "Session A");
        app.open_settings(crate::views::settings::GENERAL);
        app.state.settings.folded.insert("Workspace");
        app.paint(&mut c, None);
        let legacy = Action::Settings(crate::views::settings::SAct::Check(
            crate::prefs::LEGACY_BROWSER.label,
        ));
        assert!(!crate::prefs::check(&crate::prefs::LEGACY_BROWSER));
        click(&mut app, &mut c, legacy.clone());
        app.state.settings.open = false;
        app.paint(&mut c, None);
        assert!(app.hits.iter().any(|h| h.action == Action::BrowserTab(0)));
        assert!(
            app.hits
                .iter()
                .all(|h| h.action != Action::BrowserLibrary(Control::Edge))
        );
        app.mouse_move(4., top + 40.);
        assert!(!app.state.browser_library.open);
        let mut legacy_restart = App::new(None);
        legacy_restart.paint(&mut c, None);
        assert!(
            legacy_restart
                .hits
                .iter()
                .any(|h| h.action == Action::BrowserTab(0))
        );
        app.act(legacy);
        app.paint(&mut c, None);
        assert!(
            app.hits
                .iter()
                .all(|h| !matches!(h.action, Action::BrowserTab(_)))
        );
        app.browser_act(&Action::BrowserTab(COLLECTION));
        assert_eq!(crate::views::browser::rows(&app.state)[0].item, item);
        let mut restarted = App::new(None);
        restarted.browser_act(&Action::BrowserTab(COLLECTION));
        assert_eq!(crate::views::browser::rows(&restarted.state)[0].item, item);
        restarted.browser_row_menu(0, 250., 300.);
        restarted.state.playlist.popup = None;
        act(&mut restarted, Control::Remove(0));
        assert!(crate::views::browser::rows(&restarted.state).is_empty());
        remove_collection(&mut restarted, 0);
        assert_eq!(restarted.state.browser_tab, 0);
        app.browser_act(&Action::BrowserTab(ALL));
        app.state.browser_search.set(&label);
        assert!(
            crate::views::browser::rows(&app.state)
                .iter()
                .any(|r| r.item == item)
        );
        app.browser_act(&Action::BrowserTab(PLUGINS));
        let plugin_row = crate::views::browser::rows(&app.state)
            .iter()
            .position(|r| r.label == "ML Pure Player")
            .unwrap();
        app.state.browser_scroll = (plugin_row as f32 - 2.).max(0.) * row_h(&app.state);
        app.mouse_move(4., top + 10.);
        app.paint(&mut c, None);
        if let Some(out) = std::env::var_os("OS_BROWSER_SHOT") {
            c.pixmap
                .save_png(PathBuf::from(out).join("browser-open.png"))
                .unwrap();
        }
        app.mouse_move(490., top + 12.);
        app.paint(&mut c, None);
        if let Some(out) = std::env::var_os("OS_BROWSER_SHOT") {
            c.pixmap
                .save_png(PathBuf::from(out).join("browser-folded.png"))
                .unwrap();
        }
        app.browser_cmd(BCmd::Toggle(OPT_RIGHT));
        app.paint(&mut c, None);
        let edge = panel(&app.state, false);
        app.mouse_move(edge.cx(), edge.y + 20.);
        assert!(app.state.browser_library.open);
        app.state.native_view = Some(crate::app::Win::Browser);
        let mut pane = Canvas::new(420, 310, 1.25).unwrap();
        app.paint(&mut pane, None);
        let r = panel(&app.state, true);
        assert!(wheel(&mut app, r.cx(), r.cy(), -5.));
        assert!(app.state.browser_library.scroll > 0.);
        leave(&mut app);
        assert!(!app.state.browser_library.open);
    }
}
