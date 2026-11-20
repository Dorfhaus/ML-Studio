// SPDX-License-Identifier: GPL-3.0-or-later
//! Optional Windows x86 VST2 hosting through the upstream Carla bridge.

mod carla;
mod setup;

use os_host::{Descriptor, Format, Kind, Library, SharedPlugin};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) use setup::{INSTALL, VISIBILITY, request_install, status, tick};

pub(crate) fn is_x86(path: &Path, format: Format) -> bool {
    format == Format::Vst2 && os_vst2::pe_machine(path) == Some("x86")
}

pub(crate) fn available() -> bool {
    setup::installed().is_some()
}

struct BridgeLibrary {
    path: PathBuf,
    runtime: PathBuf,
    descriptors: Vec<Descriptor>,
}

impl Library for BridgeLibrary {
    fn path(&self) -> &Path {
        &self.path
    }
    fn plugins(&self) -> &[Descriptor] {
        &self.descriptors
    }

    fn create(self: Arc<Self>, id: &str) -> Result<SharedPlugin, String> {
        let descriptor = self
            .descriptors
            .iter()
            .find(|desc| desc.id == id)
            .ok_or("This VST2 no longer has the scanned plug-in ID")?;
        carla::create(&self.runtime, descriptor.clone())
    }
}

pub(crate) fn load(
    path: &Path,
    descriptors: Option<&[Descriptor]>,
) -> Result<Arc<dyn Library>, String> {
    if !is_x86(path, Format::Vst2) {
        return Err(
            "The plug-in file has changed architecture. Rescan your plug-in folders.".into(),
        );
    }
    let runtime =
        setup::installed().ok_or("Install the 32-bit VST2 bridge in Settings > Plugins.")?;
    let descriptors = match descriptors {
        Some(cached) => cached.to_vec(),
        None => discover(&runtime, path)?,
    };
    Ok(Arc::new(BridgeLibrary {
        path: path.to_owned(),
        runtime,
        descriptors,
    }))
}

