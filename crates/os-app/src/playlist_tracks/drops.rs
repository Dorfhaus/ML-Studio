// SPDX-License-Identifier: GPL-3.0-or-later
//! Header drops are explicit track creation, distinct from timeline audio placement.
use crate::{app::{Action, App, Win}, playlist as pl, mixer, attached_mixer as attached};
use os_model::{ClipSource, PlaylistTrackMode};

const KEY: &str = "playlist_header_audio_drop";
const LABELS: [&str; 4] = ["As audio clips", "As audio tracks", "As instrument tracks", "Always ask"];

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Preference(u8),
    Sample { path: String, row: u16, tick: u32, mode: u8 },
    Samples { paths: Vec<String>, row: u16, tick: u32, mode: u8 },
}

fn preference() -> u8 {
    crate::prefs::text(KEY).and_then(|v|v.parse().ok()).filter(|v|*v<4).unwrap_or(2)
}

pub fn menu() -> pl::PItem {
    if !crate::workspace_visibility::shown("visible_playlist-header-drop-options") { return pl::PItem::sep(); }
    pl::PItem::sub("Drop audio on track headers", LABELS.iter().enumerate().map(|(i,label)|
        pl::PItem::new(*label, pl::Cmd::HeaderDrop(Command::Preference(i as u8))).mark(pl::Mark::Radio(preference()==i as u8))
    ).collect())
}

pub fn header_at(app: &App) -> Option<u16> {
    let (x,y)=app.mouse;
    if app.top_hit(x,y).and_then(|h|h.win)!=Some(Win::Playlist) { return None; }
    app.hits.iter().rev().find_map(|h| {
        if h.win==Some(Win::Playlist) && h.rect.contains(x,y) { if let Action::PlHeader(row)=h.action { return Some(row); } }
        None
    })
}

fn blocked(app: &App, row: u16) -> Option<String> {
    if !super::writable(&app.state,row) { return Some("Choose an unlocked, unfrozen track outside a folder header".into()); }
    if super::owner(&app.state,row).is_some_and(|n|mixer::locked(&app.state,n)) { return Some(mixer::LOCK_MESSAGE.into()); }
    if app.state.rec.recording() || app.audio_rec.is_some() { return Some("Finish recording before adding a track source".into()); }
    None
}

pub fn sample(app: &mut App, path: String, row: u16) {
    samples(app,vec![path],row);
}

pub fn samples(app: &mut App, paths: Vec<String>, row: u16) {
    if paths.is_empty(){return;}
    let mode=preference();
    let tick=app.state.playlist.pos;
    if let Some(message)=blocked(app,row) { app.state.message=Some(message);return; }
    if paths.iter().any(|path|app.state.playlist.wave(path).is_none()) { app.state.message=Some("Could not read one of the files as audio".into());return; }
    if mode==3 {
        let mut items=vec![pl::PItem::head("Drop sample on Playlist track")];
        items.extend(LABELS[..3].iter().enumerate().map(|(i,label)|pl::PItem::new(*label,
            pl::Cmd::HeaderDrop(Command::Samples {paths:paths.clone(),row,tick,mode:i as u8}))));
        items.push(pl::PItem::sep());
        items.push(pl::PItem::new("Cancel",pl::Cmd::Nothing));
        app.state.playlist.popup=Some(pl::Popup{x:app.mouse.0,y:app.mouse.1,items,sub:None,scroll:[0;2],cascade:vec![]});
    } else { app.state.message=run(app,Command::Samples{paths,row,tick,mode}); }
    app.invalidate();
}

