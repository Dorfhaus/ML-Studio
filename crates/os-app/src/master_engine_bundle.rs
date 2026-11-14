// SPDX-License-Identifier: GPL-3.0-or-later
//! Release-native engine, carried inside the executable for backwards-compatible updates.
use std::{fs, io::{Cursor, Read}, path::{Path, PathBuf}, sync::OnceLock};
use sha2::{Digest, Sha256};
use serde::Deserialize;
const BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/master-engine.zip"));
#[derive(Deserialize)]
struct Manifest { os: String, arch: String, files: Vec<Entry> }
#[derive(Deserialize)]
struct Entry { path: String, sha256: String, size: u64 }
pub fn is_bundled() -> bool { !BYTES.is_empty() }
pub fn bundle_path() -> Option<PathBuf> {
    static PATH: OnceLock<Option<(PathBuf,fs::File)>> = OnceLock::new();
    PATH.get_or_init(|| {
        let root = os_paths::user_dir()?.join("Builtin Plugins");
        fs::create_dir_all(&root).ok()?;
        let hash=format!("{:x}",Sha256::digest(BYTES));
        let lease=fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(root.join(format!("{hash}.lock"))).ok()?;
        lease.lock_shared().ok()?;
        match unpack(BYTES, &root) {
            Ok(p) => {prune(&root,&hash);Some((p,lease))},
            Err(e) => { eprintln!("Master Assistant embedded engine: {e}"); None }
        }
    }).as_ref().map(|(path,_)|path.clone())
}
fn safe_path(name: &str) -> Result<&Path, String> {
    let p = Path::new(name);
    let bundle = name.split_once('/').map(|(bundle, _)| bundle);
    if name.contains('\\') || name.contains(':') || !matches!(bundle, Some(super::FILE | super::LEGACY_FILE))
        || !p.components().all(|c| matches!(c, std::path::Component::Normal(_))) {
        return Err("Invalid engine path".into());
    }
    Ok(p)
}
fn verified(root: &Path, entries: &[Entry]) -> bool {
    entries.iter().all(|e| {
        let Ok(p) = safe_path(&e.path) else { return false };
        let path = root.join(p);
        if !fs::symlink_metadata(&path).is_ok_and(|m| m.is_file() && m.len() == e.size) { return false }
        fs::read(path).is_ok_and(|b| format!("{:x}", Sha256::digest(&b)) == e.sha256)
    })
}
fn unpack(bytes: &[u8], root: &Path) -> Result<PathBuf, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let manifest: Manifest = {
        let f = zip.by_name("engine.json").map_err(|e| e.to_string())?;
        serde_json::from_reader(f.take(1024 * 1024)).map_err(|e| e.to_string())?
    };
    if manifest.os != std::env::consts::OS || manifest.arch != std::env::consts::ARCH || manifest.files.is_empty() {
        return Err("Engine platform does not match this application".into());
    }
    if zip.len() != manifest.files.len() + 1 { return Err("Unexpected engine files".into()) }
    let bundle = safe_path(&manifest.files[0].path)?.components().next().unwrap().as_os_str();
    for e in &manifest.files {
        if !safe_path(&e.path)?.starts_with(Path::new(bundle)) {
            return Err("Engine manifest contains more than one bundle".into());
        }
    }
    let hash = format!("{:x}", Sha256::digest(bytes));
    let dest = root.join(&hash);
    if verified(&dest, &manifest.files) { return Ok(dest.join(bundle)) }
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    // Each process writes its own staging directory. A complete validated directory is renamed atomically.
    let stage = root.join(format!("{hash}-{}.tmp", std::process::id()));
    if stage.exists(){let mut names:Vec<_>=manifest.files.iter().map(|e|e.path.clone()).collect();names.push("cache-files.json".into());crate::updates::retention::remove_owned(&stage,root,&names).map_err(|e|e.to_string())?;}
    fs::create_dir(&stage).map_err(|e| e.to_string())?;
    let inventory:Vec<_>=manifest.files.iter().map(|e|e.path.clone()).collect();
    fs::write(stage.join("cache-files.json"),serde_json::to_vec(&inventory).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    for e in &manifest.files {
        if e.size > 256 * 1024 * 1024 { return Err("Engine file exceeds size limit".into()) }
        let mut f = zip.by_name(&e.path).map_err(|e| e.to_string())?;
        if f.size() != e.size || f.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err("Invalid engine entry".into());
        }
        let mut data = Vec::new();
        (&mut f).take(e.size + 1).read_to_end(&mut data).map_err(|e| e.to_string())?;
        if data.len() as u64 != e.size || format!("{:x}", Sha256::digest(&data)) != e.sha256 {
            return Err("Engine checksum mismatch".into());
        }
        let path = stage.join(safe_path(&e.path)?);
        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::write(&path, data).map_err(|e| e.to_string())?;
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(f.unix_mode().unwrap_or(0o644) & 0o777)).map_err(|e| e.to_string())?;
        }
    }
    if let Err(e) = fs::rename(&stage, &dest) {
        if !verified(&dest, &manifest.files) { return Err(e.to_string()) }
    }
    Ok(dest.join(bundle))
}
// Keep current plus one inactive, managed engine version. Legacy/unknown folders
// are preserved; active engines hold shared leases, so cleanup cannot remove them.
fn prune(root:&Path,current:&str){
    let Ok(entries)=fs::read_dir(root) else{return};let mut managed=Vec::new();
    for entry in entries.flatten(){
        let path=entry.path();let name=entry.file_name().to_string_lossy().into_owned();
        let hash=name.split('-').next().unwrap_or("");
        if hash.len()!=64 || !hash.bytes().all(|c|c.is_ascii_hexdigit()) || name==current{continue}
        let meta=match fs::symlink_metadata(&path){Ok(m) if m.is_dir()&&!m.file_type().is_symlink()=>m,_=>continue};
        let temporary=name.strip_suffix(".tmp").and_then(|s|s.rsplit_once('-')).and_then(|(_,p)|p.parse::<u128>().ok());
        if name!=hash && temporary.is_none(){continue}
        if temporary.is_some_and(crate::updates::retention::process_alive){continue}
        let marker=path.join("cache-files.json");if !fs::metadata(&marker).is_ok_and(|m|m.len()<65536){continue}
        let Ok(mut names)=fs::read(marker).ok().and_then(|b|serde_json::from_slice::<Vec<String>>(&b).ok()).ok_or(()) else{continue};
        if names.is_empty() || names.len()>100 || names.iter().any(|n|safe_path(n).is_err()){continue}
        names.push("cache-files.json".into());
        managed.push((temporary.is_some(),meta.modified().ok(),path,hash.to_string(),names));
    }
    managed.sort_by(|a,b|b.1.cmp(&a.1));let mut kept=false;
    for (temporary,_,path,hash,names) in managed{
        if temporary{let _=crate::updates::retention::remove_owned(&path,root,&names);continue}
        if !temporary && !kept{kept=true;continue}
        let lock=root.join(format!("{hash}.lock"));
        let Ok(lease)=fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&lock) else{continue};
        if lease.try_lock().is_err(){continue}
        let removed=crate::updates::retention::remove_owned(&path,root,&names).is_ok();drop(lease);
        if removed{let _=fs::remove_file(lock);}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn engine_cache_retention_preserves_active_and_unknown(){
        let root=std::env::temp_dir().join(format!("engine-retention-{}",std::process::id()));fs::create_dir_all(&root).unwrap();
        let current="a".repeat(64);let active="b".repeat(64);
        for (i,c) in ['a','b','c','d','e'].into_iter().enumerate(){
            let dir=root.join(c.to_string().repeat(64));let file=format!("{}/Contents/test.bin",super::super::FILE);
            fs::create_dir_all(dir.join(&file).parent().unwrap()).unwrap();fs::write(dir.join(file.clone()),b"engine").unwrap();
            fs::write(dir.join("cache-files.json"),serde_json::to_vec(&vec![file]).unwrap()).unwrap();
            fs::File::open(&dir).ok().and_then(|f|f.set_times(fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH+std::time::Duration::from_secs(i as u64+1))).ok());
        }
        let lease=fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(root.join(format!("{active}.lock"))).unwrap();lease.lock_shared().unwrap();
        fs::create_dir(root.join("personal-presets")).unwrap();prune(&root,&current);
        assert!(root.join(&current).is_dir());assert!(root.join(&active).is_dir());assert!(root.join("personal-presets").is_dir());
        drop(lease);prune(&root,&current);
        assert_eq!(fs::read_dir(&root).unwrap().flatten().filter(|e|e.file_name().len()==64&&e.path().is_dir()).count(),2);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn engine_paths_reject_escape() {
        for p in ["../outside", "/absolute", "Lemon Master Assistant.vst3/../outside", "Lemon Master Assistant.vst3/C:evil"] {
            assert!(safe_path(p).is_err());
        }
        assert!(safe_path("Lemon Master Assistant.vst3/Contents/x86_64-win/Lemon Master Assistant.vst3").is_ok());
        assert!(safe_path("ML Master Assistant.vst3/Contents/x86_64-win/ML Master Assistant.vst3").is_ok());
    }
    #[test]
    fn embedded_release_can_extract_and_reuse() {
        if BYTES.is_empty() { return; }
        let root = std::env::temp_dir().join(format!("lemon-engine-test-{}", std::process::id()));
        let first = unpack(BYTES, &root).unwrap();
        assert!(first.is_dir());
        assert_eq!(unpack(BYTES, &root).unwrap(), first);
        fs::remove_dir_all(root).unwrap();
    }
}
