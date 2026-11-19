//! Regressions reported by the community, November 2026.
use super::*;
use winit::keyboard::{Key, KeyCode, NamedKey};

#[test]
#[ignore = "requires ML_REX_FIXTURES"]
fn community_rex_import_save_reopen_and_undo() {
    let root=PathBuf::from(std::env::var_os("ML_REX_FIXTURES").expect("fixture folder"));
    for name in ["120FourBeats.rx2","120RexTest.rex","120RcyTest.rcy"] {
        let path=root.join(name).to_string_lossy().into_owned();
        let mut app=App::new(None);app.state.project=test_project();
        crate::playlist::arr_mut(&mut app.state);
        let before=app.state.project.clone();
        let i=crate::playlist::import_audio_clip(&mut app.state,&path,0,0).unwrap();
        assert!(crate::playlist::clips(&app.state)[i].length>0);
        assert_eq!(app.state.project.channels.last().unwrap().sample_path.as_deref(),Some(path.as_str()));
        let bytes=os_flp::project::create(&app.state.project).unwrap().to_bytes().unwrap();
        let back=os_flp::project::load(&os_flp::FlpFile::parse(&bytes).unwrap()).unwrap();
        let saved=back.channels.last().unwrap().sample_path.as_ref().unwrap();
        assert_eq!(saved,&path);assert!(os_engine::sample::load_shared(std::path::Path::new(saved)).is_ok());
        assert!(crate::history::undo(&mut app.state).is_some());
        assert_eq!(app.state.project.arrangements,before.arrangements);
        assert_eq!(app.state.project.channels[..before.channels.len()],before.channels);
        // Playlist Undo removes the placement but retains the reusable audio
        // source in the picker, matching the existing WAV import workflow.
        crate::history::redo(&mut app.state);assert_eq!(crate::playlist::clips(&app.state).len(),1);
        crate::history::undo(&mut app.state);
        let before=app.state.project.clone();
        app.sample_to_channel(path.clone(),None);
        assert_eq!(app.state.project.channels[app.state.selected_channel].sample_path.as_deref(),Some(path.as_str()));
        assert!(crate::history::undo(&mut app.state).is_some());assert_eq!(app.state.project,before);
    }
}

#[test]
fn community_rename_selection_and_escape_preserve_parent() {
    let mut app = App::new(None);
    app.state.project = test_project();
    app.state.z = vec![Win::Playlist, Win::Mixer];
    crate::patterns::open_name_box(&mut app.state, crate::patterns::BoxTarget::MixerTrack(1), (100.,100.));
    let b=app.state.pat_ui.name_box.as_mut().unwrap(); b.text="Lead synth".into();
    app.key(&Key::Named(NamedKey::End),false,false);
    app.shift=true;
    for _ in 0..5 {app.key(&Key::Named(NamedKey::ArrowLeft),false,false);}
    app.shift=false;
    assert_eq!(app.state.pat_ui.name_box.as_ref().unwrap().editor.selection(),Some((5,10)));
    app.key(&Key::Character("bass".into()),false,false);
    assert_eq!(app.state.pat_ui.name_box.as_ref().unwrap().text,"Lead bass");
    app.key(&Key::Named(NamedKey::Escape),false,false);
    assert!(app.state.pat_ui.name_box.is_none());
    assert!(app.state.shown(Win::Mixer));
}

#[test]
fn community_typing_piano_keeps_c_and_t_in_both_editors() {
    let mut app=App::new(None);app.state.project=test_project();
    if !app.typing_on(){app.state.toggles.push("typing");}
    for win in [Win::Playlist,Win::PianoRoll] {
        app.state.z=vec![win];
        for code in [KeyCode::KeyC,KeyCode::KeyT] {
            assert!(app.typing_key(code,true,false,false),"{win:?} {code:?}");
            assert!(app.typing_key(code,false,false,false));
        }
    }
}

