//! Lifetime supporter activation; no payment, network or filesystem work on the audio thread.
use std::{fs, io::Read, path::Path};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;

pub const URL: &str = "https://ko-fi.com/s/833f0d3495";
pub const ACTIVATE: &str = "Activate supporter...";
const PUBLIC_KEY: [u8; 32] = [0x08,0xe0,0xcc,0x73,0xa0,0x80,0x78,0xec,0xec,0x76,0x39,0x81,0xd6,0x21,0xe6,0x72,0xb3,0x45,0x1f,0xf6,0xad,0x32,0x81,0x90,0x5b,0x60,0x7f,0xa9,0x92,0x86,0xc1,0x1d];
const FILE: &str = "Licenses/ML Studio Supporter.mlsupport";
const MAX_SIZE: u64 = 16384;

#[derive(Deserialize)]
struct Signed { payload: String, signature: String }
#[derive(Deserialize)]
struct Entitlement { product: String, version: u32, license: String, minimum_eur_cents: u64, lifetime: bool }

fn verify(bytes: &[u8], public: &[u8;32]) -> Result<(), String> {
    let parse = || -> Option<()> {
        if bytes.len() > MAX_SIZE as usize { return None; }
        let signed: Signed = serde_json::from_slice(bytes).ok()?;
        let payload = URL_SAFE_NO_PAD.decode(signed.payload).ok()?;
        let signature = Signature::from_slice(&URL_SAFE_NO_PAD.decode(signed.signature).ok()?).ok()?;
        VerifyingKey::from_bytes(public).ok()?.verify_strict(&payload, &signature).ok()?;
        let e: Entitlement = serde_json::from_slice(&payload).ok()?;
        (e.product == "ml-studio-supporter" && e.version == 1 && e.lifetime && e.minimum_eur_cents >= 1000
            && e.license.len() == 32 && e.license.bytes().all(|b| b.is_ascii_hexdigit())).then_some(())
    };
    parse().ok_or_else(|| "This is not a valid ML Studio supporter activation. Download your .mlsupport file from your Ko-fi purchase delivery link.".into())
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    fs::File::open(path).and_then(|f| f.take(MAX_SIZE+1).read_to_end(&mut bytes)).map_err(|e| format!("Could not read supporter activation: {e}"))?;
    Ok(bytes)
}
fn installed() -> bool { os_paths::user_file(FILE).and_then(|p| read(&p).ok()).is_some_and(|b| verify(&b,&PUBLIC_KEY).is_ok()) }

#[cfg(not(test))]
fn cache() -> &'static std::sync::atomic::AtomicBool {
    static CACHE: std::sync::OnceLock<std::sync::atomic::AtomicBool> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::atomic::AtomicBool::new(installed()))
}
#[cfg(test)]
thread_local! { static ACTIVE: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) }; }

pub fn active() -> bool {
    #[cfg(not(test))] { cache().load(std::sync::atomic::Ordering::Relaxed) }
    #[cfg(test)] { ACTIVE.with(|c| match c.get() { Some(b)=>b, None=>{let b=installed();c.set(Some(b));b} }) }
}
fn set_active(value: bool) {
    #[cfg(not(test))] cache().store(value,std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)] ACTIVE.with(|c| c.set(Some(value)));
}
fn install_at(source: &Path, target: &Path, public: &[u8;32]) -> Result<(),String> {
    let bytes=read(source)?;
    verify(&bytes,public)?;
    if let Some(parent)=target.parent() { fs::create_dir_all(parent).map_err(|e|e.to_string())?; }
    crate::safe_save::write(target,&bytes).map_err(|e|format!("Could not save supporter activation: {e}"))?;
    verify(&read(target)?,public)
}
pub fn import(path: &Path) -> Result<(),String> {
    let target=os_paths::user_file(FILE).ok_or("Could not find your ML Studio profile")?;
    install_at(path,&target,&PUBLIC_KEY)?;
    set_active(true);
    Ok(())
}

pub fn prompt(app: &mut crate::app::App) {
    use crate::dialog::{Answer,Body,Kind,Msg,Then};
    if active() || app.state.dialogs.iter().any(|m| matches!(&m.body,Body::Msg(msg) if msg.then==Then::SupporterPrompt)) { return; }
    app.show_msg(Msg::new(Kind::Info,"Support ML Studio",vec![
        ("ML Studio will always be free.".into(),true),
        ("Donate €10 or more once to permanently remove this message after exports and the Donate button.".into(),false),
        ("Larger contributions are highly appreciated. No subscription is required.".into(),false),
        ("Already supported? Download your activation file from the Ko-fi delivery link and activate it below.".into(),false),
    ],&[Answer::Yes,Answer::No,Answer::Cancel],Then::SupporterPrompt)
        .labelled(&["Donate €10 or more","Activate supporter","Continue free"]));
}
pub fn choose(app: &mut crate::app::App) {
    app.file_dialog(crate::filedlg::FileDlg::new(crate::filedlg::Mode::Open,"Activate supporter","supporter",crate::dialog::Then::SupporterFile)
        .filter("ML Studio supporter activation",&["mlsupport"]));
}

pub fn open() -> Result<(), String> { open_url(URL) }

