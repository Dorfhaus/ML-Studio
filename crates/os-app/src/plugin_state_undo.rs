// SPDX-License-Identifier: GPL-3.0-or-later
//! Persistent native editor edits join the ordinary project history. Only changed
//! state keys are retained; idle/parameter polling never serializes plugin assets.
use crate::app::{App, State};
use serde_json::{Map, Value};
use std::sync::{Arc, Weak, Mutex};

#[derive(Clone, Debug)]
pub struct Snapshot {
    key: usize,
    instance: Weak<Mutex<dyn os_host::Plugin>>,
    patch: String,
}
impl PartialEq for Snapshot {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && Weak::ptr_eq(&self.instance, &other.instance) && self.patch == other.patch
    }
}
fn decode(blob: &[u8]) -> Option<Value> {
    if blob.first() == Some(&b'{') { return serde_json::from_slice(blob).ok(); }
    let size = u64::from_le_bytes(blob.get(..8)?.try_into().ok()?) as usize;
    if size != blob.len().checked_sub(8)? { return None; }
    serde_json::from_slice(&blob[8..]).ok()
}
fn encode(value: &Value) -> Option<Vec<u8>> {
    let json = serde_json::to_vec(value).ok()?;
    if value.get("plugin").is_some() { return Some(json); }
    let mut blob = (json.len() as u64).to_le_bytes().to_vec();
    blob.extend(json);
    Some(blob)
}
fn valid_patch(value: &Value) -> bool {
    value.as_object().is_some_and(|root| !root.is_empty() && root.iter().all(|(section, keys)|
        matches!(section.as_str(), "fields" | "params" | "plugin") && keys.as_object().is_some_and(|keys| !keys.is_empty())))
}
impl Snapshot {
    fn plugin(&self, s: &State) -> Option<os_host::SharedPlugin> {
        let p = s.params.lists.get(&self.key)?.plugin.upgrade()?;
        Weak::ptr_eq(&Arc::downgrade(&p), &self.instance).then_some(p)
    }
    pub fn now(&self, s: &State) -> Option<Self> {
        let p = self.plugin(s)?;
        let p = p.lock().ok()?;
        let state = decode(&p.save_state()?)?;
        let shape: Value = serde_json::from_str(&self.patch).ok()?;
        let mut patch = Map::new();
        for (section, keys) in shape.as_object()? {
            let values: Map<String, Value> = keys.as_object()?.keys().map(|key|
                (key.clone(), state.get(section).and_then(|s| s.get(key)).cloned().unwrap_or(Value::Null))).collect();
            patch.insert(section.clone(), Value::Object(values));
        }
        Some(Self { key: self.key, instance: self.instance.clone(), patch: Value::Object(patch).to_string() })
    }
    pub fn put(self, s: &mut State) {
        let Some(plugin) = self.plugin(s) else { return };
        let Ok(mut p) = plugin.lock() else { return };
        let Some(mut state) = p.save_state().and_then(|blob| decode(&blob)) else { return };
        let Ok(patch) = serde_json::from_str::<Value>(&self.patch) else { return };
        for (section, keys) in patch.as_object().into_iter().flatten() {
            let Some(values) = state.get_mut(section).and_then(Value::as_object_mut) else { return };
            for (key, value) in keys.as_object().into_iter().flatten() {
                if value.is_null() { values.remove(key); } else { values.insert(key.clone(), value.clone()); }
            }
        }
        if !encode(&state).is_some_and(|blob| p.load_state(&blob)) {
            s.message = Some("The plugin could not restore this undo step".into());
            return;
        }
        // Host restoration must not echo editor gestures into a second undo step.
        p.idle();
        p.take_state_changed();
        p.take_param_events();
        if let Some(list) = s.params.lists.get_mut(&self.key) {
            list.gestures.clear();
            for param in &list.params {
                if let Some(value) = p.param_value(param.id) { list.values.get_mut().insert(param.id, value); }
            }
        }
    }
}
impl App {
    pub(crate) fn pending_plugin_state_edits(&mut self) {
        #[cfg(plugin_windows)]
        for (area, token) in crate::plugin_window::take_state_links() {
            let edit = os_hostlink::take_state_edit(token);
            let row = self.plugins.hosted.iter().find(|(_, h)| h.window.as_ref().is_some_and(|w| w.area() as isize == area)).map(|(&r, _)| r);
            if let (Some(row), Some(edit)) = (row, edit) { self.record_plugin_state_edit(row, &edit); }
        }
    }

