// SPDX-License-Identifier: GPL-3.0-or-later
//! AUv2 registered macOS instruments and insert effects.

use crate::{Descriptor, Format, HostedEffect, Instrument, Kind, Library, ParamEvent, ParamInfo, Plugin, SharedPlugin};
use std::ffi::{CStr, c_char, c_void};
use std::path::{Path, PathBuf};
use std::ptr::{self, NonNull};
use std::sync::Arc;
use os_engine::TransportInfo;

pub const REGISTRY_PATH: &str = "macOS Audio Units";
const FRAMES: usize = 4096;
const EVENT_LIMIT: usize = 2048;
const MAX_STATE: usize = 32 * 1024 * 1024;

#[repr(C)]
struct Parameter {
    id: u32,
    minimum: f32,
    maximum: f32,
    initial: f32,
    steps: u32,
    bits: u32,
    name: [c_char; 256],
}

#[repr(C)]
struct Transport {
    beat: f64,
    tempo: f64,
    bar: f64,
    frame: f64,
    loop_start: f64,
    loop_end: f64,
    next_beat: u32,
    numerator: u32,
    denominator: u32,
    playing: u32,
    cycling: u32,
}

impl Transport {
    fn at(info: &TransportInfo, rate: u32) -> Self {
        let (loop_start, loop_end) = info.looping.unwrap_or((0., 0.));
        Self {
            beat: info.beats,
            tempo: info.tempo.max(1.),
            bar: info.bar_start,
            frame: info.frame as f64,
            loop_start,
            loop_end,
            next_beat: ((info.beats.ceil() - info.beats) * info.frames_per_beat(rate)).round() as u32,
            numerator: u32::from(info.numerator.max(1)),
            denominator: u32::from(info.denominator.max(1)),
            playing: u32::from(info.playing),
            cycling: u32::from(info.looping.is_some()),
        }
    }
}

type Visitor = unsafe extern "C" fn(*mut c_void, u32, u32, u32, *const c_char, u32, bool);

unsafe extern "C" {
    fn lm_au_components(visitor: Visitor, context: *mut c_void);
    fn lm_au_new(kind: u32, subtype: u32, manufacturer: u32, status: *mut i32) -> *mut c_void;
    fn lm_au_destroy(instance: *mut c_void);
    fn lm_au_activate(instance: *mut c_void, rate: f64) -> i32;
    fn lm_au_reset(instance: *mut c_void);
    fn lm_au_bus_count(instance:*mut c_void)->u32;
    fn lm_au_render(instance: *mut c_void, bus:u32, frames: u32, left: *mut f32, right: *mut f32) -> i32;
    fn lm_au_transport(instance: *mut c_void, time: *const Transport);
    fn lm_au_midi(instance: *mut c_void, offset: u32, status: u32, data1: u32, data2: u32);
    fn lm_au_param_ids(instance: *mut c_void, ids: *mut u32, capacity: u32) -> u32;
    fn lm_au_param_info(instance: *mut c_void, id: u32, out: *mut Parameter) -> bool;
    fn lm_au_get(instance: *mut c_void, id: u32, value: *mut f32) -> bool;
    fn lm_au_set(instance: *mut c_void, id: u32, value: f32, offset: u32) -> bool;
    fn lm_au_latency(instance: *mut c_void) -> f64;
    fn lm_au_offline(instance: *mut c_void, offline: u32);
    fn lm_au_save(instance: *mut c_void, length: *mut usize, owner: *mut *mut c_void) -> *const u8;
    fn lm_au_release_data(data: *mut c_void);
    fn lm_au_load(instance: *mut c_void, bytes: *const u8, length: usize) -> bool;
    fn lm_au_has_editor(instance: *mut c_void) -> bool;
    fn lm_au_open_editor(instance: *mut c_void, parent: *mut c_void, scale: f32, width: *mut u32, height: *mut u32) -> bool;
    fn lm_au_editor_size(instance: *mut c_void, width: *mut u32, height: *mut u32) -> bool;
    fn lm_au_close_editor(instance: *mut c_void);
}

pub struct Registry {
    path: PathBuf,
    plugins: Vec<Descriptor>,
}

