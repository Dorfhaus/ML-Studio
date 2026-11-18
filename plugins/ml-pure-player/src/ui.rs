use crate::*;
use lemon_small_fx::{
    editor::Extension,
    view::{self, Hit, Model, ViewState},
};
use os_ui::{Canvas, Color, Font, Rect};
use std::sync::atomic::AtomicUsize;
const FAMILIES: [&str; 18] = [
    "All sounds",
    "Piano",
    "Chromatic percussion",
    "Organ",
    "Guitar",
    "Bass",
    "Strings",
    "Ensemble / voices",
    "Brass",
    "Reeds",
    "Pipes",
    "Synth leads",
    "Synth pads",
    "Synth effects",
    "World instruments",
    "Percussive",
    "Sound effects",
    "Drum kits",
];
pub(crate) struct Panel {
    params: Arc<Params>,
    scroll: AtomicUsize,
    family: AtomicUsize,
    family_menu: AtomicBool,
    edits: Mutex<Vec<String>>,
}
fn text(c: &mut Canvas, x: f32, y: f32, label: &str) {
    c.text(Font::new(13), x, y, Color::rgb(0xdddddd), label);
}
fn button(c: &mut Canvas, r: Rect, label: &str) {
    os_ui::kit::lemon::plug_panel(c, r);
    text(c, r.x + 10., r.y + 8., label);
}
impl Panel {
    pub fn new(params: Arc<Params>) -> Self {
        Self {
            params,
            scroll: AtomicUsize::new(0),
            family: AtomicUsize::new(0),
            family_menu: AtomicBool::new(false),
            edits: Mutex::default(),
        }
    }
    fn filtered(&self) -> Vec<native::Preset> {
        let family = self.family.load(Ordering::Relaxed);
        self.params
            .shared
            .presets
            .lock()
            .unwrap()
            .iter()
            .filter(|p| {
                family == 0
                    || if family == 17 {
                        p.bank == 128
                    } else {
                        p.bank != 128 && (p.program / 8) as usize == family - 1
                    }
            })
            .cloned()
            .collect()
    }
    fn asset(&self, path: String, context: Option<&Arc<dyn GuiContext>>) {
        if let Some(ctx) = context {
            let mut state = ctx.get_state();
            state.fields.insert(
                "pure_soundfont".into(),
                serde_json::to_string(&Asset { path }).unwrap(),
            );
            ctx.set_state(state);
        } else {
            let before = serde_json::to_string(&self.params.shared.asset()).unwrap();
            self.params.shared.replace(Asset { path });
            let mut edits = self.edits.lock().unwrap();
            if edits.len() >= 128 {
                edits.remove(0);
            }
            edits.push(serde_json::json!({"fields":{"pure_soundfont":before}}).to_string());
        }
        self.scroll.store(0, Ordering::Relaxed);
    }
    fn select(&self, p: &native::Preset, context: Option<&Arc<dyn GuiContext>>) {
        if let Some(ctx) = context {
            use nih_plug::wrapper::state::ParamValue;
            let mut state = ctx.get_state();
            state.params.insert("bank".into(), ParamValue::I32(p.bank));
            state
                .params
                .insert("program".into(), ParamValue::I32(p.program));
            ctx.set_state(state);
        } else {
            for (i, v) in [(0, p.bank), (1, p.program)] {
                self.params.fx.live.set(
                    self.params.fx.p[i].as_ptr(),
                    self.params.fx.p[i].preview_normalized(v),
                );
            }
        }
    }
    fn download(&self) {
        let s = &self.params.shared;
        s.prompt.store(false, Ordering::Release);
        s.cancel.store(false, Ordering::Release);
        s.download.store(true, Ordering::Release);
    }
    fn load(&self, context: Option<&Arc<dyn GuiContext>>) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("SoundFont 2", &["sf2"])
            .pick_file()
        {
            self.params.shared.prompt.store(false, Ordering::Release);
            self.asset(path.to_string_lossy().into_owned(), context);
        }
    }
    fn prompt(&self) -> bool {
        self.params.shared.prompt.load(Ordering::Acquire)
            && !self.params.shared.downloading.load(Ordering::Acquire)
    }
    fn draw(&self, c: &mut Canvas) {
        let s = &self.params.shared;
        if !os_plugkit::visibility::stock_dsp_controls() {
            return;
        }
        let bank = self.params.fx.v(0);
        let program = self.params.fx.v(1);
        let name = s
            .presets
            .lock()
            .unwrap()
            .iter()
            .find(|p| p.bank == bank && p.program == program)
            .map(|p| p.name.clone())
            .unwrap_or("Choose an instrument below".into());
        text(c, 352., 34., &name.chars().take(48).collect::<String>());
        text(
            c,
            352.,
            59.,
            &format!("Bank {bank} / Program {}", program + 1),
        );
        let asset = s.asset();
        let name = if asset.path.is_empty() {
            "GeneralUser GS 2.0.3".to_string()
        } else {
            PathBuf::from(asset.path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        };
        text(c, 352., 84., &name.chars().take(48).collect::<String>());
        for (i, label) in [
            "Download / repair library",
            "Load .sf2...",
            "Library folder...",
            "Audition",
        ]
        .iter()
        .enumerate()
        {
            button(c, Rect::new(16. + i as f32 * 190., 134., 178., 31.), label);
        }
        let status = s
            .status
            .lock()
            .unwrap()
            .chars()
            .take(110)
            .collect::<String>();
        text(c, 16., 177., &status);
        if s.downloading.load(Ordering::Acquire) {
            let done = s.progress.load(Ordering::Relaxed);
            let fraction = done as f32 / library::SIZE as f32;
            os_ui::kit::lemon::plug_well(c, Rect::new(16., 204., 550., 18.), 4.);
            os_ui::kit::lemon::plug_panel(
                c,
                Rect::new(16., 204., 550. * fraction.clamp(0., 1.), 18.),
            );
            text(
                c,
                16.,
                232.,
                &format!(
                    "{:.1} / {:.1} MB ({:.0}%)",
                    done as f64 / 1e6,
                    library::SIZE as f64 / 1e6,
                    fraction * 100.
                ),
            );
            button(c, Rect::new(594., 201., 168., 31.), "Cancel download");
        } else {
            let family = self.family.load(Ordering::Relaxed);
            button(
                c,
                Rect::new(16., 207., 310., 29.),
                &format!("Family: {}  v", FAMILIES[family]),
            );
            button(c, Rect::new(560., 207., 90., 29.), "Previous");
            button(c, Rect::new(662., 207., 100., 29.), "Next");
            let presets = self.filtered();
            let offset = self
                .scroll
                .load(Ordering::Relaxed)
                .min(presets.len().saturating_sub(1));
            for (i, preset) in presets.iter().skip(offset).take(12).enumerate() {
                let x = 16. + (i / 6) as f32 * 380.;
                let y = 249. + (i % 6) as f32 * 33.;
                let label = format!(
                    "{:03} / {}",
                    preset.program + 1,
                    preset.name.chars().take(36).collect::<String>()
                );
                button(c, Rect::new(x, y, 368., 29.), &label);
                if bank == preset.bank && program == preset.program {
                    c.text(Font::new(13), x + 344., y + 8., Color::rgb(0x24b7ff), "*");
                }
            }
            if self.family_menu.load(Ordering::Relaxed) {
                os_ui::kit::lemon::plug_panel(c, Rect::new(16., 244., 748., 207.));
                for (i, name) in FAMILIES.iter().enumerate() {
                    button(
                        c,
                        Rect::new(
                            24. + (i / 6) as f32 * 246.,
                            250. + (i % 6) as f32 * 32.,
                            238.,
                            29.,
                        ),
                        name,
                    );
                }
            }
            text(
                c,
                16.,
                456.,
                &format!("{} sounds / scroll or use Previous and Next", presets.len()),
            );
        }
        text(
            c,
            16.,
            482.,
            "FluidSynth / GeneralUser GS by S. Christian Collins",
        );
    }
}
impl Extension for Panel {
    fn take_state_edits(&self) -> Vec<String> {
        std::mem::take(&mut *self.edits.lock().unwrap())
    }
    fn render(
        &self,
        c: &mut Canvas,
        model: &dyn Model,
        state: &ViewState,
        hint: &str,
    ) -> Option<Vec<Hit>> {
        let hits = view::paint(
            &spec::SPEC,
            c,
            &os_theme::Palette::default(),
            model,
            state,
            hint,
        );
        self.draw(c);
        if self.prompt() && os_plugkit::visibility::stock_dsp_controls() {
            os_ui::kit::lemon::plug_panel(c, Rect::new(62., 164., 656., 230.));
            text(
                c,
                88.,
                188.,
                "Download the sound library for ML Pure Player?",
            );
            text(
                c,
                88.,
                223.,
                "Piano, bass, strings, guitars, brass, synths and drums.",
            );
            text(
                c,
                88.,
                248.,
                "GeneralUser GS / 32.3 MB / direct from the author's GitHub.",
            );
            text(
                c,
                88.,
                273.,
                "Optional. Once installed, it works offline and survives updates.",
            );
            button(c, Rect::new(88., 311., 190., 36.), "Download library");
            button(c, Rect::new(292., 311., 190., 36.), "Load my own SF2...");
            button(c, Rect::new(496., 311., 190., 36.), "Not now");
        }
        Some(hits)
    }
    fn paint(&self, _: &mut Canvas, _: &dyn Model) {}
    fn accepts_files(&self) -> bool {
        true
    }
    fn file_dropped(&self, path: &std::path::Path) {
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("sf2"))
        {
            self.params.shared.prompt.store(false, Ordering::Release);
            self.asset(path.to_string_lossy().into_owned(), None);
        }
    }
    fn wheel(&self, x: f32, y: f32, notches: f32) -> bool {
        if os_plugkit::visibility::stock_dsp_controls()
            && Rect::new(16., 244., 748., 201.).contains(x, y)
            && !self.prompt()
        {
            let len = self.filtered().len();
            let old = self.scroll.load(Ordering::Relaxed) as i32;
            self.scroll.store(
                (old - (notches * 3.).round() as i32).clamp(0, len.saturating_sub(12) as i32)
                    as usize,
                Ordering::Relaxed,
            );
            true
        } else {
            false
        }
    }
    fn click(&self, x: f32, y: f32, context: Option<&Arc<dyn GuiContext>>) -> bool {
        if !os_plugkit::visibility::stock_dsp_controls() {
            return false;
        }
        let s = &self.params.shared;
        if self.prompt() {
            if Rect::new(88., 311., 190., 36.).contains(x, y) {
                self.download();
            } else if Rect::new(292., 311., 190., 36.).contains(x, y) {
                self.load(context);
            } else if Rect::new(496., 311., 190., 36.).contains(x, y) {
                s.prompt.store(false, Ordering::Release);
            }
            return true;
        }
        if self.family_menu.load(Ordering::Relaxed) {
            for i in 0..FAMILIES.len() {
                if Rect::new(
                    24. + (i / 6) as f32 * 246.,
                    250. + (i % 6) as f32 * 32.,
                    238.,
                    29.,
                )
                .contains(x, y)
                {
                    self.family.store(i, Ordering::Relaxed);
                    self.scroll.store(0, Ordering::Relaxed);
                    break;
                }
            }
            self.family_menu.store(false, Ordering::Relaxed);
            return true;
        }
        for i in 0..4 {
            if Rect::new(16. + i as f32 * 190., 134., 178., 31.).contains(x, y) {
                match i {
                    0 => {
                        if !s.downloading.load(Ordering::Acquire) {
                            self.download()
                        }
                    }
                    1 => self.load(context),
                    2 => {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_directory(library::folder())
                            .pick_folder()
                        {
                            match library::remember_folder(&path) {
                                Ok(()) => {
                                    s.replace(s.asset());
                                    s.message(format!("Library folder: {}", path.display()));
                                }
                                Err(e) => s.message(e),
                            }
                        }
                    }
                    _ => {
                        s.preview.fetch_add(1, Ordering::Release);
                    }
                }
                return true;
            }
        }
        if s.downloading.load(Ordering::Acquire) {
            if Rect::new(594., 201., 168., 31.).contains(x, y) {
                s.cancel.store(true, Ordering::Release);
                return true;
            }
            return false;
        }
        if Rect::new(16., 207., 310., 29.).contains(x, y) {
            self.family_menu.store(true, Ordering::Relaxed);
            return true;
        }
        let presets = self.filtered();
        if Rect::new(560., 207., 202., 29.).contains(x, y) {
            let old = self.scroll.load(Ordering::Relaxed);
            self.scroll.store(
                if x < 654. {
                    old.saturating_sub(12)
                } else {
                    (old + 12).min(presets.len().saturating_sub(12))
                },
                Ordering::Relaxed,
            );
            return true;
        }
        let offset = self.scroll.load(Ordering::Relaxed);
        for (i, preset) in presets.iter().skip(offset).take(12).enumerate() {
            if Rect::new(
                16. + (i / 6) as f32 * 380.,
                249. + (i % 6) as f32 * 33.,
                368.,
                29.,
            )
            .contains(x, y)
            {
                self.select(preset, context);
                return true;
            }
        }
        false
    }
}
