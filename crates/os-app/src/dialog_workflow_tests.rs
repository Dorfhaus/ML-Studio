use super::*;
use crate::app::Action;
use crate::project_forms::{FHit, Form};
use os_ui::Canvas;
use winit::keyboard::KeyCode;

fn key(app: &mut App, k: Key, ctrl: bool) { app.key(&k,ctrl,false); }
fn shortcut(app: &mut App, s: &str) { key(app,Key::Character(s.into()),true); }
fn named(app: &mut App, n: NamedKey) { key(app,Key::Named(n),false); }
fn typed(app: &mut App, s: &str) { key(app,Key::Character(s.into()),false); }
fn hit(app: &App, action: Action) -> Rect { app.hits.iter().rev().find(|h|h.action==action).expect("visible hit").rect }
fn click(app: &mut App, r: Rect) { app.mouse_move(r.cx(),r.cy());app.mouse_down();app.mouse_up(); }
fn save(app: &App) -> &crate::save_form::SaveForm {
    match &app.state.dialogs.last().unwrap().body { Body::Project(Form::Save(f))=>f,_=>panic!("save form") }
}

#[test]
fn language_files_save_name_clipboard_mouse_selection_and_reopen() {
    let dir=std::env::temp_dir().join(format!("lemon_text_save_{}",std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut app=App::new(None);app.state.project=crate::app::test_project();
    app.open_save_form(None);
    if let Body::Project(Form::Save(f))=&mut app.state.dialogs.last_mut().unwrap().body { f.root=dir.clone();f.new_folder=false;f.copy=false;f.files=2; }
    assert!(!app.typing_key(KeyCode::Space,true,false,false));
    typed(&mut app,"Café");named(&mut app,NamedKey::Space);typed(&mut app,"Song");
    assert_eq!(save(&app).name.text,"Café Song");
    shortcut(&mut app,"a");shortcut(&mut app,"c");named(&mut app,NamedKey::End);named(&mut app,NamedKey::Space);shortcut(&mut app,"v");
    assert_eq!(save(&app).name.text,"Café Song Café Song");
    shortcut(&mut app,"z");shortcut(&mut app,"z");assert_eq!(save(&app).name.text,"Café Song");
    shortcut(&mut app,"y");assert_eq!(save(&app).name.text,"Café Song ");shortcut(&mut app,"z");
    let mut canvas=Canvas::new(1600,1000,1.25).unwrap();app.paint(&mut canvas,None);
    let a=hit(&app,Action::Dlg(DHit::TextCaret(TextTarget::Project(0),0)));
    let b=hit(&app,Action::Dlg(DHit::TextCaret(TextTarget::Project(0),4)));
    app.mouse_move(a.cx(),a.cy());app.mouse_down();app.mouse_move(b.cx(),b.cy());app.mouse_up();
    assert_eq!(save(&app).name.selection(),Some((0,4)));
    typed(&mut app,"Best");assert_eq!(save(&app).name.text,"Best Song");
    app.dialog_click(DHit::Form(FHit::Fold));app.shift=true;named(&mut app,NamedKey::Tab);app.shift=false;
    assert_eq!(save(&app).focus,4);typed(&mut app,"Electronic");
    app.dialog_click(DHit::Form(FHit::Main));assert!(!app.modal_open());
    let path=dir.join("Best Song.lmn");assert_eq!(app.state.path.as_ref(),Some(&path));assert!(path.is_file());
    let reopened=App::new(Some(path));assert_eq!(reopened.state.project.info.genre.as_deref(),Some("Electronic"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn language_files_open_path_keyboard_and_translated_forms() {
    let dir=std::env::temp_dir().join(format!("lemon_text_open_{}",std::process::id()));
    std::fs::create_dir_all(dir.join("My Samples")).unwrap();
    std::fs::write(dir.join("My Samples").join("Save.wav"),b"fixture").unwrap();
    fake::live(true);
    for language in 0..6 {
        crate::prefs::set_choice(&crate::prefs::INTERFACE_LANGUAGE,language);
        for scale in [1.,1.5] {
            let mut app=App::new(None);
            let mut canvas=Canvas::new((1280.*scale) as u32,(800.*scale) as u32,scale).unwrap();
            app.file_dialog(FileDlg::new(crate::filedlg::Mode::Open,"Open","language-test",Then::Nothing).start(Some(&dir)));
            shortcut(&mut app,"l");typed(&mut app,&dir.join("My Samples").to_string_lossy());named(&mut app,NamedKey::Enter);
            assert_eq!(app.top_file().unwrap().dir,dir.join("My Samples"));
            if language==0 && scale==1. {
                app.shift=true;shortcut(&mut app,"n");app.shift=false;
                assert!(dir.join("My Samples").join("New folder").is_dir());
            }
            assert!(app.top_file().unwrap().entries.iter().any(|e|e.name=="Save.wav"));
            app.dialog_click(DHit::NameField);typed(&mut app,"My");named(&mut app,NamedKey::Space);typed(&mut app,"Song.wav");
            assert_eq!(app.top_file().unwrap().name.text,"My Song.wav");
            app.shift=true;named(&mut app,NamedKey::ArrowLeft);app.shift=false;shortcut(&mut app,"x");
            assert_eq!(app.top_file().unwrap().name.text,"My Song.wa");shortcut(&mut app,"v");
            app.paint(&mut canvas,None);
            let name=hit(&app,Action::Dlg(DHit::NameField));let path=hit(&app,Action::Dlg(DHit::PathField));
            assert!(name.w>100. && path.w>200.);
            for action in [DHit::Accept,DHit::Cancel] { let b=hit(&app,Action::Dlg(action));assert!(b.x>=0. && b.right()<=app.state.screen.0); }
            if language==3 && scale==1.5 { if let Ok(out)=std::env::var("LEMON_LANGUAGE_PREVIEW") { canvas.pixmap.save_png(std::path::Path::new(&out).join("German-Open.png")).unwrap(); } }
            let before=app.state.project.clone();named(&mut app,NamedKey::Escape);assert_eq!(app.state.project,before);
            app.open_save_form(None);app.paint(&mut canvas,None);
            if language==1 && scale==1.5 { if let Ok(out)=std::env::var("LEMON_LANGUAGE_PREVIEW") { canvas.pixmap.save_png(std::path::Path::new(&out).join("French-Save.png")).unwrap(); } }
            named(&mut app,NamedKey::Escape);
            for menu in 0..crate::views::menus::MENUS.len() {
                app.state.menu_open=Some(menu);app.paint(&mut canvas,None);
                for h in app.hits.iter().filter(|h|matches!(h.action,Action::MenuItem(..))) {
                    assert!(h.rect.x>=0. && h.rect.right()<=app.state.screen.0+1.,"menu {menu}, language {language}, {:?}",h.rect);
                }
                if language==3 && scale==1.5 && menu==5 { if let Ok(out)=std::env::var("LEMON_LANGUAGE_PREVIEW") { canvas.pixmap.save_png(std::path::Path::new(&out).join("German-Options.png")).unwrap(); } }
            }
            app.state.menu_open=None;
        }
    }
    fake::live(false);crate::prefs::set_choice(&crate::prefs::INTERFACE_LANGUAGE,0);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn language_files_browser_empty_menu_import_cancel_undo_and_visibility() {
    use crate::browser::BCmd;
    let dir=std::env::temp_dir().join(format!("lemon_browser_files_{}",std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();let path=dir.join("Sample With Space.wav");
    os_engine::wav::write_f32_stereo(&mut std::fs::File::create(&path).unwrap(),44100,&[0.;32]).unwrap();
    let mut app=App::new(None);app.state.project=crate::app::test_project();app.state.show_browser=true;
    app.state.browser_search="no-results-for-empty-area-test".into();
    let mut canvas=Canvas::new(1280,800,1.).unwrap();app.paint(&mut canvas,None);
    let bg=hit(&app,Action::BrowserList);app.mouse_move(bg.cx(),bg.bottom()-10.);app.mouse_down_right();
    let menu=app.state.playlist.popup.as_ref().expect("empty-space menu");assert!(menu.items.iter().any(|i|i.cmd==crate::playlist::Cmd::Browser(BCmd::LoadSample)));
    app.paint(&mut canvas,None);fake::live(true);
    let first=hit(&app,Action::PlPopup(0));click(&mut app,first);
    assert!(matches!(app.top_file().unwrap().then,Then::BrowserSample));
    let before=app.state.project.clone();named(&mut app,NamedKey::Escape);assert_eq!(app.state.project,before);
    fake::live(false);fake::answer(Some(path.clone()));app.browser_cmd(BCmd::LoadSample);
    assert_eq!(app.state.project.channels.len(),before.channels.len()+1);
    assert_eq!(app.state.project.channels.last().unwrap().sample_path.as_deref(),Some(path.to_string_lossy().as_ref()));
    crate::history::undo(&mut app.state);assert_eq!(app.state.project,before);
    fake::answer(Some(dir.clone()));app.browser_cmd(BCmd::AddFolder);assert!(app.state.browser_folders.contains(&dir));
    app.state.playlist.popup=None;crate::workspace_visibility::toggle(crate::browser::EMPTY_ACTIONS);
    app.browser_empty_menu(30.,300.);assert!(app.state.playlist.popup.is_none());assert!(app.state.browser_folders.contains(&dir));assert_eq!(app.state.project,before);
    crate::workspace_visibility::toggle(crate::browser::EMPTY_ACTIONS);
    std::fs::remove_dir_all(dir).unwrap();
}
