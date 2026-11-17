// SPDX-License-Identifier: GPL-3.0-or-later
// Included by mixer.rs: structural deletion and its bounded host Undo entry.

#[derive(Debug,Clone,PartialEq)]
pub(crate) struct DeleteSnapshot {
    tracks: Vec<MixerTrack>,
    routes: Vec<(u16,u16)>,
    automations: Vec<os_model::Automation>,
    init: Vec<os_model::InitControl>,
    peaks: Vec<os_model::PeakLink>,
    controls: Vec<(u16,Vec<os_model::ControlPoint>)>,
    arrangements: Vec<os_model::Arrangement>,
    names: HashMap<(u16,u8),String>,
    inputs: HashMap<u16,crate::capture::Input>,
    armed: std::collections::BTreeSet<u16>,
    selected: u16,
    selection: Vec<u16>,
}

impl DeleteSnapshot {
    pub(crate) fn bytes(&self)->usize {self.tracks.iter().flat_map(|t|t.slots.iter().flatten()).map(|p|p.state.len()).sum()}
    pub(crate) fn now(s:&State)->Self {
        Self { tracks:s.project.mixer.clone(),routes:s.project.channels.iter().map(|c|(c.index,c.mixer_track)).collect(),
            automations:s.project.automations.clone(),init:s.project.init_controls.clone(),peaks:s.project.peak_links.clone(),
            controls:s.project.patterns.iter().map(|p|(p.number,p.controls.clone())).collect(),arrangements:s.project.arrangements.clone(),
            names:s.mixer.slot_names.clone(),inputs:s.mixer.recording.inputs.clone(),armed:s.mixer.recording.armed.clone(),
            selected:s.selected_mixer,selection:s.mixer.sel.clone() }
    }
    pub(crate) fn rescale(&mut self,from:u16,to:u16) {
        for (_,points) in &mut self.controls {for p in points {p.rescale(from,to);}}
        for a in &mut self.arrangements {
            for c in &mut a.clips {c.rescale(from,to);}
            for clips in a.tracks.iter_mut().filter_map(|t|t.frozen.as_mut()) {for c in clips {c.rescale(from,to);}}
            for m in &mut a.markers {m.rescale(from,to);}
            for n in &mut a.staff_notation {n.rescale(from,to);}
            a.timeline.rescale(from,to);
        }
    }
    pub(crate) fn put(self,s:&mut State) {
        for t in &s.project.mixer {for slot in 0..64 {s.mixer.pending.push(FxReq::Delete{track:t.index,slot});}}
        crate::peak_link::restore_removed(s,&self.peaks);
        s.project.mixer=self.tracks;s.project.automations=self.automations;s.project.init_controls=self.init;s.project.peak_links=self.peaks;
        s.project.arrangements=self.arrangements;
        for c in &mut s.project.channels {if let Some((_,t))=self.routes.iter().find(|(id,_)|*id==c.index){c.mixer_track=*t;}}
        for p in &mut s.project.patterns {if let Some((_,v))=self.controls.iter().find(|(id,_)|*id==p.number){p.controls=v.clone();}}
        s.mixer.slot_names=self.names;s.mixer.recording.inputs=self.inputs;s.mixer.recording.armed=self.armed;
        s.selected_mixer=self.selected;s.mixer.sel=self.selection;
        clear_deleted_ui(s);changed(s);
    }
}

