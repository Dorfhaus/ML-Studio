//! Shared single-line editor. Character indices stay on UTF-8 boundaries.
use winit::keyboard::{Key, NamedKey as N};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Field {
    pub text: String,
    pub caret: usize,
    pub all: bool,
    anchor: Option<usize>,
    undo: Vec<(String, usize)>,
    redo: Vec<(String, usize)>,
}

impl Field {
    pub fn set(&mut self, s: &str) { *self = Self::default(); self.text = s.into(); self.caret = self.len(); }
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
        if let Key::Character(c) = key { if ctrl && !alt {
            match c.to_ascii_lowercase().as_str() {
                "a" => self.select_all(),
                "c" | "x" => { if let Some((a,b)) = self.selection() {
                    if clipboard_set(&self.text[self.at(a)..self.at(b)]) && c.eq_ignore_ascii_case("x") { self.snapshot(); self.remove_selection(); }
                } },
                "v" => { if let Some(s) = clipboard_get() {
                    let clean:String=s.chars().filter_map(|c|match c {'\r'=>None,'\n'|'\t'=>Some(' '),c if c.is_control()=>None,c=>Some(c)}).collect();
                    self.insert(&clean);
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
            Key::Named(N::Space) if !ctrl || alt => self.insert(" "),
            Key::Character(c) if (!ctrl || alt) && !c.chars().any(char::is_control) => self.insert(c),
            _ => return false,
        }
        true
    }
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
