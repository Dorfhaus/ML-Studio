// SPDX-License-Identifier: GPL-3.0-or-later
//! Per-profile page dividers shared by mixer instances of the same effect.
use crate::app::{Action, State, Ui};
use crate::rack_mixer::Command;
use os_ui::{Font, Rect};

pub const VISIBILITY: &str = "visible_mixer-custom-strip-pages";
const END: u16 = 10_000;
const MIN_SPAN: u16 = 200;
const MAX_DIVIDERS: usize = 7;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Cuts(Vec<u16>);

impl Cuts {
    fn valid(&self) -> bool {
        self.0.len() <= MAX_DIVIDERS
            && self.points().zip(self.points().skip(1))
                .all(|(left, right)| right.checked_sub(left).is_some_and(|width| width >= MIN_SPAN))
    }

    fn points(&self) -> impl Iterator<Item = u16> + '_ {
        std::iter::once(0).chain(self.0.iter().copied()).chain([END])
    }

    fn add(&mut self) -> Option<usize> {
        if self.0.len() == MAX_DIVIDERS { return None; }
        let (index, (left, right)) = self.points().zip(self.points().skip(1))
            .enumerate().max_by_key(|(_, (left, right))| right - left)?;
        if right - left < 2 * MIN_SPAN { return None; }
        self.0.insert(index, left + (right - left) / 2);
        Some(index)
    }

    fn move_to(&mut self, index: usize, position: u16) {
        if index >= self.0.len() { return; }
        let left = index.checked_sub(1).map_or(0, |i| self.0[i]);
        let right = self.0.get(index + 1).copied().unwrap_or(END);
        self.0[index] = position.clamp(left + MIN_SPAN, right - MIN_SPAN);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Preview {
    Full,
    Automatic,
    Custom(Cuts),
}

#[derive(Clone)]
pub(crate) struct Edit {
    pub track: u16,
    pub slot: u8,
    key: String,
    name: String,
    aspect: f32,
    defaults: Cuts,
    cuts: Cuts,
    automatic: bool,
    selected: Option<usize>,
    preview: bool,
    pub span: (f32, f32),
}

impl Edit {
    #[cfg(any(windows, target_os = "macos"))]
    pub fn belongs_to(&self, desc: &os_host::Descriptor) -> bool {
        self.key == native::preference_key(desc)
    }

    pub fn same_shape(&self, size: (u32, u32)) -> bool {
        size.1 > 0 && matching(self.aspect, size.0 as f32 / size.1 as f32)
    }

    pub fn view(&self) -> Preview {
        if !self.preview { return Preview::Full; }
        if self.automatic { Preview::Automatic } else { Preview::Custom(self.cuts.clone()) }
    }

    pub fn add(&mut self) {
        if let Some(index) = self.cuts.add() {
            self.selected = Some(index);
            self.preview = false;
            self.automatic = false;
        }
    }

    pub fn remove(&mut self) {
        if let Some(index) = self.selected.or_else(|| self.cuts.0.len().checked_sub(1)) {
            self.cuts.0.remove(index);
            self.selected = index.checked_sub(1);
            self.preview = false;
            self.automatic = false;
        }
    }

    pub fn reset(&mut self) {
        self.cuts = self.defaults.clone();
        self.automatic = true;
        self.selected = None;
    }

    pub fn toggle_preview(&mut self) { self.preview = !self.preview; }

    pub fn select(&mut self, index: usize) -> Option<f32> {
        let point = *self.cuts.0.get(index)?;
        self.selected = Some(index);
        Some(f32::from(point) / f32::from(END))
    }

    pub fn drag(&mut self, index: usize, fraction: f32) {
        if !self.preview && index < self.cuts.0.len() {
            self.cuts.move_to(index, (fraction.clamp(0., 1.) * END as f32).round() as u16);
            self.automatic = false;
        }
    }
}

/// A wrapper may switch to a different editor, for example a setup page or the
/// embedded effect. Keep one saved layout for each editor shape.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Saved {
    aspect: f32,
    cuts: Cuts,
}

