// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows x64 host-side adapter to the pinned Carla 2.5.10 host-plugin DLL.
//! ABI declarations follow Carla's GPL-2.0-or-later CarlaNative.h and CarlaHost.h
//! (Filipe Coelho and contributors, upstream tag v2.5.10).

use std::ffi::{CStr, CString, c_char, c_void};
use std::path::Path;
use std::ptr::null_mut;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use os_engine::{HostedEffect, Instrument, TransportInfo};
use os_host::{Descriptor, ParamInfo, Plugin, SharedPlugin};
use os_vst2::state::Saved;

type Handle = *mut c_void;
type EngineCallback = unsafe extern "C" fn(Handle, u32, u32, i32, i32, i32, f32, *const c_char);

unsafe extern "C" fn callback(
    context: Handle,
    action: u32,
    _: u32,
    value: i32,
    _: i32,
    _: i32,
    _: f32,
    _: *const c_char,
) {
    const UI_STATE_CHANGED: u32 = 12;
    if action == UI_STATE_CHANGED {
        unsafe { &*context.cast::<AtomicBool>() }.store(value > 0, Ordering::Release);
        os_host::wake();
    }
}
type Dispatch = unsafe extern "C" fn(Handle, i32, i32, isize, Handle, f32) -> isize;
const FRAMES: usize = 4096;
const MAX_EVENTS: usize = 512;

#[repr(C)]
#[derive(Default)]
struct Beats {
    valid: bool,
    bar: i32,
    beat: i32,
    tick: f64,
    bar_start_tick: f64,
    beats_per_bar: f32,
    beat_type: f32,
    ticks_per_beat: f64,
    bpm: f64,
}