pub fn open_url(address: &str) -> Result<(), String> {
    if cfg!(test) { return Ok(()); }
    #[cfg(windows)] {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        let verb: Vec<u16> = "open\0".encode_utf16().collect();
        let url: Vec<u16> = address.encode_utf16().chain(Some(0)).collect();
        let result = unsafe { ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), url.as_ptr(), std::ptr::null(), std::ptr::null(), 1) };
        if result as isize <= 32 { return Err(format!("Could not open your browser. Visit {address}")); }
    }
    #[cfg(not(windows))] {
        let command = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
        std::process::Command::new(command).arg(address).spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer,SigningKey};
    fn certificate(payload: serde_json::Value) -> (Vec<u8>,[u8;32]) {
        let key=SigningKey::from_bytes(&[37;32]); let payload=serde_json::to_vec(&payload).unwrap();
        let signed=serde_json::json!({"payload":URL_SAFE_NO_PAD.encode(&payload),"signature":URL_SAFE_NO_PAD.encode(key.sign(&payload).to_bytes())});
        (serde_json::to_vec(&signed).unwrap(),key.verifying_key().to_bytes())
    }
    fn entitlement()->serde_json::Value {serde_json::json!({"product":"ml-studio-supporter","version":1,"license":"0123456789abcdef0123456789abcdef","minimum_eur_cents":1000,"lifetime":true})}
    #[test]
    fn supporter_rejects_tampering_wrong_product_and_insufficient_payment() {
        let (bytes,key)=certificate(entitlement()); assert!(verify(&bytes,&key).is_ok()); assert!(verify(&bytes,&PUBLIC_KEY).is_err());
        for (field,value) in [("product",serde_json::json!("voicepilot")),("minimum_eur_cents",serde_json::json!(999)),("lifetime",serde_json::json!(false)),("version",serde_json::json!(2))] {
            let mut p=entitlement(); p[field]=value; let (bytes,key)=certificate(p); assert!(verify(&bytes,&key).is_err(),"{field}");
        }
        let mut signed:serde_json::Value=serde_json::from_slice(&bytes).unwrap();signed["payload"]=serde_json::json!(URL_SAFE_NO_PAD.encode(b"tampered"));
        assert!(verify(&serde_json::to_vec(&signed).unwrap(),&key).is_err());assert!(verify(&vec![b' ';MAX_SIZE as usize+1],&key).is_err());
    }
    #[test]
    fn supporter_import_persists_and_invalid_import_keeps_existing_activation() {
        let dir=std::env::temp_dir().join(format!("ml-supporter-test-{}",std::process::id()));fs::create_dir_all(&dir).unwrap();
        let source=dir.join("download.mlsupport");let target=dir.join("profile").join(FILE);
        let(bytes,key)=certificate(entitlement());fs::write(&source,&bytes).unwrap();install_at(&source,&target,&key).unwrap();
        assert!(verify(&read(&target).unwrap(),&key).is_ok());fs::write(&source,b"invalid").unwrap();assert!(install_at(&source,&target,&key).is_err());assert_eq!(read(&target).unwrap(),bytes);
        fs::remove_dir_all(&dir).unwrap();
    }
    #[test]
    fn supporter_php_certificate_is_accepted() {
        let Some(path)=std::env::var_os("ML_SUPPORTER_SERVER_FIXTURE") else {return};
        let fixture:serde_json::Value=serde_json::from_slice(&fs::read(path).unwrap()).unwrap();let hex=fixture["public_key"].as_str().unwrap();
        let key:Vec<u8>=(0..64).step_by(2).map(|i|u8::from_str_radix(&hex[i..i+2],16).unwrap()).collect();
        assert!(verify(&serde_json::to_vec(&fixture["certificate"]).unwrap(),&key.try_into().unwrap()).is_ok());
    }
    #[test]
    fn supporter_prompt_is_dismissible_deduplicated_and_suppressed_after_activation() {
        use crate::dialog::{Answer,Body,Reply,Then};
        set_active(false);let mut app=crate::app::App::new(None);app.state.dialogs.clear();prompt(&mut app);prompt(&mut app);assert_eq!(app.state.dialogs.len(),1);
        let Body::Msg(ref msg)=app.state.dialogs[0].body else {panic!("prompt")};assert_eq!(msg.then,Then::SupporterPrompt);
        app.state.dialogs.clear();app.dialog_done(Then::SupporterPrompt,Reply::Button(Answer::Cancel));assert!(app.state.dialogs.is_empty());
        set_active(true);prompt(&mut app);assert!(app.state.dialogs.is_empty());
        let mut c=os_ui::Canvas::new(1600,1000,1.0).unwrap();
        for hidden in [false,true] {app.state.toolbar.hidden=hidden;app.paint(&mut c,None);assert!(!app.hits.iter().any(|h|h.action==crate::app::Action::Donate));}
        set_active(false);
    }

    #[test]
    fn supporter_audio_export_only_prompts_on_success_and_keeps_the_file() {
        use crate::dialog::{Body,Then};
        let mut app=crate::app::App::new(None);app.state.dialogs.clear();app.state.project=crate::app::test_project();
        let path=std::env::temp_dir().join(format!("ml-supporter-export-{}.wav",std::process::id()));
        let run=|app:&mut crate::app::App, p:&Path|{
            app.export=Some((p.display().to_string(),0.0));
            app.transport.export(&app.state.project,os_engine::PlayMode::Pattern(1),44100,crate::export::Job::wav(p));
            let start=std::time::Instant::now();
            while app.export.is_some() {app.export_tick();assert!(start.elapsed().as_secs()<30);std::thread::sleep(std::time::Duration::from_millis(5));}
        };
        set_active(false);run(&mut app,&path);assert!(path.is_file());
        assert!(app.state.dialogs.iter().any(|m|matches!(&m.body,Body::Msg(msg) if msg.then==Then::SupporterPrompt)));
        app.state.dialogs.clear();set_active(true);run(&mut app,&path);assert!(app.state.dialogs.is_empty());
        set_active(false);run(&mut app,&path.join("cannot-write-inside-a-file.wav"));assert!(app.state.dialogs.is_empty());
        fs::remove_file(path).unwrap();
    }
}
