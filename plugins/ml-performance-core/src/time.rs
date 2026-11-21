// Adapted from B.Oops FxChopper.hpp and FxStutter.hpp, Sven Jaehnichen,
// GPL-3.0-or-later. Native transport, 16-step editor and safety bounds are ML additions.
use crate::*;
use std::sync::{
    Mutex,
    atomic::{AtomicU32, Ordering},
};
pub type TimeShaper = Fx<Time>;
const CYCLES: [f64; 5] = [0.25, 0.5, 1., 2., 4.];
const VOLUME_SLOT:usize=22;
const TIME_SLOT:usize=23;
fn point_start(time:bool,slot:usize)->usize {
    if time {26+16*slot} else if slot==0 {6} else {26+16*36+16*(slot-1)}
}
const GRAPH: Rect = Rect::new(20., 386., 720., 154.);
pub struct Time {
    rate: f64,
    bpm: f64,
    phase: f64,
    host_phase: Option<f64>,
    host_sample: Option<i64>,
    expected: Option<i64>,
    line: Vec<[f32; 2]>,
    at: usize,
    valid: usize,
    smooth: [f64; 2],
    mode: f64,
    smoothing: f64,
    levels: [f64; 16],
    midi_slots: [Option<(i32,usize)>; 2],
    previous_delay: f64,
    fade_from: f64,
    fade_remaining: usize,
    head: Arc<AtomicU32>,
}
impl Default for Time {
    fn default() -> Self {
        Self {
            rate: 48000.,
            bpm: 120.,
            phase: 0.,
            host_phase: None,
            host_sample: None,
            expected: None,
            line: Vec::new(),
            at: 0,
            valid: 0,
            smooth: [1., 1.],
            mode: 0., smoothing: 0.,
            levels: [1.; 16],
            midi_slots: [None; 2],
            previous_delay: 0.,
            fade_from: 0.,
            fade_remaining: 0,
            head: Arc::new(AtomicU32::new(0)),
        }
    }
}
/// B.Oops' centered, piecewise-linear transition between adjacent chop gains.
pub(crate) fn envelope(levels: &[f64; 16], phase: f64, smoothing: f64) -> f64 {
    let t = (phase + 1e-10).rem_euclid(1.) * 16.;
    let step = t.floor() as usize;
    let frac = t - step as f64;
    let v = levels[step];
    if smoothing > 0. && frac < 0.5 * smoothing {
        v + (levels[(step + 15) % 16] - v) * (0.5 - frac / smoothing)
    } else if smoothing > 0. && frac > 1. - 0.5 * smoothing {
        v + (levels[(step + 1) % 16] - v) * (0.5 - (1. - frac) / smoothing)
    } else {
        v
    }
}
impl Time {
    fn slot(&mut self, bank:usize, knob:i32)->usize {
        if let Some((at_trigger,slot)) = self.midi_slots[bank] {
            if at_trigger == knob {return slot;}
            self.midi_slots[bank]=None;
        }
        knob as usize
    }
    fn read(&self, back: f64) -> [f64; 2] {
        let d = back.max(0.);
        let a = d.floor() as usize;
        let frac = d - a as f64;
        let get = |n: usize| {
            if n < self.valid {
                self.line[(self.at + self.line.len() - n) % self.line.len()]
            } else {
                [0.; 2]
            }
        };
        let x = get(a);
        let y = get(a + 1);
        std::array::from_fn(|i| x[i] as f64 + (y[i] - x[i]) as f64 * frac)
    }
}
impl Unit for Time {
    const SPEC: &'static Spec = &specs::TIME;
    const ID: &'static str = "com.mlstudio.timeshaper";
    const CLASS: [u8; 16] = *b"MLTimeShaper0001";
    const MIDI: MidiConfig = MidiConfig::Basic;
    fn trigger(&mut self, note:u8, p:&FxParams) {
        match note {
            24..=59 => self.midi_slots[0]=Some((p.v(VOLUME_SLOT),(note-24) as usize)),
            60..=95 => self.midi_slots[1]=Some((p.v(TIME_SLOT),(note-60) as usize)),
            _ => {}
        }
    }
    fn prepare(&mut self, rate: f32, p: &FxParams) {
        self.rate = rate as f64;
        self.line = vec![[0.; 2]; rate as usize * 24 + 8];
        self.reset(p);
    }
    fn reset(&mut self, p: &FxParams) {
        self.midi_slots = [None; 2];
        self.previous_delay = 0.;
        self.fade_remaining = 0;
        self.valid = 0;
        self.at = 0;
        self.phase = 0.;
        self.expected = None;
        self.host_phase = None;
        self.host_sample = None;
        self.mode = p.v(1) as f64;
        self.smoothing = p.v(3) as f64 / 1000.;
        self.smooth = [p.v(4) as f64 / 1000., 10f64.powf(p.v(5) as f64 / 200.)];
        self.levels = std::array::from_fn(|i| p.v(6 + i) as f64 / 1000.);
    }
    fn transport(&mut self, t: &Transport) {
        self.bpm = t
            .tempo
            .filter(|v| v.is_finite())
            .unwrap_or(120.)
            .clamp(40., 300.);
        self.host_phase = if t.playing {
            t.pos_beats().filter(|v| v.is_finite())
        } else {
            None
        };
        self.host_sample = if t.playing { t.pos_samples() } else { None };
    }
    fn audio(&mut self, l: &mut [f32], r: &mut [f32], p: &FxParams) {
        let cycle = if p.v(25)!=0 {8.}else{CYCLES[p.v(0).clamp(0,4) as usize]};
        let time_start=point_start(true,self.slot(1,p.v(TIME_SLOT)));
        let vol_start=point_start(false,self.slot(0,p.v(VOLUME_SLOT)));
        let time:[f64;16]=std::array::from_fn(|i|p.v(time_start+i) as f64/1000.);
        let vol:[f64;16]=std::array::from_fn(|i|p.v(vol_start+i) as f64/1000.);
        let mapping=time.iter().any(|v|*v>0.);
        let shape=p.v(24);
        let mode=p.v(1) as f64;
        let step = self.bpm / 60. / self.rate / cycle;
        if let (Some(pos), Some(expected)) = (self.host_sample, self.expected) {
            if pos != expected {
                self.valid = 0;
            }
        }
        self.expected = self.host_sample.map(|n| n + l.len() as i64);
        if let Some(beats) = self.host_phase {
            self.phase = (beats / cycle).rem_euclid(1.);
        }
        let length = 1. / step;
        let repeats = p.v(2) as f64;
        let slice = length / repeats;
        let smooth_target = p.v(3) as f64 / 1000.;
        let fade_frames = (self.rate * 0.005 * smooth_target).round() as usize;
        let coeff = 1. - (-1. / (0.005 * self.rate)).exp();
        let targets = [p.v(4) as f64 / 1000., 10f64.powf(p.v(5) as f64 / 200.)];
        for i in 0..l.len() {
            let x = [l[i], r[i]].map(|v| if v.is_finite() { v } else { 0. });
            self.line[self.at] = x;
            self.valid = (self.valid + 1).min(self.line.len() - 1);
            for k in 0..2 {
                self.smooth[k] += coeff * (targets[k] - self.smooth[k]);
            }
            for k in 0..16 {
                self.levels[k] += coeff * (vol[k] - self.levels[k]);
            }
            let mut wet = x.map(|v| v as f64);
            self.mode += coeff * (mode - self.mode);
            self.smoothing += coeff * (smooth_target - self.smoothing);
            let smoothing = self.smoothing;
            if self.mode > 1e-9 {
                let slice_phase = (self.phase + 1e-10).rem_euclid(1.) * repeats;
                let nr = slice_phase.floor();
                let frac = slice_phase.fract();
                wet = self.read(nr * slice);
                // Same adjacent-tap interpolation as B.Oops; bounds suppress unavailable startup history.
                if smoothing > 0. && frac < 0.5 * smoothing {
                    let old = self.read((nr - 1.).max(0.) * slice);
                    let f = 0.5 - frac / smoothing;
                    for ch in 0..2 {
                        wet[ch] += (old[ch] - wet[ch]) * f;
                    }
                } else if smoothing > 0. && frac > 1. - 0.5 * smoothing {
                    let next = self.read((nr + 1.) * slice);
                    let f = 0.5 - (1. - frac) / smoothing;
                    for ch in 0..2 {
                        wet[ch] += (next[ch] - wet[ch]) * f;
                    }
                }
            }
            for ch in 0..2 {
                wet[ch] = x[ch] as f64 + (wet[ch] - x[ch] as f64) * self.mode;
            }
            let mut delay=0.;
            if mapping {
                let pos=self.phase*15.;
                let at=(pos as usize).min(14);
                let frac=(pos-at as f64).clamp(0.,1.);
                let t=match shape {1=>0.,2=>frac*frac*(3.-2.*frac),_=>frac};
                delay=(time[at]+(time[at+1]-time[at])*t)*length*2.;
                wet=self.read(delay);
            }
            if (delay-self.previous_delay).abs()>64. {
                self.fade_from=self.previous_delay;
                self.fade_remaining=fade_frames;
            }
            self.previous_delay=delay;
            if self.fade_remaining>0 && fade_frames>0 {
                let old=self.read(self.fade_from);
                let blend=(self.fade_remaining as f64/fade_frames as f64).min(1.);
                for ch in 0..2 {wet[ch]+=(old[ch]-wet[ch])*blend;}
                self.fade_remaining-=1;
            } else {self.fade_remaining=0;}
            let level = envelope(&self.levels, self.phase, smoothing);
            l[i] = ((x[0] as f64 + (wet[0] * level - x[0] as f64) * self.smooth[0])
                * self.smooth[1]) as f32;
            r[i] = ((x[1] as f64 + (wet[1] * level - x[1] as f64) * self.smooth[0])
                * self.smooth[1]) as f32;
            self.at = (self.at + 1) % self.line.len();
            self.phase = (self.phase + step).rem_euclid(1.);
        }
        self.host_sample=self.expected;
        if let Some(beat)=self.host_phase.as_mut() {*beat+=l.len() as f64*step*cycle;}
        self.head
            .store((self.phase as f32).to_bits(), Ordering::Relaxed);
    }
    fn panel(&self, p: Arc<FxParams>) -> Arc<dyn Extension> {
        Arc::new(Panel {
            p,
            time_page:Default::default(),
            head: self.head.clone(),
            drag: Mutex::new(None),
        })
    }
}
struct Panel {
    time_page:std::sync::atomic::AtomicBool,
    p: Arc<FxParams>,
    head: Arc<AtomicU32>,
    drag: Mutex<Option<(usize, i32, Option<Arc<dyn GuiContext>>)>>,
}
impl Panel {
    fn set(&self, i: usize, v: i32, ctx: Option<&Arc<dyn GuiContext>>) {
        self.p
            .live
            .set(self.p.p[i].as_ptr(), self.p.p[i].preview_normalized(v));
        if let Some(c) = ctx {
            ParamSetter::new(c.as_ref()).set_parameter(&self.p.p[i], v);
        }
    }
    fn finish(&self, cancel: bool) {
        if let Some((i, old, ctx)) = self.drag.lock().unwrap().take() {
            if cancel {
                self.set(i, old, ctx.as_ref());
            }
            if let Some(c) = ctx {
                ParamSetter::new(c.as_ref()).end_set_parameter(&self.p.p[i]);
            }
        }
    }
    fn page(&self)->bool {self.time_page.load(Ordering::Relaxed)}
    fn start(&self)->usize {point_start(self.page(),self.p.v(if self.page(){TIME_SLOT}else{VOLUME_SLOT}) as usize)}
    fn choice(&self,i:usize,v:i32,ctx:Option<&Arc<dyn GuiContext>>){
        if let Some(c)=ctx {ParamSetter::new(c.as_ref()).begin_set_parameter(&self.p.p[i]);}
        self.set(i,v,ctx);
        if let Some(c)=ctx {ParamSetter::new(c.as_ref()).end_set_parameter(&self.p.p[i]);}
    }
    fn preset(&self, kind: usize, ctx: Option<&Arc<dyn GuiContext>>) {
        let values: [i32; 16] = std::array::from_fn(|i| if self.page() {
            let t=i as f64/15.;
            (1000.*match kind {0=>0.,1=>t,2=>0.25*t*t,_=>((i/4) as f64)/8.}) as i32
        } else {match kind {
            0 => 1000,
            1 => {
                if i % 2 == 0 {
                    1000
                } else {
                    0
                }
            }
            2 => (i % 4) as i32 * 1000 / 3,
            _ => 0,
        }});
        let shape = self.page().then_some(i32::from(kind==3));
        if let Some(c) = ctx {
            let mut state = c.get_state();
            if let Some(value)=shape {
                state.params.insert(self.p.spec.settings[24].id.into(),nih_plug::wrapper::state::ParamValue::I32(value));
            }
            for (i, v) in values.into_iter().enumerate() {
                state.params.insert(
                    self.p.spec.settings[self.start()+i].id.to_string(),
                    nih_plug::wrapper::state::ParamValue::I32(v),
                );
            }
            self.p.live.clear();
            c.set_state(state);
        } else {
            if let Some(value)=shape {self.set(24,value,None);}
            for (i, v) in values.into_iter().enumerate() {
                self.set(self.start()+i,v,None);
            }
        }
    }
}
impl Extension for Panel {
    fn paint(&self, c: &mut Canvas, _: &dyn lemon_small_fx::view::Model) {
        if !os_plugkit::visibility::stock_dsp_controls() {
            return;
        }
        let time=self.page();
        for (i,label) in (if time {["Straight","Reverse","Tape stop","Stutter"]}else{["All on","Chop","Pump","Clear"]}).iter().enumerate() {
            button(c, Rect::new(20. + i as f32 * 120., 330., 110., 28.), label);
        }
        button(c,Rect::new(508.,330.,229.,28.),if time {"Editing time • switch to volume"} else {"Editing volume • switch to time"});
        text(c,20.,367.,if time {"TIME OFFSET   •   Drag a point to change playback position"}else{"VOLUME   •   Drag a bar to change its level"});
        c.fill_rect(GRAPH, Color::rgb(0x1d1d1d));
        for i in 0..16 {
            let x = GRAPH.x + i as f32 * 45.;
            let h = self.p.v(self.start()+i) as f32 / 1000. * GRAPH.h;
            if time && i>0 {
                let prev=self.p.v(self.start()+i-1) as f32/1000.;
                let from=GRAPH.bottom()-prev*GRAPH.h;
                let to=GRAPH.bottom()-h;
                let color=Color::rgb(0x47c7c0);
                let shape=self.p.v(24);
                let mut last=(x-22.5,from);
                for step in 1..=8 {
                    let t=step as f32/8.;
                    let weight=match shape {1=>0.,2=>t*t*(3.-2.*t),_=>t};
                    let next=(x-22.5+45.*t,from+(to-from)*weight);
                    c.line(last.0,last.1,next.0,next.1,2.,color);
                    last=next;
                }
                if shape==1 {c.line(last.0,from,last.0,to,2.,color);}
            }
            if time {c.fill_rrect(Rect::new(x+18.,GRAPH.bottom()-h-3.,8.,6.),2.,Color::rgb(0x7ef1d4));}else{
            c.fill_rect(
                Rect::new(x + 3., GRAPH.y + GRAPH.h - h, 39., h),
                Color::rgb(0x249ce0),
            );
            }
            text(c, x + 15., GRAPH.y + GRAPH.h + 7., &(i + 1).to_string());
        }
        let x = GRAPH.x + f32::from_bits(self.head.load(Ordering::Relaxed)) * GRAPH.w;
        c.line(x, GRAPH.y, x, GRAPH.y + GRAPH.h, 2., Color::rgb(0xffffff));
        let slot=1+self.p.v(if time {TIME_SLOT}else{VOLUME_SLOT});
        for (r,label) in [(Rect::new(20.,574.,52.,32.),"<".into()),(Rect::new(76.,574.,132.,32.),format!("Slot {slot} / 36")),(Rect::new(212.,574.,52.,32.),">".into()),
            (Rect::new(288.,574.,182.,32.),format!("Curve   {}",["Linear","Hold","Smooth"][self.p.v(24) as usize])),(Rect::new(488.,574.,240.,32.),if self.p.v(25)==0 {"Length   •   cycle knob".into()}else{"Length   •   2 bars".into()})] {
            button(c,r,&label);
        }
    }
    fn click(&self, x: f32, y: f32, ctx: Option<&Arc<dyn GuiContext>>) -> bool {
        if !os_plugkit::visibility::stock_dsp_controls() {
            return false;
        }
        if Rect::new(508.,330.,229.,28.).contains(x,y) {self.time_page.fetch_xor(true,Ordering::Relaxed);return true}
        if (574.0..606.).contains(&y) {
            let slot=if self.page(){TIME_SLOT}else{VOLUME_SLOT};
            if (20.0..72.).contains(&x) {self.choice(slot,(self.p.v(slot)+35)%36,ctx);return true}
            if (212.0..264.).contains(&x) {self.choice(slot,(self.p.v(slot)+1)%36,ctx);return true}
            if (288.0..470.).contains(&x) {self.choice(24,(self.p.v(24)+1)%3,ctx);return true}
            if (488.0..728.).contains(&x) {self.choice(25,1-self.p.v(25),ctx);return true}
        }
        for i in 0..4 {
            if Rect::new(20. + i as f32 * 120., 330., 110., 28.).contains(x, y) {
                self.preset(i, ctx);
                return true;
            }
        }
        if GRAPH.contains(x, y) {
            let i = self.start() + (((x - GRAPH.x) / 45.) as usize).min(15);
            if let Some(c) = ctx {
                ParamSetter::new(c.as_ref()).begin_set_parameter(&self.p.p[i]);
            }
            *self.drag.lock().unwrap() = Some((i, self.p.v(i), ctx.cloned()));
            self.moved(x, y);
            return true;
        }
        false
    }
    fn moved(&self, _: f32, y: f32) -> bool {
        let d = self.drag.lock().unwrap();
        if let Some((i, _, ctx)) = &*d {
            let v = ((1. - (y - GRAPH.y) / GRAPH.h) * 1000.)
                .round()
                .clamp(0., 1000.) as i32;
            self.set(*i, v, ctx.as_ref());
            return true;
        }
        false
    }
    fn dragging(&self) -> bool {
        self.drag.lock().unwrap().is_some()
    }
    fn release(&self, _: Option<&Arc<dyn GuiContext>>) -> bool {
        let had = self.dragging();
        self.finish(false);
        had
    }
    fn character(&self, c: char) -> bool {
        if c == '\u{1b}' && self.dragging() {
            self.finish(true);
            return true;
        }
        false
    }
    fn editor_closed(&self) {
        self.finish(false);
    }
}

#[cfg(test)]
mod slot_tests {
    use super::*;
    #[test]
    fn midi_banks_are_independent_and_knob_changes_take_over() {
        let params=FxParams::new(&specs::TIME);
        let mut time=Time::default();
        time.trigger(95,&params);
        time.trigger(25,&params);
        assert_eq!(time.slot(0,0),1);
        assert_eq!(time.slot(1,0),35);
        assert_eq!(time.slot(1,3),3);
        assert_eq!(time.slot(0,0),1);
        time.reset(&params);
        assert_eq!(time.slot(0,0),0);
    }
}