fn matching(saved: f32, current: f32) -> bool {
    ((saved - current) / current).abs() <= 0.035
}

fn stored(key: &str) -> Vec<Saved> {
    let mut layouts: Vec<Saved> = crate::prefs::text(key)
        .and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
    layouts.retain(|layout| layout.aspect.is_finite() && layout.aspect > 0. && layout.cuts.valid());
    layouts
}

pub(crate) fn save(edit: Edit) {
    let mut layouts = stored(&edit.key);
    layouts.retain(|layout| !matching(layout.aspect, edit.aspect));
    if !edit.automatic {
        layouts.push(Saved { aspect: edit.aspect, cuts: edit.cuts });
    }
    let text = if layouts.is_empty() { String::new() } else { serde_json::to_string(&layouts).unwrap() };
    crate::prefs::set_text(&edit.key, &text);
    #[cfg(any(windows, target_os = "macos"))]
    native::changed();
}

pub(crate) fn active(state: &State) -> Option<&Edit> {
    state.mixer.strip_edit.as_ref().filter(|edit| {
        crate::workspace_visibility::shown(VISIBILITY) && crate::mixer::view(state) == crate::rack_mixer::VIEW
            && crate::rack_mixer::slot(state, edit.track) == Some(edit.slot)
    })
}

pub(crate) fn ruler(ui: &mut Ui, edit: &Edit, pane: Rect) -> bool {
    if edit.preview { return false; }
    let (offset, width) = edit.span;
    if width <= 0. { return true; }
    let r = Rect::new(pane.x + offset, pane.bottom() + 2., width, 14.);
    ui.c.fill_rect(r, ui.t.button);
    let at = |cut: u16| r.x + r.w * f32::from(cut) / f32::from(END);
    for (page, (left, right)) in edit.cuts.points().zip(edit.cuts.points().skip(1)).enumerate() {
        ui.c.text_center(Font::new(10), Rect::new(at(left), r.y, at(right) - at(left), r.h),
            ui.t.text, &(page + 1).to_string());
    }
    for (index, &point) in edit.cuts.0.iter().enumerate() {
        let handle = Rect::new(at(point) - 5., r.y, 10., r.h);
        let hover = ui.hit(handle, Action::RackStrip(Command::Divider(index, r.w)), "Drag to resize the adjacent pages");
        let color = if hover || edit.selected == Some(index) { ui.t.accent } else { ui.t.text };
        ui.c.fill_rect(Rect::new(handle.cx() - 1., r.y + 1., 2., r.h - 2.), color);
    }
    true
}

pub(crate) fn inspector(ui: &mut Ui, state: &State, rect: Rect) -> bool {
    let Some(edit) = active(state) else { return false; };
    ui.c.fill_rect(rect, ui.t.s(0x292f32));
    let panel = rect.inset(10.);
    ui.c.push_clip(rect);
    let label = |ui: &mut Ui, y: f32, text: &str, size: u16| {
        let font = crate::views::mixer::label_font(ui.c, Font::new(size), text, panel.w);
        ui.c.text_center(font, Rect::new(panel.x, panel.y + y, panel.w, 22.), ui.t.text, text);
    };
    label(ui, 0., "Edit plug-in pages", 14);
    label(ui, 24., &edit.name, 12);
    label(ui, 49., &format!("{} page(s) • Mixer insert {}", edit.cuts.0.len() + 1, edit.track), 11);
    label(ui, 73., if edit.preview { "Use the arrows below the editor." } else { "Drag dividers below the editor." }, 11);
    let mut button = |y, text: &'static str, action: Command, enabled| {
        let r = Rect::new(panel.x, panel.y + y, panel.w, 23.);
        let hover = enabled && ui.hit(r, Action::RackStrip(action), text);
        ui.c.fill_rrect(r, 3., if hover { ui.t.button_hi } else { ui.t.button });
        ui.c.text_center(Font::new(12), r, if enabled { ui.t.text } else { ui.t.text_dim }, text);
    };
    button(103., "Add page", Command::AddPage, edit.cuts.0.len() < MAX_DIVIDERS);
    button(130., "Remove divider", Command::RemoveDivider, !edit.cuts.0.is_empty());
    button(157., if edit.preview { "Show full editor" } else { "Preview pages" }, Command::PreviewPages, true);
    button(184., "Reset to automatic", Command::ResetPages, true);
    button(220., "Save layout", Command::SavePages, true);
    button(247., "Cancel", Command::CancelPages, true);
    ui.c.pop_clip();
    true
}

