// SPDX-License-Identifier: GPL-3.0-or-later
//! MIDI recording and buffered Touch/Latch automation takes.
use crate::app::{Action,App};
use crate::record::Opt;
use os_model::{AutoPoint,ClipSource};
use std::collections::HashMap;

pub const MODE_VISIBILITY:&str="visible_automation-record-mode";
pub const CLIPS_VISIBILITY:&str="visible_automation-record-clips";
pub fn latch()->bool {crate::settings_file::flag("automation_record_latch",false)}
pub fn separate_clips()->bool {crate::settings_file::flag("automation_record_separate_clips",false)}
#[derive(Debug)]
pub struct Gesture {
    pub(super) target:u32, pub(super) name:String, pub(super) start:u32, pub(super) end:u32, pub(super) points:Vec<AutoPoint>,
    last:std::time::Instant,
}
impl Gesture {
    fn point(&mut self,beat:f64,value:f64) {
        if let Some(p)=self.points.last_mut().filter(|p|(p.beat-beat).abs()<1e-9){p.value=value;return;}
        let n=self.points.len();
        // Keep the last two samples of a flat hold: the next movement must not ramp across the whole pause.
        if n>=3 && self.points[n-3..].iter().all(|p|p.value==value){self.points.remove(n-2);}
        self.points.push(AutoPoint::new(beat,value));
    }
}
#[derive(Debug,Default)]
pub struct Smart {
    pub initial:HashMap<u32,f64>,
    pub active:HashMap<u32,Gesture>,
    completed:Vec<Gesture>,
    held:std::collections::HashSet<u32>,
    host:std::collections::HashSet<u32>,
    arrangement:Option<u16>,
    pub origin:Option<u32>,
    pub last_tick:u32,
}
impl Smart { pub(crate) fn completed_is_empty(&self)->bool {self.completed.is_empty()} }
/// An assigned instrument's playlist row and the pattern the user currently selected.
pub(crate) fn instrument_lane(s:&crate::app::State)->Option<(u16,u16)> {
    let ch=s.project.channels.get(s.selected_channel)?.index;
    let arr=s.project.arrangement()?.number;
    s.project.instrument_lanes.iter().find(|v|v.0==ch&&v.1==arr).map(|v|(v.2,s.pattern.max(1)))
        .or_else(||crate::attached_mixer::lane(s,s.project.channels[s.selected_channel].mixer_track).map(|row|(row,s.pattern.max(1))))
}

impl App {
    pub(crate) fn route_instrument_lane(&mut self,row:usize,mixer:u16)->bool {
        use crate::{playlist as pl,history::{self,Snap}};
        let s=&mut self.state;
        let Some(ch)=s.project.channels.get(row).cloned() else {return false;};
        if !matches!(ch.kind,os_model::ChannelKind::Plugin){return false;}
        let arr=pl::arr_mut(s).number;
        let old=s.project.instrument_lanes.iter().find(|v|v.0==ch.index&&v.1==arr).copied();
        // A played recording may already exist as a pattern before its first mixer assignment.
        let existing=pl::clips(s).iter().enumerate().filter(|(_,clip)|match clip.source {
            ClipSource::Pattern(n)=>s.project.pattern(n).is_some_and(|p|p.notes.iter().any(|note|note.channel==ch.index)),
            ClipSource::Channel(id)=>id==ch.index,
        }).max_by_key(|(i,clip)|(s.playlist.sel.contains(i),clip.track==s.playlist.track,
            clip.source==ClipSource::Pattern(s.pattern),std::cmp::Reverse(*i))).map(|(_,clip)|clip.track);
        let lane=old.map(|v|v.2).or(existing).or_else(||(0..pl::TRACKS).find(|t|
            !pl::clips(s).iter().any(|c|c.track==*t) && !s.project.instrument_lanes.iter().any(|v|v.1==arr&&v.2==*t)));
        let Some(lane)=lane else {s.message=Some("No empty playlist track available".into());return true;};
        let snap=Snap::Several(vec![Snap::ChannelRoute(row,ch.mixer_track),Snap::Playlist(Box::new(pl::capture(s))),
            Snap::InstrumentLanes(s.project.instrument_lanes.clone())]);
        history::record(s,"Assign instrument to mixer and playlist",snap);
        // Routing must not create or name the next pattern, or change the user's selection.
        // Keep the legacy tuple field for project compatibility; recording uses the current pattern.
        if old.is_none() {s.project.instrument_lanes.push((ch.index,arr,lane,s.pattern.max(1)));}
        let name=crate::mixer::name(s,mixer);let color=crate::mixer::track(s,mixer).and_then(|m|m.color);
        if let Some(t)=pl::arr_mut(s).tracks.get_mut(lane as usize){t.name=Some(name);t.color=color;}
        s.project.channels[row].mixer_track=mixer;s.selected_channel=row;s.selected_mixer=mixer;
        s.playlist.track=lane;
        s.piano.dirty=true;
        self.transport.update(&crate::rack::play_project(&self.state));self.invalidate();true
    }