impl Registry {
    pub fn load(path: &Path) -> Result<Arc<Self>, String> {
        if path != Path::new(REGISTRY_PATH) {
            return Err("Audio Units are discovered through the macOS component registry".into());
        }
        let mut plugins: Vec<Descriptor> = Vec::new();
        unsafe { lm_au_components(collect, (&raw mut plugins).cast()) };
        plugins.sort_by(|a, b| a.id.cmp(&b.id));
        plugins.dedup_by(|a, b| a.id == b.id);
        Ok(Arc::new(Self { path: path.into(), plugins }))
    }
}

unsafe extern "C" fn collect(context: *mut c_void, kind: u32, subtype: u32, manufacturer: u32,
                            name: *const c_char, version: u32, instrument: bool) {
    if name.is_null() { return; }
    let list = unsafe { &mut *context.cast::<Vec<Descriptor>>() };
    let label = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    let (vendor, name) = label.split_once(':').unwrap_or(("", &label));
    let vendor = if vendor.trim().is_empty() {
        String::from_utf8_lossy(&manufacturer.to_be_bytes()).trim().to_owned()
    } else {
        vendor.trim().to_owned()
    };
    list.push(Descriptor {
        id: format!("{kind:08X}{subtype:08X}{manufacturer:08X}"),
        name: name.trim().to_owned(),
        vendor,
        version: format!("{}.{}.{}", version >> 16, (version >> 8) & 255, version & 255),
        kind: if instrument { Kind::Instrument } else { Kind::Effect },
        features: vec![],
        format: Format::AudioUnit,
        path: REGISTRY_PATH.into(),
    });
}

impl Library for Registry {
    fn path(&self) -> &Path { &self.path }
    fn plugins(&self) -> &[Descriptor] { &self.plugins }

    fn create(self: Arc<Self>, id: &str) -> Result<SharedPlugin, String> {
        let desc = self.plugins.iter().find(|p| p.id.eq_ignore_ascii_case(id))
            .ok_or_else(|| "This Audio Unit is no longer registered on this Mac".to_string())?;
        let number = |start| u32::from_str_radix(&desc.id[start..start + 8], 16).unwrap();
        let mut status = 0;
        let native = unsafe { lm_au_new(number(0), number(8), number(16), &mut status) };
        let native = NonNull::new(native).ok_or_else(|| format!("Could not open {} (Audio Unit error {status})", desc.name))?;
        let mut instance = Unit {
            native, aux:Default::default(), buses:1, desc: desc.clone(), rate: 0, requested: 0, failed_rate: 0, delay: 0, editor: false,
            notes: [false; 128], sounding: false, overflow: false,
            queue: Vec::with_capacity(EVENT_LIMIT), params: Vec::new(), observed: Vec::new(),
            transport: None,
        };
        instance.init_parameters();
        Ok(crate::share(instance, crate::guard::module_named(&self.path)))
    }
}

#[derive(Clone, Copy)]
enum Event {
    Midi([u8; 3]),
    Param(u32, f32),
}

struct Unit {
    native: NonNull<c_void>,
    aux:crate::instrument_outputs::InstrumentOutputs,
    buses:usize,
    desc: Descriptor,
    rate: u32,
    requested: u32,
    failed_rate: u32,
    delay: u32,
    editor: bool,
    notes: [bool; 128],
    sounding: bool,
    overflow: bool,
    queue: Vec<(u32, Event)>,
    params: Vec<ParamInfo>,
    observed: Vec<Option<f64>>,
    transport: Option<TransportInfo>,
}

// The host serializes calls with its existing plugin lock. Editor operations run on the UI thread.
unsafe impl Send for Unit {}

impl Drop for Unit {
    fn drop(&mut self) {
        unsafe { lm_au_destroy(self.ptr()) };
    }
}

impl Unit {
    fn ptr(&self) -> *mut c_void { self.native.as_ptr() }

