// SPDX-License-Identifier: GPL-3.0-or-later
//! Optional Melli payloads pinned by this signed DAW build. Never runs on the audio thread.
use std::{collections::HashSet, fs, io::{Read, Write}, path::{Path, PathBuf}, sync::{Mutex, OnceLock}, time::Duration};
use serde::Deserialize;
use sha2::{Digest, Sha256};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[derive(Clone, Deserialize)]
pub struct File { pub name: String, pub url: String, pub sha256: String, pub size: u64 }
#[derive(Clone, Deserialize)]
pub struct Package { pub version: u64, pub archive: File, pub model: File, pub unpacked_size: u64 }
pub fn platform() -> &'static str {
    if cfg!(windows) {"windows"} else if cfg!(target_os="linux") {"linux"}
    else if cfg!(target_arch="aarch64") {"mac-arm64"} else {"mac-x86_64"}
}
pub fn package() -> Package {
    let all: std::collections::HashMap<String, Package> = serde_json::from_str(include_str!("../../../packaging/helper/catalog.json")).expect("validated Melli catalog");
    all[platform()].clone()
}
pub fn root() -> Result<PathBuf> {
    if let Some(p)=std::env::var_os("ML_HELPER_INSTALL_ROOT") {return Ok(PathBuf::from(p))}
    // Preserve the existing Development installation on Michael's E: drive.
    if cfg!(windows) {
        let exe=std::env::current_exe()?;
        if exe.file_name().is_some_and(|s|s=="lemonstudio-development.exe") {
            if let Some(parent)=exe.parent().and_then(Path::parent) {
                let p=parent.join("ML Studio Helper"); if p.is_dir() {return Ok(p)}
            }
        }
    }
    Ok(os_paths::user_dir().ok_or("Cannot find your user folder")?.join("Melli"))
}
fn ordinary(path: &Path) -> Result<fs::Metadata> {
    let m=fs::symlink_metadata(path)?;
    #[cfg(windows)] {use std::os::windows::fs::MetadataExt; if m.file_attributes()&0x400!=0 {return Err("Helper folder contains a link or junction; preserved".into())}}
    if m.file_type().is_symlink() {return Err("Helper folder contains a symbolic link; preserved".into())}
    Ok(m)
}
fn directory(path: &Path) -> Result<()> {fs::create_dir_all(path)?;if !ordinary(path)?.is_dir(){return Err("Helper destination is not a folder".into())}Ok(())}
pub fn safe_name(s:&str) -> bool {
    !s.is_empty() && !s.contains(['\\',':']) && s.split('/').all(|p| !p.is_empty() && p!="." && p!=".." && !p.ends_with(['.',' ']) && !p.chars().any(char::is_control))
}
fn verified(path:&Path, file:&File) -> bool {
    (|| -> Result<bool> {
        if !ordinary(path)?.is_file() || fs::metadata(path)?.len()!=file.size {return Ok(false)}
        let mut input=fs::File::open(path)?;let mut hash=Sha256::new();let mut buf=[0u8;65536];
        loop {let n=input.read(&mut buf)?;if n==0 {break}hash.update(&buf[..n]);}
        Ok(format!("{:x}",hash.finalize())==file.sha256)
    })().unwrap_or(false)
}
pub fn installed(root:&Path) -> bool {
    let p=package();let current=root.join("current");
    fs::read_to_string(current.join("installed.json")).is_ok_and(|s|s==p.archive.sha256)
        && current.join("companion_qt.py").is_file() && python(&current).is_file()
        && root.join("models").join(p.model.name).metadata().is_ok_and(|m|m.len()==p.model.size)
}
pub fn python(current:&Path) -> PathBuf {current.join(if cfg!(windows){"python/pythonw.exe"}else{"python/bin/python3"})}
#[derive(Default)]
struct State {busy:bool,cancel:bool,changed:bool,message:String,ready:bool,progress:Option<(u64,u64)>}
#[derive(Clone)]
pub struct Status {pub busy:bool,pub message:String,pub ready:bool,pub progress:Option<(u64,u64)>}
impl State {fn snapshot(&self)->Status{Status{busy:self.busy,message:self.message.clone(),ready:self.ready,progress:self.progress}}}
fn state()->&'static Mutex<State>{static S:OnceLock<Mutex<State>>=OnceLock::new();S.get_or_init(Default::default)}
pub fn busy()->bool{state().lock().unwrap().busy}
pub fn cancel(){let mut s=state().lock().unwrap();s.cancel=true;s.message="Cancelling Melli download...".into();s.changed=true;}
fn cancelled()->Result<()>{if state().lock().unwrap().cancel{Err("Melli download cancelled".into())}else{Ok(())}}
fn report(message:String){let mut s=state().lock().unwrap();s.message=message;s.progress=None;s.changed=true;}
fn report_download(file:&File,count:u64){let mut s=state().lock().unwrap();s.message=format!("Downloading {}\nYou can keep working while Melli installs.",file.name);s.progress=Some((count,file.size));s.changed=true;}
pub fn status()->Status{state().lock().unwrap().snapshot()}
pub fn take_status()->Option<Status>{let mut s=state().lock().unwrap();if !std::mem::take(&mut s.changed){return None}let result=s.snapshot();s.ready=false;Some(result)}
fn download(file:&File,path:&Path)->Result<()> {
    report(format!("Checking {}...",file.name));cancelled()?;
    if verified(path,file){return Ok(())}
    if path.exists() {ordinary(path)?;return Err(format!("Existing {} failed verification; move that file aside and retry. Other files were preserved.",path.display()).into())}
    let partial=path.with_extension("part");
    if partial.exists(){if !ordinary(&partial)?.is_file(){return Err("Invalid download temporary file".into())}fs::remove_file(&partial)?;}
    let result=(||->Result<()>{
        cancelled()?;report_download(file,0);
        let response=ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(20)).timeout_read(Duration::from_secs(30)).build().get(&file.url).call()?;
        let mut input=response.into_reader();let mut output=fs::OpenOptions::new().create_new(true).write(true).open(&partial)?;
        let mut hash=Sha256::new();let mut buf=[0u8;65536];let mut count=0;let mut percent=101;
        loop {cancelled()?;let n=input.read(&mut buf)?;if n==0{break}count+=n as u64;if count>file.size{return Err("Melli download exceeded expected size".into())}
            output.write_all(&buf[..n])?;hash.update(&buf[..n]);let next=count*100/file.size;
            if next!=percent{percent=next;report_download(file,count);}}
        if count!=file.size || format!("{:x}",hash.finalize())!=file.sha256{return Err("Melli download checksum failed; please retry".into())}
        output.sync_all()?;drop(output);cancelled()?;fs::rename(&partial,path)?;Ok(())
    })();let _=fs::remove_file(&partial);result
}
/// Remove only a complete inventory of package-owned files. Unknown files stop cleanup.
pub fn remove_owned(dir:&Path)->Result<()> {
    if !dir.exists(){return Ok(())}if !ordinary(dir)?.is_dir(){return Err("Invalid Helper package directory".into())}
    let inventory=dir.join("melli-files.json");if ordinary(&inventory)?.len()>4*1024*1024{return Err("Invalid Helper inventory".into())}
    let mut names:HashSet<String>=serde_json::from_slice::<Vec<String>>(&fs::read(inventory)?)?.into_iter().collect();
    if names.len()>30000 || names.iter().any(|s|!safe_name(s)){return Err("Invalid Helper inventory".into())}
    names.insert("melli-files.json".into());names.insert("installed.json".into());
    fn visit(root:&Path,dir:&Path,names:&HashSet<String>,paths:&mut Vec<PathBuf>)->Result<()> {
        for entry in fs::read_dir(dir)?{let path=entry?.path();let m=ordinary(&path)?;let name=path.strip_prefix(root)?.to_string_lossy().replace('\\',"/");
            if m.is_dir(){if !names.iter().any(|s|s.starts_with(&(name.clone()+"/"))){return Err("Unknown folder in Melli package; preserved".into())}visit(root,&path,names,paths)?;}
            else if !m.is_file() || !names.contains(&name){return Err("Unknown file in Melli package; preserved".into())}
            paths.push(path);}
        Ok(())
    }
    let mut paths=Vec::new();visit(dir,dir,&names,&mut paths)?;
    for path in paths {if ordinary(&path)?.is_dir(){fs::remove_dir(path)?;}else{fs::remove_file(path)?;}}
    fs::remove_dir(dir)?;Ok(())
}
fn extract(archive:&Path,dest:&Path,limit:u64)->Result<()> {
    let mut zip=zip::ZipArchive::new(fs::File::open(archive)?)?;
    if zip.len()>30000{return Err("Melli archive contains too many entries".into())}
    let mut inventory=Vec::new();zip.by_name("melli-files.json")?.take(4*1024*1024+1).read_to_end(&mut inventory)?;
    if inventory.len()>4*1024*1024{return Err("Melli inventory too large".into())}
    let mut names:HashSet<String>=serde_json::from_slice::<Vec<String>>(&inventory)?.into_iter().collect();names.insert("melli-files.json".into());
    let mut found=HashSet::new();let mut total=0u64;
    for i in 0..zip.len(){let f=zip.by_index(i)?;let name=f.name()?;if !safe_name(&name) || f.is_dir() || f.unix_mode().is_some_and(|m|m&0o170000==0o120000) || !names.contains(name.as_ref()) || !found.insert(name.into_owned()){return Err("Invalid Melli archive entry".into())}total=total.checked_add(f.size()).ok_or("Melli archive too large")?;}
    if total!=limit || total>2*1024*1024*1024 || found!=names{return Err("Melli archive inventory/size mismatch".into())}
    fs::create_dir(dest)?;fs::write(dest.join("melli-files.json"),inventory)?;
    for i in 0..zip.len(){cancelled()?;let mut f=zip.by_index(i)?;let name=f.name()?.into_owned();if name=="melli-files.json"{continue}let path=dest.join(name);fs::create_dir_all(path.parent().ok_or("Invalid archive path")?)?;
        let mut out=fs::OpenOptions::new().write(true).create_new(true).open(&path)?;let size=std::io::copy(&mut f,&mut out)?;if size!=f.size(){return Err("Incomplete Melli archive entry".into())}
        #[cfg(unix)]{use std::os::unix::fs::PermissionsExt;fs::set_permissions(&path,fs::Permissions::from_mode(if f.unix_mode().unwrap_or(0)&0o111!=0{0o755}else{0o644}))?;}
    }Ok(())
}
fn install(root:&Path)->Result<()> {
    directory(root)?;let lock=fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(root.join("install.lock"))?;
    lock.try_lock().map_err(|_|"Another ML Studio instance is installing Melli")?;
    let p=package();let models=root.join("models");directory(&models)?;
    let stage=root.join("next");let old=root.join("previous");let current=root.join("current");
    remove_owned(&stage)?;
    if !current.exists() && old.exists(){fs::rename(&old,&current)?;}
    remove_owned(&old)?;
    let archive=root.join("package.zip");
    let result=(||->Result<()>{
        download(&p.model,&models.join(&p.model.name))?;
        download(&p.archive,&archive)?;report("Melli: installing verified files...".into());
        extract(&archive,&stage,p.unpacked_size)?;
        fs::write(stage.join("installed.json"),&p.archive.sha256)?;cancelled()?;
        if current.exists(){fs::rename(&current,&old)?;}
        if let Err(e)=fs::rename(&stage,&current){if old.exists(){let _=fs::rename(&old,&current);}return Err(e.into())}
        remove_owned(&old)?;Ok(())
    })();
    if archive.is_file(){let _=fs::remove_file(&archive);}
    if stage.exists(){let _=remove_owned(&stage);}
    result
}
pub fn start(remove:bool){
    let mut s=state().lock().unwrap();if s.busy{return}*s=State{busy:true,changed:true,message:if remove{"Removing Melli..."}else{"Preparing Melli download..."}.into(),..Default::default()};drop(s);
    std::thread::spawn(move||{
        let result=(||->Result<()>{let root=root()?;if !remove{return install(&root)}
            directory(&root)?;let lock=fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(root.join("install.lock"))?;lock.try_lock()?;
            for name in ["current","previous","next"]{remove_owned(&root.join(name))?;}
            let p=package();let model=root.join("models").join(&p.model.name);if model.exists(){if !verified(&model,&p.model){return Err("Model file was modified; preserved".into())}fs::remove_file(model)?;}Ok(())})();
        let mut s=state().lock().unwrap();s.busy=false;s.changed=true;s.progress=None;match result{Ok(())=>{s.ready=!remove;s.message=if remove{"Melli files removed. Saved memory remains; clear it in Memory before removal if desired."}else{"Melli is ready"}.into()},Err(e)=>s.message=format!("Melli: {e}")}
    });
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]#[ignore="uses a prepared, checksum-pinned optional package fixture"]
    fn melli_real_package_install_and_remove(){
        let root=PathBuf::from(std::env::var_os("ML_HELPER_PACKAGE_TEST_ROOT").expect("prepared fixture root"));
        assert!(root.join("package.zip").is_file());assert!(!root.join("current").exists());
        install(&root).unwrap();assert!(installed(&root));assert!(!root.join("package.zip").exists());assert!(!root.join("next").exists());
        remove_owned(&root.join("current")).unwrap();assert!(!installed(&root));assert!(root.join("models").join(package().model.name).is_file());
    }
    #[test]fn melli_catalog_and_paths(){for platform in ["windows","linux","mac-arm64","mac-x86_64"]{let all:std::collections::HashMap<String,Package>=serde_json::from_str(include_str!("../../../packaging/helper/catalog.json")).unwrap();let p=&all[platform];assert!(p.version>0);assert!(p.archive.url.starts_with("https://michaellanemusic.de/lemon-studio/helper/"));assert_eq!(p.archive.sha256.len(),64);assert!(p.unpacked_size>1000000);}for name in ["../x","/x","a//b","a\\b","a:evil","a/..","a/.","a/ "]{assert!(!safe_name(name),"{name}")}}
    #[test]fn melli_cleanup_preserves_unknown_files(){let root=std::env::temp_dir().join(format!("melli-owned-{}",std::process::id()));fs::create_dir_all(&root).unwrap();fs::write(root.join("melli-files.json"),"[\"a.txt\"]").unwrap();fs::write(root.join("a.txt"),"owned").unwrap();fs::write(root.join("project.lmn"),"user").unwrap();assert!(remove_owned(&root).is_err());assert!(root.join("a.txt").is_file());fs::remove_file(root.join("project.lmn")).unwrap();remove_owned(&root).unwrap();assert!(!root.exists());}
    #[test]fn melli_archive_validation_and_owned_extraction(){
        let root=std::env::temp_dir().join(format!("melli-archive-{}",std::process::id()));fs::create_dir_all(&root).unwrap();
        for (label,name,valid) in [("good","python/bin/python3",true),("escape","../outside",false)]{
            let archive=root.join(format!("{label}.zip"));let mut zip=zip::ZipWriter::new(fs::File::create(&archive).unwrap());
            let inventory=serde_json::to_vec(&vec![name]).unwrap();let options=zip::write::SimpleFileOptions::default().unix_permissions(0o755);
            zip.start_file("melli-files.json",options).unwrap();zip.write_all(&inventory).unwrap();zip.start_file(name,options).unwrap();zip.write_all(b"test").unwrap();zip.finish().unwrap();
            let dest=root.join(label);let result=extract(&archive,&dest,inventory.len() as u64+4);assert_eq!(result.is_ok(),valid);
            if valid {assert_eq!(fs::read(dest.join(name)).unwrap(),b"test");remove_owned(&dest).unwrap();}else{assert!(!dest.exists());assert!(!root.join("outside").exists());}
            fs::remove_file(archive).unwrap();
        }fs::remove_dir(root).unwrap();
    }
}