#[test]
fn community_loop_edges_keep_the_opposite_endpoint() {
    let mut app=App::new(None);app.state.project=test_project();
    let grid=Rect::new(200.,100.,800.,500.);
    let g=crate::playlist::PGeom::new(&app.state,grid);
    app.state.playlist.range=Some((96,384));
    app.mouse=(g.x64(384.) as f32,90.);
    crate::playlist::press_ruler(&mut app,grid,false);
    assert!(matches!(app.drag,Some(Drag::Pl(crate::playlist::PlDrag::TimeSel{t0:96,resizing:true,..}))));
    assert_eq!(app.state.playlist.range,Some((96,384)));
    app.mouse=(g.x64(96.) as f32,90.);
    crate::playlist::press_ruler(&mut app,grid,true);
    assert!(matches!(app.drag,Some(Drag::Pl(crate::playlist::PlDrag::TimeSel{t0:384,resizing:true,..}))));
}

#[test]
fn community_random_color_applies_to_selected_patterns_and_undoes() {
    use os_model::ClipSource;
    let mut app=App::new(None);app.state.project=test_project();
    app.state.project.patterns=vec![os_model::Pattern{number:1,..Default::default()},os_model::Pattern{number:2,..Default::default()},os_model::Pattern{number:3,..Default::default()}];
    app.state.pattern=1;
    app.state.playlist.picker_ui.selected=vec![ClipSource::Pattern(1),ClipSource::Pattern(2)];
    let before=app.state.project.clone();
    crate::patterns::run(&mut app.state,crate::patterns::Cmd::RandomColor,(0.,0.));
    assert!(app.state.project.patterns[..2].iter().all(|p|p.color.is_some()));
    assert_eq!(app.state.project.patterns[2].color,None);
    crate::history::undo(&mut app.state);
    assert_eq!(app.state.project,before);
}

#[test]
fn community_popup_siblings_and_escape_close_one_level() {
    use crate::playlist::{Popup,PItem,Cmd};
    let mut app=App::new(None);
    crate::prefs::set_check(&crate::prefs::HOVER_MENUS,true);
    app.state.playlist.popup=Some(Popup{x:100.,y:100.,items:vec![
        PItem::sub("Folder",vec![PItem::sub("Nested",vec![PItem::new("Leaf",Cmd::Nothing)]),PItem::new("Sibling",Cmd::Nothing)]),
        PItem::new("Other",Cmd::Nothing)],sub:None,scroll:[0;2],cascade:vec![]});
    app.hover=Some(Action::PlPopup(0));assert!(app.update_menu_hover());
    app.hover=Some(Action::PlPopupSub(0));assert!(app.update_menu_hover());
    app.key(&Key::Named(NamedKey::Escape),false,false);
    assert!(app.state.playlist.popup.as_ref().unwrap().cascade.is_empty());
    assert_eq!(app.state.playlist.popup.as_ref().unwrap().sub,Some(0));
    app.hover=Some(Action::PlPopupSub(0));app.update_menu_hover();
    app.hover=Some(Action::PlPopupSub(1));assert!(app.update_menu_hover());
    assert!(app.state.playlist.popup.as_ref().unwrap().cascade.is_empty());
    app.hover=Some(Action::PlPopup(1));assert!(app.update_menu_hover());
    assert_eq!(app.state.playlist.popup.as_ref().unwrap().sub,None);
    app.key(&Key::Named(NamedKey::Escape),false,false);
    assert!(app.state.playlist.popup.is_none());
}

#[test]
fn community_alt_resize_can_make_one_tick_notes() {
    let mut app=App::new(None);app.state.project=test_project();app.state.pattern=1;
    app.piano_grid=Some(Rect::new(100.,100.,800.,400.));
    let note=os_model::Note{length:96,..os_model::Note::step(app.state.project.channels[0].index,0)};
    app.state.project.patterns[0].notes=vec![note];
    app.alt=true;
    let g=app.piano_geom().unwrap();
    app.mouse=(400.-95.*g.ppt,200.);
    app.drag_notes(400.,200.,&[(0,note)],true);
    assert_eq!(app.state.project.patterns[0].notes[0].length,1);
}

