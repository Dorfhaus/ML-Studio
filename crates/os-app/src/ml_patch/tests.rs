use super::*;
use crate::{
    history, mixer,
    plugin_menu::{self, PlugCmd},
};
fn app() -> (App, usize) {
    let mut a = App::new(None);
    a.state.project = crate::app::test_project();
    crate::playlist::arr_mut(&mut a.state);
    a.plugins.effects = plugins::builtin_effects();
    a.state.plugdb.rebuild(&a.plugins);
    a.state.selected_mixer = 1;
    let i = a.plugins.effect_index(core::NAME).unwrap();
    assert!(a.plugins.effect_ready(i));
    assert!(include_str!("../../assets/native-effects-groups.json").contains("ML Patch"));
    plugin_menu::run(
        &mut a,
        PlugCmd::Effect {
            index: i,
            slot: Some(0),
        },
    );
    a.mixer_tick();
    a.params_tick();
    let key = plugins::fx_key(1, 0);
    assert!(form(&a.state).is_some());
    (a, key)
}
#[test]
fn ml_patch_normal_picker_project_preset_undo_and_visibility() {
    let (mut a, key) = app();
    let balance = a.plugins.effect_index("ML Balance").unwrap();
    command(&mut a, Command::Choose(balance));
    let g = graph(&a, key).unwrap();
    assert_eq!(g.lock().unwrap().doc.nodes.len(), 1);
    let saved = a.plugins.state(key).unwrap();
    history::undo(&mut a.state);
    assert_eq!(g.lock().unwrap().doc.nodes.len(), 0);
    history::redo(&mut a.state);
    assert_eq!(g.lock().unwrap().doc.nodes.len(), 1);
    let mut p = a.state.project.clone();
    crate::project_plugins::store_hosted(&mut p, &a.plugins).unwrap();
    let file = os_flp::project::create(&p).unwrap();
    let p = os_flp::project::load(&file).unwrap();
    let mut b = App::new(None);
    b.plugins.effects = plugins::builtin_effects();
    b.state.project = p;
    b.sync_effects();
    assert_eq!(
        b.plugins.state(key).expect(&format!(
            "reopen: {:?}, slots {:?}, failed {:?}",
            b.state.message,
            b.state
                .project
                .mixer
                .iter()
                .flat_map(|m| m.slots.iter().filter_map(move |s| s.as_ref().map(|s| (
                    m.index,
                    s.name.clone(),
                    s.state.len(),
                    s.state.iter().take(12).copied().collect::<Vec<_>>()
                ))))
                .collect::<Vec<_>>(),
            b.fx_failed
        )),
        saved
    );
    let d = core::descriptor();
    let h = crate::presets::Header {
        plugin: d.name.clone(),
        id: d.uid(),
        format: d.format.as_str().into(),
        body: String::new(),
    };
    let encoded = crate::presets::encode(&h, &saved);
    let (_, blob) = crate::presets::decode(&encoded).unwrap();
    assert_eq!(blob, saved);
    command(&mut a, Command::Choose(balance));
    assert_eq!(g.lock().unwrap().doc.nodes.len(), 2);
    crate::presets::restore_state(&mut a, key, blob).unwrap();
    assert_eq!(g.lock().unwrap().doc.nodes.len(), 1);
    history::undo(&mut a.state);
    assert_eq!(g.lock().unwrap().doc.nodes.len(), 2);
    history::redo(&mut a.state);
    assert_eq!(a.plugins.state(key).unwrap(), saved);
    if let Some(out) = std::env::var_os("ML_PATCH_RENDER_DIR") {
        let path = std::path::PathBuf::from(out).join("ml-patch-roundtrip.lmn");
        a.save_to(path.clone());
        let loaded =
            os_flp::project::load(&os_flp::FlpFile::parse(&std::fs::read(path).unwrap()).unwrap())
                .unwrap();
        assert!(
            loaded
                .mixer
                .iter()
                .any(|m| m.slots.iter().flatten().any(|s| s.name == core::NAME))
        );
    }
    crate::settings_file::set_flag(VISIBILITY, false);
    a.state.plugdb.rebuild(&a.plugins);
    assert!(
        !a.state
            .plugdb
            .lists
            .fx
            .iter()
            .any(|e| e.label == core::NAME)
    );
    assert_eq!(a.plugins.state(key).unwrap(), saved);
    crate::settings_file::set_flag(VISIBILITY, true);
    a.state.plugdb.rebuild(&a.plugins);
    assert_eq!(
        mixer::track(&a.state, 1).unwrap().slots[0]
            .as_ref()
            .unwrap()
            .name,
        core::NAME
    );
}
#[test]
fn ml_patch_map_connections_drag_cancel_and_scaled_render() {
    let (mut a, key) = app();
    let balance = a.plugins.effect_index("ML Balance").unwrap();
    command(&mut a, Command::Choose(balance));
    let g = graph(&a, key).unwrap();
    command(&mut a, Command::Out(2));
    command(&mut a, Command::In(2, false));
    assert!(
        form(&a.state)
            .unwrap()
            .error
            .as_deref()
            .unwrap()
            .contains("Invalid")
    );
    assert_eq!(g.lock().unwrap().doc.edges.len(), 2);
    command(&mut a, Command::Out(0));
    command(&mut a, Command::In(1, false));
    assert_eq!(g.lock().unwrap().doc.edges.len(), 3);
    command(&mut a, Command::Undo);
    assert_eq!(g.lock().unwrap().doc.edges.len(), 2);
    command(&mut a, Command::Redo);
    assert_eq!(g.lock().unwrap().doc.edges.len(), 3);
    let old = g.lock().unwrap().doc.nodes[0].x;
    command(&mut a, Command::Select(2));
    motion(&mut a, 550., 350.);
    super::key(&mut a, &Key::Named(NamedKey::Escape), false, false);
    assert_eq!(g.lock().unwrap().doc.nodes[0].x, old);
    for design in [os_theme::Design::Lemon, os_theme::Design::Citrus] {
        a.set_design(design, false);
        for scale in [1., 1.5, 2.] {
            a.state.screen = (1200., 780.);
            let mut c =
                os_ui::Canvas::new((1200. * scale) as u32, (780. * scale) as u32, scale).unwrap();
            a.paint(&mut c, None);
            assert!(
                a.hits
                    .iter()
                    .any(|h| h.action == Action::Dlg(DHit::Patch(Command::Out(0))))
            );
            if let Some(out) = std::env::var_os("ML_PATCH_RENDER_DIR") {
                c.pixmap
                    .save_png(
                        std::path::PathBuf::from(out)
                            .join(format!("ml-patch-{design:?}-{scale}.png")),
                    )
                    .unwrap();
            }
        }
    }
}
