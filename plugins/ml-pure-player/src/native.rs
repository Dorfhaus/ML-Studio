//! FluidSynth 2.6.2 C API. The engine and its complete LGPL source are bundled.
use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    path::Path,
};
type Ptr = *mut c_void;
unsafe extern "C" {
    fn new_fluid_settings() -> Ptr;
    fn delete_fluid_settings(p: Ptr);
    fn fluid_settings_setnum(p: Ptr, key: *const c_char, value: f64) -> c_int;
    fn fluid_settings_setint(p: Ptr, key: *const c_char, value: c_int) -> c_int;
    fn new_fluid_synth(settings: Ptr) -> Ptr;
    fn delete_fluid_synth(p: Ptr);
    fn fluid_synth_sfload(p: Ptr, file: *const c_char, reset: c_int) -> c_int;
    fn fluid_synth_get_sfont_by_id(p: Ptr, id: c_int) -> Ptr;
    fn fluid_sfont_iteration_start(p: Ptr);
    fn fluid_sfont_iteration_next(p: Ptr) -> Ptr;
    fn fluid_preset_get_name(p: Ptr) -> *const c_char;
    fn fluid_preset_get_banknum(p: Ptr) -> c_int;
    fn fluid_preset_get_num(p: Ptr) -> c_int;
    fn fluid_synth_program_select(
        p: Ptr,
        channel: c_int,
        sfont: c_int,
        bank: c_int,
        program: c_int,
    ) -> c_int;
    fn fluid_synth_set_channel_type(p: Ptr, channel: c_int, kind: c_int) -> c_int;
    fn fluid_synth_noteon(p: Ptr, channel: c_int, key: c_int, velocity: c_int) -> c_int;
    fn fluid_synth_noteoff(p: Ptr, channel: c_int, key: c_int) -> c_int;
    fn fluid_synth_cc(p: Ptr, channel: c_int, cc: c_int, value: c_int) -> c_int;
    fn fluid_synth_pitch_bend(p: Ptr, channel: c_int, value: c_int) -> c_int;
    fn fluid_synth_channel_pressure(p: Ptr, channel: c_int, value: c_int) -> c_int;
    fn fluid_synth_all_sounds_off(p: Ptr, channel: c_int) -> c_int;
    fn fluid_synth_system_reset(p: Ptr) -> c_int;
    fn fluid_synth_set_gain(p: Ptr, value: f32);
    fn fluid_synth_set_reverb_group_level(p: Ptr, group: c_int, value: f64) -> c_int;
    fn fluid_synth_set_chorus_group_level(p: Ptr, group: c_int, value: f64) -> c_int;
    fn fluid_synth_write_float(
        p: Ptr,
        n: c_int,
        l: Ptr,
        lo: c_int,
        ls: c_int,
        r: Ptr,
        ro: c_int,
        rs: c_int,
    ) -> c_int;
}
#[derive(Clone, Debug)]
pub struct Preset {
    pub bank: i32,
    pub program: i32,
    pub name: String,
}
pub struct Synth {
    pub revision: u64,
    settings: Ptr,
    pub synth: Ptr,
    id: i32,
    pub presets: Vec<Preset>,
    selected: (i32, i32),
    resample: Option<Resample>,
}
struct Resample {
    samples: [[f32; 64]; 2],
    at: usize,
    pair: [[f32; 2]; 2],
    phase: f64,
    step: f64,
    primed: bool,
}
impl Resample {
    fn new(rate: u32) -> Self {
        Self {
            samples: [[0.; 64]; 2],
            at: 64,
            pair: [[0.; 2]; 2],
            phase: 0.,
            step: 96000. / rate as f64,
            primed: false,
        }
    }
    fn point(&mut self, synth: Ptr) -> [f32; 2] {
        if self.at == 64 {
            unsafe {
                fluid_synth_write_float(
                    synth,
                    64,
                    self.samples[0].as_mut_ptr().cast(),
                    0,
                    1,
                    self.samples[1].as_mut_ptr().cast(),
                    0,
                    1,
                );
            };
            self.at = 0;
        }
        let point = [self.samples[0][self.at], self.samples[1][self.at]];
        self.at += 1;
        point
    }
    fn render(&mut self, synth: Ptr, l: &mut [f32], r: &mut [f32]) {
        if !self.primed {
            self.pair = [self.point(synth), self.point(synth)];
            self.primed = true;
        }
        for (l, r) in l.iter_mut().zip(r) {
            let out: [f32; 2] = std::array::from_fn(|ch| {
                self.pair[0][ch] + (self.pair[1][ch] - self.pair[0][ch]) * self.phase as f32
            });
            *l = out[0];
            *r = out[1];
            self.phase += self.step;
            while self.phase >= 1. {
                self.phase -= 1.;
                self.pair[0] = self.pair[1];
                self.pair[1] = self.point(synth);
            }
        }
    }
}
// A prepared engine moves from the loader to one audio thread, never used concurrently.
unsafe impl Send for Synth {}
impl Synth {
    pub fn new(path: &Path, rate: u32) -> Result<Self, String> {
        let metadata = std::fs::metadata(path).map_err(|_| {
            "SoundFont not found. Download the library or locate your .sf2 file.".to_string()
        })?;
        if !metadata.is_file() || metadata.len() > 512 * 1024 * 1024 {
            return Err("Choose an SF2 file up to 512 MB".into());
        }
        let file = CString::new(path.to_str().ok_or("SoundFont path is not valid Unicode")?)
            .map_err(|_| "Invalid file path")?;
        unsafe {
            let settings = new_fluid_settings();
            if settings.is_null() {
                return Err("FluidSynth could not create settings".into());
            }
            // FluidSynth officially supports up to 96 kHz. Upsample its output above that rate.
            if fluid_settings_setnum(
                settings,
                c"synth.sample-rate".as_ptr(),
                rate.min(96000) as f64,
            ) != 0
            {
                delete_fluid_settings(settings);
                return Err("Unsupported SoundFont sample rate".into());
            }
            fluid_settings_setnum(settings, c"synth.gain".as_ptr(), 0.16);
            fluid_settings_setint(settings, c"synth.polyphony".as_ptr(), 128);
            fluid_settings_setint(settings, c"synth.cpu-cores".as_ptr(), 1);
            fluid_settings_setint(settings, c"synth.threadsafe-api".as_ptr(), 0);
            let synth = new_fluid_synth(settings);
            if synth.is_null() {
                delete_fluid_settings(settings);
                return Err("FluidSynth could not start".into());
            }
            let mut this = Self {
                revision: 0,
                settings,
                synth,
                id: 0,
                presets: Vec::new(),
                selected: (-1, -1),
                resample: if rate > 96000 {
                    Some(Resample::new(rate))
                } else {
                    None
                },
            };
            this.id = fluid_synth_sfload(synth, file.as_ptr(), 0);
            if this.id < 0 {
                return Err(
                    "Could not load this SoundFont. Choose a valid, uncompressed SF2 file.".into(),
                );
            }
            let font = fluid_synth_get_sfont_by_id(synth, this.id);
            if font.is_null() {
                return Err("SoundFont has no readable bank".into());
            }
            fluid_sfont_iteration_start(font);
            loop {
                let preset = fluid_sfont_iteration_next(font);
                if preset.is_null() {
                    break;
                }
                this.presets.push(Preset {
                    bank: fluid_preset_get_banknum(preset),
                    program: fluid_preset_get_num(preset),
                    name: CStr::from_ptr(fluid_preset_get_name(preset))
                        .to_string_lossy()
                        .chars()
                        .take(100)
                        .collect(),
                });
                if this.presets.len() > 16384 {
                    return Err("SoundFont contains too many presets".into());
                }
            }
            if this.presets.is_empty() {
                return Err("SoundFont contains no instruments".into());
            }
            this.presets.sort_by_key(|p| (p.bank, p.program));
            Ok(this)
        }
    }
    pub fn select(&mut self, bank: i32, program: i32) {
        if self.selected == (bank, program) {
            return;
        }
        let selected = self
            .presets
            .iter()
            .find(|p| p.bank == bank && p.program == program)
            .unwrap_or(&self.presets[0]);
        unsafe {
            fluid_synth_all_sounds_off(self.synth, 0);
            fluid_synth_set_channel_type(self.synth, 0, if selected.bank == 128 { 1 } else { 0 });
            fluid_synth_program_select(self.synth, 0, self.id, selected.bank, selected.program);
        }
        self.selected = (bank, program);
    }
    pub fn controls(&mut self, gain: f32, reverb: f64, chorus: f64) {
        unsafe {
            fluid_synth_set_gain(self.synth, gain);
            fluid_synth_set_reverb_group_level(self.synth, -1, reverb);
            fluid_synth_set_chorus_group_level(self.synth, -1, chorus);
        }
    }
    pub fn note(&mut self, key: u8, velocity: f32, on: bool) {
        unsafe {
            if on && velocity > 0. {
                fluid_synth_noteon(
                    self.synth,
                    0,
                    key as i32,
                    (velocity * 127.).round().clamp(1., 127.) as i32,
                );
            } else {
                fluid_synth_noteoff(self.synth, 0, key as i32);
            }
        }
    }
    pub fn cc(&mut self, cc: u8, value: f32) {
        unsafe {
            fluid_synth_cc(
                self.synth,
                0,
                cc as i32,
                (value * 127.).round().clamp(0., 127.) as i32,
            );
        }
    }
    pub fn bend(&mut self, value: f32) {
        unsafe {
            fluid_synth_pitch_bend(
                self.synth,
                0,
                (value * 16383.).round().clamp(0., 16383.) as i32,
            );
        }
    }
    pub fn pressure(&mut self, value: f32) {
        unsafe {
            fluid_synth_channel_pressure(
                self.synth,
                0,
                (value * 127.).round().clamp(0., 127.) as i32,
            );
        }
    }
    pub fn reset(&mut self) {
        unsafe {
            fluid_synth_system_reset(self.synth);
        };
        self.selected = (-1, -1);
        if let Some(resample) = &mut self.resample {
            resample.at = 64;
            resample.primed = false;
            resample.phase = 0.;
        }
    }
    pub fn render(&mut self, l: &mut [f32], r: &mut [f32]) {
        if l.is_empty() {
            return;
        }
        if let Some(resample) = &mut self.resample {
            resample.render(self.synth, l, r);
            return;
        }
        unsafe {
            if fluid_synth_write_float(
                self.synth,
                l.len() as i32,
                l.as_mut_ptr().cast(),
                0,
                1,
                r.as_mut_ptr().cast(),
                0,
                1,
            ) != 0
            {
                l.fill(0.);
                r.fill(0.);
            }
        }
    }
}

#[cfg(test)]
#[test]
fn empty_event_segment_does_not_prefetch_silence_at_high_rates() {
    let mut synth = Synth::new(
        std::path::Path::new(&std::env::var("ML_PURE_TEST_FONT").unwrap()),
        192000,
    )
    .unwrap();
    synth.render(&mut [], &mut []);
    assert!(!synth.resample.as_ref().unwrap().primed);
}
impl Drop for Synth {
    fn drop(&mut self) {
        unsafe {
            delete_fluid_synth(self.synth);
            delete_fluid_settings(self.settings);
        }
    }
}
