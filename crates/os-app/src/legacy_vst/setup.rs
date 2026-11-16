// SPDX-License-Identifier: GPL-3.0-or-later
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use sha2::{Digest, Sha256};

pub(super) const DLL: &str = "libcarla_host-plugin.dll";
const BRIDGE: &str = "carla-bridge-win32.exe";
pub(super) const DISCOVERY: &str = "carla-discovery-win32.exe";
pub(crate) const INSTALL: &str = "Download 32-bit VST2 bridge";
pub(crate) const VISIBILITY: &str = "visible_legacy-vst-setup";
const VERSION: &str = "Carla 2.5.10";
const DOWNLOAD_SIZE: u64 = 247_507_775;
const DOWNLOAD_HASH: &str = "56264cd860b3e6e7b29221fb1e46d7d8a288fb001500f830bcd739ec7b115650";
const URL: &str =
    "https://github.com/falkTX/Carla/releases/download/v2.5.10/Carla-2.5.10-win64.zip";
const FILES: [(&str, u64, &str); 3] = [
    (
        DLL,
        9_482_752,
        "9e4a5727bb631d0f8856c158042223b73731846f4161282fc41d62366f1dd5b3",
    ),
    (
        BRIDGE,
        5_511_168,
        "79514b815ba95b04d34e41074da7f4088621a6d3902d716ac4d6d8574fe0dadb",
    ),
    (
        DISCOVERY,
        4_865_536,
        "3c9871cc933eb3af323481ad6a202ee9574e0ee9abfc99f624e1965246e3d70f",
    ),
];

fn destination() -> Option<PathBuf> {
    os_paths::user_file("Components")?
        .join("Legacy VST2")
        .join(VERSION)
        .into()
}

pub(super) fn installed() -> Option<PathBuf> {
    let path = destination()?;
    FILES
        .iter()
        .all(|(name, bytes, _)| {
            fs::symlink_metadata(path.join(name))
                .is_ok_and(|meta| meta.is_file() && meta.len() == *bytes)
        })
        .then_some(path)
}

enum Status {
    Idle,
    Downloading,
    Done(Result<(), String>),
}
static JOB: Mutex<Status> = Mutex::new(Status::Idle);

pub(crate) fn status() -> &'static str {
    match &*JOB.lock().unwrap() {
        Status::Downloading => "Downloading the Carla 2.5.10 package (248 MB)...",
        _ if installed().is_some() => {
            "32-bit VST2 bridge installed. Rescan to find your 32-bit plug-ins."
        }
        _ => {
            "Optional Carla bridge for older 32-bit Windows VST2 plug-ins. Downloads 248 MB from GitHub."
        }
    }
}

pub(crate) fn request_install(app: &mut crate::app::App) {
    if installed().is_some() {
        app.start_scan(crate::plugin_db::Mode::All);
        return;
    }
    let mut job = JOB.lock().unwrap();
    if !matches!(*job, Status::Idle) {
        return;
    }
    *job = Status::Downloading;
    drop(job);
    std::thread::spawn(|| {
        let result = download().map_err(|err| err.to_string());
        *JOB.lock().unwrap() = Status::Done(result);
        os_host::wake();
    });
    app.invalidate();
}

pub(crate) fn tick(app: &mut crate::app::App) {
    let mut job = JOB.lock().unwrap();
    if !matches!(*job, Status::Done(_)) {
        return;
    }
    let Status::Done(result) = std::mem::replace(&mut *job, Status::Idle) else {
        return;
    };
    drop(job);
    match result {
        Ok(()) => app.start_scan(crate::plugin_db::Mode::All),
        Err(err) => app.state.message = Some(format!("32-bit VST2 bridge setup failed: {err}")),
    }
    app.invalidate();
}

type Error = Box<dyn std::error::Error>;

fn download() -> Result<(), Error> {
    let dest = destination().ok_or("The ML Studio profile is unavailable")?;
    let parent = dest
        .parent()
        .ok_or("Cannot locate the bridge component folder")?;
    fs::create_dir_all(parent)?;
    // Share one staging location between sessions. A setup interrupted by
    // closing the app leaves at most one package, which the next attempt reaps.
    let setup_lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(parent.join(".setup.lock"))?;
    setup_lock
        .try_lock()
        .map_err(|_| "Another ML Studio session is installing the bridge")?;
    if installed().is_some() {
        return Ok(());
    }
    let stage = parent.join(".installing");
    match fs::create_dir(&stage) {
        Ok(()) => (),
        Err(err)
            if err.kind() == io::ErrorKind::AlreadyExists
                && fs::symlink_metadata(&stage)?.file_type().is_dir() =>
        {
            clear_stage(&stage)
        }
        Err(err) => return Err(err.into()),
    }
    let result = (|| {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(15))
            .timeout_read(Duration::from_secs(30))
            .build();
        let response = agent
            .get(URL)
            .set("User-Agent", "ML-Studio-Component-Setup")
            .call()?;
        let archive = stage.join("Carla.zip");
        let mut output = File::create_new(&archive)?;
        if io::copy(
            &mut response.into_reader().take(DOWNLOAD_SIZE + 1),
            &mut output,
        )? != DOWNLOAD_SIZE
        {
            return Err("The downloaded Carla package has the wrong size".into());
        }
        output.flush()?;
        drop(output);
        unpack(&archive, &stage, &dest)
    })();
    clear_stage(&stage);
    let _ = fs::remove_dir(stage);
    result
}

fn clear_stage(stage: &Path) {
    for (file, _, _) in FILES {
        let _ = fs::remove_file(stage.join(file));
    }
    let _ = fs::remove_file(stage.join("Carla.zip"));
}

fn digest(path: &Path) -> io::Result<String> {
    let mut sha = Sha256::new();
    io::copy(&mut File::open(path)?, &mut sha)?;
    Ok(format!("{:x}", sha.finalize()))
}

pub(super) fn verify_discovery(path: &Path) -> Result<(), String> {
    let (_, _, hash) = FILES[2];
    if digest(&path.join(DISCOVERY)).ok().as_deref() != Some(hash) {
        return Err("Carla discovery failed verification".into());
    }
    Ok(())
}

pub(super) fn verify_runtime(path: &Path) -> Result<(), String> {
    for (name, _, hash) in FILES {
        if digest(&path.join(name)).ok().as_deref() != Some(hash) {
            return Err(format!("Carla component {name} failed verification"));
        }
    }
    Ok(())
}

fn unpack(archive: &Path, stage: &Path, dest: &Path) -> Result<(), Error> {
    if digest(archive)? != DOWNLOAD_HASH {
        return Err("Carla package checksum mismatch".into());
    }
    let mut bundle = zip::ZipArchive::new(File::open(archive)?)?;
    for (name, expected, _) in FILES {
        let mut member = bundle.by_name(&format!("Carla-2.5.10-win64/Carla/{name}"))?;
        if member.size() != expected {
            return Err(format!("Unexpected Carla component size for {name}").into());
        }
        let mut output = File::create_new(stage.join(name))?;
        let size = io::copy(&mut member.by_ref().take(expected + 1), &mut output)?;
        if size != expected {
            return Err(format!("Invalid Carla component length for {name}").into());
        }
        output.sync_all()?;
    }
    verify_runtime(stage)?;
    // Leave a complete previous installation usable and let any running instance keep its files.
    fs::remove_file(archive)?;
    match fs::rename(stage, dest) {
        Ok(()) => Ok(()),
        Err(_) if verify_runtime(dest).is_ok() => Ok(()),
        Err(err) => Err(format!("Could not install to {}: {err}", dest.display()).into()),
    }
}