pub(crate) fn cancel(state: &mut State) -> bool {
    state.mixer.strip_edit.take().is_some()
}

#[cfg(any(windows, target_os = "macos"))]
pub(super) mod native {
    use super::*;
    use crate::strip_geometry::{Page, Profile, pages};
    use sha2::{Digest, Sha256};
    use std::sync::atomic::{AtomicU64, Ordering};

    static REVISION: AtomicU64 = AtomicU64::new(0);
    pub(super) fn changed() { REVISION.fetch_add(1, Ordering::Relaxed); }

    pub(crate) fn preference_key(desc: &os_host::Descriptor) -> String {
        format!("strip_pages_{:x}", Sha256::digest(desc.uid().as_bytes()))
    }

    #[derive(Default)]
    pub(crate) struct Paging {
        descriptor: Option<(String, String, Profile)>,
        revision: u64,
        saved: Vec<Saved>,
        preview: Option<Preview>,
        applied: Option<(Profile, (u32, u32), Option<Preview>, bool, u64)>,
    }

    fn aspect(size: (u32, u32)) -> f32 { size.0 as f32 / size.1 as f32 }

    impl Paging {
        pub fn assign(&mut self, desc: &os_host::Descriptor) {
            let key = preference_key(desc);
            self.saved = stored(&key);
            self.revision = REVISION.load(Ordering::Relaxed);
            self.descriptor = Some((key, desc.name.clone(), crate::strip_geometry::profile(desc)));
        }

        pub fn preview(&mut self, preview: Option<&Preview>) {
            if self.preview.as_ref() != preview { self.preview = preview.cloned(); }
        }

        fn saved_for(&self, size: (u32, u32)) -> Option<&Cuts> {
            self.saved.iter().rev().find(|saved| matching(saved.aspect, aspect(size))).map(|saved| &saved.cuts)
        }

        pub fn update(&mut self, size: (u32, u32)) -> Option<Vec<Page>> {
            let (key, _, profile) = self.descriptor.as_ref()?;
            let revision = REVISION.load(Ordering::Relaxed);
            let enabled = crate::workspace_visibility::shown(VISIBILITY);
            let profile = profile.visible();
            let preview = self.preview.as_ref().filter(|_| enabled);
            let options = (profile, size, preview.cloned(), enabled, revision);
            if self.applied.as_ref() == Some(&options) { return None; }
            self.applied = Some(options);
            if revision != self.revision {
                self.saved = stored(key);
                self.revision = revision;
            }
            Some(match preview {
                Some(Preview::Full) => pages(Profile::Whole, size),
                Some(Preview::Custom(cuts)) => split(cuts, size.0),
                Some(Preview::Automatic) => pages(profile, size),
                None => self.saved_for(size).filter(|_| enabled).map_or_else(|| pages(profile, size), |cuts| split(cuts, size.0)),
            })
        }

