// SPDX-License-Identifier: GPL-3.0-or-later
//! Commit a take as one editable curve per target. No work is done on the audio callback.
use crate::{app::State,playlist as pl,smart_recording::Gesture};
use os_model::{AutoMode,AutoPoint,Automation,Clip,ClipSource};

struct Layer { start:f64,end:f64,offset:f64,curve:Automation }
impl Layer {fn value(&self,at:f64)->f64 {self.curve.value_at(at-self.start+self.offset).unwrap_or(0.5)}}

/// Keep ordinary, complete segments in their original shape. Only clipped nonlinear segments
/// (and Smooth segments whose neighbour dependence could change) need local resampling.
fn piece(out:&mut Vec<AutoPoint>,layer:&Layer,a:f64,b:f64,origin:f64) {
    let from=a-layer.start+layer.offset;
    let to=b-layer.start+layer.offset;
    let points=&layer.curve.points;
    let i=points.partition_point(|p|p.beat<=from+1e-10);
    let start=layer.value(a);
    if out.last().is_none_or(|p|(p.value-start).abs()>1e-7) {
        out.push(AutoPoint{mode:AutoMode::Hold,..AutoPoint::new(a-origin,start)});
    }
    if let (Some(left),Some(right))=(i.checked_sub(1).and_then(|j|points.get(j)),points.get(i)) {
        if (left.beat-from).abs()<1e-9 && (right.beat-to).abs()<1e-9 && right.mode!=AutoMode::Smooth {
            out.push(AutoPoint{beat:b-origin,..*right});return;
        }
        let straight=right.tension==0. && matches!(right.mode,AutoMode::Single|AutoMode::Double|AutoMode::Single2|AutoMode::Double2|AutoMode::HalfSine|AutoMode::Single3|AutoMode::Double3);
        if straight || right.mode==AutoMode::Hold {
            out.push(AutoPoint{mode:if right.mode==AutoMode::Hold{AutoMode::Hold}else{AutoMode::Single},..AutoPoint::new(b-origin,layer.value(b))});return;
        }
        // Oscillating modes can contain up to 2,002 transitions per source segment.
        let oscillating=matches!(right.mode,AutoMode::Wave|AutoMode::Pulse|AutoMode::Stairs|AutoMode::SmoothStairs);
        let seeds=if oscillating {((8192.*(to-from)/(right.beat-left.beat).max(1e-9)).ceil() as usize).max(16)}else{8};
        for n in 0..seeds {
            let x=a+(b-a)*n as f64/seeds as f64;let y=a+(b-a)*(n+1) as f64/seeds as f64;
            refine(out,layer,x,y,layer.value(x),layer.value(y),origin,0);
        }
    }else{out.push(AutoPoint::new(b-origin,layer.value(b)));}
}
fn refine(out:&mut Vec<AutoPoint>,layer:&Layer,a:f64,b:f64,va:f64,vb:f64,origin:f64,depth:u8) {
    let mid=(a+b)*0.5;let vm=layer.value(mid);
    let error=[0.25,0.5,0.75].into_iter().map(|t|(layer.value(a+(b-a)*t)-(va+(vb-va)*t)).abs()).fold(0.,f64::max);
    if error>0.000001 && depth<16 {
        refine(out,layer,a,mid,va,vm,origin,depth+1);refine(out,layer,mid,b,vm,vb,origin,depth+1);
    }else{out.push(AutoPoint::new(b-origin,vb));}
}

fn curve(layers:&[Layer],recorded:usize,start:f64,end:f64,initial:f64)->Vec<AutoPoint> {
    let mut cuts=vec![start,end];
    for layer in layers {
        cuts.extend([layer.start,layer.end]);
        cuts.extend(layer.curve.points.iter().map(|p|p.beat+layer.start-layer.offset).filter(|&b|b>layer.start&&b<layer.end));
    }
    cuts.retain(|x|x.is_finite()&&*x>=start&&*x<=end);cuts.sort_by(f64::total_cmp);cuts.dedup_by(|a,b|(*a-*b).abs()<1e-10);
    let mut out=Vec::new();let mut held=initial;
    for span in cuts.windows(2) {
        let (a,b)=(span[0],span[1]);let mid=(a+b)*0.5;
        // Last recorded pass wins, including loop passes; otherwise the existing audible curve wins.
        let active=layers[recorded..].iter().rev().find(|l|l.start<=mid&&mid<l.end)
            .or_else(||layers[..recorded].iter().rev().find(|l|l.start<=mid&&mid<l.end));
        if let Some(layer)=active {piece(&mut out,layer,a,b,start);held=layer.value(b);}
        else {
            if out.is_empty(){out.push(AutoPoint::new(a-start,held));}
            out.push(AutoPoint{mode:AutoMode::Hold,..AutoPoint::new(b-start,held)});
        }
    }
    // Collapse only redundant flat points; never erase a step or reshape a curved segment.
    let mut compact:Vec<AutoPoint>=Vec::new();
    for point in out {
        let n=compact.len();
        if n>=2 && compact[n-2].value==point.value && compact[n-1].value==point.value
            && matches!(point.mode,AutoMode::Single|AutoMode::Hold) && compact[n-1].tension==0. && point.tension==0.
            && matches!(compact[n-1].mode,AutoMode::Single|AutoMode::Hold) {compact.pop();}
        compact.push(point);
    }
    compact
}