    fn init_parameters(&mut self) {
        let p = self.ptr();
        let count = unsafe { lm_au_param_ids(p, ptr::null_mut(), 0) }.min(16_384);
        let mut ids = vec![0; count as usize];
        let found = unsafe { lm_au_param_ids(p, ids.as_mut_ptr(), count) };
        for id in ids.into_iter().take(found as usize) {
            let mut info: Parameter = unsafe { std::mem::zeroed() };
            if !unsafe { lm_au_param_info(p, id, &mut info) } { continue; }
            let (min, max) = (f64::from(info.minimum), f64::from(info.maximum));
            if !min.is_finite() || !max.is_finite() || max <= min || info.bits & 2 == 0 { continue; }
            let initial = f64::from(info.initial);
            self.params.push(ParamInfo {
                id, name: unsafe { CStr::from_ptr(info.name.as_ptr()) }.to_string_lossy().into_owned(),
                unit: String::new(), group: String::new(), min, max,
                default: if initial.is_finite() { ((initial - min) / (max - min)).clamp(0., 1.) } else { 0. },
                steps: info.steps, automatable: info.bits & 5 == 1, read_only: info.bits & 1 == 0,
                hidden: false, bypass: false, enumerated: info.steps > 0,
            });
        }
        self.refresh_values();
    }

    fn read(&self, param: &ParamInfo) -> Option<f64> {
        let mut value = 0.;
        (unsafe { lm_au_get(self.ptr(), param.id, &mut value) } && value.is_finite())
            .then(|| ((f64::from(value) - param.min) / (param.max - param.min)).clamp(0., 1.))
    }

    fn plain(param: &ParamInfo, normalized: f64) -> f32 {
        let n = normalized.clamp(0., 1.);
        let n = if param.steps > 0 {
            let steps = f64::from(param.steps);
            (n * steps).round() / steps
        } else { n };
        (param.min + n * (param.max - param.min)) as f32
    }

    fn refresh_values(&mut self) {
        self.observed = self.params.iter().map(|param| self.read(param)).collect();
    }

    fn queue(&mut self, offset: u32, event: Event) {
        if self.queue.len() < EVENT_LIMIT {
            self.queue.push((offset, event));
        } else {
            self.overflow = true;
        }
    }

    fn automate(&mut self, offset: u32, index: u32, value: f64) {
        if let Some(param) = self.params.get(index as usize).filter(|p| p.automatable && value.is_finite()) {
            self.queue(offset, Event::Param(index, Self::plain(param, value)));
        }
    }

    fn clear(&mut self) {
        if self.rate != 0 {
            if self.desc.is_instrument() {
                for channel in 0..16 {
                    unsafe { lm_au_midi(self.ptr(), 0, 0xb0 + channel, 120, 0) };
                }
            }
            unsafe { lm_au_reset(self.ptr()) };
        }
        self.queue.clear();
        self.notes.fill(false);
        self.sounding = false;
        self.overflow = false;
        self.transport = None;
    }

    fn render(&mut self, rate: u32, left: &mut [f32], right: &mut [f32], instrument: bool) {
        if instrument { self.aux.begin(left.len()); left.fill(0.); right.fill(0.); }
        if rate != self.rate || rate == 0 {
            if rate != self.failed_rate { self.requested = rate; }
            self.queue.clear();
            self.transport = None;
            return;
        }
        if self.overflow { self.clear(); }
        let p = self.ptr();
        let n = left.len().min(right.len());
        let time = self.transport.take().unwrap_or(TransportInfo {
            playing: false, tempo: 120., frame: 0, beats: 0., bar_start: 0., bar: 0,
            numerator: 4, denominator: 4, looping: None,
        });
        let mut auxiliary_sounding=false;
        for start in (0..n).step_by(FRAMES) {
            let end = (start + FRAMES).min(n);
            let time = if time.playing { time.advanced(start as u64, rate) } else { time };
            unsafe { lm_au_transport(p, &Transport::at(&time, rate)) };
            for &(frame, event) in &self.queue {
                let frame = (frame as usize).min(n - 1);
                if frame < start || frame >= end { continue; }
                let offset = (frame - start) as u32;
                match event {
                    Event::Midi([status, a, b]) => unsafe {
                        lm_au_midi(p, offset, u32::from(status), u32::from(a), u32::from(b));
                    },
                    Event::Param(index, value) => {
                        let i = index as usize;
                        if unsafe { lm_au_set(p, self.params[i].id, value, offset) } {
                            let param = &self.params[i];
                            self.observed[i] = Some(((f64::from(value) - param.min) / (param.max - param.min)).clamp(0., 1.));
                        }
                    }
                }
            }
            unsafe {
                lm_au_render(p, 0,(end - start) as u32, left[start..end].as_mut_ptr(), right[start..end].as_mut_ptr());
            }
            if instrument && self.buses>1 {
                let mut l=[0.;FRAMES];let mut r=[0.;FRAMES];
                for bus in 1..self.buses {
                    let n=end-start;
                    if unsafe{lm_au_render(p,bus as u32,n as u32,l.as_mut_ptr(),r.as_mut_ptr())}==0 {
                        auxiliary_sounding |= self.aux.route(bus-1,start,(&l[..n],&r[..n]),(&mut left[start..end],&mut right[start..end]));
                    }
                }
            }
        }
        self.queue.clear();
        if instrument {
            self.sounding = auxiliary_sounding || left.iter().chain(right.iter()).any(|sample| sample.abs() > 1e-6);
        }
    }
}

