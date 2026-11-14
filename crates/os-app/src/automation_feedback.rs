// SPDX-License-Identifier: GPL-3.0-or-later
//! Heard automation values are a display overlay, never project edits or recording input.
use crate::{app::{Action,App,State},knobs::KnobId};
use os_model::target::Target;

pub(crate) fn knob_target(s:&State,id:KnobId)->Option<Target> {
    Some(match id {
        KnobId::ChanVol(i)=>Target::ChannelVolume(s.project.channels.get(i)?.index),
        KnobId::ChanPan(i)=>Target::ChannelPan(s.project.channels.get(i)?.index),
        KnobId::ChanPitch(i)=>Target::ChannelPitch(s.project.channels.get(i)?.index),
        KnobId::MasterPitch=>Target::MainPitch,
        KnobId::MixPan(t)=>Target::MixerPan(t),KnobId::MixSep(t)=>Target::MixerSeparation(t),
        KnobId::SlotMix(t,k)=>Target::SlotMix(t,k),KnobId::Send(t,d)=>Target::MixerSend(t,d),
        KnobId::Eq(t,b,f)=>Target::MixerEq(t,b,f),KnobId::Strip(t,k)=>Target::MixerStrip(t,k),
        _=>return None,
    })
}
pub(crate) fn action_target(s:&State,a:&Action)->Option<Target> {
    match *a {
        Action::Knob(k)=>knob_target(s,k),Action::Fader(t)=>Some(Target::MixerVolume(t)),
        Action::Tempo=>Some(Target::Tempo),
        Action::GenericValue(key,index)=>crate::params::target_of(s,key,index).map(os_model::target::decode),
        Action::Mix(crate::mixer::MHit::SlotLed(k))=>Some(Target::SlotOn(s.selected_mixer,k)),
        _=>None,
    }
}
impl State {
    pub(crate) fn displayed_volume(&self,t:u16)->f64 {
        self.params.playback.get(&Target::MixerVolume(t)).copied()
            .unwrap_or_else(||crate::mixer::track(self,t).and_then(|m|m.volume).unwrap_or(0.8))
    }
    pub(crate) fn displayed_slot_on(&self,t:u16,k:u8)->bool {
        self.params.playback.get(&Target::SlotOn(t,k)).map_or_else(
            ||crate::mixer::track(self,t).is_none_or(|m|m.slot_on[k as usize]),|&v|v>=0.5)
    }
}
impl App {
    pub(crate) fn automation_feedback_tick(&mut self) {
        if let Some(values)=self.transport.automation_values(){self.show_automation_values(values);}
    }
    pub(crate) fn show_automation_values(&mut self,values:Vec<(Target,f64)>) {
        let mut next:std::collections::HashMap<_,_>=values.into_iter().filter(|(_,v)|v.is_finite()).collect();
        // The mouse wins while writing; render-ahead feedback must not pull the held knob backwards.
        for &target in self.state.rec.smart.active.keys(){next.remove(&os_model::target::decode(target));}
        let previous=std::mem::replace(&mut self.state.params.playback,next);
        let current=&self.state.params.playback;
        let changed:std::collections::HashSet<_>=previous.keys().chain(current.keys()).copied()
            .filter(|t|previous.get(t)!=current.get(t)).collect();
        if changed.is_empty(){return;}
        let mut rects=Vec::new();
        for hit in &self.hits {
            let control=action_target(&self.state,&hit.action).is_some_and(|t|changed.contains(&t));
            let strip=matches!(hit.action,Action::Mix(crate::mixer::MHit::Strip(t)) if changed.contains(&Target::MixerVolume(t)));
            if control||strip{rects.push(hit.rect.outset(12.));}
        }
        for r in rects {self.damage_rect(r);}
        if changed.iter().any(|t|matches!(t,Target::MixerVolume(_)|Target::MixerPan(_)|Target::SlotOn(..))) {
            self.damage_live(crate::app::Live::AttachedInspector);
        }
    }
}
#[cfg(plugin_windows)]
pub(crate) fn window_info(s:&State,row:usize)->Option<crate::plugin_window::ChannelInfo> {
    let mut info=if row>=crate::plugins::FX_KEY {crate::mixer::fx_info(&s.project,&s.mixer,row)}else{crate::app::channel_info(&s.project,row)}?;
    if let Some((t,k))=crate::plugins::fx_of(row){info.enabled=s.displayed_slot_on(t,k);}
    else if let Some(ch)=s.project.channels.get(row) {
        if let Some(&v)=s.params.playback.get(&Target::ChannelVolume(ch.index)){info.volume=v;}
        if let Some(&v)=s.params.playback.get(&Target::ChannelPan(ch.index)){info.pan=2.*v-1.;}
        if let Some(&v)=s.params.playback.get(&Target::ChannelPitch(ch.index)){info.pitch_cents=((v-0.5)*9600.).round() as i32;}
    }
    Some(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recorded_and_manual_clips_feed_original_controls_without_project_edits() {
        use os_model::{AutoPoint,Clip,ClipSource,target};
        for recorded in [false,true] {
            let mut app=App::new(None);app.state.project=crate::app::test_project();
            app.state.project.patterns[0].notes.push(os_model::Note::step(0,0));
            crate::playlist::arr_mut(&mut app.state).clips=vec![Clip::new(0,384,2,ClipSource::Pattern(1))];
            let tgt=target::channel(0,target::CHANNEL_VOLUME);
            let ch=if recorded {crate::playlist::recorded_automation(&mut app.state,tgt,"Volume",0,384,0.2).unwrap()}
                else {crate::playlist::create_automation(&mut app.state,tgt,"Volume",0.2)};
            app.state.project.automation_mut(ch).unwrap().points=vec![AutoPoint::new(0.,0.2),AutoPoint::new(2.,0.8)];
            let original=app.state.project.clone();
            let mut engine=os_engine::Engine::new(&original,&mut os_engine::SampleLibrary::default(),48000,os_engine::PlayMode::Song);
            let mut previous=0.;
            for tick in [0,96,192] {
                engine.seek_tick(tick);engine.next_tick();
                let mut values=Vec::new();engine.automation_values(&mut values);app.show_automation_values(values);
                let current=app.state.knob(KnobId::ChanVol(0));assert!(current>previous);previous=current;
                #[cfg(plugin_windows)]
                assert!((window_info(&app.state,0).unwrap().volume-f64::from(current)).abs()<1e-6);
            }
            assert_eq!(app.state.project,original);
            // A user edit takes precedence over delayed playback feedback while writing.
            app.state.rec.armed=true;crate::settings_file::set_flag("rec_automation",true);
            crate::settings_file::set_flag("rec_filter_explicit",true);app.transport.pretend_at(192.,true);
            app.smart_begin_host(&Action::Knob(KnobId::ChanVol(0)));
            app.state.set_knob(KnobId::ChanVol(0),0.6);
            app.smart_record_host(&Action::Knob(KnobId::ChanVol(0)));
            app.show_automation_values(vec![(Target::ChannelVolume(0),0.1)]);
            assert!((app.state.knob(KnobId::ChanVol(0))-0.6).abs()<1e-6);
        }
    }
}
