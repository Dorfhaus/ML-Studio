//! Shared single-line editor. Character indices stay on UTF-8 boundaries.
use winit::keyboard::{Key, NamedKey as N};
use os_ui::{Color, Font, Rect};
use crate::app::{Action, Ui};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Target {
    Browser,
    PickerSearch,
    PickerName,
    PluginManager,
    Performance,
    Visibility,
    MenuName,
    Info(crate::views::settings::InfoField),
    Folder(usize, crate::search_folders::Col),
}

impl Target {
    fn field(self, app: &mut crate::app::App) -> Option<&mut Field> {
        match self {
            Self::Browser => {
                app.state.browser_search_focus = true;
                Some(app.state.browser_field_mut())
            }
            Self::PickerSearch => crate::plugin_picker::focus_search(&mut app.state.plugin_picker),
            Self::PickerName => crate::plugin_picker::focus_name(&mut app.state.plugin_picker),
            Self::PluginManager => {
                app.state.plugdb.finding = true;
                Some(&mut app.state.plugdb.find)
            }
            Self::Performance => {
                app.state.perf.finding = true;
                Some(&mut app.state.perf.find)
            }
            Self::Visibility => {
                app.state.settings.visibility_search_focused = true;
                Some(&mut app.state.settings.visibility_search)
            }
            Self::MenuName => {
                app.state.settings.menu_editor.name_focused = true;
                Some(&mut app.state.settings.menu_editor.name)
            }
            Self::Info(field) => {
                app.info_act(field);
                Some(&mut app.state.settings.info_edit.as_mut()?.text)
            }
            Self::Folder(row, column) => {
                app.search_folder_cell(row, column);
                Some(&mut app.state.settings.folder_edit.as_mut()?.text)
            }
        }
    }

    pub(crate) fn press(self, app: &mut crate::app::App, index: usize) {
        let shift = app.shift;
        if let Some(field) = self.field(app) {
            field.place(index, shift);
            app.drag = Some(crate::app::Drag::Text(self));
            app.invalidate();
        }
    }