impl Instrument for Unit {
    fn set_split_outputs(&mut self,split:bool){self.aux.split=split;}
    fn extra_outputs(&self)->usize{self.aux.count()}
    fn extra_output(&mut self,port:usize,l:&mut[f32],r:&mut[f32])->bool{self.aux.read(port,l,r)}
    fn note_on(&mut self, offset: u32, key: u8, velocity: f32) {
        if key < 128 {
            self.notes[key as usize] = true;
            self.queue(offset, Event::Midi([0x90, key, (velocity.clamp(0., 1.) * 127.).round().max(1.) as u8]));
        }
    }
    fn note_off(&mut self, offset: u32, key: u8) {
        if key < 128 {
            self.notes[key as usize] = false;
            self.queue(offset, Event::Midi([0x80, key, 0]));
        }
    }
    fn midi(&mut self, offset: u32, bytes: [u8; 3]) { self.queue(offset, Event::Midi(bytes)); }
    fn process(&mut self, rate: u32, left: &mut [f32], right: &mut [f32]) { self.render(rate, left, right, true); }
    fn reset(&mut self) { self.clear(); }
    fn active(&self) -> bool { self.sounding || self.notes.contains(&true) || !self.queue.is_empty() }
    fn param(&mut self, offset: u32, index: u32, value: f64) { self.automate(offset, index, value); }
    fn transport(&mut self, info: &TransportInfo) { self.transport = Some(*info); }
    fn reported_delay(&self) -> u32 { self.delay }
}

impl HostedEffect for Unit {
    fn process_fx(&mut self, rate: u32, left: &mut [f32], right: &mut [f32]) { self.render(rate, left, right, false); }
    fn reset_fx(&mut self) { self.clear(); }
    fn param_fx(&mut self, offset: u32, index: u32, value: f64) { self.automate(offset, index, value); }
    fn transport_fx(&mut self, info: &TransportInfo) { self.transport = Some(*info); }
    fn reported_delay_fx(&self) -> u32 { self.delay }
}

impl Plugin for Unit {
    fn desc(&self) -> &Descriptor { &self.desc }

    fn activate(&mut self, rate: u32) -> bool {
        if self.rate == rate && rate != 0 { return true; }
        self.requested = 0;
        let status = unsafe { lm_au_activate(self.ptr(), f64::from(rate)) };
        self.rate = if status == 0 { rate } else { 0 };
        self.buses=if status==0{(unsafe{lm_au_bus_count(self.ptr())}) as usize}else{0};
        self.aux.configure(self.buses.saturating_sub(1),FRAMES);
        self.failed_rate = if status == 0 { 0 } else { rate };
        self.delay = (unsafe { lm_au_latency(self.ptr()) } * f64::from(self.rate)).round().max(0.) as u32;
        if self.params.is_empty() { self.init_parameters(); }
        status == 0
    }

    fn idle(&mut self) {
        if self.requested != 0 { self.activate(self.requested); }
        if self.rate != 0 {
            self.delay = (unsafe { lm_au_latency(self.ptr()) } * f64::from(self.rate)).round().max(0.) as u32;
        }
    }

    fn save_state(&self) -> Option<Vec<u8>> {
        let (mut length, mut owner) = (0, ptr::null_mut());
        let data = unsafe { lm_au_save(self.ptr(), &mut length, &mut owner) };
        let state = if !data.is_null() && length <= MAX_STATE {
            Some(unsafe { std::slice::from_raw_parts(data, length) }.to_vec())
        } else { None };
        unsafe { lm_au_release_data(owner) };
        state
    }

    fn load_state(&mut self, data: &[u8]) -> bool {
        let ok = !data.is_empty() && data.len() <= MAX_STATE && unsafe { lm_au_load(self.ptr(), data.as_ptr(), data.len()) };
        if ok { self.refresh_values(); }
        ok
    }