#[cfg(test)]
mod delete_insert_tests {
    use super::*;
    #[test]
    fn delete_inserts_keyboard_and_live_effect_restore() {
        use winit::keyboard::{Key,NamedKey};
        let mut app=App::new(None);
        app.plugins=crate::plugins::Plugins::builtins();
        let mut eq=lemon_parametric_eq_2::params::default_state();
        lemon_parametric_eq_2::params::set_field(&mut eq,1,-600);
        track_mut(&mut app.state,2).unwrap().slots[0]=Some(os_model::SlotPlugin{name:crate::plugins::EQ_FILE_NAME.into(),state:eq});
        app.sync_effects();
        let effect=crate::plugins::fx_key(2,0);
        let saved=app.plugins.eq_file_state(effect).expect("live EQ loaded");
        assert!(key(&mut app,&Key::Character("a".into()),true,false));
        assert!(key(&mut app,&Key::Named(NamedKey::Delete),false,false));
        app.mixer_tick();assert_eq!(inserts(&app.state),0);assert!(!app.plugins.hosted.contains_key(&effect));
        app.undo_key(false);app.mixer_tick();app.sync_effects();
        assert_eq!(app.plugins.eq_file_state(effect),Some(saved));
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("Four inserts.lmn");
        app.state.selected_mixer=5;app.state.mixer.sel=(5..=inserts(&app.state)).collect();
        run(&mut app,MCmd::Delete(5));app.save_to(path.clone());
        let mut reopened=App::new(None);reopened.open(path);
        assert_eq!(inserts(&reopened.state),4);
        assert!(track(&reopened.state,2).unwrap().slots[0].is_some());
    }
    #[test]
    fn delete_inserts_empty_mixer_plus_works_in_every_layout() {
        for layout in [COMPACT,WIDE,crate::rack_mixer::VIEW,10,11,12,13,14,15,16,17] {
            let mut app=App::new(None);
            app.state.z=vec![Win::Mixer];app.state.show_browser=false;
            app.state.startup_layout_pending=false;
            app.state.wins[Win::Mixer.slot().unwrap()].maxed=true;
            app.state.mixer.layout=layout;
            app.state.selected_mixer=1;app.state.mixer.sel=(1..=inserts(&app.state)).collect();
            run(&mut app,MCmd::Delete(1));
            let mut canvas=os_ui::Canvas::new(1500,900,1.).unwrap();
            app.paint(&mut canvas,None);
            let r=app.hits.iter().rev().find(|h|h.action==Action::Mix(MHit::Add)).unwrap_or_else(||panic!("Missing + in layout {layout}")).rect;
            assert!(r.x>=0. && r.right()<=1500. && r.y>=0. && r.bottom()<=900.);
            app.mouse_move(r.cx(),r.cy());app.mouse_down();app.mouse_up();
            assert_eq!(inserts(&app.state),1,"+ in layout {layout}");
        }
    }
    #[test]
    fn delete_inserts_locked_selection_is_atomic() {
        let mut app=App::new(None);
        track_mut(&mut app.state,2).unwrap().edit_locked=true;
        let before=app.state.project.clone();
        app.state.selected_mixer=1;app.state.mixer.sel=vec![1,2,3];
        assert!(run(&mut app,MCmd::Delete(1)).is_some());
        assert_eq!(app.state.project,before);
    }
    #[test]
    fn delete_inserts_all_undo_redo_save_and_add() {
        let mut app=App::new(None);
        let before=app.state.project.clone();
        let n=inserts(&app.state);
        app.state.selected_mixer=1;app.state.mixer.sel=(0..=n).collect();app.state.mixer_scroll=10000.;
        run(&mut app,MCmd::Delete(1));
        assert_eq!(inserts(&app.state),0);assert_eq!(max_insert(&app.state.project),0);
        assert_eq!(app.state.project.mixer.len(),2);assert_eq!(app.state.selected_mixer,0);assert_eq!(app.state.mixer_scroll,0.);
        assert!(app.state.project.channels.iter().all(|c|c.mixer_track==0));
        crate::history::undo(&mut app.state);assert_eq!(app.state.project,before);
        crate::history::redo(&mut app.state);assert_eq!(inserts(&app.state),0);
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("Master only.lmn");
        app.save_to(path.clone());assert!(path.is_file(),"{:?}",app.state.message);
        let back=os_flp::project::load(&os_flp::FlpFile::parse(&std::fs::read(&path).unwrap()).unwrap()).unwrap();
        assert_eq!(back.mixer.len(),2);assert!(back.channels.iter().all(|c|c.mixer_track==0));
        let mut reopened=App::new(None);reopened.open(path);
        assert_eq!(inserts(&reopened.state),0);assert_eq!(reopened.state.selected_mixer,0);
        hit(&mut reopened,MHit::Add);assert_eq!(inserts(&reopened.state),1);
        hit(&mut app,MHit::Add);assert_eq!(inserts(&app.state),1);assert_eq!(app.state.selected_mixer,1);
        assert_eq!(track(&app.state,1).unwrap().routes,vec![0]);
        crate::history::undo(&mut app.state);assert_eq!(inserts(&app.state),0);
        crate::history::redo(&mut app.state);assert_eq!(inserts(&app.state),1);
    }
    #[test]
    fn delete_inserts_preserves_surviving_routing_automation_and_sidechain() {
        use os_model::target as t;
        let mut app=App::new(None);
        app.state.project.channels[0].mixer_track=2;
        track_mut(&mut app.state,3).unwrap().name=Some("Keep".into());
        track_mut(&mut app.state,3).unwrap().routes=vec![0,4];
        let reference=os_model::PluginRef{format:"clap".into(),name:"Comp".into(),path:"Comp.clap".into(),..Default::default()};
        let blob=os_flp::wrapper::set_input_port(&os_flp::wrapper::encode(None,&reference),1,2).unwrap();
        track_mut(&mut app.state,1).unwrap().routes=vec![4];
        track_mut(&mut app.state,4).unwrap().slots[0]=Some(os_model::SlotPlugin{name:os_flp::wrapper::WRAPPER.into(),state:blob});
        for n in [2,3] {app.state.project.init_controls.push(os_model::InitControl{head:0,target:t::mixer(n,t::MIXER_PAN),value:123});}
        app.state.selected_mixer=2;app.state.mixer.sel.clear();
        let before=app.state.project.clone();
        run(&mut app,MCmd::Delete(2));
        assert_eq!(app.state.project.channels[0].mixer_track,0);
        assert_eq!(name(&app.state,2),"Keep");assert_eq!(track(&app.state,2).unwrap().routes,vec![0,3]);
        let controls:Vec<_>=app.state.project.init_controls.iter().filter(|c|c.value==123).map(|c|c.target).collect();
        assert_eq!(controls,vec![t::mixer(2,t::MIXER_PAN)]);
        assert_eq!(crate::project_plugins::slot_ports(&app.state.project,3,0),(0,2));
        crate::history::undo(&mut app.state);assert_eq!(app.state.project,before);
    }
}