    fn smart_automation_live(&self)->bool {
        Opt::Automation.on() && self.state.rec.armed && self.state.rec.count.is_none()
            && !self.pre_roll_active() && self.transport.playing() && !self.transport.paused()
    }
    pub(crate) fn smart_begin_parameter(&mut self,target:u32,value:f64,held:bool) {
        self.start_on_input(Opt::Automation);
        if !self.smart_automation_live(){return;}
        self.state.rec.smart.initial.entry(target).or_insert(value);
        if held {self.state.rec.smart.held.insert(target);}
    }
    pub(crate) fn smart_begin_host(&mut self,a:&Action) {
        if let Some((target,_,v))=self.automation_target(a){self.smart_begin_parameter(target,v,false);}
    }
    pub(crate) fn smart_record_host(&mut self,a:&Action) {
        if let Some(t)=crate::automation_feedback::action_target(&self.state,a){self.state.params.playback.remove(&t);}
        self.start_on_input(Opt::Automation);
        if !self.smart_automation_live() && !self.state.params.capture_armed {return;}
        if let Some((target,name,v))=self.automation_target(a){
            if self.drag.is_some() {self.state.rec.smart.held.insert(target);self.state.rec.smart.host.insert(target);}
            if !self.capture_control(target,&name,v) {self.smart_record_value(target,name,v);}
        }
    }
    pub(crate) fn smart_end_host(&mut self) {
        let targets:Vec<_>=self.state.rec.smart.host.drain().collect();
        for target in targets {self.smart_end_parameter(target);}
    }
    pub(crate) fn smart_end_parameter(&mut self,target:u32) {
        self.smart_record_tick();
        self.state.rec.smart.held.remove(&target);
        if !latch() {let tick=self.transport.tick().max(0.).round() as u32;self.smart_close(target,tick);}
    }
    fn smart_close(&mut self,target:u32,tick:u32) {
        if let Some(mut g)=self.state.rec.smart.active.remove(&target) {
            g.end=tick.max(g.start+1);
            let beat=f64::from(g.end-g.start)/f64::from(self.state.project.ppq.max(1));
            if let Some(last)=g.points.last().copied() {if last.beat<beat {g.points.push(AutoPoint::new(beat,last.value));}}
            self.state.rec.smart.completed.push(g);
        }
        self.transport.write_automation(target,None);
    }
    pub(crate) fn smart_record_value(&mut self,target:u32,name:String,value:f64) {
        self.start_on_input(Opt::Automation);
        if !self.smart_automation_live() || !value.is_finite(){return;}
        self.smart_record_tick();
        self.state.params.playback.remove(&os_model::target::decode(target));
        let tick=self.transport.tick().max(0.).round() as u32;
        let arr=self.state.project.arrangement().map(|a|a.number);
        let smart=&mut self.state.rec.smart;
        smart.arrangement=arr;smart.last_tick=tick;
        let g=smart.active.entry(target).or_insert_with(||Gesture {
            target,name,start:tick,end:tick+1,points:Vec::new(),last:std::time::Instant::now(),
        });
        let beat=f64::from(tick.saturating_sub(g.start))/f64::from(self.state.project.ppq.max(1));
        let v=value.clamp(0.,1.);
        g.point(beat,v);
        g.last=std::time::Instant::now();g.end=tick+1;
        self.transport.write_automation(target,Some(v));
        self.invalidate();
    }
    pub(crate) fn smart_record_tick(&mut self) {
        let live=self.smart_automation_live();
        let tick=if live {self.transport.tick().max(0.).round() as u32}else{self.state.rec.smart.last_tick};
        let switched=self.state.rec.smart.arrangement.is_some_and(|a|self.state.project.arrangement().map(|a|a.number)!=Some(a));
        // A loop wrap closes the previous pass before recording the next one.
        let wrapped=live && tick<self.state.rec.smart.last_tick;
        let close_at=if wrapped {
            self.selection_loop().filter(|&(start,end)|tick>=start && tick<end && self.state.rec.smart.last_tick<end)
                .map_or(self.state.rec.smart.last_tick.saturating_add(1),|(_,end)|end)
        }else{tick};
        let ends:Vec<_>=self.state.rec.smart.active.iter().filter(|(target,g)|
            !live || switched || wrapped || (!latch() && !self.state.rec.smart.held.contains(target) && g.last.elapsed().as_millis()>=250))
            .map(|(&target,_)|target).collect();
        for target in ends {
            let resume=if wrapped && !switched && (latch() || self.state.rec.smart.held.contains(&target)) {
                self.state.rec.smart.active.get(&target).and_then(|g|g.points.last().map(|p|(g.name.clone(),p.value)))
            } else {None};
            self.smart_close(target,close_at);
            if let Some((name,value))=resume {
                self.state.rec.smart.active.insert(target,Gesture{target,name,start:tick,end:tick+1,
                    points:vec![AutoPoint::new(0.,value)],last:std::time::Instant::now()});
                self.transport.write_automation(target,Some(value));
            }
        }
        if live {
            let ppq=f64::from(self.state.project.ppq.max(1));
            for g in self.state.rec.smart.active.values_mut() {
                if let Some(last)=g.points.last().copied(){g.point(f64::from(tick.saturating_sub(g.start))/ppq,last.value);}
            }
            self.state.rec.smart.last_tick=tick;
        }
        if !live || switched {self.smart_finish();}
    }
    /// Save the current part without ending a held/latching gesture.
    pub(crate) fn smart_checkpoint(&mut self) {
        self.params_tick();self.smart_record_tick();
        let resume:Vec<_>=self.state.rec.smart.active.values().filter_map(|g|g.points.last().map(|p|(g.target,g.name.clone(),p.value))).collect();
        let held=self.state.rec.smart.held.clone();let host=self.state.rec.smart.host.clone();
        self.smart_finish();
        if self.smart_pending(){return;}
        for (target,name,value) in resume {
            self.smart_begin_parameter(target,value,held.contains(&target));
            self.smart_record_value(target,name,value);
        }
        self.state.rec.smart.host=host;
    }
    pub(crate) fn smart_cancel(&mut self) {
        for &target in self.state.rec.smart.active.keys(){self.transport.write_automation(target,None);}
        self.state.rec.smart=Smart::default();
    }
    pub(crate) fn smart_pending(&self)->bool { !self.state.rec.smart.active.is_empty() || !self.state.rec.smart.completed.is_empty() }
    pub(crate) fn smart_finish(&mut self) {
        let tick=self.state.rec.smart.last_tick;
        let targets:Vec<_>=self.state.rec.smart.active.keys().copied().collect();
        for target in targets {self.smart_close(target,tick);}
        let smart=std::mem::take(&mut self.state.rec.smart);
        if smart.completed.is_empty(){return;}
        let selection=self.state.project.current_arrangement;
        if let Some(arr)=smart.arrangement {self.state.project.current_arrangement=Some(arr);}
        let history=self.state.history.begin_sequence();
        let mut count=0;
        let mut pending=Vec::new();
        let completed=if separate_clips(){smart.completed}else{
            let mut groups:std::collections::BTreeMap<u32,Vec<Gesture>>=Default::default();
            for g in smart.completed{groups.entry(g.target).or_default().push(g);}
            for (target,gestures) in groups {
                let initial=smart.initial.get(&target).copied().unwrap_or(gestures[0].points[0].value);
                if crate::recording_merge::commit(&mut self.state,target,&gestures,initial){count+=1;}else{pending.extend(gestures);}
            }
            Vec::new()
        };
        for g in completed {
            if crate::playlist::recording_destination(&self.state,g.target,g.start,g.end).is_none() {pending.push(g);continue;}

            // Split at existing clip starts so a later old clip cannot override this recorded pass.
            let mut cuts=vec![g.start,g.end];
            for c in crate::playlist::clips(&self.state) {
                if c.position>g.start && c.position<g.end && let ClipSource::Channel(ch)=c.source
                    && self.state.project.automation(ch).is_some_and(|a|a.targets.contains(&g.target)) {cuts.push(c.position);}
            }
            cuts.sort_unstable();cuts.dedup();
            for span in cuts.windows(2) {
                let Some(ch)=crate::playlist::recorded_automation(&mut self.state,g.target,&g.name,span[0],span[1],smart.initial.get(&g.target).copied().unwrap_or(g.points[0].value)) else {continue;};
                let ppq=f64::from(self.state.project.ppq.max(1));
                let from=f64::from(span[0]-g.start)/ppq;let to=f64::from(span[1]-g.start)/ppq;
                let value=|at:f64| {
                    let next=g.points.partition_point(|p|p.beat<=at);
                    if next==0 {return g.points[0].value;}
                    let a=g.points[next-1];
                    match g.points.get(next) {Some(b) if b.beat>a.beat=>a.value+(b.value-a.value)*((at-a.beat)/(b.beat-a.beat)),_=>a.value}
                };
                let mut points=vec![AutoPoint::new(0.,value(from))];
                points.extend(g.points.iter().filter(|p|p.beat>from&&p.beat<to).map(|p|AutoPoint::new(p.beat-from,p.value)));
                points.push(AutoPoint::new(to-from,value(to)));
                if let Some(a)=self.state.project.automation_mut(ch){a.points=points;}
                count+=1;
            }
        }
        self.state.history.end_sequence_named(history,"Record automation");
        self.state.project.current_arrangement=selection;
        self.state.piano.dirty=true;
        let missed=pending.len();
        self.state.rec.smart.completed=pending;
        if missed>0 {self.state.rec.smart.arrangement=smart.arrangement;self.state.rec.smart.initial=smart.initial;}
        self.state.message=Some(if missed>0 {format!("{missed} automation gestures are still buffered. Free an unlocked Playlist lane and press Stop again to keep them.")}
            else {format!("Recorded {count} automation clips. Undo removes this automation take.")});
        self.invalidate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recorded_automation_overlays_owner_and_restores_with_undo_and_reopen() {
        use crate::playlist as pl;
        use os_model::{Clip,ClipSource,target};
        for target in [target::channel(0,target::CHANNEL_VOLUME),target::mixer(1,target::MIXER_VOLUME),target::slot_param(1,0,0)] {
            let mut app=recording(false);
            app.state.project.channels[0].mixer_track=1;
            app.state.project.patterns[0].notes.push(os_model::Note::step(0,0));
            let music=Clip::new(0,768,3,ClipSource::Pattern(1));
            pl::arr_mut(&mut app.state).clips=vec![music.clone()];
            app.state.history.reset();
            app.smart_begin_parameter(target,0.2,true);write(&mut app,target,0.8);
            app.transport.pretend_at(192.,true);app.smart_end_parameter(target);
            app.transport.stop();app.smart_record_tick();
            let ch=app.state.project.automations[0].channel;
            let clip=&pl::clips(&app.state)[1];
            assert_eq!((clip.track,clip.position,clip.length),(3,96,96));
            assert_eq!(pl::clips(&app.state)[0],music);
            assert_eq!(app.state.playlist.automation_owners.get(&ch),Some(&3));
            assert_eq!(app.state.playlist.automation_views.get(&3),Some(&ch));
            let file=os_flp::project::create(&app.state.project).unwrap();
            let back=os_flp::project::load(&os_flp::FlpFile::parse(&file.to_bytes().unwrap()).unwrap()).unwrap();
            let placement=|clips:&[Clip]|clips.iter().map(|c|(c.track,c.position,c.length,c.source,c.muted,c.window)).collect::<Vec<_>>();
            assert_eq!(placement(&back.arrangement().unwrap().clips),placement(pl::clips(&app.state)));
            let reopened_music=back.arrangement().unwrap().clips[0].clone();
            assert_eq!(crate::history::undo(&mut app.state),Some("Record automation"));
            assert_eq!(pl::clips(&app.state),&[music.clone()]);
            crate::history::redo(&mut app.state);
            assert_eq!(app.state.playlist.automation_views.get(&3),Some(&ch));
            app.state.project=back;app.state.playlist.automation_owners.clear();
            app.state.playlist.automation_views.clear();app.state.playlist.automation_last.clear();
            pl::run(&mut app,pl::Cmd::TrackAutomationToggle(3));
            assert_eq!(app.state.playlist.automation_views.get(&3),Some(&ch));
            pl::run(&mut app,pl::Cmd::TrackAutomationToggle(3));
            assert!(!app.state.playlist.automation_views.contains_key(&3));
            assert_eq!(pl::clips(&app.state)[0],reopened_music);
            assert!(!pl::clips(&app.state)[1].muted,"overlay visibility must not mute playback");
        }
    }
    fn recording(latching:bool)->App {
        // These legacy tests explicitly exercise the optional separate-gesture workflow.
        crate::settings_file::set_flag("automation_record_separate_clips",true);
        crate::settings_file::set_flag("automation_record_latch",latching);
        crate::settings_file::set_flag("rec_automation",true);
        crate::settings_file::set_flag("rec_filter_explicit",true);
        let mut app=App::new(None);app.state.project=crate::app::test_project();
        crate::playlist::arr_mut(&mut app.state).clips.clear();
        app.state.rec.armed=true;app.transport.pretend_at(96.,true);app
    }
    fn write(app:&mut App,target:u32,value:f64){app.smart_record_value(target,"Test control".into(),value);}
    #[test]
    fn touch_and_latch_preserve_old_curves_and_roundtrip_undo() {
        for latching in [false,true] {
            let mut app=recording(latching);let target=os_model::target::mixer(1,os_model::target::MIXER_VOLUME);
            app.state.playlist.range=Some((0,384));
            let old=crate::playlist::create_automation(&mut app.state,target,"Old",0.2);
            let original=app.state.project.automation(old).unwrap().clone();
            app.state.history.reset();app.smart_begin_parameter(target,0.2,true);write(&mut app,target,0.8);
            app.transport.pretend_at(144.,true);app.smart_end_parameter(target);
            assert_eq!(app.state.rec.smart.active.is_empty(),!latching);
            app.transport.pretend_at(288.,true);app.smart_record_tick();app.transport.stop();app.smart_record_tick();
            assert_eq!(app.state.project.automation(old),Some(&original));
            let clip=crate::playlist::clips(&app.state).last().unwrap();
            assert_eq!((clip.position,clip.length),(96,if latching{192}else{48}));
            let saved=os_flp::project::create(&app.state.project).unwrap().to_bytes().unwrap();
            let back=os_flp::project::load(&os_flp::FlpFile::parse(&saved).unwrap()).unwrap();
            assert_eq!(back.automations.len(),2);
            assert_eq!(crate::history::undo(&mut app.state),Some("Record automation"));
            assert_eq!(app.state.project.automations,vec![original]);
            crate::history::redo(&mut app.state);assert_eq!(app.state.project.automations.len(),2);
        }
    }
    #[test]
    fn wheel_timeout_loops_and_filter_do_not_leave_controls_latched() {
        let mut app=recording(false);let target=os_model::target::TEMPO;
        write(&mut app,target,0.5);
        app.state.rec.smart.active.get_mut(&target).unwrap().last-=std::time::Duration::from_secs(1);
        app.transport.pretend_at(120.,true);app.smart_record_tick();assert!(app.state.rec.smart.active.is_empty());
        app.smart_begin_parameter(target,0.5,true);write(&mut app,target,0.7);
        app.transport.pretend_at(380.,true);app.smart_record_tick();
        app.transport.pretend_at(0.,true);app.smart_record_tick();
        assert_eq!(app.state.rec.smart.active[&target].start,0);
        app.transport.pretend_at(24.,true);app.smart_end_parameter(target);
        app.transport.stop();app.smart_record_tick();assert_eq!(app.state.project.automations.len(),3);
        crate::settings_file::set_flag("rec_automation",false);app.transport.pretend_at(48.,true);write(&mut app,target,0.3);
        assert!(!app.smart_pending());
    }
    #[test]
    fn checkpoint_and_cancel_keep_recording_and_do_not_lose_the_take() {
        let mut app=recording(true);let target=os_model::target::TEMPO;
        write(&mut app,target,0.5);app.transport.pretend_at(192.,true);app.smart_checkpoint();
        assert_eq!(app.state.project.automations.len(),1);
        assert_eq!(app.state.rec.smart.active[&target].start,192);
        app.transport.pretend_at(288.,true);app.smart_record_tick();app.smart_finish();
        assert_eq!(app.state.project.automations.len(),2);
        app.transport.pretend_at(384.,true);write(&mut app,target,0.7);
        app.cancel_recording();assert_eq!(app.state.project.automations.len(),2);assert!(!app.smart_pending());
    }
    #[test]
    fn stationary_holds_stay_flat_without_growing_unbounded_points() {
        let mut app=recording(true);let target=os_model::target::TEMPO;
        write(&mut app,target,0.2);
        for tick in 97..192 {app.transport.pretend_at(f64::from(tick),true);app.smart_record_tick();}
        assert_eq!(app.state.rec.smart.active[&target].points.len(),3);
        app.transport.pretend_at(192.,true);write(&mut app,target,0.8);
        let points=&app.state.rec.smart.active[&target].points;
        assert_eq!(points[points.len()-2].value,0.2);
        assert!((points[points.len()-2].beat-95./96.).abs()<1e-9);
        assert_eq!(points.last().unwrap().value,0.8);
    }
    #[test]
    fn saving_a_copy_retains_committed_automation_in_the_live_project() {
        let mut app=recording(true);let target=os_model::target::TEMPO;
        write(&mut app,target,0.5);app.transport.pretend_at(192.,true);
        let path=std::env::temp_dir().join(format!("touch-copy-{}.flp",std::process::id()));
        assert!(app.save_copy(path.clone()).unwrap().is_ok(),"{:?}",app.state.message);
        assert_eq!(app.state.project.automations.len(),1);
        assert!(app.state.rec.smart.active.contains_key(&target));
        let bytes=std::fs::read(&path).unwrap();let back=os_flp::project::load(&os_flp::FlpFile::parse(&bytes).unwrap()).unwrap();
        assert_eq!(back.automations.len(),1);std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn bypass_and_typed_host_values_are_captured_without_native_gestures() {
        let mut app=recording(true);let before=crate::mixer::track(&app.state,1).unwrap().slot_on[0];
        assert_eq!(crate::mixer::toggle_slot(&mut app.state,1,0),None);app.params_tick();
        let bypass=os_model::target::slot(1,0,os_model::target::SLOT_ON);
        assert_eq!(app.state.rec.smart.active[&bypass].points[0].value,f64::from(!before));
        app.state.params.record_edits.push(crate::typein::Target::Knob(crate::knobs::KnobId::ChanPitch(0)));
        app.state.project.channels[0].pitch_cents=100;app.params_tick();
        let pitch=os_model::target::channel(0,os_model::target::CHANNEL_PITCH);
        assert!((app.state.rec.smart.active[&pitch].points[0].value-(0.5+100./9600.)).abs()<1e-9);
    }
    #[test]
    fn record_mode_menu_defaults_to_touch_and_visibility_preserves_choice() {
        let mut app=App::new(None);assert!(!latch());app.record_menu(10.,10.);
        assert!(app.state.playlist.popup.as_ref().unwrap().items.iter().any(|i|i.label=="Automation mode"));
        app.rec_cmd(crate::record::RecCmd::AutomationMode(true));assert!(latch());
        crate::settings_file::set_flag(MODE_VISIBILITY,false);app.record_menu(10.,10.);
        assert!(!app.state.playlist.popup.as_ref().unwrap().items.iter().any(|i|i.label=="Automation mode"));assert!(latch());
    }
    #[test]
    fn later_existing_clips_cannot_override_the_new_latched_take() {
        let mut app=recording(true);let target=os_model::target::TEMPO;
        for start in [0,192] {app.state.playlist.range=Some((start,start+384));crate::playlist::create_automation(&mut app.state,target,"Old",0.1);}
        write(&mut app,target,0.9);app.transport.pretend_at(288.,true);app.smart_record_tick();app.smart_finish();
        let clips=crate::playlist::clips(&app.state);assert_eq!(clips.len(),4);
        assert_eq!((clips[2].position,clips[2].length),(96,96));assert_eq!((clips[3].position,clips[3].length),(192,96));
    }
    #[test]
    fn routing_instruments_preserves_selected_pattern_and_names() {
        let mut app=App::new(None);app.state.project=crate::app::test_project();
        let mut second=app.state.project.channels[0].clone();second.index=1;
        app.state.project.channels=vec![app.state.project.channels[0].clone(),second];
        app.state.project.patterns=vec![os_model::Pattern{number:1,..Default::default()},os_model::Pattern{number:2,..Default::default()}];
        for (row,name,pat) in [(0,"Diva",1),(1,"Vital",2)] {
            app.state.project.channels[row].kind=os_model::ChannelKind::Plugin;
            app.state.project.channels[row].name=Some(name.into());
            app.state.pattern=pat;app.state.project.current_pattern=Some(pat);
            assert!(app.route_instrument_lane(row,10+row as u16));
            assert_eq!(app.state.pattern,pat);
            assert_eq!(app.state.project.current_pattern,Some(pat));
            assert_eq!(instrument_lane(&app.state).unwrap().1,pat);
            assert_eq!(app.state.project.patterns.len(),2);
            assert!(app.state.project.patterns.iter().all(|p|p.name.is_none()));
        }
        // An old saved lane association cannot redirect a newly chosen pattern.
        app.state.pattern=1;
        app.state.project.patterns[0].name=Some("My verse".into());
        assert!(app.route_instrument_lane(1,12));
        assert_eq!(instrument_lane(&app.state).unwrap().1,1);
        assert_eq!(app.state.project.patterns[0].name.as_deref(),Some("My verse"));
        assert_eq!(app.state.project.patterns.len(),2);
    }

    #[test]
    fn assigning_mixer_reuses_the_recorded_playlist_row() {
        use crate::playlist as pl;
        let mut app = App::new(None);
        app.state.project = crate::app::test_project();
        app.state.project.channels[0].kind = os_model::ChannelKind::Plugin;
        app.state.project.patterns[0].notes.push(os_model::Note::step(0, 0));
        let clip = os_model::Clip::new(384, 384, 3, ClipSource::Pattern(1));
        pl::arr_mut(&mut app.state).clips = vec![clip];
        app.route_sample_to(0, 5);
        assert_eq!(instrument_lane(&app.state), Some((3, 1)));
        assert_eq!(app.state.project.channels[0].mixer_track, 5);
        let clips = pl::clips(&app.state);
        assert_eq!(clips.len(), 1);
        assert_eq!((clips[0].track, clips[0].position), (3, 384));
    }

    #[test]
    fn instrument_lane_records_midi_and_roundtrips() {
        use crate::{playlist as pl,record::{Opt,Extra},step_edit::Src};
        let mut app=App::new(None);app.state.project=crate::app::test_project();
        app.state.project.channels[0].kind=os_model::ChannelKind::Plugin;
        app.state.project.channels[0].name=Some("ML Citrus".into());
        app.state.project.channels[0].plugin=Some("Sytrus".into());
        pl::arr_mut(&mut app.state).clips.clear();
        pl::arr_mut(&mut app.state).clips.push(os_model::Clip::new(0,384,0,ClipSource::Pattern(99)));
        assert!(app.route_instrument_lane(0,10));
        let (lane,pat)=instrument_lane(&app.state).unwrap();assert_eq!(lane,1);
        assert_eq!(app.state.project.channels[0].mixer_track,10);
        assert!(pl::track_info(&app.state,lane).unwrap().name.is_some());
        let count=app.state.project.patterns.len();
        app.route_instrument_lane(0,9);assert_eq!(app.state.project.patterns.len(),count);
        // Even a legacy starts-playback preference must not start playback on arm.
        crate::mixer_recording::flag(Opt::StartsPlayback,true);
        app.toggle_record();assert!(app.state.rec.armed);assert!(!app.transport.playing());
        app.toggle_record();
        // No output device: simulate running transport and inject synthetic MIDI events.
        app.transport.pretend_at(0.,true);app.toggle_record();
        assert!(Opt::Notes.on()&&!Opt::Automation.on()&&!Opt::Audio.on());
        assert!(app.state.rec.armed&&app.state.playlist.popup.is_none());
        app.record_down_with(Src::Midi(60),60,100,Extra::default());
        app.transport.pretend_at(96.,true);app.record_up_with(Src::Midi(60),None);
        assert!(app.state.project.pattern(pat).unwrap().notes.iter().any(|n|n.key==60&&n.length>0));
        assert!(pl::clips(&app.state).iter().any(|c|c.track==lane&&c.source==ClipSource::Pattern(pat)));
        // Stop, then record beyond the existing clip: preserve old notes and the
        // timeline gap, extend one clip (including a held note crossing a bar).
        app.toggle_record();app.transport.stop();
        let bar=app.state.project.bar_ticks();
        app.toggle_record();assert!(!app.transport.playing());
        app.transport.pretend_at(f64::from(3*bar),true);
        app.record_down_with(Src::Midi(64),64,100,Extra::default());
        app.transport.pretend_at(f64::from(4*bar+96),true);
        app.record_up_with(Src::Midi(64),None);
        app.end_take();
        let notes=&app.state.project.pattern(pat).unwrap().notes;
        assert!(notes.iter().any(|n|n.key==60&&n.position==0));
        assert!(notes.iter().any(|n|n.key==64&&n.position==3*bar&&n.length==bar+96));
        let clips:Vec<_>=pl::clips(&app.state).iter().filter(|c|c.source==ClipSource::Pattern(pat)).collect();
        assert_eq!(clips.len(),1);assert_eq!(clips[0].position,0);assert_eq!(clips[0].length,5*bar);
        let f=os_flp::project::create(&app.state.project).unwrap();
        let back=os_flp::project::load(&os_flp::FlpFile::parse(&f.to_bytes().unwrap()).unwrap()).unwrap();
        assert_eq!(back.instrument_lanes,app.state.project.instrument_lanes);
        let mut applied=f.clone();os_flp::project::apply(&back,&mut applied).unwrap();
        assert_eq!(os_flp::project::load(&applied).unwrap().instrument_lanes,back.instrument_lanes);
        app.toggle_record();
        pl::run(&mut app,pl::Cmd::Record(Opt::Automation));
        app.toggle_record();assert!(Opt::Automation.on()&&Opt::Notes.on());
    }

    #[test]
    fn smart_mode_arms_without_a_question_and_writes_automation_clips() {
        let mut app=App::new(None);app.state.project=crate::app::test_project();
        crate::mixer_recording::flag(Opt::Smart,true);
        app.transport.pretend_at(0.,true);
        crate::playlist::run(&mut app,crate::playlist::Cmd::Record(Opt::Automation));
        app.toggle_record();assert!(app.state.rec.armed&&app.state.playlist.popup.is_none());
        assert!(Opt::Notes.on()&&Opt::Automation.on()&&!Opt::Audio.on());
        app.state.rec.smart.origin=Some(0);app.transport.pretend_at(96.,true);
        app.smart_begin_host(&Action::Fader(1));
        crate::mixer::track_mut(&mut app.state,1).unwrap().volume=Some(0.25);
        app.smart_record_host(&Action::Fader(1));
        assert!(app.state.project.automations.is_empty(),"do not rebuild the engine on every mouse movement");
        assert_eq!(app.state.rec.smart.active.len(),1);
        app.transport.pretend_at(192.,true);
        crate::mixer::track_mut(&mut app.state,1).unwrap().volume=Some(0.7);
        app.smart_record_host(&Action::Fader(1));
        assert_eq!(app.state.rec.smart.active.len(),1,"same target continues its take");
        app.transport.pretend_at(288.,true);app.smart_record_tick();app.transport.stop();app.smart_record_tick();
        assert!(app.state.rec.smart.active.is_empty());
        assert_eq!(crate::playlist::clips(&app.state).last().unwrap().length,192);
        assert_eq!(app.state.project.automations[0].points.last().unwrap().value,0.7);
        app.state.rec.armed=false;app.transport.pretend_at(384.,true);
        let count=app.state.project.automations.len();app.smart_record_value(os_model::target::TEMPO,"Tempo".into(),0.5);
        assert_eq!(app.state.project.automations.len(),count,"Record off means no automatic capture");
    }
    #[test]
    fn smart_mode_records_midi_notes_without_an_audio_input() {
        let mut app=App::new(None);app.state.project=crate::app::test_project();app.state.song_mode=false;
        crate::mixer_recording::flag(Opt::Smart,true);app.transport.pretend_at(0.,false);app.toggle_record();
        app.transport.pretend_at(0.,false);
        app.record_down_with(crate::step_edit::Src::Midi(60),60,100,crate::record::Extra::default());
        app.transport.pretend_at(96.,false);
        app.record_up_with(crate::step_edit::Src::Midi(60),None);
        assert!(app.state.project.pattern(1).unwrap().notes.iter().any(|n|n.key==60&&n.length>0));
        assert!(app.audio_rec.is_none());
        assert!(app.state.project.automations.is_empty(),"notes do not create unrelated automation");
    }
}