#[repr(C)]
#[derive(Default)]
struct Time {
    playing: bool,
    frame: u64,
    microseconds: u64,
    beats: Beats,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Midi {
    time: u32,
    port: u8,
    size: u8,
    data: [u8; 4],
}

struct Clock {
    rate: u32,
    offline: bool,
    time: Time,
}
unsafe extern "C" fn buffer(_: Handle) -> u32 {
    FRAMES as u32
}
unsafe extern "C" fn rate(clock: Handle) -> f64 {
    unsafe { f64::from((*clock.cast::<Clock>()).rate) }
}
unsafe extern "C" fn offline(clock: Handle) -> bool {
    unsafe { (*clock.cast::<Clock>()).offline }
}
unsafe extern "C" fn time(clock: Handle) -> *const Time {
    unsafe { &(*clock.cast::<Clock>()).time }
}
unsafe extern "C" fn midi_out(_: Handle, _: *const Midi) -> bool {
    false
}
unsafe extern "C" fn host_dispatch(
    _: Handle,
    _: i32,
    _: i32,
    _: isize,
    _: Handle,
    _: f32,
) -> isize {
    0
}

#[repr(C)]
struct Host {
    clock: Handle,
    resource_dir: *const c_char,
    name: *const c_char,
    parent: usize,
    buffer: unsafe extern "C" fn(Handle) -> u32,
    rate: unsafe extern "C" fn(Handle) -> f64,
    offline: unsafe extern "C" fn(Handle) -> bool,
    time: unsafe extern "C" fn(Handle) -> *const Time,
    midi_out: unsafe extern "C" fn(Handle, *const Midi) -> bool,
    ui_callbacks: [Handle; 6],
    dispatch: Dispatch,
}

#[repr(C)]
struct Native {
    metadata: [u32; 9],
    strings: [*const c_char; 4],
    instantiate: unsafe extern "C" fn(*const Host) -> Handle,
    cleanup: unsafe extern "C" fn(Handle),
    parameter_and_editor: [Handle; 13],
    activate: unsafe extern "C" fn(Handle),
    deactivate: unsafe extern "C" fn(Handle),
    process: unsafe extern "C" fn(Handle, *mut *mut f32, *mut *mut f32, u32, *const Midi, u32),
    state: [Handle; 2],
    dispatch: Dispatch,
}

#[repr(C)]
struct Info {
    kind: u32,
    category: u32,
    hints: u32,
    available: u32,
    enabled: u32,
}
#[repr(C)]
struct Parameter {
    name: *const c_char,
    symbol: *const c_char,
    unit: *const c_char,
}

macro_rules! functions {
    ($($name:ident: $ty:ty),+ $(,)?) => {
        struct Api { _library: libloading::Library, $( $name: $ty ),+ }
        impl Api {
            fn load(runtime: &Path) -> Result<Self, String> {
                super::setup::verify_runtime(runtime)?;
                let library = unsafe { os_host::load_module(&runtime.join(super::setup::DLL))? };
                $(let $name = *unsafe { library.get::<$ty>(concat!(stringify!($name), "\0").as_bytes()) }
                    .map_err(|err| format!("Carla API {}: {err}", stringify!($name)))?;)+
                Ok(Self { _library: library, $($name),+ })
            }
        }
    };
}

functions! {
    carla_get_native_rack_plugin: unsafe extern "C" fn() -> *const Native,
    carla_create_native_plugin_host_handle: unsafe extern "C" fn(*const Native, Handle) -> Handle,
    carla_host_handle_free: unsafe extern "C" fn(Handle),
    carla_set_engine_callback: unsafe extern "C" fn(Handle, Option<EngineCallback>, Handle),
    carla_set_engine_option: unsafe extern "C" fn(Handle, u32, i32, *const c_char),
    carla_add_plugin: unsafe extern "C" fn(Handle, u32, u32, *const c_char, *const c_char, *const c_char, i64, Handle, u32) -> bool,
    carla_get_last_error: unsafe extern "C" fn(Handle) -> *const c_char,
    carla_get_plugin_info: unsafe extern "C" fn(Handle, u32) -> *const Info,
    carla_set_active: unsafe extern "C" fn(Handle, u32, bool),
    carla_get_program_count: unsafe extern "C" fn(Handle, u32) -> u32,
    carla_get_current_program_index: unsafe extern "C" fn(Handle, u32) -> i32,
    carla_get_program_name: unsafe extern "C" fn(Handle, u32, u32) -> *const c_char,
    carla_set_program: unsafe extern "C" fn(Handle, u32, u32),
    carla_get_parameter_count: unsafe extern "C" fn(Handle, u32) -> u32,
    carla_get_parameter_info: unsafe extern "C" fn(Handle, u32, u32) -> *const Parameter,
    carla_get_current_parameter_value: unsafe extern "C" fn(Handle, u32, u32) -> f32,
    carla_set_parameter_value: unsafe extern "C" fn(Handle, u32, u32, f32),
    carla_prepare_for_save: unsafe extern "C" fn(Handle, u32),
    carla_get_chunk_data: unsafe extern "C" fn(Handle, u32) -> *const c_char,
    carla_set_chunk_data: unsafe extern "C" fn(Handle, u32, *const c_char),
    carla_get_plugin_latency: unsafe extern "C" fn(Handle, u32) -> u32,
    carla_show_custom_ui: unsafe extern "C" fn(Handle, u32, bool),
    carla_juce_init: unsafe extern "C" fn(),
    carla_juce_idle: unsafe extern "C" fn(),
}

fn string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

struct Engine {
    api: Arc<Api>,
    native: &'static Native,
    instance: Handle,
    engine: Handle,
    _host: Box<Host>,
    _resources: CString,
    clock: Box<Clock>,
    active: bool,
    editor: Arc<AtomicBool>,
}

// The allocated descriptor, host callbacks and clock remain valid for the engine lifetime.
// Each instance is serialized by the mutex shared between the UI and its processing worker.
unsafe impl Send for Engine {}

impl Engine {
    fn new(runtime: &Path, desc: &Descriptor) -> Result<Self, String> {
        static API: OnceLock<Arc<Api>> = OnceLock::new();
        let api = match API.get() {
            Some(api) => api.clone(),
            None => {
                let api = Arc::new(Api::load(runtime)?);
                unsafe {
                    (api.carla_juce_init)();
                }
                API.get_or_init(|| api).clone()
            }
        };
        let resources =
            CString::new(runtime.to_string_lossy().as_bytes()).map_err(|err| err.to_string())?;
        let path =
            CString::new(desc.path.to_string_lossy().as_bytes()).map_err(|err| err.to_string())?;
        let id = u32::from_str_radix(&desc.id, 16).map_err(|err| err.to_string())? as i32;
        let mut clock = Box::new(Clock {
            rate: 48000,
            offline: false,
            time: Time::default(),
        });
        let host = Box::new(Host {
            clock: (&mut *clock as *mut Clock).cast(),
            resource_dir: resources.as_ptr(),
            name: c"ML Studio".as_ptr(),
            parent: 0,
            buffer,
            rate,
            offline,
            time,
            midi_out,
            ui_callbacks: [null_mut(); 6],
            dispatch: host_dispatch,
        });
        unsafe {
            let native = (api.carla_get_native_rack_plugin)()
                .as_ref()
                .ok_or("Carla has no rack descriptor")?;
            let instance = (native.instantiate)(&*host);
            if instance.is_null() {
                return Err("Carla could not initialize the bridge host".into());
            }
            let engine = (api.carla_create_native_plugin_host_handle)(native, instance);
            let editor = Arc::new(AtomicBool::new(false));
            let result = Self {
                api,
                native,
                instance,
                engine,
                _host: host,
                _resources: resources,
                clock,
                active: false,
                editor,
            };
            if engine.is_null() {
                return Err("Carla could not create the bridge engine".into());
            }
            (result.api.carla_set_engine_callback)(
                engine,
                Some(callback),
                Arc::as_ptr(&result.editor).cast_mut().cast(),
            );
            (result.api.carla_set_engine_option)(engine, 20, 0, result._resources.as_ptr());
            (result.api.carla_set_engine_option)(engine, 21, 0, result._resources.as_ptr());
            if !(result.api.carla_add_plugin)(
                engine,
                3,
                5,
                path.as_ptr(),
                null_mut(),
                null_mut(),
                i64::from(id),
                null_mut(),
                8,
            ) {
                return Err(format!(
                    "Carla could not load {}: {}",
                    desc.name,
                    string((result.api.carla_get_last_error)(engine))
                ));
            }
            Ok(result)
        }
    }

