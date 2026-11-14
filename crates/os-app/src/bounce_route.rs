//! Routing policy shared by rendered audio workflows.
use crate::{app::State, history::Snap, mixer};

pub fn next_free() -> bool { crate::prefs::check(&crate::prefs::BOUNCE_FREE) }

pub fn free_insert(s: &State) -> Result<u16, String> {
    let available = s.project.mixer.iter().find(|m| m.index > 0
        && !mixer::locked(s,m.index) && !m.clip_fx && m.name.is_none()
        && m.enabled && m.slots.iter().all(Option::is_none) && m.rack_strip.is_none()
        && m.strip.iter().all(|v|*v==0) && !m.tune_enabled && m.deess.unwrap_or(0)==0
        && m.eq==os_model::EQ_DEFAULT && m.routes==[0] && m.hardware_output.is_none()
        && m.volume.unwrap_or(0.8)==0.8 && m.pan.unwrap_or(0.)==0.
        && m.stereo_separation.unwrap_or(0.)==0. && !m.polarity && !m.swap
        && m.latency_in==0. && m.latency_out==0. && m.send_levels.is_empty()
        && !s.project.automations.iter().any(|a|a.targets.iter().any(|t|crate::channel_bounce::mixer_target(*t)==Some(m.index)))
        && !s.project.init_controls.iter().any(|v|crate::channel_bounce::mixer_target(v.target)==Some(m.index))
        && !s.project.patterns.iter().any(|p|p.controls.iter().any(|v|crate::channel_bounce::mixer_target(v.kind)==Some(m.index)))
        && !s.mixer.recording.inputs.contains_key(&m.index) && !s.mixer.recording.armed.contains(&m.index)
        && !s.project.channels.iter().any(|c|c.mixer_track==m.index)
        && !s.project.mixer.iter().any(|other|other.routes.contains(&m.index))
        && !s.project.peak_links.iter().any(|l|l.source==m.index));
    if let Some(m)=available { return Ok(m.index); }
    if mixer::inserts(s)>=4093 { return Err("No free Mixer insert is available for this bounce".into()); }
    Ok(mixer::inserts(s)+1)
}

pub fn snapshot(s:&State, n:u16) -> Option<Snap> {
    (n>mixer::inserts(s)).then(||Snap::MixerInsert{n,count:1,remove:true,previous:s.selected_mixer,selection:s.mixer.sel.clone()})
}
pub fn with_insert(s:&State,n:Option<u16>,edit:Snap)->Snap {
    if let Some(insert)=n.and_then(|n|snapshot(s,n)) {Snap::WithMixerInsert{insert:Box::new(insert),edit:Box::new(edit)}}else{edit}
}
pub fn ensure(s:&mut State,n:u16) {
    if n>mixer::inserts(s) { mixer::add_track(s,None); }
}

pub fn neutralize(p:&mut os_model::Project) {
    for m in &mut p.mixer {
        let index=m.index;
        *m=os_model::MixerTrack::new(index);
        m.strip_bypassed=true;
    }
    p.peak_links.clear();
}