#[test]
fn community_inline_rename_supports_selection_and_cancel() {
    let mut app=App::new(None);app.state.project=test_project();app.state.z=vec![Win::Mixer];
    let before=app.state.project.mixer[1].name.clone();
    crate::playlist::start_edit(&mut app.state,crate::playlist::EditTarget::MixerName(1),(100.,100.));
    app.state.playlist.edit.as_mut().unwrap().text="Lead synth".into();
    app.key(&Key::Named(NamedKey::End),false,false);app.shift=true;
    for _ in 0..5 {app.key(&Key::Named(NamedKey::ArrowLeft),false,false);}
    app.shift=false;app.key(&Key::Character("bass".into()),false,false);
    assert_eq!(app.state.playlist.edit.as_ref().unwrap().text,"Lead bass");
    app.key(&Key::Character("z".into()),true,false);
    assert_eq!(app.state.playlist.edit.as_ref().unwrap().text,"Lead synth");
    app.key(&Key::Named(NamedKey::Escape),false,false);
    assert!(app.state.playlist.edit.is_none());assert!(app.state.shown(Win::Mixer));
    assert_eq!(app.state.project.mixer[1].name,before);
}

#[test]
fn community_mixer_random_color_menu_undoes() {
    use crate::mixer::MCmd;
    let mut app=App::new(None);app.state.project=test_project();
    app.state.selected_mixer=1;app.state.mixer.sel=vec![1,2];
    let before=app.state.project.mixer.clone();
    let menu=crate::mixer::track_menu(&app.state,1,0.,0.);
    let cmd=menu.items.iter().find(|x|x.cmd==crate::playlist::Cmd::Mixer(MCmd::RandomColor(1))).unwrap().cmd.clone();
    crate::playlist::run(&mut app,cmd);
    assert!(app.state.project.mixer[1..3].iter().all(|m|m.color.is_some()));
    crate::history::undo(&mut app.state);assert_eq!(app.state.project.mixer,before);
}

#[test]
fn community_rename_mouse_selection_uses_visible_caret_positions() {
    let mut app=App::new(None);app.state.project=test_project();app.state.z=vec![Win::Mixer];
    let mut canvas=Canvas::new(1200,800,1.).unwrap();
    for inline in [true,false] {
        if inline {
            crate::playlist::start_edit(&mut app.state,crate::playlist::EditTarget::MixerName(1),(300.,300.));
            app.state.playlist.edit.as_mut().unwrap().text="Lead synth".into();
        } else {
            crate::patterns::open_name_box(&mut app.state,crate::patterns::BoxTarget::MixerTrack(1),(300.,300.));
            app.state.pat_ui.name_box.as_mut().unwrap().text="Lead synth".into();
        }
        app.paint(&mut canvas,None);
        let action=|i|if inline {Action::InlineText(i)}else{Action::Pat(crate::patterns::Hit::Text(i))};
        let a=app.hits.iter().find(|h|h.action==action(5)).unwrap().rect;
        let b=app.hits.iter().find(|h|h.action==action(10)).unwrap().rect;
        app.mouse_move(a.cx(),a.cy());app.mouse_down();
        app.mouse_move(b.cx(),b.cy());app.mouse_up();
        let editor=if inline {&app.state.playlist.edit.as_ref().unwrap().editor}else{&app.state.pat_ui.name_box.as_ref().unwrap().editor};
        assert_eq!(editor.selection(),Some((5,10)));
        app.key(&Key::Character("bass".into()),false,false);
        let text=if inline {&app.state.playlist.edit.as_ref().unwrap().text}else{&app.state.pat_ui.name_box.as_ref().unwrap().text};
        assert_eq!(text,"Lead bass");
        app.key(&Key::Named(NamedKey::Escape),false,false);
    }
}
