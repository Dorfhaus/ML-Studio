// SPDX-License-Identifier: GPL-3.0-or-later

//! Scratch beside a job's destination. Reclaim only marked, abandoned jobs
//! in the same directory and reject any unfamiliar files or links.
use std::{fs, io, path::Path, time::Duration};

const MARKER: &str = ".ml-studio-job";
const GRACE: Duration = Duration::from_secs(24 * 60 * 60);

pub(crate) fn reclaim(parent: &Path, kind: &str, files: &[&str]) {
    sweep(parent, kind, files, GRACE);
}

pub(crate) fn create(parent: &Path, kind: &str, files: &[&str]) -> io::Result<tempfile::TempDir> {
    reclaim(parent, kind, files);
    let prefix = format!(".ml-{kind}-{}-", std::process::id());
    let dir = tempfile::Builder::new().prefix(&prefix).tempdir_in(parent)?;
    fs::write(dir.path().join(MARKER), kind)?;
    Ok(dir)
}

/// Clear the cleanup marker before exposing files as persistent project media.
pub(crate) fn unmark(dir: &tempfile::TempDir) -> io::Result<()> {
    fs::remove_file(dir.path().join(MARKER))
}

fn sweep(parent: &Path, kind: &str, files: &[&str], grace: Duration) {
    let Ok(entries) = fs::read_dir(parent) else { return };
    let prefix = format!(".ml-{kind}-");
    let mut files: Vec<String> = files.iter().map(|s| (*s).into()).collect();
    files.push(MARKER.into());
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some((pid, nonce)) = name.to_str().and_then(|n| n.strip_prefix(&prefix))
            .and_then(|n| n.split_once('-')) else { continue };
        let Ok(pid) = pid.parse() else { continue };
        if nonce.is_empty() {
            continue;
        }
        let path = entry.path();
        let marker = path.join(MARKER);
        if !crate::updates::retention::plain(&path) || !crate::updates::retention::plain(&marker) {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(&marker) else { continue };
        if !meta.is_file() || meta.len() != kind.len() as u64
            || !meta.modified().ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age >= grace)
            || fs::read(&marker).ok().as_deref() != Some(kind.as_bytes())
            || crate::updates::retention::process_alive(pid)
        {
            continue;
        }
        let _ = crate::updates::retention::remove_owned(&path, parent, &files);
    }
}

const REVISION_FILES: &str = ".ml-revision-files.json";
const REVISION_LOCK: &str = ".ml-revision-in-use";

/// Mark only payloads extracted and validated by this version of ML Studio.
pub(crate) fn mark_revision(dir: &Path, files: &[String]) -> io::Result<()> {
    fs::write(dir.join(REVISION_FILES), serde_json::to_vec(files)?)
}

/// Keep the current revision and one inactive rollback. Other running
/// instances hold a shared lease on their own revision.
pub(crate) fn keep_revision(parent: &Path, current: &Path) -> io::Result<()> {
    use crate::updates::retention::{plain, remove_owned};
    use std::{collections::HashMap, fs::File, path::PathBuf, sync::{Mutex, OnceLock}};

    static LEASES: OnceLock<Mutex<HashMap<PathBuf, File>>> = OnceLock::new();
    let mut leases = LEASES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    if leases.contains_key(current) {
        return Ok(());
    }

    let open = |path: &Path| {
        if fs::symlink_metadata(path).is_ok() && !plain(path) {
            return Err(io::Error::other("Invalid managed cache lock"));
        }
        fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)
    };
    let guard = open(&parent.join(".ml-revision-maintenance"))?;
    guard.lock()?;
    if !plain(current) || !current.is_dir() {
        return Err(io::Error::other("Invalid managed cache revision"));
    }
    let lease = open(&current.join(REVISION_LOCK))?;
    lease.lock_shared()?;
    leases.insert(current.into(), lease);
    let mut older = Vec::new();
    for e in fs::read_dir(parent)?.flatten() {
        let path = e.path();
        if path == current || !plain(&path) || !path.is_dir() {
            continue;
        }
        let manifest = path.join(REVISION_FILES);
        if !plain(&manifest) || !fs::metadata(&manifest).is_ok_and(|m| m.is_file() && m.len() < 512 * 1024) {
            continue;
        }
        let Ok(files) = fs::read(&manifest).and_then(|b| Ok(serde_json::from_slice::<Vec<String>>(&b)?)) else { continue };
        if files.is_empty() || files.len() > 8192 {
            continue;
        }
        older.push((e.metadata().and_then(|m| m.modified()).ok(), path, files));
    }
    older.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, path, mut files) in older.into_iter().skip(1) {
        let Ok(lease) = open(&path.join(REVISION_LOCK)) else { continue };
        if lease.try_lock().is_err() {
            continue;
        }
        // No new lease can be acquired while the maintenance lock is held.
        drop(lease);
        files.extend([REVISION_LOCK.into(), REVISION_FILES.into()]);
        let _ = remove_owned(&path, parent, &files);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_preserves_live_unmarked_and_unknown_jobs() {
        let parent = tempfile::tempdir().unwrap();
        let create = |name: &str, marked: bool, unknown: bool| {
            let p = parent.path().join(name);
            fs::create_dir(&p).unwrap();
            fs::write(p.join("scratch.wav"), b"test").unwrap();
            if marked { fs::write(p.join(MARKER), b"test").unwrap(); }
            if unknown { fs::write(p.join("project.lmn"), b"project").unwrap(); }
            p
        };
        let dead = (100_000..200_000).find(|&pid| !crate::updates::retention::process_alive(pid))
            .expect("unused PID");
        let abandoned = create(&format!(".ml-test-{dead}-old"), true, false);
        let live = create(&format!(".ml-test-{}-live", std::process::id()), true, false);
        let unmarked = create(&format!(".ml-test-{dead}-unmarked"), false, false);
        let unknown = create(&format!(".ml-test-{dead}-unknown"), true, true);
        sweep(parent.path(), "test", &["scratch.wav"], Duration::ZERO);
        assert!(!abandoned.exists());
        assert!(live.exists() && unmarked.exists() && unknown.join("project.lmn").is_file());
    }
}