    fn activate(&mut self, rate: u32) {
        if self.active && self.clock.rate == rate {
            return;
        }
        unsafe {
            if self.active {
                (self.native.deactivate)(self.instance);
            }
            if self.clock.rate != rate {
                self.clock.rate = rate;
                (self.native.dispatch)(self.instance, 2, 0, 0, null_mut(), rate as f32);
            }
            (self.native.activate)(self.instance);
            (self.api.carla_set_active)(self.engine, 0, true);
        }
        self.active = true;
    }

    fn handle_block(&mut self, block: &mut Block) {
        self.activate(block.rate);
        if self.clock.offline != block.offline {
            self.clock.offline = block.offline;
            unsafe {
                (self.native.dispatch)(
                    self.instance,
                    3,
                    0,
                    isize::from(block.offline),
                    null_mut(),
                    0.0,
                );
            }
        }
        self.clock.time = block
            .song
            .as_ref()
            .map(|song| {
                let unit = f64::from(song.denominator.max(1)) / 4.0;
                let beat = (song.beats - song.bar_start).max(0.0) * unit;
                Time {
                    playing: song.playing,
                    frame: song.frame,
                    microseconds: song.frame.saturating_mul(1_000_000)
                        / u64::from(block.rate.max(1)),
                    beats: Beats {
                        valid: true,
                        bar: song.bar.saturating_add(1),
                        beat: beat.floor() as i32 + 1,
                        tick: beat.fract() * 960.0,
                        bar_start_tick: song.bar_start * unit * 960.0,
                        beats_per_bar: f32::from(song.numerator),
                        beat_type: f32::from(song.denominator),
                        ticks_per_beat: 960.0,
                        bpm: song.tempo * unit,
                    },
                }
            })
            .unwrap_or_default();
        for &(id, value) in &block.params {
            unsafe {
                (self.api.carla_set_parameter_value)(self.engine, 0, id, value);
            }
        }
        let mut input = [block.input[0].as_mut_ptr(), block.input[1].as_mut_ptr()];
        let mut output = [block.output[0].as_mut_ptr(), block.output[1].as_mut_ptr()];
        unsafe {
            (self.native.process)(
                self.instance,
                input.as_mut_ptr(),
                output.as_mut_ptr(),
                block.frames as u32,
                block.events.as_ptr(),
                block.events.len() as u32,
            );
        }
    }

