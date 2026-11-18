use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
pub const SIZE: u64 = 32_319_396;
pub const SHA: &str = "9575028c7a1f589f5770fccc8cff2734566af40cd26ed836944e9a5152688cfe";
pub const URL: &str = "https://raw.githubusercontent.com/mrbumpy409/GeneralUser-GS/684543d5e5efaef08d02be50dcda8d552478fa60/GeneralUser-GS.sf2";
pub const FILE: &str = "GeneralUser-GS-2.0.3.sf2";
pub const LICENSE: &str = include_str!("../GENERALUSER-LICENSE.txt");
static DOWNLOAD: Mutex<()> = Mutex::new(());
pub fn folder() -> PathBuf {
    if let Some(p) = std::env::var_os("ML_PURE_LIBRARY_DIR") {
        return p.into();
    }
    if let Some(p) = os_paths::user_file("pure-player-library-folder.txt") {
        if let Ok(text) = std::fs::read_to_string(p) {
            let p = PathBuf::from(text.trim());
            if p.is_absolute() {
                return p;
            }
        }
    }
    #[cfg(windows)]
    if Path::new("E:/ML Studio").is_dir() {
        return PathBuf::from("E:/ML Studio/Libraries/ML Pure Player");
    }
    os_paths::user_file("Libraries/ML Pure Player").unwrap_or_else(std::env::temp_dir)
}
pub fn remember_folder(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("Choose an absolute library folder".into());
    }
    std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    let config = os_paths::user_file("pure-player-library-folder.txt")
        .ok_or("No writable profile folder")?;
    if let Some(parent) = config.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(config, path.to_string_lossy().as_bytes()).map_err(|e| e.to_string())
}
pub fn installed(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() == SIZE)
}
fn verified(path: &Path) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut len = 0;
    loop {
        let Ok(n) = file.read(&mut buf) else {
            return false;
        };
        if n == 0 {
            break;
        }
        len += n as u64;
        hash.update(&buf[..n]);
    }
    len == SIZE && format!("{:x}", hash.finalize()) == SHA
}
pub fn download(
    root: &Path,
    progress: &AtomicU64,
    cancel: impl Fn() -> bool,
) -> Result<PathBuf, String> {
    let _guard = DOWNLOAD.try_lock().map_err(
        |_| "Another ML Pure Player is downloading the library. Try again when it finishes.",
    )?;
    let dest = root.join(FILE);
    if verified(&dest) {
        std::fs::write(root.join("GeneralUser-GS-LICENSE.txt"), LICENSE)
            .map_err(|e| e.to_string())?;
        progress.store(SIZE, Ordering::Relaxed);
        return Ok(dest);
    }
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    // OS lock also protects simultaneous Stock/Development processes. Released on exit/crash.
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("library-download.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock().map_err(
        |_| "Another ML Pure Player process is downloading this library. Please try again later.",
    )?;
    let part = root.join("GeneralUser-GS-2.0.3.sf2.part");
    if part.exists() {
        std::fs::remove_file(&part).map_err(|e| e.to_string())?;
    }
    let result = (|| {
        if cancel() {
            return Err("Download cancelled".into());
        }
        let response = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(5))
            .build()
            .get(URL)
            .call()
            .map_err(|e| format!("Download failed: {e}. Retry or load your own SF2."))?;
        let mut reader = response.into_reader();
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&part)
            .map_err(|e| e.to_string())?;
        transfer(&mut reader, &mut file, progress, &cancel)?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        if cancel() {
            return Err("Download cancelled".into());
        }
        if dest.exists() {
            std::fs::remove_file(&dest).map_err(|e| e.to_string())?;
        }
        std::fs::rename(&part, &dest).map_err(|e| e.to_string())?;
        std::fs::write(root.join("GeneralUser-GS-LICENSE.txt"), LICENSE)
            .map_err(|e| e.to_string())?;
        Ok(dest)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(part);
    }
    result
}
pub(crate) fn transfer(
    reader: &mut impl Read,
    writer: &mut impl Write,
    progress: &AtomicU64,
    cancel: &impl Fn() -> bool,
) -> Result<(), String> {
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut len = 0;
    let start = Instant::now();
    progress.store(0, Ordering::Relaxed);
    loop {
        if cancel() {
            return Err("Download cancelled".into());
        }
        if start.elapsed() > Duration::from_secs(300) {
            return Err("Download timed out. Please retry.".into());
        }
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        len += n as u64;
        if len > SIZE {
            return Err("Downloaded file exceeds expected size".into());
        }
        writer.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        hash.update(&buf[..n]);
        progress.store(len, Ordering::Relaxed);
    }
    if len != SIZE || format!("{:x}", hash.finalize()) != SHA {
        return Err("Library verification failed. No library was installed; please retry.".into());
    }
    Ok(())
}