pub fn run(app: &mut App, command: Command) -> Option<String> {
    match command {
        Command::Samples{paths,row,tick,mode} => {
            if paths.is_empty()||mode>2{return None;}
            if let Some(message)=blocked(app,row){return Some(message);}
            if paths.iter().any(|p|app.state.playlist.wave(p).is_none()){return Some("Could not read one of the files as audio".into());}
            if mode==2&&app.state.project.patterns.iter().map(|p|p.number).max().unwrap_or(0) as usize+paths.len()>u16::MAX as usize{
                return Some("No pattern number is available".into());
            }
            if mode==2&&!app.plugins.available.iter().any(|p|p.desc.name==crate::plugins::LEMON_SAMPLER&&crate::project_plugins::is_builtin(&p.desc)){
                return Some("ML Sampler is not available".into());
            }
            if mode==1&&crate::playlist_links::mode(&app.state,row)==PlaylistTrackMode::Instrument{
                return Some("Choose an empty or audio track for a linked audio drop".into());
            }
            let creates_output=mode>0&&super::owner(&app.state,row).is_none();
            if creates_output&&mixer::inserts(&app.state)>=4093{return Some("The mixer has reached its track limit".into());}
            let before=pl::capture(&app.state);let selected=app.state.selected_mixer;
            let previous=app.state.history.begin_sequence();
            // Create the shared output outside the individual drops. Its structural
            // undo wrapper must surround all edits, so redo inserts it before naming it.
            let output=creates_output.then(||{
                let n=mixer::add_track(&mut app.state,None);attached::bind(&mut app.state,row,n);n
            });
            let mut message=None;
            for path in paths {message=run(app,Command::Sample{path,row,tick,mode});}
            if let Some(n)=output{app.state.history.wrap_attached_instrument(before,n,selected);}
            app.state.history.end_sequence_named(previous,"Drop samples on track header");
            message
        }
        Command::Preference(mode) => { crate::prefs::set_text(KEY,&mode.min(3).to_string());app.invalidate();None }
        Command::Sample{path,row,tick,mode} => {
            if let Some(message)=blocked(app,row) { return Some(message); }
            if app.state.playlist.wave(&path).is_none() { return Some("Could not read the file as audio".into()); }
            if mode==2 {
                let Some(index)=app.plugins.available.iter().position(|p|p.desc.name==crate::plugins::LEMON_SAMPLER && crate::project_plugins::is_builtin(&p.desc)) else {return Some("ML Sampler is not available".into());};
                return instrument(app,index,row,tick,Some(path));
            }
            if mode>1 { return None; }
            if mode==1 && crate::playlist_links::mode(&app.state,row)==PlaylistTrackMode::Instrument {
                return Some("Choose an empty or audio track for a linked audio drop".into());
            }
            if mode==1 && super::owner(&app.state,row).is_none() && mixer::inserts(&app.state)>=4093 { return Some("The mixer has reached its track limit".into()); }
            let name=sample_name(&path);
            let s=&mut app.state;
            let before=pl::capture(s);
            let selected=s.selected_mixer;
            let creates_output=mode==1 && super::owner(s,row).is_none();
            let previous=s.history.begin_sequence();
            crate::playlist_links::snapshot(s,"Drop audio on track header");
            let output=if mode==1 { Some(super::owner(s,row).unwrap_or_else(||mixer::add_track(s,None))) } else { None };
            if let Some(n)=output {
                attached::bind(s,row,n);
                pl::arr_mut(s).tracks[row as usize].mode=PlaylistTrackMode::Audio;
                rename_output(s,n,&name);
            }
            // A header explicitly targets this lane and the play cursor, rather than searching other lanes.
            let clip=pl::add_audio_clip(s,&path,row,tick).expect("audio was validated before mutation");
            if let (Some(n),ClipSource::Channel(id))=(output,pl::clips(s)[clip].source) {
                if let Some(ch)=s.project.channels.iter_mut().find(|c|c.index==id) { ch.mixer_track=n; }
            }
            pl::arr_mut(s).tracks[row as usize].name=Some(name.clone());
            s.playlist.track=row;
            if creates_output {s.history.wrap_attached_instrument(before,output.unwrap(),selected);}
            s.history.end_sequence_named(previous,"Drop audio on track header");
            attached::clear(s);
            app.invalidate();
            Some(format!("Added {name} as {}",if mode==1{"an audio track"}else{"an audio clip"}))
        }
    }
}

fn sample_name(path: &str) -> String {
    std::path::Path::new(path).file_stem().map(|v|v.to_string_lossy().into_owned()).unwrap_or_else(||"Sample".into())
}

fn rename_output(s: &mut crate::app::State,n:u16,name:&str) {
    let old=mixer::track(s,n).and_then(|m|m.name.clone());
    crate::history::record(s,"Name track",crate::history::Snap::TrackName(n,old));
    if let Some(m)=mixer::track_mut(s,n) {m.name=Some(name.into());}
}