    fn save_chunk(&self) -> Option<Vec<u8>> {
        unsafe {
            (self.api.carla_prepare_for_save)(self.engine, 0);
            let encoded = string((self.api.carla_get_chunk_data)(self.engine, 0));
            let chunk = STANDARD
                .decode(encoded)
                .ok()
                .filter(|bytes| !bytes.is_empty())?;
            let state = os_vst2::fxp::parse(&chunk)
                .and_then(|mut bank| bank.programs.pop().map(|(_, state)| state))
                .unwrap_or(Saved::Chunk(chunk));
            Some(state.to_bytes())
        }
    }

    fn info(&self) -> (bool, bool) {
        unsafe { (self.api.carla_get_plugin_info)(self.engine, 0).as_ref() }
            .map_or((false, false), |info| {
                (info.hints & 8 != 0, info.enabled & 8 != 0)
            })
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            if self.active {
                (self.native.deactivate)(self.instance);
            }
            if !self.engine.is_null() {
                (self.api.carla_host_handle_free)(self.engine);
            }
            (self.native.cleanup)(self.instance);
        }
    }
}

struct Block {
    input: [[f32; FRAMES]; 2],
    output: [[f32; FRAMES]; 2],
    events: Vec<Midi>,
    params: Vec<(u32, f32)>,
    frames: usize,
    rate: u32,
    offline: bool,
    song: Option<TransportInfo>,
}

enum Work {
    Audio(Box<Block>),
    SaveChunk(mpsc::Sender<Option<Vec<u8>>>),
    Stop,
}

struct Bridge {
    desc: Descriptor,
    engine: Arc<Mutex<Engine>>,
    work: mpsc::SyncSender<Work>,
    worker: Option<std::thread::JoinHandle<()>>,
    result: mpsc::Receiver<Box<Block>>,
    block: Option<Box<Block>>,
    events: Vec<Midi>,
    parameters: Vec<(u32, f32)>,
    song: Option<TransportInfo>,
    offline: bool,
    failed: bool,
    waiting_since: Option<Instant>,
    editor: Arc<AtomicBool>,
    has_editor: bool,
    chunks: bool,
    latency: u32,
    param_info: Vec<ParamInfo>,
}

pub(super) fn create(runtime: &Path, desc: Descriptor) -> Result<SharedPlugin, String> {
    let engine = Engine::new(runtime, &desc)?;
    let (has_editor, chunks) = engine.info();
    let editor = engine.editor.clone();
    let latency = unsafe { (engine.api.carla_get_plugin_latency)(engine.engine, 0) };
    let param_info = (0..unsafe { (engine.api.carla_get_parameter_count)(engine.engine, 0) }
        .min(65536))
        .map(|id| {
            let info =
                unsafe { (engine.api.carla_get_parameter_info)(engine.engine, 0, id).as_ref() };
            ParamInfo {
                id,
                name: info.map_or_else(|| format!("Parameter {}", id + 1), |p| string(p.name)),
                unit: info.map_or_else(String::new, |p| string(p.unit)),
                group: String::new(),
                min: 0.0,
                max: 1.0,
                default: f64::from(unsafe {
                    (engine.api.carla_get_current_parameter_value)(engine.engine, 0, id)
                }),
                steps: 0,
                automatable: true,
                read_only: false,
                hidden: false,
                bypass: false,
                enumerated: false,
            }
        })
        .collect();
    let engine = Arc::new(Mutex::new(engine));
    let worker_engine = engine.clone();
    let (work, jobs) = mpsc::sync_channel::<Work>(1);
    let (done, result) = mpsc::sync_channel(1);
    let worker = std::thread::Builder::new()
        .name("Carla VST2 audio".into())
        .spawn(move || {
            while let Ok(job) = jobs.recv() {
                let Ok(mut engine) = worker_engine.lock() else {
                    break;
                };
                match job {
                    Work::Audio(mut block) => {
                        engine.handle_block(&mut block);
                        drop(engine);
                        if done.send(block).is_err() {
                            break;
                        }
                    }
                    Work::SaveChunk(reply) => {
                        let state = engine.save_chunk();
                        drop(engine);
                        let _ = reply.send(state);
                    }
                    Work::Stop => break,
                }
            }
        })
        .map_err(|err| format!("Could not start the Carla audio worker: {err}"))?;
    let block = Box::new(Block {
        input: [[0.; FRAMES]; 2],
        output: [[0.; FRAMES]; 2],
        events: Vec::with_capacity(MAX_EVENTS),
        params: Vec::with_capacity(MAX_EVENTS),
        frames: 0,
        rate: 48000,
        offline: false,
        song: None,
    });
    Ok(Arc::new(Mutex::new(Bridge {
        desc,
        engine,
        work,
        worker: Some(worker),
        result,
        block: Some(block),
        events: Vec::with_capacity(MAX_EVENTS),
        parameters: Vec::with_capacity(MAX_EVENTS),
        song: None,
        offline: false,
        failed: false,
        waiting_since: None,
        editor,
        has_editor,
        chunks,
        latency,
        param_info,
    })))
}

