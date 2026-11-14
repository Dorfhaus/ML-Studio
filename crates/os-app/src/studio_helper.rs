// SPDX-License-Identifier: GPL-3.0-or-later
//! Optional local companion. Inference and installation run outside the audio thread.
use std::{sync::Mutex, process::{Child, Command}, time::{Instant,Duration}};
use crate::{app::App, dialog::{Msg,Kind,Answer,Then,spans},helper_install as install};
pub const LABEL: &str = "ML Studio Helper (Melli)";
pub const VISIBILITY: &str = "visible_studio-helper";
static CHILD: Mutex<Option<Child>> = Mutex::new(None);
static HEARTBEAT: Mutex<Option<(Instant,String)>> = Mutex::new(None);
static REQUEST_CHECK: Mutex<Option<Instant>> = Mutex::new(None);
pub fn available() -> bool { crate::workspace_visibility::shown(VISIBILITY) }
pub fn open(app:&mut App){
    if !available(){return}
    if install::busy(){app.show_msg(progress_message(&install::status()));return}
    match install::root(){
        Ok(root) if install::installed(&root)=>if let Err(e)=launch(){app.state.message=Some(e);app.invalidate();},
        Ok(_)=>app.show_msg(Msg::new(Kind::Question,LABEL,spans(&[&format!("Download Melli's optional files? Model: {:.0} MB from Hugging Face. Runtime: {:.0} MB. Total: {:.0} MB; allow 2 GB free disk space. No Python installation or account is required. Chats run offline after installation. Melli can retain up to 24 editable notes (16 KiB); transcripts are not saved.\n\nWindows x64, Linux x64 (glibc 2.36+) and macOS 14+ (Apple Silicon or Intel). GPU acceleration depends on your hardware and drivers; CPU fallback is available.", install::package().model.size as f64/1_000_000.0, install::package().archive.size as f64/1_000_000.0, (install::package().model.size+install::package().archive.size) as f64/1_000_000.0)]),&[Answer::Yes,Answer::Cancel],Then::MelliInstall).labelled(&["Download and install","Cancel"])),
        Err(e)=>{app.state.message=Some(e.to_string());app.invalidate();}
    }
}
fn progress_message(status:&install::Status)->Msg {
    let mut msg=Msg::new(Kind::Info,"Installing Melli",spans(&[&status.message]),&[Answer::Yes,Answer::Cancel],Then::MelliProgress).labelled(&["Cancel download","Run in background"]);
    msg.focus=1;msg.progress=Some(status.progress.unwrap_or((0,0)));msg
}
pub fn start_download(app:&mut App){install::start(false);app.show_msg(progress_message(&install::status()));}
fn launch()->Result<(),String>{
    let mut child=CHILD.lock().map_err(|_|"Helper state unavailable")?;
    if let Some(p)=child.as_mut(){if p.try_wait().map_err(|e|e.to_string())?.is_none(){return Ok(())}}
    let root=install::root().map_err(|e|e.to_string())?;let current=root.join("current");let data=root.join("data");
    std::fs::create_dir_all(&data).map_err(|e|e.to_string())?;
    let guide=data.join("user-guide.md");std::fs::write(&guide,crate::help::helper_text()).map_err(|e|e.to_string())?;
    let mut command=Command::new(install::python(&current));
    command.arg(current.join("companion_qt.py")).arg("--guide").arg(guide).arg("--session").arg(data.join("session.json")).arg("--parent-pid").arg(std::process::id().to_string())
        .current_dir(&current).env("ML_HELPER_HOME",&current).env("ML_HELPER_DATA",&data).env("ML_HELPER_MODEL",root.join("models").join(install::package().model.name))
        .env("PYTHONDONTWRITEBYTECODE","1").env("PYTHONUTF8","1").env("HF_HUB_OFFLINE","1")
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(windows)]{use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
    *HEARTBEAT.lock().unwrap()=None;
    *child=Some(command.spawn().map_err(|e|format!("Could not start Melli: {e}"))?);Ok(())
}
pub fn tick(app:&mut App){
    if let Some(status)=install::take_status(){
        app.state.message=Some(match status.progress {Some((done,total)) if total>0=>format!("Melli download: {}% - Help > {LABEL} for progress",done*100/total),_=>status.message.clone()});
        for modal in &mut app.state.dialogs {if let crate::dialog::Body::Msg(msg)=&mut modal.body {if msg.then==Then::MelliProgress {
            if status.busy {msg.text=spans(&[&status.message]);msg.progress=Some(status.progress.unwrap_or((0,0)));}
            else {*msg=Msg::new(Kind::Info,LABEL,spans(&[&status.message]),&[Answer::Ok],Then::Nothing);}
        }}}
        app.invalidate();if status.ready&&available(){if let Err(e)=launch(){app.state.message=Some(e)}}
    }
    let mut check=REQUEST_CHECK.lock().unwrap();if check.is_some_and(|t|t.elapsed()<Duration::from_secs(2)){return}*check=Some(Instant::now());drop(check);
    if let Ok(root)=install::root(){let request=root.join("data/remove-request");if std::fs::read_to_string(&request).is_ok_and(|s|s==std::process::id().to_string()){
        let _=std::fs::remove_file(request);
        app.show_msg(Msg::new(Kind::Question,LABEL,spans(&["Remove Melli's downloaded model and runtime files? You can download them again from Help. Your saved notes stay in the data folder; use Memory > Forget everything to erase them."]),&[Answer::Yes,Answer::Cancel],Then::MelliRemove).labelled(&["Remove files","Keep files"]));
    }}
}
pub fn heartbeat(app:&mut App,window:&winit::window::Window){
    let Ok(mut child)=CHILD.lock() else{return};let Some(p)=child.as_mut() else{return};
    if let Ok(Some(code))=p.try_wait(){if !code.success(){app.state.message=Some("Melli could not open. Check the Helper requirements in the User Guide (including Linux desktop libraries).".into());app.invalidate();}*child=None;return}
    let scale=window.scale_factor();let size=window.inner_size().to_logical::<f64>(scale);
    let pos=window.inner_position().ok().map(|p|p.to_logical::<f64>(scale));
    let state=serde_json::json!({"bounds":pos.map(|p|[p.x,p.y,size.width,size.height]),"visible":!window.is_minimized().unwrap_or(false),"focused":window.has_focus()});
    let text=state.to_string();let mut last=HEARTBEAT.lock().unwrap();
    if last.as_ref().is_some_and(|(time,value)|time.elapsed()<Duration::from_secs(2)&&value==&text){return}
    if let Ok(root)=install::root(){if std::fs::write(root.join("data/session.json"),&text).is_ok(){*last=Some((Instant::now(),text));}}
}
pub fn shutdown(){
    install::cancel();signal_close();
    if let Ok(mut child)=CHILD.lock(){if let Some(mut p)=child.take(){let active=p.try_wait().ok().flatten().is_none();if active{let _=p.kill();let _=p.wait();if let Ok(root)=install::root(){let _=std::fs::remove_file(root.join("data/session.json"));}}}}
}
pub fn visibility_changed(){if !available(){shutdown();}}
fn signal_close(){
    // Retain compatibility with already-running Development Tk companions.
    #[cfg(windows)]{
        use std::ffi::c_void;
        #[link(name="kernel32")]unsafe extern "system"{fn OpenEventW(access:u32,inherit:i32,name:*const u16)->*mut c_void;fn SetEvent(event:*mut c_void)->i32;fn CloseHandle(handle:*mut c_void)->i32;}
        let name:Vec<u16>=format!("Local\\MLStudioHelperStop-{}",std::process::id()).encode_utf16().chain(Some(0)).collect();
        unsafe{let event=OpenEventW(2,0,name.as_ptr());if !event.is_null(){SetEvent(event);CloseHandle(event);}}
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn melli_download_panel_keeps_real_progress_and_safe_dismissal() {
        let mut app=crate::app::App::new(None);
        let mut canvas=os_ui::Canvas::new(1000,700,1.25).unwrap();
        let mut previous=Vec::new();
        for done in [0,246108288,492216576] {
            let status=super::install::Status{busy:true,ready:false,message:"Downloading Qwen3.5-0.8B-UD-Q3_K_XL.gguf\nYou can keep working while Melli installs.".into(),progress:Some((done,492216576))};
            let msg=super::progress_message(&status);
            assert_eq!(msg.progress,status.progress);assert_eq!(msg.escape(),crate::dialog::Answer::Cancel);
            assert_eq!(msg.label(msg.focus),"Run in background");assert_eq!(msg.label(0),"Cancel download");
            app.state.dialogs.clear();app.show_msg(msg);app.invalidate();app.paint(&mut canvas,None);
            assert_ne!(previous,canvas.pixmap.data());previous=canvas.pixmap.data().to_vec();
            if let Some(out)=std::env::var_os("OS_SHOT_DIR") {let out=std::path::PathBuf::from(out);std::fs::create_dir_all(&out).unwrap();canvas.pixmap.save_png(out.join(format!("melli-download-{done}.png"))).unwrap();}
        }
    }
    #[cfg(windows)]
    #[test]
    fn helper_close_signal_reaches_an_existing_companion() {
        use std::ffi::c_void;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn CreateEventW(attributes: *const c_void, manual: i32, initial: i32, name: *const u16) -> *mut c_void;
            fn WaitForSingleObject(handle: *mut c_void, millis: u32) -> u32;
            fn CloseHandle(handle: *mut c_void) -> i32;
        }
        let name: Vec<u16> = format!("Local\\MLStudioHelperStop-{}", std::process::id()).encode_utf16().chain(Some(0)).collect();
        unsafe {
            let event = CreateEventW(std::ptr::null(), 0, 0, name.as_ptr());
            assert!(!event.is_null());
            super::signal_close();
            assert_eq!(WaitForSingleObject(event, 1000), 0);
            CloseHandle(event);
        }
    }
    #[test]
    fn helper_menu_visibility_is_independent_of_voice_and_projects() {
        use crate::{workspace_visibility as visibility, views::menus};
        assert!(visibility::GROUPS.iter().any(|(_, rows)|rows.iter().any(|(_, key)|*key==super::VISIBILITY)));
        assert_eq!(menus::item_at(menus::HELP_MENU, menus::HELP_STUDIO_HELPER).unwrap().label, super::LABEL);
        assert!(crate::voice_runtime::LOCAL_AI_ON_HOLD);
    }
}