fn clear_deleted_ui(s:&mut State) {
    s.mixer_scroll=0.;s.mixer.solo=None;s.mixer.wheel_grab=None;s.mixer.insertion_hover=None;s.mixer.insertion_ready=false;
    s.mixer.meters.clear();s.mixer.peak_view.clear();s.mixer.slot_scroll.clear();s.mixer.slots_shown.clear();
    s.mixer.rack_pages.clear();s.mixer.rack_errors.clear();s.mixer.rack_min_height.clear();s.mixer.rack_auto_width.clear();
    crate::attached_mixer::clear(s);
}

/// Delete references to removed inserts; retarget surviving insert identities.
fn delete_target(id:u32,map:&impl Fn(u16)->Option<u16>)->Option<u32> {
    use os_model::target::{self as t,Target::*};
    Some(match t::decode(id) {
        MixerVolume(n)=>t::mixer(map(n)?,t::MIXER_VOLUME),MixerPan(n)=>t::mixer(map(n)?,t::MIXER_PAN),
        MixerSeparation(n)=>t::mixer(map(n)?,t::MIXER_SEPARATION),
        MixerEq(n,b,k)=>t::mixer(map(n)?,[t::EQ_GAIN,t::EQ_FREQ,t::EQ_WIDTH][k as usize]+b),
        MixerStrip(n,k)=>t::strip(map(n)?,k),MixerSend(n,d)=>t::send(map(n)?,map(d)?),
        SlotOn(n,k)=>t::slot(map(n)?,k,t::SLOT_ON),SlotMix(n,k)=>t::slot(map(n)?,k,t::SLOT_MIX),
        SlotParam(n,k,p)=>t::slot_param(map(n)?,k,p),_=>id,
    })
}