/// Used by both sample and generator header drops. Existing instruments and clips are retained.
pub fn instrument(app:&mut App,index:usize,row:u16,tick:u32,sample:Option<String>) -> Option<String> {
    if let Some(message)=blocked(app,row) {return Some(message);}
    let desc=app.plugins.available.get(index)?.desc.clone();
    if super::owner(&app.state,row).is_none() && mixer::inserts(&app.state)>=4093 {return Some("The mixer has reached its track limit".into());}
    let Some(pattern)=app.state.project.patterns.iter().map(|p|p.number).max().unwrap_or(0).checked_add(1) else {return Some("No pattern number is available".into());};
    let name=sample.as_ref().map_or(desc.name,|p|sample_name(p));
    let before=pl::capture(&app.state);
    let selected=app.state.selected_mixer;
    let creates_output=super::owner(&app.state,row).is_none();
    let previous=app.state.history.begin_sequence();
    crate::playlist_links::snapshot(&mut app.state,"Drop instrument on track header");
    crate::plugin_menu::add_generator(app,index,None);
    let s=&mut app.state;
    let ch=s.selected_channel;
    let n=super::owner(s,row).unwrap_or_else(||mixer::add_track(s,None));
    rename_output(s,n,&name);
    if let Some(path)=sample {
        let path=crate::data_folder::adopt(s,&path);
        let c=&mut s.project.channels[ch];
        c.sample_path=Some(path.clone());
        crate::rack::read_tags(c,std::path::Path::new(&path));
    }
    s.project.channels[ch].name=Some(name.clone());
    s.project.channels[ch].mixer_track=n;
    let id=s.project.channels[ch].index;
    let length=s.project.bar_ticks().max(1);
    s.project.patterns.push(os_model::Pattern{number:pattern,name:Some(name.clone()),length:Some(length),..Default::default()});
    s.pattern=pattern;
    attached::bind(s,row,n);
    let arr=pl::arr_mut(s);
    arr.tracks[row as usize].name=Some(name.clone());
    arr.tracks[row as usize].mode=PlaylistTrackMode::Instrument;
    let number=arr.number;
    s.project.instrument_lanes.push((id,number,row,pattern));
    s.playlist.source=Some(ClipSource::Pattern(pattern));
    s.playlist.picker=pl::PickKind::Pattern;
    let clip=pl::place(s,row,tick);
    s.playlist.sel=vec![clip];
    s.playlist.track=row;
    s.selected_mixer=n;
    if creates_output {s.history.wrap_attached_instrument(before,n,selected);}
    s.history.end_sequence_named(previous,"Drop instrument on track header");
    s.piano.dirty=true;
    attached::clear(s);
    app.invalidate();
    Some(format!("Added playable instrument track {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> App {
        let mut a=App::new(None);
        a.state.project=crate::app::default_project();
        a.state.project.arrangements.clear();
        pl::arr_mut(&mut a.state);
        a.plugins=crate::plugins::Plugins::builtins();
        a.state.z=vec![Win::Playlist];
        a.state.show_browser=false;
        a
    }
    fn wav() -> std::path::PathBuf {
        let path=os_paths::user_dir().unwrap().join("header-drop-test.wav");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        os_engine::wav::write_f32_stereo(&mut std::fs::File::create(&path).unwrap(),48000,&[0.1;9600]).unwrap();
        path
    }
    #[test]
    fn header_sources_batch_select_audio_role_and_reopen() {
        let _lock=crate::capture::TEST_RECORDING.lock().unwrap_or_else(|e|e.into_inner());
        let first=wav();let second=first.with_file_name("header-second.wav");std::fs::copy(&first,&second).unwrap();
        let paths:Vec<String>=[first,second].iter().map(|p|p.to_string_lossy().into_owned()).collect();
        for attached_view in [false,true]{
            let mut a=app();if attached_view{attached::toggle(&mut a);}
            crate::prefs::set_text(KEY,"2");
            let mut canvas=os_ui::Canvas::new(1600,1000,1.).unwrap();a.paint(&mut canvas,None);
            let r=a.hits.iter().find(|h|h.action==Action::PlHeader(2)).unwrap().rect;a.mouse=(r.cx(),r.cy());
            let before=a.state.project.clone();
            a.browser_drop(crate::views::browser::Item::Samples(paths.clone()),true);
            a.state.pending_open=None;
            let choices=super::super::instruments(&a.state,2);assert_eq!(choices.len(),2);
            assert!(choices.iter().all(|&ch|a.state.project.channels[ch].kind==os_model::ChannelKind::Sampler));
            assert_eq!(super::super::header_instrument(&a.state,2),Some(choices[1]));
            let after=a.state.project.clone();
            crate::history::undo(&mut a.state).unwrap();assert_eq!(a.state.project,before,"batch drop is a single undo");
            crate::history::redo(&mut a.state).unwrap();assert_eq!(a.state.project,after);
            a.paint(&mut canvas,None);
            let menu=Action::Mix(mixer::MHit::PlaylistInstruments(2));
            assert!(a.hits.iter().any(|h|h.action==menu));a.act(menu);
            let items=&a.state.playlist.popup.as_ref().unwrap().items;
            assert_eq!(items.len(),3);
            let select=items[1].cmd.clone();pl::run(&mut a,select);
            assert_eq!(a.state.selected_channel,choices[0]);
            assert_eq!(super::super::header_instrument(&a.state,2),Some(choices[0]));
            let binding=a.state.project.instrument_lanes.last().unwrap();assert_eq!(a.state.pattern,binding.3);
            let selected=a.state.project.clone();
            crate::history::undo(&mut a.state).unwrap();assert_eq!(a.state.project,after);
            crate::history::redo(&mut a.state).unwrap();assert_eq!(a.state.project,selected);
            let reopened=os_flp::project::load(&os_flp::project::create(&selected).unwrap()).unwrap();
            assert_eq!(reopened.instrument_lanes,selected.instrument_lanes);
            a.state.project=reopened;assert_eq!(super::super::header_instrument(&a.state,2),Some(choices[0]));
            a.state.hosted_rows.push(choices[0]);
            a.act(Action::Mix(mixer::MHit::PlaylistInstrument(2)));assert_eq!(a.state.pending_open,Some(choices[0]));
            a.state.pending_open=None;
            a.act(Action::Mix(mixer::MHit::AttachedPiano(2)));assert!(a.state.shown(Win::PianoRoll));
            assert!(mixer::right(&mut a,mixer::MHit::AttachedRecord(2)));
            assert!(a.state.playlist.popup.as_ref().unwrap().items.iter().any(|i|matches!(i.cmd,pl::Cmd::Mixer(mixer::MCmd::PlaylistInput(2,None)))));
            // An impossible ASIO device exercises accepted selection without opening a physical input.
            let input=crate::capture::Input{device:"Synthetic unavailable ASIO".into(),asio:true,first:0,stereo:false};
            let lanes=a.state.project.instrument_lanes.clone();let channels=a.state.project.channels.clone();
            super::super::set_input(&mut a,2,Some(input.clone()));
            assert_eq!(crate::playlist_links::mode(&a.state,2),PlaylistTrackMode::Audio);
            assert_eq!(super::super::header_instrument(&a.state,2),None);
            assert_eq!(a.state.project.instrument_lanes,lanes);assert_eq!(a.state.project.channels,channels);
            a.paint(&mut canvas,None);assert!(!a.hits.iter().any(|h|matches!(h.action,Action::Mix(mixer::MHit::PlaylistInstrument(2)|mixer::MHit::PlaylistInstruments(2)))));
            let reopened=os_flp::project::load(&os_flp::project::create(&a.state.project).unwrap()).unwrap();
            assert_eq!(reopened.arrangement().unwrap().tracks[2].mode,PlaylistTrackMode::Audio);
            super::super::set_input(&mut a,2,None);assert_eq!(super::super::header_instrument(&a.state,2),Some(choices[0]));
            pl::arr_mut(&mut a.state).tracks[2].locked=true;let locked=a.state.project.clone();
            super::super::set_input(&mut a,2,Some(input));assert_eq!(a.state.project,locked);
            super::super::choose_instrument(&mut a,2,channels[choices[1]].index);assert_eq!(a.state.project,locked);
            pl::arr_mut(&mut a.state).tracks[2].locked=false;
            let before=a.state.project.clone();samples(&mut a,vec![paths[0].clone(),"missing.wav".into()],2);assert_eq!(a.state.project,before);
            crate::prefs::set_text(KEY,"3");samples(&mut a,paths.clone(),2);assert_eq!(a.state.project,before);
            pl::run(&mut a,pl::Cmd::Nothing);assert_eq!(a.state.project,before,"cancel batch picker changes nothing");
        }
    }
    #[test]
    fn default_header_sample_is_playable_and_records_midi_on_its_lane() {
        use crate::{record::Extra,step_edit::Src};
        crate::prefs::set_text(KEY,"");
        assert_eq!(preference(),2);
        let path=wav();
        for attached_view in [false,true] {
            let mut a=app();
            if attached_view {attached::toggle(&mut a);}
            let mut canvas=os_ui::Canvas::new(1600,1000,1.).unwrap();a.paint(&mut canvas,None);
            let row=2;
            let header=a.hits.iter().find(|h|h.action==Action::PlHeader(row)).unwrap().rect;
            a.mouse=(header.cx(),header.cy());
            if attached_view {a.drop_file_at_plugin(path.clone(),None);}
            else {a.browser_drop(crate::views::browser::Item::Sample(path.to_string_lossy().into_owned()),true);}
            assert!(a.state.playlist.popup.is_none(),"default drop needs no chooser");
            let ch=a.state.selected_channel;
            assert_eq!(super::super::header_instrument(&a.state,row),Some(ch));
            assert_eq!(a.state.project.channels[ch].kind,os_model::ChannelKind::Sampler);
            let pattern=a.state.pattern;
            assert!(a.state.project.pattern(pattern).unwrap().notes.is_empty());
            // Host the actual dropped sample, but do not open its editor or an audio device.
            a.state.pending_open=None;a.plugin_tick();
            assert_eq!(a.plugins.sampler_path(ch).as_deref(),a.state.project.channels[ch].sample_path.as_deref());
            {
                let mut sampler=a.plugins.hosted[&ch].plugin.lock().unwrap();
                sampler.note_on(0,60,1.);
                let (mut left,mut right)=(vec![0.;4096],vec![0.;4096]);
                sampler.process(48000,&mut left,&mut right);
                assert!(left.iter().chain(&right).all(|v|v.is_finite()));
                assert!(left.iter().chain(&right).any(|v|v.abs()>0.001),"sample produces audio from notes");
                sampler.note_off(0,60);
            }
            attached::record_lane(&mut a,row);
            assert!(a.state.rec.armed && crate::record::Opt::Notes.on());
            assert!(a.audio_rec.is_none());
            a.transport.pretend_at(0.,true);
            a.record_down_with(Src::Midi(60),60,100,Extra::default());
            a.transport.pretend_at(96.,true);a.record_up_with(Src::Midi(60),None);a.end_take();
            let id=a.state.project.channels[ch].index;
            assert!(a.state.project.pattern(pattern).unwrap().notes.iter().any(|n|n.channel==id&&n.key==60&&n.length>0));
            assert!(pl::clips(&a.state).iter().any(|c|c.track==row&&c.source==ClipSource::Pattern(pattern)));
            let back=os_flp::project::load(&os_flp::project::create(&a.state.project).unwrap()).unwrap();
            assert_eq!(back.instrument_lanes,a.state.project.instrument_lanes);
            assert_eq!(back.pattern(pattern).unwrap().notes,a.state.project.pattern(pattern).unwrap().notes);
        }
        for mode in 0..4 {crate::prefs::set_text(KEY,&mode.to_string());assert_eq!(preference(),mode);}
        crate::prefs::set_text(KEY,"");
    }
    #[test]
    fn workflow_header_drop_modes_undo_reopen_and_routing() {
        let path=wav().to_string_lossy().into_owned();
        for mode in 0..3 {
            for attached_view in [false,true] {
                let mut a=app();
                if attached_view { attached::toggle(&mut a); }
                let before=a.state.project.clone();
                let count=a.state.history.done();
                run(&mut a,Command::Sample{path:path.clone(),row:2,tick:192,mode});
                assert_eq!(a.state.history.done(),count+1);
                let after=a.state.project.clone();
                let clip=pl::clips(&a.state).last().unwrap();
                assert_eq!((clip.track,clip.position),(2,192));
                let channel=a.state.project.channels.last().unwrap();
                if mode==2 {
                    assert_eq!(channel.kind,os_model::ChannelKind::Sampler);
                    assert!(channel.sample_path.is_some());
                    assert!(matches!(clip.source,ClipSource::Pattern(_)));
                    assert!(a.state.project.pattern(a.state.pattern).unwrap().notes.is_empty());
                    assert_eq!(crate::playlist_links::instrument(&a.state,2),Some(a.state.selected_channel));
                } else {assert_eq!(channel.kind,os_model::ChannelKind::AudioClip);}
                if mode>0 {assert!(channel.mixer_track>0);assert_eq!(super::super::owner(&a.state,2),Some(channel.mixer_track));}
                let file=os_flp::project::create(&after).unwrap();
                let back=os_flp::project::load(&file).unwrap();
                assert_eq!(back.instrument_lanes,after.instrument_lanes);
                assert_eq!(back.arrangement().unwrap().tracks[2].mode,after.arrangement().unwrap().tracks[2].mode);
                assert_eq!(back.channels.last().unwrap().mixer_track,channel.mixer_track);
                if mode==2 {
                    let ch=a.state.selected_channel;
                    a.state.selected_channel=0;
                    let clip=pl::clips(&a.state).len()-1;
                    pl::run(&mut a,pl::Cmd::ClipEditSource(clip));
                    assert_eq!(a.state.selected_channel,ch,"reopening an empty pattern selects its instrument");
                }
                crate::history::undo(&mut a.state).unwrap();assert_eq!(a.state.project,before,"mode {mode} attached {attached_view}");
                crate::history::redo(&mut a.state).unwrap();assert_eq!(a.state.project,after);
                if mode==1 {
                    let n=a.state.project.channels.last().unwrap().mixer_track;
                    pl::add_audio_clip(&mut a.state,&path,2,384).unwrap();
                    assert_eq!(a.state.project.channels.last().unwrap().mixer_track,n,"subsequent grid drops follow linked audio tracks");
                }
            }
        }
    }
    #[test]
    fn workflow_header_drop_cancel_locks_and_destinations() {
        let path=wav();let name=path.to_string_lossy().into_owned();
        let mut a=app();
        crate::prefs::set_text(KEY,"3");
        let before=a.state.project.clone();
        let mut canvas=os_ui::Canvas::new(1600,900,1.).unwrap();a.paint(&mut canvas,None);
        let r=a.hits.iter().find(|h|h.action==Action::PlHeader(2)).unwrap().rect;
        a.mouse=(r.cx(),r.cy());
        assert_eq!(header_at(&a),Some(2));
        a.drop_file_at_plugin(path.clone(),None);
        assert_eq!(a.state.project,before);
        assert!(a.state.playlist.popup.is_some());
        pl::run(&mut a,pl::Cmd::Nothing);assert_eq!(a.state.project,before);
        a.browser_drop(crate::views::browser::Item::Sample(name.clone()),true);
        let command=a.state.playlist.popup.as_ref().unwrap().items.iter().find(|i|i.label=="As instrument tracks").unwrap().cmd.clone();
        pl::arr_mut(&mut a.state).tracks[2].locked=true;
        let locked=a.state.project.clone();
        assert!(pl::run(&mut a,command).is_some());assert_eq!(a.state.project,locked);
        pl::arr_mut(&mut a.state).tracks[2].locked=false;
        crate::prefs::set_text(KEY,"2");
        a.drop_file_at_plugin(path,None);
        assert_eq!(a.state.project.channels.last().unwrap().kind,os_model::ChannelKind::Sampler);
        let now=a.state.project.clone();
        assert!(run(&mut a,Command::Sample{path:"missing.wav".into(),row:2,tick:0,mode:2}).is_some());
        assert_eq!(a.state.project,now);
        let n=super::super::owner(&a.state,2).unwrap();
        mixer::track_mut(&mut a.state,n).unwrap().edit_locked=true;
        let now=a.state.project.clone();
        sample(&mut a,name,2);assert_eq!(a.state.project,now);
        crate::prefs::set_text(KEY,"3");
    }
    #[test]
    fn workflow_generator_header_drop_and_visibility() {
        let mut a=app();let before=a.state.project.clone();
        let index=a.plugins.available.iter().position(|p|p.desc.name!=crate::plugins::LEMON_SAMPLER).unwrap();
        instrument(&mut a,index,1,0,None);
        assert_eq!(a.state.project.channels.last().unwrap().kind,os_model::ChannelKind::Plugin);
        let after=a.state.project.clone();
        crate::workspace_visibility::toggle("visible_playlist-header-drop-options");
        assert!(menu().children.is_empty());assert_eq!(a.state.project,after);
        crate::workspace_visibility::toggle("visible_playlist-header-drop-options");
        crate::history::undo(&mut a.state).unwrap();assert_eq!(a.state.project,before);
        crate::history::redo(&mut a.state).unwrap();assert_eq!(a.state.project,after);
    }
}