        pub fn edit(&self, track: u16, slot: u8, size: (u32, u32)) -> Option<Edit> {
            let (key, name, profile) = self.descriptor.as_ref()?;
            let mut defaults = Cuts(pages(profile.visible(), size).into_iter()
                .map(|page| (page.right / size.0 as f32 * END as f32).round() as u16)
                .filter(|&point| point < END).collect());
            defaults.0.sort_unstable();
            defaults.0.dedup();
            if !defaults.valid() { defaults = Cuts::default(); }
            let saved = stored(key);
            let custom = saved.iter().rev().find(|layout| matching(layout.aspect, aspect(size)));
            Some(Edit {
                track, slot, key: key.clone(), name: name.clone(), aspect: aspect(size),
                cuts: custom.map_or_else(|| defaults.clone(), |layout| layout.cuts.clone()),
                defaults, automatic: custom.is_none(), selected: None, preview: false, span: (0., 0.),
            })
        }
    }

    fn split(cuts: &Cuts, width: u32) -> Vec<Page> {
        let x = |position| f32::from(position) / f32::from(END) * width as f32;
        cuts.points().zip(cuts.points().skip(1))
            .map(|(left, right)| Page { left: x(left), right: x(right) }).collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn strip_layout_saves_by_effect_and_shape_across_editor_reopens() {
            let mut desc = crate::plugins::builtin_effects()[0].desc.clone();
            desc.id = "custom-layout-test".into();
            let mut paging = Paging::default();
            paging.assign(&desc);
            let regular = (1200, 600);
            let expanded = (1760, 600);
            assert_eq!(paging.update(regular).unwrap().len(), 1);
            let mut edit = paging.edit(1, 0, regular).unwrap();
            edit.add();
            edit.add();
            edit.drag(0, 0.3);
            edit.drag(1, 0.67);
            paging.preview(Some(&edit.view()));
            assert_eq!(paging.update(regular).unwrap().len(), 1);
            edit.toggle_preview();
            paging.preview(Some(&edit.view()));
            let expected = [Page {left: 0., right: 360.}, Page {left: 360., right: 804.}, Page {left: 804., right: 1200.}];
            let actual = paging.update(regular).unwrap();
            for (page, expected) in actual.iter().zip(expected) {
                assert!((page.left - expected.left).abs() < 0.001 && (page.right - expected.right).abs() < 0.001);
            }
            assert_eq!(actual.len(), 3);
            save(edit);
            paging.preview(None);
            assert_eq!(paging.update(regular).unwrap(), actual);

            let mut reopened = Paging::default();
            // The format and ID remain stable across installation path and version changes.
            desc.path.push("Moved");
            desc.version.push_str(".1");
            reopened.assign(&desc);
            let zoomed = reopened.update((2400, 1200)).unwrap();
            assert_eq!(zoomed, actual.iter().map(|page| Page { left: page.left * 2., right: page.right * 2. }).collect::<Vec<_>>());
            assert_eq!(reopened.update(expanded).unwrap().len(), 1);
            let mut other_shape = reopened.edit(2, 1, expanded).unwrap();
            other_shape.add();
            save(other_shape);
            assert_eq!(reopened.update(expanded).unwrap().len(), 2);

            let key = VISIBILITY;
            crate::settings_file::set_flag(key, false);
            assert_eq!(reopened.update(regular).unwrap().len(), 1);
            crate::settings_file::set_flag(key, true);
            assert_eq!(reopened.update(regular).unwrap().len(), 3);
            let mut clear = reopened.edit(1, 0, regular).unwrap();
            clear.reset();
            save(clear);
            assert_eq!(reopened.update(regular).unwrap().len(), 1);
            assert_eq!(reopened.update(expanded).unwrap().len(), 2);
        }

        #[test]
        fn strip_layout_rejects_corrupt_dividers_and_clamps_drag() {
            for cuts in [Cuts(vec![65535]), Cuts(vec![5000, 4999]), Cuts(vec![1]), Cuts(vec![END])] {
                assert!(!cuts.valid());
            }
            let mut cuts = Cuts(vec![3000, 6700]);
            cuts.move_to(0, 8000);
            assert_eq!(cuts.0, [6500, 6700]);
            cuts.move_to(0, 0);
            assert_eq!(cuts.0, [MIN_SPAN, 6700]);
            assert!(cuts.valid());
        }
    }
}