fn delete_inserts(app:&mut App,mut gone:Vec<u16>)->Option<String> {
    let s=&app.state;
    gone.retain(|n|*n>0 && *n<=inserts(s));gone.sort_unstable();gone.dedup();
    if gone.is_empty(){return Some("No Mixer inserts to delete; Master is kept".into());}
    if app.audio_rec.is_some() || s.rec.recording() || !s.mixer.recording.live.is_empty() {return Some("Stop recording before deleting Mixer inserts".into());}
    if gone.iter().any(|&n|protected_routes(s,n)||group_shift_locked(s,n)){return Some(LOCK_MESSAGE.into());}
    app.mixer_tick();app.params_tick();app.store_builtin_states();
    let mut saved=app.state.project.clone();
    if let Err(e)=crate::project_plugins::store_hosted(&mut saved,&app.plugins){return Some(e);}
    let s=&mut app.state;
    s.project.mixer=saved.mixer;
    crate::history::record(s,"Delete Mixer inserts",crate::history::Snap::MixerDelete(Box::new(DeleteSnapshot::now(s))));
    let old=s.project.mixer.clone();
    let map=|n:u16|(!gone.contains(&n)).then(||n-gone.partition_point(|&v|v<n) as u16);
    let mut retained=s.project.peak_links.clone();
    retained.retain(|l|map(l.source).is_some() && delete_target(l.target,&map).is_some());
    crate::peak_link::restore_removed(s,&retained);
    for l in &mut retained {l.source=map(l.source).unwrap();l.target=delete_target(l.target,&map).unwrap();}
    s.project.peak_links=retained;
    for a in &mut s.project.automations {a.targets=a.targets.iter().filter_map(|id|delete_target(*id,&map)).collect();}
    s.project.init_controls.retain_mut(|c|if let Some(id)=delete_target(c.target,&map){c.target=id;true}else{false});
    for p in &mut s.project.patterns {p.controls.retain_mut(|c|if let Some(id)=delete_target(c.kind,&map){c.kind=id;true}else{false});}
    s.project.mixer=old.iter().filter_map(|t|{
        let index=map(t.index)?;let mut out=t.clone();out.index=index;
        out.routes=t.routes.iter().filter_map(|n|map(*n)).collect();
        out.send_levels=t.send_levels.iter().filter_map(|(n,v)|map(*n).map(|n|(n,*v))).collect();
        let before:Vec<_>=old.iter().filter(|v|v.index!=t.index && v.routes.contains(&t.index)).map(|v|v.index).collect();
        let after:Vec<_>=before.iter().filter_map(|n|map(*n)).collect();
        let port=|p:usize|p.checked_sub(1).and_then(|i|before.get(i)).and_then(|n|map(*n)).and_then(|n|after.iter().position(|v|*v==n)).map_or(0,|i|i+1);
        for slot in out.slots.iter_mut().flatten() {
            if slot.name==crate::plugins::EQ_FILE_NAME {
                use os_engine::effects::eq::routed::{sidechain_port,set_sidechain_port};
                let p=sidechain_port(&slot.state);set_sidechain_port(&mut slot.state,port(p));
            }
            if slot.name==os_flp::wrapper::WRAPPER {
                for (bus,p) in os_flp::wrapper::input_ports(&slot.state).into_iter().enumerate() {
                    if p>0 {if let Some(v)=os_flp::wrapper::set_input_port(&slot.state,bus,port(p as usize) as i32){slot.state=v;}}
                }
            }
        }
        Some(out)
    }).collect();
    for c in &mut s.project.channels {c.mixer_track=map(c.mixer_track).unwrap_or(0);}
    for a in &mut s.project.arrangements {
        for t in &mut a.tracks {t.attached_mixer=t.attached_mixer.and_then(map);}
        a.attached_hidden=a.attached_hidden.iter().filter_map(|n|map(*n)).collect();
    }
    s.mixer.recording.inputs=std::mem::take(&mut s.mixer.recording.inputs).into_iter().filter_map(|(n,v)|map(n).map(|n|(n,v))).collect();
    s.mixer.recording.armed=s.mixer.recording.armed.iter().filter_map(|n|map(*n)).collect();
    s.mixer.slot_names=std::mem::take(&mut s.mixer.slot_names).into_iter().filter_map(|((n,k),v)|map(n).map(|n|((n,k),v))).collect();
    for &n in &gone {for slot in 0..64 {s.mixer.pending.push(FxReq::Delete{track:n,slot});}}
    let remap=old.iter().filter_map(|t|map(t.index).map(|n|(t.index,n))).filter(|(a,b)|a!=b)
        .flat_map(|(a,b)|(0..64).map(move|k|((a,k),(b,k)))).collect();
    s.mixer.pending.push(FxReq::Remap(remap));
    s.selected_mixer=map(s.selected_mixer).unwrap_or_else(||gone[0].min(inserts(s)));
    s.mixer.sel=s.mixer.sel.iter().filter_map(|n|map(*n)).collect();
    clear_deleted_ui(s);changed(s);app.invalidate();
    Some(format!("Deleted {} Mixer insert(s)",gone.len()))
}