    pub(crate) fn record_plugin_state_edit(&mut self, key: usize, patch: &str) {
        let Ok(value) = serde_json::from_str::<Value>(patch) else { return };
        if !valid_patch(&value) || patch.len() > os_hostlink::STATE_EDIT_BUDGET { return; }
        let Some(hosted) = self.plugins.hosted.get(&key) else { return };
        let instance = Arc::downgrade(&hosted.plugin);
        crate::history::native_state(&mut self.state, Snapshot { key, instance, patch: patch.to_owned() });
        self.plugins.mark_edited(key);
    }
}
pub fn bytes(snap: &crate::history::Snap) -> usize {
    use crate::history::Snap;
    match snap {
        Snap::NativeState(s) => s.patch.len(),
        Snap::MixerDelete(s) => s.bytes(),
        Snap::Several(items) => items.iter().map(bytes).sum(),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app(name: &str) -> (App, usize) {
        let mut app = App::new(None);
        app.plugins = crate::plugins::Plugins::builtins();
        let index = app.plugins.index_of(name).unwrap();
        let row = crate::app::add_plugin_channel(&mut app.state.project, name);
        app.plugins.create(row, index).unwrap();
        app.state.hosted_rows.push(row);
        app.params_tick();
        (app, row)
    }
    fn state(app: &App, row: usize) -> Value {
        decode(&app.plugins.hosted[&row].plugin.lock().unwrap().save_state().unwrap()).unwrap()
    }
    fn load(app: &mut App, row: usize, value: &Value) {
        let mut p = app.plugins.hosted[&row].plugin.lock().unwrap();
        assert!(p.load_state(&encode(value).unwrap()));
        p.idle();
    }
    #[test]
    fn native_citrus_edit_uses_standard_undo_and_redo_in_both_modes() {
        for alternate in [true, false] {
            let (mut app, row) = app(crate::citrus::CITRUS);
            crate::prefs::set_check(&crate::prefs::ALTERNATE_UNDO, alternate);
            let before = state(&app, row);
            let params = Arc::new(citrus::CitrusParams::default());
            let mut logic = citrus::editor::Logic::new(params.clone(), Arc::new(std::sync::atomic::AtomicBool::new(false)));
            logic.typed(&citrus::editor::TypeTarget::Field(citrus::ctl::Field::Soften), "50%");
            let patch = logic.take_state_edit().expect("native type-in sends undo");
            let mut after = before.clone();
            after["fields"][citrus::STATE_FIELD] = Value::String(serde_json::to_string(&*params.state.read().unwrap()).unwrap());
            assert_ne!(before["fields"], after["fields"]);
            load(&mut app, row, &after);
            app.record_plugin_state_edit(row, &patch);
            app.plugin_undo_key(false, false);
            assert_eq!(state(&app, row)["fields"], before["fields"], "Ctrl+Z restores Citrus");
            app.plugin_undo_key(false, true);
            assert_eq!(state(&app, row)["fields"], after["fields"], "Ctrl+Shift+Z redoes even with alternate mode off");
            let count = app.state.history.entries().len();
            for _ in 0..3 { app.params_tick(); }
            assert_eq!(app.state.history.entries().len(), count, "restore does not echo edits");
        }
        crate::prefs::set_check(&crate::prefs::ALTERNATE_UNDO, true);
    }
    #[test]
    fn native_wave_preset_state_undo_preserves_unrelated_keys_and_instance() {
        let (mut app, row) = app("ML Wave");
        let before = state(&app, row);
        let preset = serde_json::to_value(lemon_wave::table_editor::TableEditor::factory_state(1)).unwrap();
        let mut after = before.clone();
        after["fields"] = preset["fields"].clone();
        after["params"] = preset["params"].clone();
        load(&mut app, row, &after);
        let after = state(&app, row); // the plugin normalizes optional preset fields
        let mut patch = Map::new();
        for section in ["fields", "params"] {
            let old = before[section].as_object().unwrap();
            let new = after[section].as_object().unwrap();
            let keys: std::collections::BTreeSet<_> = old.keys().chain(new.keys()).collect();
            let changes: Map<String, Value> = keys.into_iter().filter(|key| old.get(*key) != new.get(*key))
                .map(|key| (key.clone(), old.get(key).cloned().unwrap_or(Value::Null))).collect();
            if !changes.is_empty() { patch.insert(section.into(), Value::Object(changes)); }
        }
        let patch = Value::Object(patch).to_string();
        assert_ne!(patch, "{}");
        app.record_plugin_state_edit(row, &patch);
        app.plugin_undo_key(false, false);
        assert_eq!(state(&app, row)["fields"], before["fields"]);
        app.plugin_undo_key(false, true);
        assert_eq!(state(&app, row)["fields"], after["fields"]);
        let curve_before = state(&app, row);
        let mut curve = lemon_wave::modulation::Curves::decode(curve_before["fields"][lemon_wave::modulation::FIELD].as_str().unwrap()).unwrap();
        curve.samples[2][8] = 0.37;
        let mut curve_after = curve_before.clone();
        curve_after["fields"][lemon_wave::modulation::FIELD] = Value::String(curve.encode());
        load(&mut app, row, &curve_after);
        let patch = serde_json::json!({"fields": {lemon_wave::modulation::FIELD: curve_before["fields"][lemon_wave::modulation::FIELD]}}).to_string();
        app.record_plugin_state_edit(row, &patch);
        app.plugin_undo_key(false, false);
        assert_eq!(state(&app, row)["fields"], curve_before["fields"], "Wave curve undo");
        assert_eq!(state(&app, row)["params"], curve_before["params"], "unrelated controls preserved");
        app.plugin_undo_key(false, true);
        assert_eq!(state(&app, row)["fields"], curve_after["fields"], "Wave curve redo");
        let replacement = app.plugins.index_of(crate::citrus::CITRUS).unwrap();
        app.plugins.create(row, replacement).unwrap();
        app.params_tick();
        let untouched = state(&app, row);
        app.plugin_undo_key(false, false);
        assert_eq!(state(&app, row), untouched, "stale state never loads into a replacement plugin");
    }
    #[test]
    #[ignore = "requires the separately built device-free Plasma editor fixture"]
    fn native_plasma_asset_and_bulk_edits_use_host_ctrl_z() {
        let path = std::path::PathBuf::from(std::env::var_os("ML_PLASMA_EDITOR_TEST_LIBRARY").expect("Plasma fixture"));
        let module = unsafe { os_host::load_module(&path) }.unwrap();
        let edit = unsafe { *module.get::<unsafe extern "C" fn(i32) -> bool>(b"ml_plasma_editor_test_edit\0").unwrap() };
        let library = os_clap::Library::load(&path).unwrap();
        let descriptor = library.plugins[0].clone();
        let mut app = App::new(None);
        app.plugins = crate::plugins::Plugins::default();
        app.plugins.available.push(crate::plugins::Available { lib: library, desc: descriptor });
        let row = crate::app::add_plugin_channel(&mut app.state.project, ml_plasma::NAME);
        app.plugins.create(row, 0).unwrap();
        app.state.hosted_rows.push(row);
        assert!(app.plugins.hosted[&row].plugin.lock().unwrap().state_undo_supported());
        app.params_tick();
        assert!(unsafe { edit(0) }); // seed saved custom table
        app.plugins.hosted[&row].plugin.lock().unwrap().idle();
        app.params_tick();
        assert!(unsafe { edit(7) }, "native editor size survives state reload");
        assert!(unsafe { edit(8) }, "popup slider accepts a fractional drag from zero percent");
        for command in [1, 2, 6] { // asset-only, bulk edits, and filter send controls
            let before = state(&app, row);
            let count = app.state.history.entries().len();
            assert!(unsafe { edit(command) });
            app.plugins.hosted[&row].plugin.lock().unwrap().idle();
            app.params_tick();
            let after = state(&app, row);
            assert_ne!(before["plugin"], after["plugin"]);
            assert_eq!(app.state.history.entries().len(), count + 1, "one grouped edit");
            app.plugin_undo_key(false, false);
            assert_eq!(state(&app, row)["plugin"], before["plugin"], "host Ctrl+Z command {command}");
            app.plugin_undo_key(false, true);
            assert_eq!(state(&app, row)["plugin"], after["plugin"], "host redo command {command}");
            app.params_tick();
            assert_eq!(app.state.history.entries().len(), count + 1, "no restore echo");
        }
    }

}