    pub(crate) fn drag(self, app: &mut crate::app::App, x: f32) {
        let index = nearest(app, x, |action| match *action {
            Action::TextCaret(target, index) if target == self => Some(index),
            _ => None,
        });
        if let Some(index) = index && let Some(field) = self.field(app) {
            field.place(index, true);
            app.invalidate();
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Field {
    pub text: String,
    pub caret: usize,
    pub all: bool,
    anchor: Option<usize>,
    undo: Vec<(String, usize)>,
    redo: Vec<(String, usize)>,
    scroll: std::cell::Cell<f32>,
}

impl Field {
    pub fn set(&mut self, s: &str) { *self = Self::default(); self.text = s.into(); self.caret = self.len(); }
    pub fn clear(&mut self) { *self = Self::default(); }
    pub fn as_str(&self) -> &str { &self.text }
    pub fn is_empty(&self) -> bool { self.text.is_empty() }
    pub fn len(&self) -> usize { self.text.chars().count() }
    pub fn select_all(&mut self) { self.all = !self.text.is_empty(); self.anchor = None; self.caret = self.len(); }
    pub fn at(&self, i: usize) -> usize { self.text.char_indices().nth(i).map_or(self.text.len(), |(b, _)| b) }
    pub fn before_caret(&self) -> &str { &self.text[..self.at(self.caret)] }
    pub fn selection(&self) -> Option<(usize, usize)> {
        if self.all { return Some((0, self.len())); }
        self.anchor.filter(|&a| a != self.caret).map(|a| (a.min(self.caret), a.max(self.caret)))
    }
    pub fn place(&mut self, pos: usize, extend: bool) {
        if extend { if self.anchor.is_none() { self.anchor = Some(if self.all { 0 } else { self.caret }); } }
        else { self.anchor = None; }
        self.all = false; self.caret = pos.min(self.len());
    }
    fn snapshot(&mut self) {
        if self.undo.len() >= 64 { self.undo.remove(0); }
        self.undo.push((self.text.clone(), self.caret)); self.redo.clear();
    }
    fn remove_selection(&mut self) -> bool {
        let Some((a,b)) = self.selection() else { return false };
        self.text.replace_range(self.at(a)..self.at(b), ""); self.place(a, false); true
    }
    pub fn insert(&mut self, s: &str) {
        if s.is_empty() { return; }
        self.snapshot(); self.remove_selection();
        self.text.insert_str(self.at(self.caret), s);
        self.place(self.caret + s.chars().count(), false);
    }
    fn insert_limited(&mut self, s: &str, limit: usize) {
        let selected = self.selection().map_or(0, |(a, b)| b - a);
        let room = limit.saturating_sub(self.len() - selected);
        let end = s.char_indices().nth(room).map_or(s.len(), |(end, _)| end);
        self.insert(&s[..end]);
    }
    fn word(&self, right: bool) -> usize {
        let chars: Vec<_> = self.text.chars().collect(); let mut i = self.caret.min(chars.len());
        let word = |c: char| c.is_alphanumeric() || c == '_';
        if right {
            if i < chars.len() { let kind = word(chars[i]); while i < chars.len() && !chars[i].is_whitespace() && word(chars[i]) == kind { i += 1; } }
            while i < chars.len() && chars[i].is_whitespace() { i += 1; }
        } else {
            while i > 0 && chars[i-1].is_whitespace() { i -= 1; }
            if i > 0 { let kind = word(chars[i-1]); while i > 0 && !chars[i-1].is_whitespace() && word(chars[i-1]) == kind { i -= 1; } }
        }
        i
    }
    pub fn key(&mut self, key: &Key, ctrl: bool, alt: bool) -> bool { self.key_mod(key, ctrl, alt, false) }
    pub fn key_mod(&mut self, key: &Key, ctrl: bool, alt: bool, shift: bool) -> bool {
        self.key_limited(key, ctrl, alt, shift, 32768)
    }
    pub fn key_multiline(&mut self, key: &Key, ctrl: bool, alt: bool, shift: bool) -> bool {
        self.edit_key(key, ctrl, alt, shift, 32768, true)
    }
    pub fn key_limited(&mut self, key: &Key, ctrl: bool, alt: bool, shift: bool, limit: usize) -> bool {
        self.edit_key(key, ctrl, alt, shift, limit, false)
    }
    fn edit_key(&mut self, key: &Key, ctrl: bool, alt: bool, shift: bool, limit: usize, multiline: bool) -> bool {
        if let Key::Character(c) = key { if ctrl && !alt {
            match c.to_ascii_lowercase().as_str() {
                "a" => self.select_all(),
                "c" | "x" => { if let Some((a,b)) = self.selection() {
                    if clipboard_set(&self.text[self.at(a)..self.at(b)]) && c.eq_ignore_ascii_case("x") { self.snapshot(); self.remove_selection(); }
                } },
                "v" => { if let Some(s) = clipboard_get() {
                    let clean: String = s.chars().filter_map(|c| match c {
                        '\r' => None,
                        '\n' if multiline => Some('\n'),
                        '\n' | '\t' => Some(' '),
                        c if c.is_control() => None,
                        c => Some(c),
                    }).take(limit).collect();
                    self.insert_limited(&clean, limit);
                } },
                "z" | "y" => {
                    let redo = c.eq_ignore_ascii_case("y") || shift;
                    let state = if redo { self.redo.pop() } else { self.undo.pop() };
                    if let Some((text, caret)) = state {
                        let old = (std::mem::replace(&mut self.text, text), self.caret);
                        if redo { self.undo.push(old); } else { self.redo.push(old); }
                        self.place(caret, false);
                    }
                },
                _ => return false,
            }
            return true;
        } }
        match key {
            Key::Named(N::Backspace | N::Delete) => {
                let back = matches!(key, Key::Named(N::Backspace));
                if self.selection().is_some() { self.snapshot(); self.remove_selection(); }
                else {
                    let end = if ctrl && !alt { self.word(!back) } else if back { self.caret.saturating_sub(1) } else { (self.caret + 1).min(self.len()) };
                    if end != self.caret { self.snapshot(); self.anchor = Some(end); self.remove_selection(); }
                }
            },
            Key::Named(N::ArrowLeft | N::ArrowRight | N::Home | N::End) => {
                let pos = match key {
                    Key::Named(N::Home) => 0, Key::Named(N::End) => self.len(),
                    _ => {
                        let right = matches!(key, Key::Named(N::ArrowRight));
                        if !shift && self.selection().is_some() { let (a,b) = self.selection().unwrap(); if right {b} else {a} }
                        else if ctrl && !alt { self.word(right) }
                        else if right { (self.caret+1).min(self.len()) } else { self.caret.saturating_sub(1) }
                    }
                }; self.place(pos, shift);
            },
            Key::Named(N::Space) if !ctrl || alt => self.insert_limited(" ", limit),
            Key::Character(c) if (!ctrl || alt) && !c.chars().any(char::is_control) => self.insert_limited(c, limit),
            _ => return false,
        }
        true
    }
}

impl From<&str> for Field {
    fn from(text: &str) -> Self {
        let mut field = Self::default();
        field.set(text);
        field
    }
}

/// Draw the same selection, caret and horizontal scrolling inside any field's own frame.
pub(crate) fn paint(
    ui: &mut Ui, inner: Rect, field: &Field, focused: bool, font: Font,
    ink: Color, selection: Color, placeholder: &str,
) -> f32 {
    let height = ui.c.line_height(font);
    let y = (inner.cy() - height / 2.0).round();
    let caret = ui.c.text_width(font, field.before_caret());
    let width = (inner.w - 2.0).max(1.0);
    let scroll = if focused {
        let max = (ui.c.text_width(font, &field.text) - width).max(0.0);
        let scroll = field.scroll.get().clamp(0.0, max);
        let scroll = scroll.max(caret - width).min(caret).max(0.0);
        field.scroll.set(scroll);
        scroll
    } else { 0.0 };
    let x = inner.x - scroll;
    if inner.is_empty() { return x; }
    ui.c.push_clip(inner);
    if let Some((a, b)) = field.selection().filter(|_| focused) {
        let left = ui.c.text_width(font, &field.text[..field.at(a)]);
        let right = ui.c.text_width(font, &field.text[..field.at(b)]);
        ui.c.fill_rect(Rect::new(x + left, y, right - left, height), selection);
    }
    let empty = field.is_empty() && !focused;
    ui.c.text(font, x, y, if empty { ui.t.text_faint } else { ink }, if empty { placeholder } else { &field.text });
    if focused {
        ui.c.fill_rect(Rect::new((x + caret).round(), y, 1.0, height), ui.t.glyph);
    }
    ui.c.pop_clip();
    x
}

/// Half-character hit cells pick the nearest text boundary, even after horizontal scrolling.
pub(crate) fn carets(
    ui: &mut Ui, bounds: Rect, field: &Field, font: Font, origin: f32,
    action: impl Fn(usize) -> Action, hint: &'static str,
) {
    if bounds.is_empty() { return; }
    let text = &field.text;
    let bytes: Vec<_> = text.char_indices().map(|(i, _)| i).chain([text.len()]).collect();
    let position = |ui: &mut Ui, index: usize| origin + ui.c.text_width(font, &text[..bytes[index]]);
    let first = bytes.partition_point(|&byte| origin + ui.c.text_width(font, &text[..byte]) < bounds.x);
    let mut left = bounds.x;
    for i in first.saturating_sub(1)..bytes.len() {
        let right = if i + 1 == bytes.len() { bounds.right() } else {
            ((position(ui, i) + position(ui, i + 1)) * 0.5).min(bounds.right())
        };
        if right > left {
            ui.hit(Rect::new(left, bounds.y, right - left, bounds.h), action(i), hint);
            left = right;
        }
        if left >= bounds.right() { break; }
    }
}

pub(crate) fn nearest(app: &crate::app::App, x: f32, index: impl Fn(&Action) -> Option<usize>) -> Option<usize> {
    app.hits.iter().filter_map(|h| {
        let distance = (h.rect.x - x).max(x - h.rect.right()).max(0.0);
        Some((index(&h.action)?, distance))
    })
        .min_by(|a, b| a.1.total_cmp(&b.1)).map(|(index, _)| index)
}

// Keep ownership alive for X11/Wayland as well as Windows/macOS. Tests never touch the user's clipboard.
#[cfg(not(test))]
thread_local! { static CLIPBOARD: std::cell::RefCell<Option<arboard::Clipboard>> = const { std::cell::RefCell::new(None) }; }
#[cfg(not(test))]
fn clipboard_set(s: &str) -> bool { CLIPBOARD.with(|c| { let mut c=c.borrow_mut(); if c.is_none() { *c=arboard::Clipboard::new().ok(); } c.as_mut().is_some_and(|c|c.set_text(s.to_owned()).is_ok()) }) }
#[cfg(not(test))]
fn clipboard_get() -> Option<String> { CLIPBOARD.with(|c| { let mut c=c.borrow_mut(); if c.is_none() { *c=arboard::Clipboard::new().ok(); } c.as_mut()?.get_text().ok() }) }
#[cfg(test)]
thread_local! { static CLIPBOARD: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) }; }
#[cfg(test)]
fn clipboard_set(s: &str) -> bool { CLIPBOARD.with(|c| *c.borrow_mut()=s.into()); true }
#[cfg(test)]
fn clipboard_get() -> Option<String> { Some(CLIPBOARD.with(|c|c.borrow().clone())) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dialog_text_editing_unicode_clipboard_and_history() {
        let mut f=Field::default(); f.set("Café song");
        f.key_mod(&Key::Named(N::ArrowLeft),true,false,true);
        assert_eq!(f.selection(),Some((5,9)));
        f.key(&Key::Character("x".into()),true,false); assert_eq!(f.text,"Café ");
        f.key(&Key::Character("v".into()),true,false); assert_eq!(f.text,"Café song");
        f.key(&Key::Named(N::Space),false,false); f.insert("二");
        assert_eq!(f.text,"Café song 二");
        f.key(&Key::Character("z".into()),true,false); assert_eq!(f.text,"Café song ");
        f.key_mod(&Key::Character("z".into()),true,false,true); assert_eq!(f.text,"Café song 二");
        f.select_all(); clipboard_set("New\r\nname\t2026"); f.key(&Key::Character("v".into()),true,false);
        assert_eq!(f.text,"New name 2026");
        f.key(&Key::Named(N::Backspace),true,false); assert_eq!(f.text,"New name ");
        f.place(0,false); f.place(3,true); f.insert("Best"); assert_eq!(f.text,"Best name ");
        f.key(&Key::Character("@".into()),true,true); assert!(f.text.starts_with("Best@"));
        for _ in 0..100 { f.insert("x"); } assert_eq!(f.undo.len(),64);
    }
}