fn discover(runtime: &Path, path: &Path) -> Result<Vec<Descriptor>, String> {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    setup::verify_discovery(runtime)?;
    // The real loader stays in the x86 process. Always reap it, including if a DLL closes stdout or hangs.
    let mut child = Command::new(runtime.join(setup::DISCOVERY))
        .args([std::ffi::OsStr::new("vst2"), path.as_os_str()])
        .env("CARLA_DISCOVERY_NO_PROCESSING_CHECKS", "1")
        .creation_flags(0x0800_0000)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|err| format!("Carla discovery could not start: {err}"))?;
    let output = child.stdout.take().expect("piped discovery output");
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        output
            .take(1024 * 1024)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let deadline = Instant::now() + Duration::from_secs(12);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("The 32-bit plug-in discovery timed out".into());
            }
        }
    };
    if !status.success() {
        return Err(format!("32-bit discovery failed ({status})"));
    }
    // A plug-in can inherit stdout in another process. Never wait for that process to close it.
    for _ in 0..5 {
        if reader.is_finished() {
            let bytes = reader
                .join()
                .map_err(|_| "32-bit discovery reader failed")?
                .map_err(|err| err.to_string())?;
            return parse_discovery(path, &String::from_utf8_lossy(&bytes));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("32-bit discovery did not close its output".into())
}

fn parse_discovery(path: &Path, lines: &str) -> Result<Vec<Descriptor>, String> {
    let mut records = Vec::new();
    let mut fields = None;
    let mut error = None;
    for line in lines
        .lines()
        .filter_map(|line| line.strip_prefix("carla-discovery::"))
    {
        let Some((field, value)) = line.split_once("::") else {
            continue;
        };
        match field {
            "init" => fields = Some(std::collections::HashMap::new()),
            "error" => error = Some(value),
            "end" => {
                let Some(data) = fields.take() else { continue };
                let Some(id) = data
                    .get("uniqueId")
                    .and_then(|value: &&str| value.parse::<i64>().ok())
                else {
                    continue;
                };
                let num = |key| {
                    data.get(key)
                        .and_then(|value| value.parse::<u32>().ok())
                        .unwrap_or(0)
                };
                let kind = match (num("audio.ins"), num("audio.outs"), num("midi.ins")) {
                    (0, 1..=2, 1..) => Kind::Instrument,
                    (1..=2, 1..=2, _) => Kind::Effect,
                    _ => Kind::Other,
                };
                let name = data.get("name").copied().filter(|name| !name.is_empty());
                let Some(name) = name else { continue };
                records.push(Descriptor {
                    format: Format::Vst2,
                    id: format!("{:08X}", id as u32),
                    name: name.into(),
                    vendor: data.get("maker").copied().unwrap_or("").into(),
                    version: String::new(),
                    features: vec![os_host::VST2_X86.into()],
                    kind,
                    path: path.to_owned(),
                });
            }
            field => {
                if let Some(fields) = &mut fields {
                    fields.insert(field, value);
                }
            }
        }
    }
    if records.is_empty() {
        return Err(error
            .unwrap_or("Carla found no VST2 plug-in in the DLL")
            .into());
    }
    Ok(records)
}

// This test fetches the same official component the Settings button installs.
// OPENSTUDIO_USER_DIR must point to a disposable, empty profile and
// ML_STUDIO_CARLA_FIXTURES to a directory with the synthetic VST2 DLLs.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::App,
        plugin_db::{self, Mode, Status},
        plugin_menu,
    };
    use std::time::{Duration, Instant};

    fn wait_for_scan(app: &mut App, expected: impl Fn(&App) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(90);
        while !expected(app) && Instant::now() < deadline {
            tick(app);
            app.scan_tick();
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(expected(app), "{:?}", app.state.message);
    }

    #[test]
    #[ignore = "requires the official Carla download and synthetic 32-bit VST2 fixtures"]
    fn legacy_vst2_settings_scan_save_reopen_and_export() {
        let inputs =
            PathBuf::from(std::env::var_os("ML_STUDIO_CARLA_FIXTURES").expect("fixture directory"));
        let profile = os_paths::user_dir().unwrap();
        assert!(!available(), "use a fresh profile");
        let fixtures = profile.join("Test VST2");
        std::fs::create_dir_all(&fixtures).unwrap();
        for name in ["fixture32.dll", "fixture-effect32.dll"] {
            std::fs::copy(inputs.join(name), fixtures.join(name)).unwrap();
        }
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state.file = None;
        app.state.path = None;
        app.state.plugdb.prefs.standard_off = plugin_db::search_dirs(&app.state.plugdb.prefs)
            .iter()
            .map(|folder| folder.path.to_string_lossy().into_owned())
            .collect();
        app.state.plugdb.prefs.folders = vec![plugin_db::Folder {
            path: fixtures.to_string_lossy().into_owned(),
            on: true,
        }];
        app.start_scan(Mode::All);
        wait_for_scan(&mut app, |app| {
            app.state.plugdb.cache.files.iter().any(|file| {
                file.path.ends_with("fixture32.dll") && file.status == Status::Unsupported
            })
        });

        use crate::app::Action;
        use crate::views::settings::{self, SAct};
        app.open_settings(settings::PLUGINS);
        let mut canvas = os_ui::Canvas::new(1920, 1080, 1.0).unwrap();
        app.paint(&mut canvas, None);
        let download = app.hits.iter()
            .find(|hit| hit.action == Action::Settings(SAct::Button(INSTALL)))
            .expect("bridge installer must be clickable in Settings > Plugins")
            .rect;
        app.mouse_move(download.cx(), download.cy());
        app.mouse_down();
        app.mouse_up();
        wait_for_scan(&mut app, |app| {
            app.state
                .plugdb
                .lists
                .gens
                .iter()
                .any(|entry| entry.label == "Bridge Test Synth (32-bit VST2)")
        });
        let generator = app
            .state
            .plugdb
            .lists
            .gens
            .iter()
            .find(|entry| entry.name == "Bridge Test Synth")
            .unwrap()
            .index;
        let row = app.state.project.channels.len();
        plugin_menu::add_generator(&mut app, generator, None);
        app.plugin_tick();
        let descriptor = app.plugins.descriptor(row).unwrap();
        assert!(descriptor.is_vst2_x86());
        let plugin = app.plugins.hosted[&row].plugin.clone();
        {
            let mut plug = plugin.lock().unwrap();
            assert_eq!(
                plug.programs()
                    .iter()
                    .map(|p| p.2.as_str())
                    .collect::<Vec<_>>(),
                ["Quiet", "Loud"]
            );
            assert!(plug.select_program(0, 1));
            assert!(plug.set_param(0, 0.59));
        }
        let effect = app
            .state
            .plugdb
            .lists
            .fx
            .iter()
            .find(|entry| entry.name == "Bridge Test Gain")
            .unwrap()
            .index;
        let track = app.state.project.channels[row].mixer_track;
        crate::mixer::select(&mut app.state, track, false);
        plugin_menu::add_effect(&mut app, effect, Some(0));
        app.plugin_tick();
        let key = crate::plugins::fx_key(track, 0);
        assert!(
            app.plugins
                .descriptor(key)
                .is_some_and(|desc| desc.is_vst2_x86()),
            "{:?}",
            app.state.message
        );
        let effect = &app.plugins.hosted[&key].plugin;
        assert!(effect.lock().unwrap().set_param(0, 0.5));
        let channel = app.state.project.channels[row].index;
        let ppq = u32::from(app.state.project.ppq);
        app.state.project.patterns[0].notes = vec![os_model::Note {
            length: ppq * 2,
            key: 60,
            ..os_model::Note::step(channel, 0)
        }];
        crate::playlist::arr_mut(&mut app.state)
            .clips
            .push(os_model::Clip::new(
                0,
                ppq * 2,
                0,
                os_model::ClipSource::Pattern(1),
            ));
        let path = profile.join("Carla verification.lmn");
        app.save_to(path.clone());
        assert!(path.is_file(), "{:?}", app.state.message);
        let saved_state = app.plugins.state(row).expect("bridged state");
        let saved_fx = app.plugins.state(key).expect("bridged effect state");
        let raw = os_vst2::state::Saved::parse(&saved_state, true).expect("VST2 state");
        assert_eq!(
            raw,
            os_vst2::state::Saved::Chunk(0.59f32.to_le_bytes().to_vec())
        );
        let cache = app.state.plugdb.cache.clone();
        // Unit tests intentionally omit all real profile I/O in the database.
        // Persist a serialized copy for the public-style CLI export check.
        std::fs::write(
            profile.join("plugin_cache.json"),
            serde_json::to_vec(&cache).unwrap(),
        )
        .unwrap();
        drop(plugin);
        app.reset_hosted();
        drop(app);
        crate::plugins::sweep_retired();

        let mut reopened = App::new(None);
        let mut cache = cache;
        let new = fixtures.join("fixture64.dll");
        std::fs::copy(inputs.join("fixture64.dll"), &new).unwrap();
        let discovered = crate::plugin_scan::scan_here(&new, Format::Vst2);
        assert_eq!(discovered.status, Status::Ok, "{:?}", discovered);
        cache.files.push(plugin_db::FileScan {
            path: new.to_string_lossy().into_owned(),
            format: "vst2".into(),
            plugins: discovered.plugins,
            status: Status::Ok,
            size: 0,
            modified: 0,
            error: String::new(),
        });
        reopened.apply_scan(cache);
        let native = reopened
            .state
            .plugdb
            .lists
            .gens
            .iter()
            .find(|entry| entry.name == "Bridge Test Synth")
            .unwrap();
        assert!(!reopened.plugins.available[native.index].desc.is_vst2_x86());
        reopened.open(path.clone());
        reopened.sync_wrapped();
        assert!(
            reopened
                .plugins
                .descriptor(row)
                .is_some_and(|desc| desc.is_vst2_x86()),
            "{:?}",
            reopened.state.message
        );
        assert_eq!(
            reopened.plugins.state(row).as_deref(),
            Some(saved_state.as_slice())
        );
        assert_eq!(
            reopened.plugins.state(key).as_deref(),
            Some(saved_fx.as_slice())
        );
        reopened.reset_hosted();
        drop(reopened);
        crate::plugins::sweep_retired();

        // The standalone public-style executable renders the saved project using
        // the serialized scan cache, the same way an installed copy reopens it.
    }
}
