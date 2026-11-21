use super::{Available, Descriptor, Library};
use os_host::{Format, Kind, SharedPlugin};
use std::{
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

#[cfg(windows)]
const BUNDLE: &[u8] = include_bytes!("../../assets/time12/win.zip");
#[cfg(target_os = "linux")]
const BUNDLE: &[u8] = include_bytes!("../../assets/time12/linux.zip");
#[cfg(target_os = "macos")]
const BUNDLE: &[u8] = include_bytes!("../../assets/time12/macos.zip");

struct Stock(Descriptor);

pub(super) fn available() -> Available {
    let desc = Descriptor {
        format: Format::Vst3,
        id: "ABCDEF019182FAEB54696C72746D3132".into(),
        name: "TIME-12".into(),
        vendor: "Tilr".into(),
        version: "1.2.3".into(),
        kind: Kind::Effect,
        features: vec!["Fx".into()],
        path: std::env::current_exe().unwrap_or_default(),
    };
    Available {
        lib: Arc::new(Stock(desc.clone())),
        desc,
    }
}

fn unpack() -> Result<PathBuf, String> {
    let root = os_paths::user_file("Stock/TIME-12/1.2.3/TIME-12.vst3").ok_or("Profile folder unavailable")?;
    let mut archive = zip::ZipArchive::new(Cursor::new(BUNDLE)).map_err(|e| e.to_string())?;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|e| e.to_string())?;
        let target = root.join(file.enclosed_name().ok_or("Invalid bundled file name")?);
        let parent = target.parent().ok_or("Invalid bundle path")?;
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut bytes).map_err(|e| e.to_string())?;
        if fs::read(&target).is_ok_and(|data| data == bytes) {
            continue;
        }
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        temp.write_all(&bytes).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            temp.as_file()
                .set_permissions(fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
        }
        temp.persist(&target).map_err(|e| e.to_string())?;
    }
    Ok(root)
}
impl Library for Stock {
    fn path(&self) -> &Path {
        &self.0.path
    }
    fn plugins(&self) -> &[Descriptor] {
        std::slice::from_ref(&self.0)
    }
    fn create(self: Arc<Self>, _: &str) -> Result<SharedPlugin, String> {
        static MODULE: OnceLock<Arc<dyn Library>> = OnceLock::new();
        let lib = match MODULE.get() {
            Some(l) => l.clone(),
            None => {
                let module = super::load_file(&unpack()?, Format::Vst3)?;
                MODULE.get_or_init(|| module).clone()
            }
        };
        let id = lib
            .plugins()
            .iter()
            .find(|d| d.name == "TIME-12" && d.kind == Kind::Effect)
            .ok_or("TIME-12 class missing from stock module")?
            .id
            .clone();
        lib.create(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stock_time12_reopens_by_vst3_identity() {
        let item = available();
        let saved = crate::project_plugins::plugin_ref(&item.desc, Vec::new());
        assert!(saved.path.ends_with(".vst3"));
        let _instance = item.lib.create(&item.desc.id).expect("load the bundled TIME-12 effect");
    }
}