impl Drop for Bridge {
    fn drop(&mut self) {
        if self.work.try_send(Work::Stop).is_err() {
            return;
        }
        let Some(worker) = self.worker.take() else {
            return;
        };
        let deadline = Instant::now() + Duration::from_millis(100);
        while !worker.is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        if worker.is_finished() {
            // An idle worker releases its Engine reference before the UI
            // tears down the Carla instance and its child process.
            let _ = worker.join();
        }
    }
}

impl Bridge {
    fn queue_param(&mut self, id: u32, value: f64) {
        if (id as usize) < self.param_info.len() && self.parameters.len() < MAX_EVENTS {
            self.parameters.push((id, value.clamp(0., 1.) as f32));
        }
    }

    fn render(&mut self, rate: u32, left: &mut [f32], right: &mut [f32], effect: bool) {
        if !effect {
            left.fill(0.0);
            right.fill(0.0);
        }
        let song = self.song.take();
        if let Some(since) = self.waiting_since {
            match self.result.try_recv() {
                Ok(block) => {
                    self.block = Some(block);
                    self.waiting_since = None;
                    // Discard the late audio. Release notes whose note-off may have
                    // arrived while the bridge was still processing the old block.
                    for channel in 0..16 {
                        if self.events.len() == MAX_EVENTS {
                            break;
                        }
                        self.events.insert(
                            0,
                            Midi {
                                time: 0,
                                port: 0,
                                size: 3,
                                data: [0xb0 | channel, 120, 0, 0],
                            },
                        );
                    }
                }
                Err(_) => {
                    self.failed = since.elapsed() >= Duration::from_secs(1);
                    self.events.clear();
                    self.parameters.clear();
                    return;
                }
            }
        }
        for (index, (left, right)) in left
            .chunks_mut(FRAMES)
            .zip(right.chunks_mut(FRAMES))
            .enumerate()
        {
            let Some(mut block) = self.block.take() else {
                break;
            };
            let offset = (index * FRAMES) as u32;
            block.frames = left.len();
            block.rate = rate;
            block.offline = self.offline;
            block.song = song.map(|timing| timing.advanced(u64::from(offset), rate));
            block.input[0][..left.len()].copy_from_slice(left);
            block.input[1][..right.len()].copy_from_slice(right);
            block.output[0][..left.len()].fill(0.);
            block.output[1][..right.len()].fill(0.);
            block.events.clear();
            for event in &self.events {
                if let Some(time) = event
                    .time
                    .checked_sub(offset)
                    .filter(|&n| n < left.len() as u32)
                {
                    block.events.push(Midi { time, ..*event });
                }
            }
            block.params.clear();
            if index == 0 {
                block.params.extend_from_slice(&self.parameters);
            }
            if self.work.try_send(Work::Audio(block)).is_err() {
                self.failed = true;
                break;
            }
            // A bridge hang may take a second inside Carla. Bound our own wait to
            // one audio block, with Windows scheduling margin for very short blocks.
            let timeout = if self.offline {
                Duration::from_secs(3)
            } else {
                Duration::from_secs_f64(
                    (left.len() as f64 / f64::from(rate.max(1))).clamp(0.003, 0.025),
                )
            };
            match self.result.recv_timeout(timeout) {
                Ok(block) => {
                    left.copy_from_slice(&block.output[0][..left.len()]);
                    right.copy_from_slice(&block.output[1][..right.len()]);
                    self.block = Some(block);
                }
                Err(mpsc::RecvTimeoutError::Timeout) if !self.offline => {
                    self.waiting_since = Some(Instant::now());
                    break;
                }
                Err(_) => {
                    self.failed = true;
                    break;
                }
            }
        }
        self.events.clear();
        self.parameters.clear();
    }
}