    fn set_offline(&mut self, offline: bool) { unsafe { lm_au_offline(self.ptr(), u32::from(offline)) }; }
    fn has_editor(&self) -> bool { unsafe { lm_au_has_editor(self.ptr()) } }
    fn editor_open(&self) -> bool { self.editor }
    fn open_editor(&mut self, parent: *mut c_void, scale: f32) -> Result<(u32, u32), String> {
        let (mut width, mut height) = (0, 0);
        self.editor = unsafe { lm_au_open_editor(self.ptr(), parent, scale, &mut width, &mut height) };
        if self.editor { Ok((width, height)) } else { Err("This Audio Unit could not open a Cocoa editor".into()) }
    }
    fn editor_size(&self) -> Option<(u32, u32)> {
        let (mut width, mut height) = (0, 0);
        unsafe { lm_au_editor_size(self.ptr(), &mut width, &mut height) }.then_some((width, height))
    }
    fn close_editor(&mut self) {
        self.editor = false;
        unsafe { lm_au_close_editor(self.ptr()) };
    }
    fn param_count(&self) -> usize { self.params.len() }
    fn param_info(&self, index: usize) -> Option<ParamInfo> { self.params.get(index).cloned() }
    fn param_value(&self, id: u32) -> Option<f64> { self.params.iter().find(|p| p.id == id).and_then(|p| self.read(p)) }

    fn set_param(&mut self, id: u32, value: f64) -> bool {
        let Some(index) = self.params.iter().position(|p| p.id == id && !p.read_only && value.is_finite()) else { return false };
        let value = Self::plain(&self.params[index], value);
        let ok = unsafe { lm_au_set(self.ptr(), id, value, 0) };
        if ok { self.observed[index] = self.read(&self.params[index]); }
        ok
    }

    fn take_param_events(&mut self) -> Vec<ParamEvent> {
        if !self.editor { return Vec::new(); }
        let mut events = Vec::new();
        for (i, param) in self.params.iter().enumerate().filter(|(_, p)| p.automatable) {
            let value = self.read(param);
            if let (Some(before), Some(now)) = (self.observed[i], value) {
                if (before - now).abs() > 1e-5 { events.push(ParamEvent::Value(param.id, now)); }
            }
            self.observed[i] = value;
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_audio_units_process_midi_audio_and_restore_state() {
        let registry = Registry::load(Path::new(REGISTRY_PATH)).unwrap();
        let effect_id = "617566786C7061736170706C"; // Apple's AULowpass
        let synth_id = "61756D75646C73206170706C"; // Apple's DLS Synth
        let fx = registry.clone().create(effect_id).expect("Apple low-pass effect");
        let synth = registry.clone().create(synth_id).expect("Apple DLS instrument");
        let mut synth = synth.lock().unwrap();
        assert!(synth.activate(48_000));
        let (mut left, mut right) = ([0.; 1024], [0.; 1024]);
        synth.note_on(128, 60, 0.8);
        synth.process(48_000, &mut left, &mut right);
        assert!(left[128..].iter().chain(&right[128..]).any(|v| v.abs() > 1e-5));
        synth.note_off(0, 60);
        synth.reset();
        let mut fx = fx.lock().unwrap();
        assert!(fx.activate(48_000));
        let cutoff = (0..fx.param_count()).find_map(|i| fx.param_info(i)
            .filter(|p| p.automatable && p.name.to_ascii_lowercase().contains("cutoff")))
            .expect("automatable cutoff");
        assert!(fx.set_param(cutoff.id, 0.05));
        let state = fx.save_state().expect("AU state");
        for (i, sample) in left.iter_mut().enumerate() {
            *sample = if i % 2 == 0 { 0.5 } else { -0.5 };
        }
        right.copy_from_slice(&left);
        fx.process_fx(48_000, &mut left, &mut right);
        assert!(left[256..].iter().all(|v| v.is_finite() && v.abs() < 0.1));
        let restored = registry.create(effect_id).unwrap();
        let mut restored = restored.lock().unwrap();
        assert!(restored.activate(48_000));
        assert!(restored.load_state(&state));
        assert!((restored.param_value(cutoff.id).unwrap() - 0.05).abs() < 0.001);
    }
}
