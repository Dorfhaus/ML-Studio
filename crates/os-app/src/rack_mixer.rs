// SPDX-License-Identifier: GPL-3.0-or-later
//! Real plugin editors above Lemon's mixer faders. No proxy controls.
use crate::{app::{App,State,Action,Win},mixer,playlist::{Cmd,PItem,Popup}};
use os_model::RackStrip;
use os_ui::Rect;
pub const VIEW:u8=32;
pub const LINK_VISIBILITY:&str="visible_mixer-ssl360-link";
pub const LINK_PAGER_VISIBILITY: &str = "visible_mixer-ssl360-link-pager";
pub const LINK_GUIDE:&str="Use SSL 360 Link in Channel Strip view";
/// Recognize the effect wrapper, not SSL's controller software, instruments or Link Bus.
fn is_ssl_link(name:&str,vendor:&str,format:os_host::Format)->bool{
    let key=|s:&str|s.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_ascii_lowercase();
    format==os_host::Format::Vst3
        && matches!(key(vendor).as_str(),"solidstatelogic"|"ssl")
        && matches!(key(name).as_str(),"ssl360link"|"360link")
}
pub fn ssl_link(d:&os_host::Descriptor)->bool{d.kind==os_host::Kind::Effect&&is_ssl_link(&d.name,&d.vendor,d.format)}
/// Original editors are measured during direct loading, before assignment.
/// Formats provide no "vertical solo strip" capability flag.
pub fn vertical_shape(size: (u32, u32)) -> bool {
    let (width, height) = size;
    (1..=4096).contains(&width) && (1..=16384).contains(&height)
}
// Curated original single-channel portrait interfaces, reviewed against installed
// inventory and manufacturer manuals. Unknown names are not offered as candidates.
fn strip_family(name:&str,vendor:&str)->Option<&'static str>{
    if !vendor.eq_ignore_ascii_case("Waves") {return None;}
    let name=name.to_ascii_lowercase();
    let base=[" mono/stereo"," stereo"," mono"].iter().find_map(|suffix|name.strip_suffix(suffix)).unwrap_or(&name);
    match base {
        "scheps omni channel 2"=>Some("Scheps Omni Channel 2"),
        "abbey road redd.17"|"abbey road redd.37.51"=>Some("Abbey Road REDD"),
        "abbey road emi tg12345 ch"=>Some("EMI TG12345"),
        "rchannel"=>Some("Renaissance Channel"),
        "magma channel strip"=>Some("Magma Channel Strip"),
        "sslchannel"=>Some("SSL E-Channel"),
        "sslgchannel"=>Some("SSL G-Channel"),
        "ssl ev2 channel"=>Some("SSL EV2 Channel"),
        _=>None,
    }
}
fn spec_key(uid:&str)->String{use sha2::{Digest,Sha256};format!("vertical_strip_{:x}",Sha256::digest(uid.as_bytes()))}
#[derive(serde::Serialize,serde::Deserialize)]
struct Spec{version:String,path:String,width:u32,height:u32}
fn saved(uid:&str)->Option<Spec>{serde_json::from_str(&crate::prefs::text(&spec_key(uid))?).ok()}
pub fn observe_editor(d:&os_host::Descriptor,size:(u32,u32),scale:f32){
    let key=spec_key(&d.uid());
    if !d.is_effect()||!scale.is_finite()||scale<=0.||!vertical_shape(size) {return;}
    let value=Spec{version:d.version.clone(),path:d.path.to_string_lossy().into_owned(),width:(size.0 as f32/scale).round() as u32,height:(size.1 as f32/scale).round() as u32};
    if let Ok(value)=serde_json::to_string(&value){if crate::prefs::text(&key).as_deref()!=Some(&value){crate::prefs::set_text(&key,&value);}}
}
pub fn vertical_spec(d:&os_host::Descriptor)->Option<(u32,u32)>{let s=saved(&d.uid())?;(d.is_effect()&&s.version==d.version&&s.path==d.path.to_string_lossy()&&vertical_shape((s.width,s.height))).then_some((s.width,s.height))}
fn eligible_slot(s:&State,n:u16,k:u8)->bool{
    k != 10 && !(n == 0 && crate::master_assistant::slot(s)==Some(k))
        && mixer::track(s,n).is_some_and(|m| m.slots.get(k as usize).is_some_and(Option::is_some))
}
#[derive(Clone,Debug,PartialEq)]
pub enum Command {
    Menu(u16), Choose(u16,usize,bool), Existing(u16,u8), CopyAll(u16), Remove(u16), Retry(u16), Open(u16),
    LinkHelp, Comfortable(bool), Width(u16), Page(u16,i8), EditPages(u16), AddPage, RemoveDivider,
    Divider(usize, f32), PreviewPages, ResetPages, SavePages, CancelPages,
}
fn item(s:impl Into<String>,c:Command)->PItem{PItem::new(s.into(),Cmd::Mixer(mixer::MCmd::Rack(c)))}
pub fn menu(s:&State,n:u16)->Vec<PItem>{
    if n==0{return vec![PItem::head("Vertical strips belong to insert channels")];}
    let link_visible=crate::workspace_visibility::shown(LINK_VISIBILITY);
    let choices=|all|{let mut groups=std::collections::BTreeMap::<String,Vec<PItem>>::new();for e in &s.plugdb.lists.fx {let link=is_ssl_link(&e.name,&e.vendor,e.format);if strip_family(&e.name,&e.vendor).is_none()&&!(link_visible&&link){continue;}groups.entry(if link{"Solid State Logic".into()}else if e.vendor.is_empty(){"Other".into()}else{e.vendor.clone()}).or_default().push(item(e.label.clone(),Command::Choose(n,e.index,all)));}groups.into_iter().map(|(v,e)|PItem::sub(v,e)).collect()};
    let mut v=vec![PItem::head("Channel strips"),PItem::sub("Choose channel strip",choices(false)),PItem::sub("Fill all insert channels with",choices(true))];
    if link_visible&&crate::workspace_visibility::shown(crate::help::VISIBILITY){v.push(item("SSL 360 Link: setup and mapping...",Command::LinkHelp));}
    let mut effects = std::collections::BTreeMap::<&str, Vec<PItem>>::new();
    for effect in &s.plugdb.lists.fx {
        if !link_visible && is_ssl_link(&effect.name, &effect.vendor, effect.format) { continue; }
        effects.entry(&effect.vendor).or_default().push(item(effect.label.clone(), Command::Choose(n, effect.index, false)));
    }
    if !effects.is_empty() {
        v.push(PItem::sub("All effects", effects.into_iter()
            .map(|(vendor, entries)| PItem::sub(if vendor.is_empty() {"Other"} else {vendor}.to_owned(), entries)).collect()));
    }
    if let Some(m)=mixer::track(s,n){
        v.push(PItem::sub("Use an existing effect",m.slots.iter().enumerate().filter_map(|(k,p)|{p.as_ref()?;(k!=10&&eligible_slot(s,n,k as u8)&&!(n==0&&crate::master_assistant::slot(s)==Some(k as u8))).then(||item(format!("{}: {}",k+1,mixer::slot_name(s,n,k as u8).unwrap_or_default()),Command::Existing(n,k as u8)))}).collect()));
        if m.rack_strip.is_some(){
            if cfg!(any(windows, target_os = "macos")) && crate::workspace_visibility::shown(crate::strip_layout::VISIBILITY) {
                v.push(item("Edit plug-in pages…", Command::EditPages(n)));
            }
            v.extend([item("Copy this rack and settings to all inserts",Command::CopyAll(n)),item("Open strip in plugin window",Command::Open(n)),item("Dock original interface here",Command::Retry(n)),item("Remove strip (keep bypassed effect)",Command::Remove(n))]);
        }
    }
    if crate::workspace_visibility::shown("visible_mixer-strip-control-size") { v.push(PItem::sub("Control size", vec![item("Compact",Command::Comfortable(false)),item("Comfortable",Command::Comfortable(true))])); }
    v.push(PItem::sub("Channel width",[96,128,160,192,224,256,320].into_iter().map(|w|item(format!("{w} px"),Command::Width(w))).collect()));v
}
pub fn slot(s:&State,n:u16)->Option<u8>{Some(mixer::track(s,n)?.rack_strip.as_ref()?.slot)}
pub fn detach(s:&mut State,n:u16,k:u8){if slot(s,n)!=Some(k){return;}if s.mixer.strip_edit.as_ref().is_some_and(|edit|edit.track==n){crate::strip_layout::cancel(s);}s.mixer.rack_pages.remove(&n);s.mixer.rack_min_height.remove(&n);if let Some(m)=mixer::track_mut(s,n){if let Some(h)=m.rack_strip.take(){m.strip_bypassed=h.native_bypassed;m.slot_on[10]=h.eq_on;m.slot_on[k as usize]=false;}}s.piano.dirty=true;}
fn bind(s:&mut State,n:u16,k:u8){s.mixer.rack_auto_width.remove(&n);s.mixer.rack_min_height.remove(&n);s.mixer.rack_pages.remove(&n);if let Some(old)=slot(s,n){detach(s,n,old);}if let Some(m)=mixer::track_mut(s,n){m.rack_strip=Some(RackStrip{slot:k,native_bypassed:m.strip_bypassed,eq_on:m.slot_on[10]});m.strip_bypassed=true;m.slot_on[10]=false;m.slot_on[k as usize]=true;m.slot_mix[k as usize]=1.;}s.piano.dirty=true;}
fn save(app:&mut App,tracks:&[u16])->Result<Vec<mixer::ChannelCopy>,String>{app.store_builtin_states();crate::project_plugins::store_hosted(&mut app.state.project,&app.plugins)?;Ok(tracks.iter().filter_map(|n|mixer::channel_copy(&app.state,*n)).collect())}
fn record(app:&mut App,old:Vec<mixer::ChannelCopy>){crate::history::record(&mut app.state,"Channel strip mixer",crate::history::Snap::RackStrips(old));}
#[cfg(any(windows,target_os="macos"))]
fn validate(app:&mut App,t:u16,k:u8)->Result<(),String>{
    let owner=if app.state.native_view==Some(Win::Mixer){app.rack_parent}else{app.plugins.owner};
    if owner==0{return Ok(());}
    let r=app.state.win_rect(Win::Mixer);let scale=app.state.ui_scale;
    let height=crate::views::mixer::vertical_pane_height(r);
    let area=[0,0,((app.state.mixer.rack_width-6.)*scale).round() as i32,(height*scale).round() as i32];
    app.plugins.validate_vertical_rack(crate::plugins::fx_key(t,k),owner,scale,area)
}
fn choose(app:&mut App,n:u16,index:usize,all:bool,state:Option<Vec<u8>>)->Result<String,String>{
    if !app.plugins.effect_ready(index){return Err("Plugin is still being checked. Try again shortly.".into());}
    let desc=app.plugins.effects.get(index).ok_or("That plugin is no longer in the list. Reopen the picker.")?.desc.clone();
    if n==0||!desc.is_effect(){return Err("Choose an insert effect for a Mixer insert.".into());}
    let tracks:Vec<u16>=if all {(1..=mixer::inserts(&app.state)).filter(|t|state.is_none()||*t!=n).collect()}else{vec![n]};
    if tracks.iter().any(|&t| mixer::locked(&app.state, t)) { return Err(mixer::LOCK_MESSAGE.into()); }
    let mut dest=Vec::new();for t in &tracks {let m=mixer::track(&app.state,*t).ok_or("The destination mixer channel no longer exists.")?;let Some(k)=(0..64).find(|k|*k!=10&&m.slots[*k].is_none())else{return Err(format!("{} has no empty effect slot. No channels changed.",mixer::name(&app.state,*t)))};dest.push((*t,k as u8));}
    if dest.is_empty(){return Err("No other insert channels to fill.".into());}
    let old=save(app,&tracks)?;let mut made=Vec::new();
    for &(t,k) in &dest {if let Err(e)=app.plugins.create_rack_effect(t,k,index,state.as_deref()){for (t,k) in made{app.plugins.drop_key(crate::plugins::fx_key(t,k));}return Err(format!("Could not load {}: {e}. Existing channels preserved.",desc.name));}made.push((t,k));
        #[cfg(any(windows,target_os="macos"))] if let Err(e)=validate(app,t,k){for (t,k) in made{app.plugins.drop_key(crate::plugins::fx_key(t,k));}return Err(e);}
    }
    record(app,old);
    for &(t,k) in &dest {let r=crate::project_plugins::plugin_ref(&desc,app.plugins.state(crate::plugins::fx_key(t,k)).unwrap_or_default());let p=if desc.format==os_host::Format::Native {os_model::SlotPlugin{name:desc.id.clone(),state:r.state}}else{os_model::SlotPlugin{name:os_flp::wrapper::WRAPPER.into(),state:os_flp::wrapper::encode(None,&r)}};mixer::track_mut(&mut app.state,t).ok_or("The destination mixer channel no longer exists.")?.slots[k as usize]=Some(p);bind(&mut app.state,t,k);}
    app.transport.set_effects(app.plugins.effects());app.params_tick();mixer::set_layout(&mut app.state,VIEW);
    Ok(format!("{} loaded on {} independent channel(s).",desc.name,dest.len()))
}
fn report(app:&mut App,n:u16,result:Result<String,String>)->Option<String>{
    match result {
        Ok(message)=>{app.state.mixer.rack_errors.remove(&n);Some(message)},
        Err(error)=>{
            let message=format!("{}: {error}",mixer::name(&app.state,n));
            app.state.mixer.rack_errors.insert(n,error);
            app.tell(crate::dialog::Kind::Error,"Channel strip could not load",crate::dialog::spans(&[&message]));
            Some(message)
        }
    }
}
fn copy_all(app: &mut App, n: u16) -> Result<String,String> {
    let k = slot(&app.state,n).ok_or("Choose a channel strip on this insert first.")?;
    // View changes can retire editor/host entries. Restore the project's effect before
    // looking up its descriptor or settings, just as reopening an ordinary FX slot does.
    app.sync_effects();
    app.sync_wrapped();
    let key = crate::plugins::fx_key(n,k);
    let d = app.plugins.descriptor(key).ok_or("The source strip could not load. Open its effect slot and retry.")?;
    let i = app.plugins.effects.iter().position(|a|a.desc.id==d.id && a.desc.format==d.format)
        .ok_or("The source strip is missing from the plugin list. Rescan it and retry.")?;
    if let Some(h)=app.plugins.hosted.get(&key) {
        let mut plugin=h.plugin.lock().map_err(|_|"The source strip is unavailable. Reload it and retry.")?;
        plugin.idle();
    }
    let state=app.plugins.state(key).ok_or("The source strip could not save its settings. No channels changed.")?;
    choose(app,n,i,true,Some(state))
}
pub fn run(app:&mut App,c:Command)->Option<String>{
    let target = match c {
        Command::Choose(t, ..) | Command::Existing(t, _) | Command::Remove(t) | Command::Retry(t) | Command::Open(t) => Some(t),
        _ => None,
    };
    if let Some(t) = target.filter(|t| mixer::locked(&app.state, *t)) { return report(app,t,Err(mixer::LOCK_MESSAGE.into())); }
    if c==Command::LinkHelp{crate::help::open_topic(app,LINK_GUIDE);return None;}
    if let Command::Open(n)=c{if let Some(k)=slot(&app.state,n){app.open_fx(n,k);}app.invalidate();return None;}
    if let Command::Menu(n)=c {app.state.playlist.popup=Some(Popup{x:app.mouse.0,y:app.mouse.1,items:menu(&app.state,n),sub:None,scroll:[0;2],cascade:vec![]});app.invalidate();return None;}
    if let Command::Page(n,d)=c{#[cfg(any(windows,target_os="macos"))] if let Some(k)=slot(&app.state,n){if let Some(e)=app.plugins.hosted.get_mut(&crate::plugins::fx_key(n,k)).and_then(|h|h.rack_editor.as_mut()){e.page_by(d);}}app.invalidate();return None;}
    if matches!(c, Command::EditPages(_) | Command::AddPage | Command::RemoveDivider | Command::Divider(..)
        | Command::PreviewPages | Command::ResetPages | Command::SavePages | Command::CancelPages) {
        return edit_pages(app, c);
    }
    if app.state.rec.recording()||app.state.rec.audio{return Some("Stop recording before changing channel strips.".into());}
    let out=match c {
        Command::Page(..)=>None,
        Command::Comfortable(on)=>{app.state.mixer.rack_comfortable=on;crate::prefs::set_text("mixer_channel_strip_comfortable",if on {"1"}else{"0"});None},
        Command::Choose(n,i,all)=>{let result=choose(app,n,i,all,None);report(app,n,result)},
        Command::CopyAll(n)=>{let result=copy_all(app,n);report(app,n,result)},
        Command::Existing(n,k)=>{if n==0||!eligible_slot(&app.state,n,k){return None;}app.sync_effects();app.sync_wrapped();if !app.plugins.hosted.contains_key(&crate::plugins::fx_key(n,k)){return Some("Wait for that effect to load first.".into());}let old=match save(app,&[n]){Ok(v)=>v,Err(e)=>return Some(e)};#[cfg(any(windows,target_os="macos"))] if let Err(e)=validate(app,n,k){return Some(e);}record(app,old);#[cfg(plugin_windows)] app.plugins.close_window(crate::plugins::fx_key(n,k));bind(&mut app.state,n,k);mixer::set_layout(&mut app.state,VIEW);None},
        Command::Remove(n)=>{let k=slot(&app.state,n)?;let old=match save(app,&[n]){Ok(v)=>v,Err(e)=>return Some(e)};record(app,old);detach(&mut app.state,n,k);None},
        Command::Retry(n)=>{#[cfg(any(windows,target_os="macos"))]if let Some(k)=slot(&app.state,n){app.plugins.close_window(crate::plugins::fx_key(n,k));}None},
        Command::Width(w)=>{let width=f32::from(w.clamp(96,320));app.state.mixer_scroll*=width/app.state.mixer.rack_width;app.state.mixer.rack_width=width;crate::prefs::set_text("mixer_channel_strip_width", &w.to_string());None},
        Command::Menu(_)|Command::Open(_)|Command::LinkHelp=>None,
        _ => unreachable!("page editing commands were handled above"),
    };app.invalidate();out
}

fn edit_pages(app: &mut App, command: Command) -> Option<String> {
    use crate::strip_layout;
    if !crate::workspace_visibility::shown(strip_layout::VISIBILITY) { return None; }
    match command {
        Command::EditPages(track) => {
            #[cfg(any(windows, target_os = "macos"))]
            {
                if strip_layout::active(&app.state).is_some() {
                    return Some("Save or cancel the current page layout first.".into());
                }
                let slot = slot(&app.state, track)?;
                let edit = app.plugins.hosted.get(&crate::plugins::fx_key(track, slot))
                    .and_then(|host| host.rack_editor.as_ref()).and_then(|editor| editor.edit(track, slot));
                let Some(edit) = edit else { return Some("Open the docked editor before editing its pages.".into()); };
                app.state.mixer.strip_edit = Some(edit);
            }
            #[cfg(not(any(windows, target_os = "macos")))]
            let _ = track;
        }
        Command::CancelPages => { strip_layout::cancel(&mut app.state); }
        Command::SavePages => {
            if let Some(edit) = strip_layout::active(&app.state).cloned() {
                #[cfg(any(windows, target_os = "macos"))]
                if !app.plugins.descriptor(crate::plugins::fx_key(edit.track, edit.slot))
                    .is_some_and(|desc| edit.belongs_to(&desc)) {
                    strip_layout::cancel(&mut app.state);
                    return Some("The effect changed. Open Edit plug-in pages again.".into());
                }
                strip_layout::save(edit);
                strip_layout::cancel(&mut app.state);
                return Some("Plug-in page layout saved for future sessions.".into());
            }
        }
        _ => {
            if strip_layout::active(&app.state).is_none() { return None; }
            let edit = app.state.mixer.strip_edit.as_mut()?;
            match command {
                Command::AddPage => edit.add(),
                Command::RemoveDivider => edit.remove(),
                Command::PreviewPages => edit.toggle_preview(),
                Command::ResetPages => edit.reset(),
                Command::Divider(index, width) => {
                    let start = edit.select(index)?;
                    app.drag = Some(crate::app::Drag::StripDivider { index, width, x: app.mouse.0, start });
                }
                _ => unreachable!(),
            }
        }
    }
    None
}
/// Viewport hit areas are the same geometry used to parent the real native editors.
#[derive(Clone,Copy)]
pub struct Pane {pub track:u16,pub rect:Rect,pub clip:Rect}
pub fn geometry(s:&State,r:Rect)->Vec<Pane>{crate::views::mixer::vertical_panes(s,r)}
pub fn paint_pager(ui:&mut crate::app::Ui,s:&State,n:u16,r:Rect){
    if slot(s,n).is_none(){return;}
    let edit = crate::strip_layout::active(s);
    if let Some(edit) = edit.filter(|edit| edit.track == n) {
        if crate::strip_layout::ruler(ui, edit, r) { return; }
    }
    let Some((index,count))=s.mixer.rack_pages.get(&n).copied() else { return; };
    let y = r.bottom()+2.;
    let mut r = r;
    if cfg!(any(windows, target_os = "macos")) && edit.is_none()
        && crate::workspace_visibility::shown(crate::strip_layout::VISIBILITY)
        && !s.mixer.rack_errors.contains_key(&n)
    {
        let button = Rect::new(r.right()-29., y, 29., 14.);
        let hover = ui.hit(button, Action::RackStrip(Command::EditPages(n)), "Edit the section widths for this plug-in");
        ui.c.fill_rrect(button, 3., if hover { ui.t.button_hi } else { ui.t.button });
        ui.c.text_center(os_ui::Font::new(9), button, ui.t.text, "Edit");
        r.w -= 32.;
    }
    if count < 2 { return; }
    let w=(r.w*0.24).min(22.);
    for (x,d,on) in [(r.x,-1,index>0),(r.right()-w,1,index+1<count)] {
        let b=Rect::new(x,y,w,14.);ui.c.fill_rrect(b,3.,ui.t.button);
        ui.c.text_center(os_ui::Font::new(12),b,if on{ui.t.text}else{ui.t.text_dim},if d<0{"<"}else{">"});
        if on{ui.hit(b,Action::RackStrip(Command::Page(n,d)),"Slide to the next complete plugin section");}
    }
    ui.c.text_center(os_ui::Font::new(9),Rect::new(r.x+w,y,(r.w-2.*w).max(0.),14.),ui.t.text_dim,&format!("{} / {}",index+1,count));
}
pub fn paint_slot(ui:&mut crate::app::Ui,s:&State,n:u16,r:Rect){
    use os_ui::Font;
    ui.c.fill_rrect(r,2.,ui.t.s(0x171b1d));
    ui.c.stroke_rrect(r.outset(1.),2.,1.,ui.t.s(0x596264));
    if slot(s,n).is_none(){
        let b=Rect::new(r.x+3.,r.cy()-24.,r.w-6.,48.);
        ui.c.text_center(Font::new(20),Rect::new(b.x,b.y,b.w,24.),ui.t.text_dim,if s.mixer.rack_errors.contains_key(&n){"!"}else{"+"});
        ui.c.text_center(Font::new(9),Rect::new(b.x,b.y+24.,b.w,18.),ui.t.text_dim,if s.mixer.rack_errors.contains_key(&n){"Load failed"}else{"Vertical strip"});
        ui.hit(r,Action::RackStrip(Command::Menu(n)),s.mixer.rack_errors.get(&n).cloned().unwrap_or_else(||"Choose a compatible vertical channel strip".into()));
    }else{
        let error=s.mixer.rack_errors.get(&n);
        ui.c.text_center(Font::new(10),r,ui.t.text_dim,if error.is_some(){"Strip unavailable"}else{"Opening strip"});
        ui.hit(r,Action::RackStrip(Command::Menu(n)),error.cloned().unwrap_or_else(||"Vertical channel strip options".into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn ssl_link_picker_visibility_and_descriptor_guard(){
        let (mut app,index)=setup();let mut d=app.plugins.effects[index].desc.clone();
        d.format=os_host::Format::Vst3;d.name="SSL 360 Link".into();d.vendor="Solid State Logic".into();
        assert!(ssl_link(&d));
        for (name,vendor,format) in [("360 Link","SSL",os_host::Format::Vst3),("SSL 360 Link Bus","Solid State Logic",os_host::Format::Vst3),("SSL 360 Link","Other",os_host::Format::Vst3),("SSL 360 Link","Solid State Logic",os_host::Format::Vst2)]{
            assert_eq!(is_ssl_link(name,vendor,format),name=="360 Link");
        }
        let mut instrument=d.clone();instrument.kind=os_host::Kind::Instrument;assert!(!ssl_link(&instrument));
        app.state.plugdb.lists.fx.push(crate::plugin_db::Entry{builtin:false,index,name:d.name.clone(),label:d.name.clone(),uid:d.uid(),vendor:d.vendor.clone(),format:d.format,category:"Effect",favorite:false});
        let original=app.state.project.clone();
        let present=|items:&[PItem],all:bool|items.iter().flat_map(|i|&i.children).flat_map(|i|&i.children).any(|i|i.cmd==Cmd::Mixer(mixer::MCmd::Rack(Command::Choose(1,index,all))));
        crate::settings_file::set_flag(LINK_VISIBILITY,true);
        assert!(present(&menu(&app.state,1),false)&&present(&menu(&app.state,1),true));
        crate::settings_file::set_flag(LINK_VISIBILITY,false);
        let hidden=menu(&app.state,1);assert!(!present(&hidden,false)&&!present(&hidden,true));
        assert!(!hidden.iter().any(|i|i.label.contains("SSL 360 Link")));
        assert!(ssl_link(&d),"Visibility must not disable existing project instances");assert_eq!(app.state.project,original);
        crate::settings_file::set_flag(LINK_VISIBILITY,true);
        assert!(present(&menu(&app.state,1),false));
    }
    fn setup()->(App,usize){let mut app=App::new(None);app.state.project=crate::app::test_project();app.state.project.mixer.truncate(4);app.state.project.arrangements.push(os_flp::project::default_arrangement());app.reset_hosted();app.plugins.effects=crate::plugins::builtin_effects();let index=app.plugins.effects.iter().position(|a|a.desc.name==crate::plugins::LEMON_EQ).unwrap();app.plugins.effects[index].desc.name="Magma Channel Strip Mono".into();app.plugins.effects[index].desc.vendor="Waves".into();(app,index)}
    #[test]fn rack_mixer_picker_groups_all_effects_and_recommends_known_strips(){
        let (mut app,index)=setup();let d=&app.plugins.effects[index].desc;
        for (name,vendor) in [("Magma Channel Strip Mono","Waves"),("RChannel Stereo","Waves"),("SSLChannel Stereo","Waves"),("SSLGChannel Mono","Waves"),("SSL EV2 Channel Stereo","Waves"),("GOLD5","Acqua"),("Pro-Q 4","FabFilter"),("NLS Channel Stereo","Waves"),("AudioTrack Mono","Waves"),("Virtual Mix Rack","Slate Digital"),("Scheps Omni Channel 2 Stereo","Waves")] {
            app.state.plugdb.lists.fx.push(crate::plugin_db::Entry{builtin:false,index,name:name.into(),label:name.into(),uid:d.uid(),vendor:vendor.into(),format:d.format,category:"Effect",favorite:false});
        }
        let items=menu(&app.state,1);let picker=items.iter().find(|v|v.label=="Choose channel strip").unwrap();
        let labels:Vec<_>=picker.children.iter().flat_map(|v|v.children.iter()).map(|v|v.label.as_ref()).collect();
        assert_eq!(labels.len(),6);assert!(!labels.iter().any(|n|n.contains("AudioTrack")||n.contains("GOLD")||n.contains("Virtual Mix Rack")));assert!(labels.contains(&"RChannel Stereo")&&labels.contains(&"Magma Channel Strip Mono"));
        let all = items.iter().find(|item| item.label == "All effects").unwrap();
        let all: Vec<_> = all.children.iter().flat_map(|vendor| &vendor.children).map(|item| item.label.as_ref()).collect();
        assert_eq!(all.len(), 11);
        assert!(all.contains(&"Pro-Q 4") && all.contains(&"GOLD5") && all.contains(&"Virtual Mix Rack"));
        assert!(strip_family("RChannel Mono/Stereo","Waves").is_some());assert!(strip_family("RChannel Stereo","Other vendor").is_none());
        let before=app.state.project.clone();
        let result=crate::playlist::run(&mut app,Cmd::Mixer(mixer::MCmd::Rack(Command::Choose(1,usize::MAX,false))));
        assert!(result.unwrap().contains("no longer in the list"));assert_eq!(app.state.project,before);
        assert!(!app.state.dialogs.is_empty());assert!(app.state.mixer.rack_errors.contains_key(&1));
        assert!(vertical_shape((400,600)));assert!(vertical_shape((600,400)));
    }
    #[test]fn rack_mixer_pager_stays_outside_editor_and_minimum_height_preserves_full_strip(){
        let (mut app,index)=setup();run(&mut app,Command::Choose(1,index,false));app.state.mixer.rack_pages.insert(1,(0,6));
        app.state.startup_layout_pending=false;app.state.z=vec![Win::Mixer];app.state.show_browser=false;
        for (w,h) in [(1280,900),(1000,650)]{
            app.state.wins[Win::Mixer.slot().unwrap()]=crate::app::WinState{rect:Rect::new(0.,83.,w as f32,h as f32-83.),maxed:false,mini:false};
            let mut c=os_ui::Canvas::new(w,h,1.).unwrap();app.paint(&mut c,None);
            let pane=geometry(&app.state,app.state.win_rect(Win::Mixer)).into_iter().find(|p|p.track==1).unwrap();
            let next=app.hits.iter().find(|h|h.action==Action::RackStrip(Command::Page(1,1))).unwrap().rect;
            assert!(next.y>=pane.rect.bottom());let fader=app.hits.iter().find(|h|h.action==Action::Fader(1)).unwrap().rect;assert!(fader.y>next.bottom());
            if let Some(dir)=std::env::var_os("OS_SHOT_DIR"){c.pixmap.save_png(std::path::PathBuf::from(dir).join(format!("section-pager-{w}.png"))).unwrap();}
        }
        app.state.mixer.rack_min_height.insert(1,614.);let h=minimum_height(&app.state);assert!(crate::views::mixer::vertical_pane_height(Rect::new(0.,83.,1280.,h))>=614.);
        run(&mut app,Command::Remove(1));assert_eq!(minimum_height(&app.state),0.);assert!(!app.state.mixer.rack_pages.contains_key(&1));
    }
    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn rack_mixer_custom_layout_accepts_ruler_drag_and_saves_only_on_request() {
        let (mut app, index) = setup();
        run(&mut app, Command::Choose(1, index, false));
        let slot = slot(&app.state, 1).unwrap();
        let key = crate::plugins::fx_key(1, slot);
        let desc = app.plugins.descriptor(key).unwrap();
        let mut paging = crate::strip_layout::native::Paging::default();
        paging.assign(&desc);
        app.state.startup_layout_pending = false;
        app.state.z = vec![Win::Mixer];
        app.state.show_browser = false;
        app.state.mixer_scroll = 0.;
        app.state.mixer.rack_auto_width.insert(1, 366.);
        app.state.wins[Win::Mixer.slot().unwrap()] = crate::app::WinState {
            rect: Rect::new(0., 83., 1280., 817.), maxed: false, mini: false,
        };
        let mut canvas = os_ui::Canvas::new(1280, 900, 1.).unwrap();
        let size = (360, 300);
        let start = |app: &mut App| {
            let mut edit = paging.edit(1, slot, size).unwrap();
            edit.span = (0., 360.);
            app.state.mixer.strip_edit = Some(edit);
        };
        let click = |app: &mut App, canvas: &mut os_ui::Canvas, command| {
            app.paint(canvas, None);
            let hit = app.hits.iter().find(|h| matches!(&h.action, Action::RackStrip(action) if action == &command)).unwrap().rect;
            app.mouse_move(hit.cx(), hit.cy());
            app.mouse_down();
            app.mouse_up();
        };
        start(&mut app);
        click(&mut app, &mut canvas, Command::AddPage);
        app.paint(&mut canvas, None);
        let divider = app.hits.iter().find(|h| matches!(h.action, Action::RackStrip(Command::Divider(..)))).unwrap().rect;
        app.mouse_move(divider.cx(), divider.cy());
        app.mouse_down();
        app.mouse_move(divider.cx() - 60., divider.cy());
        app.mouse_up();
        click(&mut app, &mut canvas, Command::PreviewPages);
        let mut preview = crate::strip_layout::native::Paging::default();
        preview.assign(&desc);
        preview.preview(Some(&app.state.mixer.strip_edit.as_ref().unwrap().view()));
        assert!((preview.update(size).unwrap()[0].right - 120.).abs() < 0.02);
        click(&mut app, &mut canvas, Command::CancelPages);
        assert!(app.state.mixer.strip_edit.is_none());
        assert!(crate::prefs::text(&crate::strip_layout::native::preference_key(&desc)).is_none());
        start(&mut app);
        click(&mut app, &mut canvas, Command::AddPage);
        click(&mut app, &mut canvas, Command::SavePages);
        let mut reopened = crate::strip_layout::native::Paging::default();
        reopened.assign(&desc);
        let pages = reopened.update(size).unwrap();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].right, 180.);
    }
    #[test]fn rack_mixer_native_width_moves_neighbors_and_keeps_fader_alignment(){
        let (mut app,index)=setup();run(&mut app,Command::Choose(1,index,false));mixer::set_layout(&mut app.state,VIEW);
        app.state.mixer.rack_auto_width.insert(1,64.);
        assert_eq!(crate::views::mixer::channel_width(&app.state,1),128.);
        app.state.mixer.rack_auto_width.insert(1,326.);
        assert_eq!(crate::views::mixer::channel_width(&app.state,1),326.);
        assert_eq!(crate::views::mixer::channel_offset(&app.state,2),326.);
        app.state.startup_layout_pending=false;app.state.z=vec![Win::Mixer];app.state.show_browser=false;
        app.state.wins[Win::Mixer.slot().unwrap()]=crate::app::WinState{rect:Rect::new(0.,83.,1280.,817.),maxed:false,mini:false};
        let mut c=os_ui::Canvas::new(1280,900,1.).unwrap();app.paint(&mut c,None);
        let panes=geometry(&app.state,app.state.win_rect(Win::Mixer));let a=panes.iter().find(|p|p.track==1).unwrap();let b=panes.iter().find(|p|p.track==2).unwrap();assert!(a.rect.right()<=b.rect.x);
        let fader=app.hits.iter().find(|h|h.action==Action::Fader(1)).unwrap().rect;assert!(fader.cx()>a.rect.x&&fader.cx()<a.rect.right());assert!(fader.y>a.rect.bottom());
        if let Some(dir)=std::env::var_os("OS_SHOT_DIR"){c.pixmap.save_png(std::path::PathBuf::from(dir).join("adaptive-strip-width.png")).unwrap();}
        run(&mut app,Command::Remove(1));assert_eq!(crate::views::mixer::channel_offset(&app.state,2),128.);
    }
    #[test]fn rack_mixer_analog_deck_fits_variable_widths(){
        let (mut app,index)=setup();run(&mut app,Command::Choose(1,index,false));
        app.state.startup_layout_pending=false;app.state.z=vec![Win::Mixer];app.state.show_browser=false;
        app.state.project.mixer.iter_mut().find(|m|m.index==1).unwrap().separator=true;
        for (width,height,comfortable) in [(84.,650,false),(128.,900,false),(300.,900,false),(96.,650,true),(256.,900,true)]{
            app.state.mixer.rack_comfortable=comfortable;
            app.state.project.mixer.iter_mut().find(|m|m.index==1).unwrap().volume=Some(if width==84. {1.}else{0.8});
            app.state.mixer.meters=(0..4).map(|_|crate::meters::Meter{db:if width==128. {[-2.,-14.]}else{[-120.;2]},..Default::default()}).collect();
            app.state.mixer.rack_auto_width.insert(1,width);
            app.state.wins[Win::Mixer.slot().unwrap()]=crate::app::WinState{rect:Rect::new(0.,83.,1280.,height as f32-83.),maxed:false,mini:false};
            let mut c=os_ui::Canvas::new(1280,height,1.).unwrap();app.paint(&mut c,None);
            let pane=geometry(&app.state,app.state.win_rect(Win::Mixer)).into_iter().find(|p|p.track==1).unwrap();
            for action in [Action::Fader(1),Action::Knob(crate::knobs::KnobId::MixPan(1)),Action::Mix(mixer::MHit::Effects(1)),Action::Mix(mixer::MHit::Disk(1))]{
                let r=app.hits.iter().find(|h|h.action==action).unwrap().rect;
                assert!(r.x>=pane.rect.x&&r.right()<=pane.rect.right(),"{action:?} rect {r:?} outside {pane:?} at width {width}",pane=pane.rect);assert!(r.y>pane.rect.bottom());
            }
            let hit=|a|app.hits.iter().find(|h|h.action==a).unwrap().rect;
            let fader=hit(Action::Fader(1));let pan=hit(Action::Knob(crate::knobs::KnobId::MixPan(1)));
            let rec=hit(Action::Mix(mixer::MHit::Disk(1)));let fx=hit(Action::Mix(mixer::MHit::Effects(1)));
            let led=hit(Action::Mix(mixer::MHit::Led(1)));
            let polarity=hit(Action::Mix(mixer::MHit::Polarity(1)));
            let swap=hit(Action::Mix(mixer::MHit::Swap(1)));
            assert!(pan.right()+3.<=fader.x && led.cx()==pan.cx());
            assert!(led.bottom()+2.<=pan.y && pan.bottom()+4.<=polarity.y && polarity.bottom()+3.<=swap.y);
            assert!(rec.x>=fader.right()+4. && fx.x==rec.x && fx.y>=rec.bottom()+4.);
            assert!(fader.h-26.>=100.,"long fader remains usable at short mixer heights");
            assert!(fader.bottom()+4.<=app.state.win_rect(Win::Mixer).bottom()-61.,"full-down handle clears routing controls");
            assert!(hit(Action::Mix(mixer::MHit::Led(1))).y>=pane.rect.bottom()+39.);
            let navigation:Vec<_>=app.hits.iter().filter(|h|h.rect.w==20. && h.rect.h==12. && h.rect.x==pan.cx()-10. && h.rect.y>pan.bottom()).collect();
            assert_eq!(navigation.len(),5);
            assert!(navigation.iter().all(|h|h.rect.bottom()<=app.state.win_rect(Win::Mixer).bottom()-26.),"teleport buttons clear the bottom scrollbar");
            let rate=crate::views::mixer::fader_per_px(&app.state,1);assert!((rate*(fader.h-26.) as f64-1.).abs()<0.0001);
            if let Some(dir)=std::env::var_os("OS_SHOT_DIR"){c.pixmap.save_png(std::path::PathBuf::from(dir).join(format!("analog-strip-{width}.png"))).unwrap();}
            if let Some(dir)=std::env::var_os("OS_SHOT_DIR"){
                for layout in [VIEW,mixer::WIDE] {
                    mixer::set_layout(&mut app.state,layout);
                    for on in [false,true] {
                        let m=mixer::track_mut(&mut app.state,1).unwrap();m.polarity=on;m.swap=on;
                        app.paint(&mut c,None);
                        c.pixmap.save_png(std::path::PathBuf::from(&dir).join(format!("switches-{layout}-{width}-{on}.png"))).unwrap();
                    }
                }
                mixer::set_layout(&mut app.state,VIEW);
                let m=mixer::track_mut(&mut app.state,1).unwrap();m.polarity=false;m.swap=false;
            }
        }
    }
    #[test]fn rack_mixer_direct_picker_needs_no_prior_fx_window(){
        let (mut app,index)=setup();let d=&app.plugins.effects[index].desc;
        assert!(vertical_spec(d).is_none());
        app.state.plugdb.lists.fx.push(crate::plugin_db::Entry{builtin:false,index,name:"Magma Channel Strip Mono".into(),label:"Magma Channel Strip Mono".into(),uid:d.uid(),vendor:"Waves".into(),format:d.format,category:"Effect",favorite:false});
        let items=menu(&app.state,1);let choose=items.iter().find(|v|v.label=="Choose channel strip").unwrap();
        assert_eq!(choose.children[0].children[0].label,"Magma Channel Strip Mono");
        app.state.startup_layout_pending=false;app.state.show_browser=false;app.state.z=vec![Win::Mixer];
        app.state.playlist.popup=Some(Popup{x:200.,y:180.,items,sub:Some(1),scroll:[0;2],cascade:vec![(0,0)]});
        let mut canvas=os_ui::Canvas::new(1280,900,1.).unwrap();app.paint(&mut canvas,None);
        let target=app.hits.iter().find(|h|h.action==Action::PlPopupDeep(2,0)).expect("Deep plugin menu entry must be clickable").rect;
        app.mouse_move(target.cx(),target.cy());app.mouse_down();app.mouse_up();
        assert_eq!(slot(&app.state,1),Some(0),"Actual menu click must assign the plugin");
    }
    #[test]fn rack_mixer_fill_after_view_switches_and_missing_source_reports_error(){
        let (mut app,index)=setup();
        let d=&app.plugins.effects[index].desc;
        app.state.plugdb.lists.fx.push(crate::plugin_db::Entry{builtin:false,index,name:d.name.clone(),label:d.name.clone(),uid:d.uid(),vendor:d.vendor.clone(),format:d.format,category:"Effect",favorite:false});
        app.state.startup_layout_pending=false;app.state.show_browser=false;app.state.z=vec![Win::Mixer];
        let mut canvas=os_ui::Canvas::new(1280,900,1.).unwrap();
        for layout in [VIEW,mixer::WIDE,mixer::TRADITIONAL,VIEW] {mixer::set_layout(&mut app.state,layout);app.paint(&mut canvas,None);}
        let items=menu(&app.state,1);
        app.state.playlist.popup=Some(Popup{x:200.,y:180.,items,sub:Some(2),scroll:[0;2],cascade:vec![(0,0)]});
        app.paint(&mut canvas,None);
        let target=app.hits.iter().find(|h|h.action==Action::PlPopupDeep(2,0)).unwrap().rect;
        app.mouse_move(target.cx(),target.cy());app.mouse_down();app.mouse_up();
        assert!(slot(&app.state,1).is_some() && slot(&app.state,2).is_some());
        assert!(app.state.message.as_deref().unwrap().contains("2 independent"));
        crate::history::undo(&mut app.state);
        assert!(slot(&app.state,1).is_none() && slot(&app.state,2).is_none());
        let message=run(&mut app,Command::CopyAll(1)).unwrap();
        assert!(message.contains("Choose a channel strip"));
        assert!(!app.state.dialogs.is_empty());
    }
    #[test]fn rack_mixer_accepts_unrecognized_effects_and_checks_editor_sizes(){
        let (mut app,index)=setup();let mut d=app.plugins.effects[index].desc.clone();
        for size in [(0,500),(100,17000)] {observe_editor(&d,size,1.);assert!(vertical_spec(&d).is_none());}
        observe_editor(&d,(120,600),1.);assert_eq!(vertical_spec(&d),Some((120,600)));
        d.version.push('x');assert!(vertical_spec(&d).is_none());
        d.name="Pro-Q 4".into();observe_editor(&d,(1200,600),1.);assert_eq!(vertical_spec(&d),Some((1200,600)));
        app.plugins.effects[index].desc.name="Pro-Q 4".into();let before=app.state.project.clone();
        assert!(run(&mut app,Command::Choose(1,index,false)).unwrap().contains("1 independent"));
        assert!(slot(&app.state,1).is_some());assert_ne!(app.state.project,before);
    }
    #[test]fn rack_mixer_empty_view_keeps_faders_and_no_proxy_controls(){
        let (mut app,_)=setup();let original=app.state.project.clone();mixer::set_layout(&mut app.state,VIEW);assert_eq!(app.state.project,original);
        app.state.startup_layout_pending=false;app.state.z=vec![Win::Mixer];app.state.show_browser=false;
        for (w,h) in [(1280,900),(1000,650)] {app.state.wins[Win::Mixer.slot().unwrap()]=crate::app::WinState{rect:Rect::new(0.,83.,w as f32,h as f32-83.),maxed:false,mini:false};let mut c=os_ui::Canvas::new(w,h,1.).unwrap();app.paint(&mut c,None);
            assert!(app.hits.iter().any(|h|h.action==Action::Fader(1)));
            for action in [Action::Mix(mixer::MHit::Polarity(1)),Action::Mix(mixer::MHit::Swap(1)),Action::Mix(mixer::MHit::Effects(1)),Action::Knob(crate::knobs::KnobId::MixPan(1))]{assert!(app.hits.iter().any(|h|h.action==action));}assert!(app.hits.iter().any(|h|h.action==Action::RackStrip(Command::Menu(1))));assert!(!app.hits.iter().any(|h|matches!(h.action,Action::Knob(crate::knobs::KnobId::Strip(1,..)))));
            let pane=geometry(&app.state,app.state.win_rect(Win::Mixer)).into_iter().find(|p|p.track==1).unwrap();let fader=app.hits.iter().find(|h|h.action==Action::Fader(1)).unwrap().rect;assert!(pane.rect.bottom()<fader.y);
            if let Some(dir)=std::env::var_os("OS_SHOT_DIR"){c.pixmap.save_png(std::path::PathBuf::from(dir).join(format!("real-strip-empty-{w}.png"))).unwrap();}
            app.mouse_move(fader.cx(),fader.cy());app.mouse_down();app.mouse_move(fader.cx(),fader.cy()-12.);app.mouse_up();assert!(mixer::track(&app.state,1).unwrap().volume.unwrap()>0.8);
        }
        mixer::set_layout(&mut app.state,mixer::WIDE);assert_eq!(app.state.mixer.layout,mixer::WIDE);mixer::set_layout(&mut app.state,mixer::TRADITIONAL);assert!(mixer::traditional(&app.state));
    }
    #[test]fn rack_mixer_independent_copy_save_restore_and_undo(){
        let (mut app,index)=setup();let native=app.state.project.mixer[1].clone();run(&mut app,Command::Choose(1,index,false));assert_eq!(slot(&app.state,1),Some(0));assert!(app.state.project.mixer[1].strip_bypassed);assert!(!app.state.project.mixer[1].slots_enabled);
        let key=crate::plugins::fx_key(1,0);let id={let p=app.plugins.hosted[&key].plugin.lock().unwrap();(0..p.param_count()).filter_map(|i|p.param_info(i)).find(|p|p.name=="Main level").unwrap().id};
        {let mut p=app.plugins.hosted[&key].plugin.lock().unwrap();p.set_param(id,0.7);p.idle();}
        let message=run(&mut app,Command::CopyAll(1)).unwrap();assert!(message.contains("1 independent"),"{message}");let k2=crate::plugins::fx_key(2,slot(&app.state,2).unwrap());assert!(!std::sync::Arc::ptr_eq(&app.plugins.hosted[&key].plugin,&app.plugins.hosted[&k2].plugin));assert!((app.plugins.hosted[&k2].plugin.lock().unwrap().param_value(id).unwrap()-0.7).abs()<1e-5);assert!(slot(&app.state,0).is_none());
        crate::project_plugins::store_hosted(&mut app.state.project,&app.plugins).unwrap();let file=os_flp::project::create(&app.state.project).unwrap();let back=os_flp::project::load(&file).unwrap();assert_eq!(back.mixer[1].rack_strip,app.state.project.mixer[1].rack_strip);
        let (mut reopened,_)=setup();reopened.plugins.effects=crate::plugins::builtin_effects();reopened.state.project=back;reopened.sync_wrapped();assert!((reopened.plugins.hosted[&key].plugin.lock().unwrap().param_value(id).unwrap()-0.7).abs()<1e-5);
        run(&mut app,Command::Remove(1));assert!(slot(&app.state,1).is_none());assert_eq!(app.state.project.mixer[1].strip_bypassed,native.strip_bypassed);assert_eq!(app.state.project.mixer[1].slot_on[10],native.slot_on[10]);crate::history::undo(&mut app.state);assert_eq!(slot(&app.state,1),Some(0));
    }
    #[test]fn rack_mixer_batch_capacity_extra_slots_and_legacy_retirement(){
        let (mut app,index)=setup();for k in 0..64{app.state.project.mixer[2].slots[k]=Some(os_model::SlotPlugin{name:"Keep".into(),state:vec![7]});}let old=app.state.project.clone();assert!(run(&mut app,Command::Choose(1,index,true)).unwrap().contains("No channels changed"));assert_eq!(app.state.project,old);
        for k in 0..10{app.state.project.mixer[1].slots[k]=Some(os_model::SlotPlugin{name:"Keep".into(),state:vec![7]});}run(&mut app,Command::Choose(1,index,false));assert_eq!(slot(&app.state,1),Some(11));let mut file=os_flp::project::create(&app.state.project).unwrap();assert_eq!(os_flp::project::load(&file).unwrap().mixer[1].rack_strip.as_ref().unwrap().slot,11);
        file.chunks.retain(|c|c.tag!=os_flp::Tag(*b"LRCK"));let mut data=vec![1];data.extend(br#"[[1,{"slot":11,"native_bypassed":false,"eq_on":true}]]"#);file.chunks.push(os_flp::Chunk{tag:os_flp::Tag(*b"LHST"),body:os_flp::ChunkBody::Raw(data)});let back=os_flp::project::load(&file).unwrap();assert!(back.mixer[1].rack_strip.is_none());assert!(!back.mixer[1].strip_bypassed);assert!(back.mixer[1].slot_on[10]);assert!(!back.mixer[1].slot_on[11]);
    }
}

pub fn minimum_height(s:&State)->f32{
    s.mixer.rack_min_height.iter().filter(|(n,_)|slot(s,**n).is_some()).map(|(_,h)|crate::views::mixer::mixer_height_for_editor(*h)).fold(0.,f32::max)
}