impl Instrument for Bridge {
    fn note_on(&mut self, offset: u32, key: u8, velocity: f32) {
        self.midi(
            offset,
            [
                0x90,
                key.min(127),
                (velocity * 127.).round().clamp(1., 127.) as u8,
            ],
        );
    }
    fn note_off(&mut self, offset: u32, key: u8) {
        self.midi(offset, [0x80, key.min(127), 0]);
    }
    fn midi(&mut self, time: u32, bytes: [u8; 3]) {
        if self.events.len() < MAX_EVENTS {
            let size = if matches!(bytes[0] & 0xf0, 0xc0 | 0xd0) {
                2
            } else {
                3
            };
            self.events.push(Midi {
                time,
                port: 0,
                size,
                data: [bytes[0], bytes[1], bytes[2], 0],
            });
        }
    }
    fn process(&mut self, rate: u32, left: &mut [f32], right: &mut [f32]) {
        self.render(rate, left, right, false);
    }
    fn active(&self) -> bool {
        !self.failed
    }
    fn transport(&mut self, song: &TransportInfo) {
        self.song = Some(*song);
    }
    fn param(&mut self, _: u32, id: u32, value: f64) {
        self.queue_param(id, value);
    }
    fn tick_params(&self) -> bool {
        true
    }
    fn reported_delay(&self) -> u32 {
        self.latency
    }
    fn reset(&mut self) {
        self.events.clear();
        self.parameters.clear();
        for channel in 0..16 {
            self.midi(0, [0xb0 | channel, 120, 0]);
        }
    }
}

impl HostedEffect for Bridge {
    fn process_fx(&mut self, rate: u32, left: &mut [f32], right: &mut [f32]) {
        self.render(rate, left, right, true);
    }
    fn param_fx(&mut self, _: u32, id: u32, value: f64) {
        self.queue_param(id, value);
    }
    fn tick_params_fx(&self) -> bool {
        true
    }
    fn transport_fx(&mut self, song: &TransportInfo) {
        self.song = Some(*song);
    }
    fn reset_fx(&mut self) {
        self.reset();
    }
    fn reported_delay_fx(&self) -> u32 {
        self.latency
    }
}