pub(super) fn commit(s:&mut State,target:u32,gestures:&[Gesture],initial:f64)->bool {
    let ppq=f64::from(s.project.ppq.max(1));
    let mut old:Vec<(usize,Clip,Automation)>=pl::clips(s).iter().enumerate().filter_map(|(i,c)|{
        let ClipSource::Channel(ch)=c.source else{return None;};
        let a=s.project.automation(ch)?;
        (!c.muted && pl::track_on(s,c.track) && a.targets.contains(&target)).then(||(i,c.clone(),a.clone()))
    }).collect();
    old.sort_by_key(|(i,c,_)|(c.position,*i));
    let start=gestures.iter().map(|g|g.start).chain(old.iter().map(|(_,c,_)|c.position)).min().unwrap();
    let end=gestures.iter().map(|g|g.end).chain(old.iter().map(|(_,c,_)|c.end())).max().unwrap();
    if old.iter().any(|(_,c,_)|pl::track_info(s,c.track).is_some_and(|r|r.locked||r.frozen.is_some()||r.folder)) {return false;}
    if old.is_empty() && pl::recording_destination(s,target,start,end).is_none(){return false;}
    let mut layers=Vec::new();
    // Pattern event lanes are lower-priority automation, and Touch restores their latest value.
    let has_events=matches!(os_model::target::decode(target),os_model::target::Target::ChannelVolume(_)|os_model::target::Target::ChannelPan(_)|os_model::target::Target::ChannelPitch(_));
    let schedule=if has_events{os_engine::engine::schedule_controls(&s.project,os_engine::PlayMode::Song)}else{Vec::new()};
    let mut events:Vec<_>=schedule.into_iter()
        .filter(|&(tick,ch,param,_)|tick<u64::from(end)&&os_model::target::channel(ch as u16,param)==target)
        .filter_map(|(tick,_,_,value)|os_model::InitControl{head:0,target,value}.unit().map(|value|(tick,value))).collect();
    events.sort_by_key(|e|e.0);
    if let Some(&(first,_))=events.first() {
        let mut points=Vec::new();
        for (tick,value) in events {points.push(AutoPoint{mode:AutoMode::Hold,..AutoPoint::new((tick-first) as f64/ppq,value)});}
        layers.push(Layer{start:first as f64/ppq,end:f64::from(end)/ppq,offset:0.,curve:Automation{channel:0,targets:vec![target],points,header:Vec::new(),tail:Vec::new()}});
    }
    layers.extend(old.iter().map(|(_,c,a)|Layer{start:f64::from(c.position)/ppq,end:f64::from(c.end())/ppq,offset:c.window.map_or(0.,|w|w.0),curve:a.clone()}));
    let recorded=layers.len();
    for g in gestures {
        layers.push(Layer{start:f64::from(g.start)/ppq,end:f64::from(g.end)/ppq,offset:0.,
            curve:Automation{channel:0,targets:vec![target],points:g.points.clone(),header:Vec::new(),tail:Vec::new()}});
    }
    let points=curve(&layers,recorded,f64::from(start)/ppq,f64::from(end)/ppq,initial);
    let reusable=old.iter().find(|(_,_,a)|a.targets==[target] && s.project.arrangements.iter().flat_map(|a|&a.clips).filter(|c|c.source==ClipSource::Channel(a.channel)).count()==1);
    // Tail plus Playlist includes any independent copies needed for shared/multi-target curves.
    let tail=crate::history::Snap::Tail(Box::new(crate::history::Tail::now(s,s.project.channels.len())));
    pl::snapshot_with(s,"Record automation",tail);
    let (ch,mut clip)=if let Some((_,clip,a))=reusable {(a.channel,clip.clone())}else {
        let ch=if let Some((_,clip,_))=old.first(){pl::recorded_automation_on_row(s,target,&gestures[0].name,start,end,initial,clip.track)}
            else{pl::recorded_automation(s,target,&gestures[0].name,start,end,initial).unwrap()};
        (ch,pl::clips(s).last().unwrap().clone())
    };
    // Remove only current-arrangement instances of this target. Shared source curves and other
    // arrangements remain intact. Multi-target clips get an independent copy of their other links.
    let mut replacements=std::collections::HashMap::new();
    for (index,original,a) in &old {
        if a.targets.len()>1 {
            let id=s.project.channels.iter().map(|c|c.index).max().unwrap_or(0)+1;
            let mut channel=s.project.channel(a.channel).unwrap().clone();channel.index=id;s.project.channels.push(channel);
            let mut other=a.clone();other.channel=id;other.targets.retain(|&t|t!=target);s.project.automations.push(other);
            let mut c=original.clone();c.source=ClipSource::Channel(id);replacements.insert(*index,c);
            if s.playlist.automation_owners.contains_key(&a.channel){s.playlist.automation_owners.insert(id,original.track);}
        }
    }
    let indexes:std::collections::HashSet<_>=old.iter().map(|(i,_,_)|*i).collect();
    let clips=std::mem::take(&mut pl::arr_mut(s).clips);
    pl::arr_mut(s).clips=clips.into_iter().enumerate().filter_map(|(i,c)|{
        if indexes.contains(&i){replacements.remove(&i)}else if c.source==ClipSource::Channel(ch){None}else{Some(c)}
    }).collect();
    clip.position=start;clip.length=end-start;clip.window=None;clip.source=ClipSource::Channel(ch);
    s.project.automation_mut(ch).unwrap().points=points;
    let row=clip.track;pl::arr_mut(s).clips.push(clip);
    if old.iter().any(|(_,_,a)|s.playlist.automation_owners.contains_key(&a.channel)) || s.playlist.automation_owners.contains_key(&ch)
        || pl::clips(s).iter().any(|c|c.track==row&&matches!(c.source,ClipSource::Pattern(_))) {
        s.playlist.automation_owners.insert(ch,row);s.playlist.automation_views.insert(row,ch);s.playlist.automation_last.insert(row,ch);
    }
    s.playlist.sel=vec![pl::clips(s).len()-1];s.playlist.track=row;s.playlist.source=Some(ClipSource::Channel(ch));
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use os_model::target;
    fn app()->App {
        crate::settings_file::set_flag("rec_automation",true);crate::settings_file::set_flag("rec_filter_explicit",true);
        crate::settings_file::set_flag("automation_record_latch",false);crate::settings_file::set_flag("automation_record_separate_clips",false);
        let mut app=App::new(None);app.state.project=crate::app::test_project();
        app.state.project.patterns[0].notes.push(os_model::Note::step(0,0));
        pl::arr_mut(&mut app.state).clips=vec![Clip::new(0,768,3,ClipSource::Pattern(1))];
        app.state.rec.armed=true;app
    }
    fn gesture(app:&mut App,t:u32,start:u32,end:u32,v:f64) {
        app.transport.pretend_at(f64::from(start),true);app.smart_begin_parameter(t,0.5,true);
        app.smart_record_value(t,"Volume".into(),v);
        app.transport.pretend_at(f64::from(end),true);app.smart_end_parameter(t);
    }
    fn finish(app:&mut App){app.transport.stop();app.smart_record_tick();}
    fn placed(s:&State,t:u32)->Vec<&Clip>{pl::clips(s).iter().filter(|c|matches!(c.source,ClipSource::Channel(ch) if s.project.automation(ch).is_some_and(|a|a.targets.contains(&t)))).collect()}
    #[test]
    fn repeated_gestures_and_later_takes_reuse_one_clip_without_gap_ramps() {
        let mut app=app();let t=target::channel(0,target::CHANNEL_VOLUME);
        gesture(&mut app,t,96,144,0.2);gesture(&mut app,t,240,288,0.8);finish(&mut app);
        let clips=placed(&app.state,t);assert_eq!(clips.len(),1);assert_eq!((clips[0].track,clips[0].position,clips[0].end()),(3,96,288));
        let ClipSource::Channel(ch)=clips[0].source else{panic!()};
        assert!((app.state.project.automation(ch).unwrap().value_at(1.).unwrap()-0.2).abs()<1e-6);
        let before=app.state.project.automation(ch).unwrap().clone();
        gesture(&mut app,t,336,384,0.4);finish(&mut app);
        assert_eq!(placed(&app.state,t).len(),1);assert_eq!(placed(&app.state,t)[0].source,ClipSource::Channel(ch));
        assert_eq!(placed(&app.state,t)[0].end(),384);
        assert_eq!(crate::history::undo(&mut app.state),Some("Record automation"));
        assert_eq!(app.state.project.automation(ch),Some(&before));assert_eq!(placed(&app.state,t)[0].end(),288);
        crate::history::redo(&mut app.state);assert_eq!(placed(&app.state,t)[0].end(),384);
        let bytes=os_flp::project::create(&app.state.project).unwrap().to_bytes().unwrap();
        app.state.project=os_flp::project::load(&os_flp::FlpFile::parse(&bytes).unwrap()).unwrap();
        gesture(&mut app,t,480,528,0.6);finish(&mut app);
        assert_eq!(placed(&app.state,t).len(),1);assert_eq!(placed(&app.state,t)[0].end(),528);
    }
    #[test]
    fn untouched_curves_and_later_clips_survive_recording_and_loop_overwrites() {
        let mut app=app();let t=target::channel(0,target::CHANNEL_VOLUME);
        let ch=pl::create_automation(&mut app.state,t,"Existing",0.1);
        let points=vec![AutoPoint::new(0.,0.1),AutoPoint{tension:0.6,..AutoPoint::new(2.,0.9)},AutoPoint::new(8.,0.3)];
        app.state.project.automation_mut(ch).unwrap().points=points;
        let original=app.state.project.automation(ch).unwrap().clone();
        gesture(&mut app,t,96,144,0.25);gesture(&mut app,t,288,336,0.75);
        // Another loop pass replaces only its touched part of the earlier take.
        gesture(&mut app,t,108,120,0.6);finish(&mut app);
        assert_eq!(placed(&app.state,t).len(),1);
        let merged=app.state.project.automation(ch).unwrap();
        for tick in [24.,72.,150.,180.,240.,350.,600.] {
            assert!((merged.value_at(tick/96.).unwrap()-original.value_at(tick/96.).unwrap()).abs()<0.00001,"untouched tick {tick}");
        }
        for (tick,want) in [(100.,0.25),(110.,0.6),(130.,0.25),(300.,0.75)] {
            assert!((merged.value_at(tick/96.).unwrap()-want).abs()<1e-6);
        }
    }
    #[test]
    fn shared_sources_and_other_parameter_links_are_not_overwritten() {
        let mut app=app();let t=target::channel(0,target::CHANNEL_VOLUME);let other=target::channel(0,target::CHANNEL_PAN);
        let ch=pl::create_automation(&mut app.state,t,"Shared",0.3);
        app.state.project.automation_mut(ch).unwrap().targets.push(other);
        let original=app.state.project.automation(ch).unwrap().clone();
        let mut another=app.state.project.arrangement().unwrap().clone();another.number+=1;
        app.state.project.arrangements.push(another.clone());app.state.history.reset();
        gesture(&mut app,t,96,192,0.8);finish(&mut app);
        assert_eq!(placed(&app.state,t).len(),1);assert_eq!(placed(&app.state,other).len(),1);
        assert_eq!(app.state.project.automation(ch),Some(&original));
        assert_eq!(app.state.project.arrangements.last(),Some(&another));
        crate::history::undo(&mut app.state);assert_eq!(placed(&app.state,t)[0].source,ClipSource::Channel(ch));
        crate::history::redo(&mut app.state);assert_eq!(placed(&app.state,t).len(),1);assert_eq!(placed(&app.state,other).len(),1);
    }
    #[test]
    fn checkpoint_and_cancel_keep_one_clip_and_separate_mode_is_optional() {
        let mut app=app();let t=target::channel(0,target::CHANNEL_VOLUME);
        gesture(&mut app,t,96,144,0.2);app.smart_checkpoint();
        gesture(&mut app,t,240,288,0.8);app.smart_checkpoint();assert_eq!(placed(&app.state,t).len(),1);
        let saved=app.state.project.automation(app.state.project.automations[0].channel).unwrap().clone();
        gesture(&mut app,t,336,384,0.4);app.smart_cancel();assert_eq!(app.state.project.automations[0],saved);
        app.rec_cmd(crate::record::RecCmd::SeparateAutomationClips(true));
        gesture(&mut app,t,480,528,0.6);gesture(&mut app,t,576,624,0.7);finish(&mut app);
        assert_eq!(placed(&app.state,t).len(),3);
    }
    #[test]
    fn one_clip_is_default_and_visibility_does_not_change_the_setting() {
        assert!(!crate::smart_recording::separate_clips());
        let mut app=App::new(None);app.record_menu(10.,10.);
        assert!(app.state.playlist.popup.as_ref().unwrap().items.iter().any(|i|i.label=="Record as separate clips"));
        app.rec_cmd(crate::record::RecCmd::SeparateAutomationClips(true));
        crate::settings_file::set_flag(crate::smart_recording::CLIPS_VISIBILITY,false);app.record_menu(10.,10.);
        assert!(!app.state.playlist.popup.as_ref().unwrap().items.iter().any(|i|i.label=="Record as separate clips"));
        assert!(crate::smart_recording::separate_clips());
    }
    #[test]
    fn overlapping_windowed_clips_and_pattern_events_keep_their_gap_values() {
        let mut app=app();let t=target::channel(0,target::CHANNEL_VOLUME);
        app.state.project.patterns[0].controls=vec![os_model::ControlPoint::new(0,0,0,3840),os_model::ControlPoint::new(180,0,0,7680)];
        gesture(&mut app,t,96,144,0.2);gesture(&mut app,t,240,288,0.8);finish(&mut app);
        let ClipSource::Channel(ch)=placed(&app.state,t)[0].source else{panic!()};
        let curve=app.state.project.automation(ch).unwrap();
        assert!((curve.value_at(60./96.).unwrap()-0.3).abs()<1e-6);
        assert!((curve.value_at(110./96.).unwrap()-0.6).abs()<1e-6);
        let later=pl::recorded_automation(&mut app.state,t,"Later",192,384,0.1).unwrap();
        app.state.project.automation_mut(later).unwrap().points=vec![AutoPoint::new(0.,0.),AutoPoint::new(4.,1.)];
        pl::arr_mut(&mut app.state).clips.last_mut().unwrap().window=Some((1.,3.));
        gesture(&mut app,t,216,240,0.9);finish(&mut app);
        assert_eq!(placed(&app.state,t).len(),1);
        let ClipSource::Channel(ch)=placed(&app.state,t)[0].source else{panic!()};
        let p=app.state.project.automation(ch).unwrap();
        assert!((p.value_at((228.-96.)/96.).unwrap()-0.9).abs()<1e-6);
        assert!((p.value_at((300.-96.)/96.).unwrap()-(1.+108./96.)/4.).abs()<1e-6);
        let mut e=os_engine::Engine::new(&app.state.project,&mut os_engine::SampleLibrary::default(),48000,os_engine::PlayMode::Song);
        e.seek_tick(228);e.next_tick();let mut values=Vec::new();e.automation_values(&mut values);
        assert!(values.iter().any(|(target,value)|*target==os_model::target::Target::ChannelVolume(0)&&(*value-0.9).abs()<1e-6));
    }
    #[test]
    fn latch_checkpoints_extend_one_curve_and_locked_takes_keep_their_mode() {
        let mut app=app();let t=target::channel(0,target::CHANNEL_VOLUME);
        crate::settings_file::set_flag("automation_record_latch",true);
        gesture(&mut app,t,96,144,0.2);app.transport.pretend_at(192.,true);app.smart_checkpoint();
        app.transport.pretend_at(288.,true);app.smart_record_tick();finish(&mut app);
        assert_eq!(placed(&app.state,t).len(),1);assert_eq!(placed(&app.state,t)[0].end(),288);
        pl::arr_mut(&mut app.state).tracks[3].locked=true;
        gesture(&mut app,t,336,384,0.7);finish(&mut app);assert!(app.smart_pending());
        assert!(app.rec_cmd(crate::record::RecCmd::SeparateAutomationClips(true)).is_some());
        assert!(!crate::smart_recording::separate_clips());
        pl::arr_mut(&mut app.state).tracks[3].locked=false;app.smart_finish();
        assert!(!app.smart_pending());assert_eq!(placed(&app.state,t).len(),1);assert_eq!(placed(&app.state,t)[0].end(),384);
    }
}
