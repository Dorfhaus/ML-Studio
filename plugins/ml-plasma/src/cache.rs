//! Keep two managed engine versions, without deleting active or unknown files.
use super::*;
pub fn lease(root:&Path,hash:&str)->Result<fs::File,String>{
    fs::create_dir_all(root).map_err(|e|e.to_string())?;
    let f=fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(root.join(format!("{hash}.lock"))).map_err(|e|e.to_string())?;
    f.lock_shared().map_err(|e|e.to_string())?;Ok(f)
}
fn plain(path:&Path)->bool{
    let Ok(m)=fs::symlink_metadata(path)else{return false};
    #[cfg(windows)]{use std::os::windows::fs::MetadataExt;if m.file_attributes()&0x400!=0{return false}}
    !m.file_type().is_symlink()&&(m.is_file()||m.is_dir())
}
fn inventory(root:&Path,dir:&Path,names:&[String],paths:&mut Vec<PathBuf>)->bool{
    if !plain(dir)||!dir.is_dir()||paths.len()>512{return false}
    let Ok(entries)=fs::read_dir(dir)else{return false};
    for entry in entries{let Ok(e)=entry else{return false};let p=e.path();if !plain(&p){return false}
        let Ok(rel)=p.strip_prefix(root)else{return false};let rel=rel.to_string_lossy().replace('\\',"/");
        if p.is_dir(){if !names.iter().any(|n|n.starts_with(&(rel.clone()+"/")))||!inventory(root,&p,names,paths){return false}}
        else if !names.contains(&rel){return false}paths.push(p);
    }true
}
pub fn prune(root:&Path,current:&str){
    let Ok(entries)=fs::read_dir(root)else{return};let mut versions=Vec::new();
    for entry in entries.flatten(){let p=entry.path();let name=entry.file_name().to_string_lossy().into_owned();
        if name==current||name.len()!=64||!name.bytes().all(|c|c.is_ascii_hexdigit())||!plain(&p)||!p.is_dir(){continue}
        let marker=p.join("cache-files.json");if !plain(&marker)||!marker.metadata().is_ok_and(|m|m.len()<65536){continue}
        let Some(mut names)=fs::read(marker).ok().and_then(|b|serde_json::from_slice::<Vec<String>>(&b).ok())else{continue};
        if names.is_empty()||names.len()>256||names.iter().any(|n|safe_path(n).is_err()){continue}names.push("cache-files.json".into());
        let mut paths=Vec::new();if !inventory(&p,&p,&names,&mut paths){continue}
        versions.push((p.metadata().and_then(|m|m.modified()).ok(),p,name,paths));
    }
    versions.sort_by(|a,b|b.0.cmp(&a.0));
    for (_,dir,hash,paths) in versions.into_iter().skip(1){
        let lock=root.join(format!("{hash}.lock"));if lock.exists()&&!plain(&lock){continue}
        let Ok(lease)=fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&lock)else{continue};
        if lease.try_lock().is_err(){continue}
        let result=(||->std::io::Result<()>{for p in paths{if !plain(&p){return Err(std::io::Error::other("Cache entry changed"))}if p.is_dir(){fs::remove_dir(p)?}else{fs::remove_file(p)?}}fs::remove_dir(dir)})();
        drop(lease);if result.is_ok(){let _=fs::remove_file(lock);}
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn plasma_cache_is_bounded_and_preserves_user_files(){
        let root=std::env::temp_dir().join(format!("plasma-cache-{}",std::process::id()));fs::create_dir_all(&root).unwrap();
        for c in ['a','b','c','d']{let dir=root.join(c.to_string().repeat(64));fs::create_dir_all(&dir).unwrap();fs::write(dir.join(FILE),b"fixture").unwrap();fs::write(dir.join("cache-files.json"),serde_json::to_vec(&vec![FILE]).unwrap()).unwrap();}
        let unknown=root.join("e".repeat(64));fs::create_dir(&unknown).unwrap();fs::write(unknown.join("project.lmn"),b"user").unwrap();
        prune(&root,&"a".repeat(64));assert_eq!(fs::read_dir(&root).unwrap().flatten().filter(|e|e.path().is_dir()).count(),3);assert!(unknown.join("project.lmn").is_file());fs::remove_dir_all(root).unwrap();
    }
}