impl Plugin for Bridge {
    fn desc(&self) -> &Descriptor {
        &self.desc
    }
    fn activate(&mut self, rate: u32) -> bool {
        if let Ok(mut engine) = self.engine.try_lock() {
            engine.activate(rate);
            return true;
        }
        false
    }
    fn idle(&mut self) {
        if let Ok(engine) = self.engine.try_lock() {
            unsafe {
                (engine.api.carla_juce_idle)();
            }
        }
    }
    fn latency(&self) -> u32 {
        self.latency
    }
    fn stopped(&self) -> Option<u32> {
        self.failed.then_some(0x00000102)
    }
    fn set_offline(&mut self, offline: bool) {
        self.offline = offline;
    }
    fn has_editor(&self) -> bool {
        self.has_editor
    }
    fn editor_open(&self) -> bool {
        self.editor.load(Ordering::Acquire)
    }
    fn open_editor(&mut self, parent: Handle, _: f32) -> Result<(u32, u32), String> {
        if !self.has_editor {
            return Err("The plug-in has no native editor".into());
        }
        if !parent.is_null() {
            return Err("32-bit plug-in editors open in their own window".into());
        }
        let engine = self.engine.try_lock().map_err(|_| "Carla is unavailable")?;
        self.editor.store(true, Ordering::Release);
        unsafe {
            (engine.api.carla_show_custom_ui)(engine.engine, 0, true);
        }
        Ok((0, 0))
    }
    fn editor_size(&self) -> Option<(u32, u32)> {
        None
    }
    fn close_editor(&mut self) {
        if self.editor.swap(false, Ordering::AcqRel) {
            if let Ok(engine) = self.engine.try_lock() {
                unsafe {
                    (engine.api.carla_show_custom_ui)(engine.engine, 0, false);
                }
            }
        }
    }
    fn programs(&self) -> Vec<(u32, u32, String)> {
        let Ok(engine) = self.engine.try_lock() else {
            return Vec::new();
        };
        let count = unsafe { (engine.api.carla_get_program_count)(engine.engine, 0) };
        (0..count.min(16384))
            .map(|i| {
                let name =
                    string(unsafe { (engine.api.carla_get_program_name)(engine.engine, 0, i) });
                (
                    0,
                    i,
                    if name.is_empty() {
                        format!("Program {}", i + 1)
                    } else {
                        name
                    },
                )
            })
            .collect()
    }
    fn select_program(&mut self, _: u32, index: u32) -> bool {
        let Ok(engine) = self.engine.try_lock() else {
            return false;
        };
        unsafe {
            if index >= (engine.api.carla_get_program_count)(engine.engine, 0) {
                return false;
            }
            (engine.api.carla_set_program)(engine.engine, 0, index);
        }
        true
    }
    fn param_count(&self) -> usize {
        self.param_info.len()
    }
    fn param_info(&self, index: usize) -> Option<ParamInfo> {
        self.param_info.get(index).cloned()
    }
    fn param_value(&self, id: u32) -> Option<f64> {
        self.param_info.get(id as usize)?;
        let engine = self.engine.try_lock().ok()?;
        Some(f64::from(unsafe {
            (engine.api.carla_get_current_parameter_value)(engine.engine, 0, id)
        }))
    }
    fn set_param(&mut self, id: u32, value: f64) -> bool {
        if self.param_info.get(id as usize).is_none() {
            return false;
        }
        let Ok(engine) = self.engine.try_lock() else {
            return false;
        };
        unsafe {
            (engine.api.carla_set_parameter_value)(
                engine.engine,
                0,
                id,
                value.clamp(0., 1.) as f32,
            );
        }
        true
    }
    fn save_state(&self) -> Option<Vec<u8>> {
        if self.chunks {
            let (reply, state) = mpsc::channel();
            self.work.try_send(Work::SaveChunk(reply)).ok()?;
            // Carla can wait up to a minute on a stalled plug-in. Let the project
            // save use its existing snapshot if the worker cannot reply promptly.
            return state.recv_timeout(Duration::from_secs(2)).ok().flatten();
        }
        let engine = self.engine.try_lock().ok()?;
        unsafe {
            let values = (0..self.param_info.len())
                .map(|i| (engine.api.carla_get_current_parameter_value)(engine.engine, 0, i as u32))
                .collect();
            let index = (engine.api.carla_get_current_program_index)(engine.engine, 0);
            Some(
                Saved::Params {
                    program: (index >= 0).then_some(index),
                    values,
                }
                .to_bytes(),
            )
        }
    }
    fn load_state(&mut self, data: &[u8]) -> bool {
        let Some(state) = Saved::parse(data, self.chunks) else {
            return false;
        };
        let Ok(engine) = self.engine.try_lock() else {
            return false;
        };
        unsafe {
            match state {
                Saved::Chunk(chunk) if self.chunks && !chunk.is_empty() => {
                    let encoded = CString::new(STANDARD.encode(chunk)).expect("base64 has no NUL");
                    (engine.api.carla_set_chunk_data)(engine.engine, 0, encoded.as_ptr());
                }
                Saved::Params { program, values } => {
                    if let Some(program) = program.filter(|p| *p >= 0) {
                        (engine.api.carla_set_program)(engine.engine, 0, program as u32);
                    }
                    for (i, value) in values.into_iter().take(self.param_info.len()).enumerate() {
                        (engine.api.carla_set_parameter_value)(engine.engine, 0, i as u32, value);
                    }
                }
                _ => return false,
            }
        }
        true
    }
}
