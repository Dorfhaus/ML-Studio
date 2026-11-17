// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded REX loop decoding. Statically linked VelociLoops; no external runtime.
use crate::Sample;
use std::{collections::VecDeque, ffi::c_void, path::{Path,PathBuf}, sync::{Mutex,OnceLock}};
const INPUT_LIMIT:u64=64*1024*1024;
#[derive(Clone,Debug)]
pub struct Metadata { pub bpm:f32, pub beats:f64, pub slices:Vec<f64> }
type Stamp=(u64,Option<std::time::SystemTime>);
fn cache()-> &'static Mutex<VecDeque<(PathBuf,Stamp,Metadata)>> {
    static CACHE:OnceLock<Mutex<VecDeque<(PathBuf,Stamp,Metadata)>>>=OnceLock::new();
    CACHE.get_or_init(Default::default)
}
pub fn is_rex(path:&Path)->bool {
    path.extension().and_then(|e|e.to_str()).is_some_and(|e| ["rx2","rex","rcy"].iter().any(|x|e.eq_ignore_ascii_case(x)))
}
unsafe extern "C" {
    fn ml_rex_open(data:*const u8,size:usize,info:*mut i32)->*mut c_void;
    fn ml_rex_close(handle:*mut c_void);
    fn ml_rex_render(handle:*mut c_void,left:*mut f32,right:*mut f32,cues:*mut i32)->bool;
}
struct Handle(*mut c_void);
impl Drop for Handle {fn drop(&mut self){unsafe{ml_rex_close(self.0)}}}
fn decode(bytes:&[u8])->Result<(Sample,Metadata),String> {
    if bytes.len() as u64>INPUT_LIMIT {return Err("REX files are limited to 64 MiB".into());}
    let mut info=[0;5];
    let handle=Handle(unsafe{ml_rex_open(bytes.as_ptr(),bytes.len(),info.as_mut_ptr())});
    if handle.0.is_null(){return Err("Invalid, unsupported or oversized REX loop (maximum 16,777,216 frames)".into());}
    let [rate,frames,tempo,ppq,slices]=info;
    if !(8000..=192000).contains(&rate) || !(1..=16*1024*1024).contains(&frames) || !(1..=65536).contains(&slices) || tempo<=0 || ppq<=0 {
        return Err("Invalid REX metadata".into());
    }
    let mut sample=Sample{rate:rate as u32,left:vec![0.;frames as usize],right:vec![0.;frames as usize]};
    let mut cues=vec![0;slices as usize];
    if !unsafe{ml_rex_render(handle.0,sample.left.as_mut_ptr(),sample.right.as_mut_ptr(),cues.as_mut_ptr())} ||
        sample.left.iter().chain(&sample.right).any(|v|!v.is_finite()) {
        return Err("Cannot decode REX audio".into());
    }
    cues.retain(|&p|p>=0 && p<frames); cues.sort_unstable(); cues.dedup();
    let meta=Metadata{bpm:tempo as f32/1000.,beats:f64::from(ppq)/15360.,slices:cues.into_iter().map(|p|f64::from(p)/f64::from(rate)).collect()};
    Ok((sample,meta))
}
pub fn read(path:&Path)->Result<Sample,String> {
    let stat=std::fs::metadata(path).map_err(|e|e.to_string())?;
    if stat.len()>INPUT_LIMIT{return Err("REX files are limited to 64 MiB".into());}
    let bytes=std::fs::read(path).map_err(|e|e.to_string())?;
    let (sample,meta)=decode(&bytes)?;
    if let Ok(mut c)=cache().lock(){
        c.retain(|(p,_,_)|p!=path);c.push_back((path.to_owned(),(stat.len(),stat.modified().ok()),meta));
        while c.len()>8{c.pop_front();}
    }
    Ok(sample)
}
pub fn metadata(path:&Path)->Option<Metadata> {
    if !is_rex(path){return None;}
    let stat=std::fs::metadata(path).ok()?;let stamp=(stat.len(),stat.modified().ok());
    if let Ok(c)=cache().lock() && let Some((_,_,m))=c.iter().find(|(p,s,_)|p==path && *s==stamp){return Some(m.clone());}
    read(path).ok()?;
    cache().lock().ok()?.iter().find(|(p,s,_)|p==path && *s==stamp).map(|(_,_,m)|m.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    unsafe extern "C" {
        fn vl_create_new(channels:i32,rate:i32,tempo:i32,error:*mut i32)->*mut c_void;
        fn vl_add_slice(file:*mut c_void,ppq:i32,left:*const f32,right:*const f32,frames:i32)->i32;
        fn vl_save_to_memory(file:*mut c_void,buffer:*mut c_void,size:*mut usize)->i32;
        fn vl_close(file:*mut c_void);
    }
    pub fn fixture(channels:i32)->Vec<u8> {unsafe{
        let file=vl_create_new(channels,44100,120000,std::ptr::null_mut());assert!(!file.is_null());
        let left:Vec<_>=(0..22050).map(|i| (i as f32*0.03).sin()*0.25).collect();let right:Vec<_>=left.iter().map(|v|-v*0.5).collect();
        for ppq in [0,15360,30720,46080]{assert!(vl_add_slice(file,ppq,left.as_ptr(),right.as_ptr(),left.len() as i32)>=0);}
        let mut size=0;assert_eq!(vl_save_to_memory(file,std::ptr::null_mut(),&mut size),0);
        let mut bytes=vec![0u8;size];assert_eq!(vl_save_to_memory(file,bytes.as_mut_ptr().cast(),&mut size),0);vl_close(file);bytes.truncate(size);bytes
    }}
    #[test]
    fn rex_synthetic_loop_keeps_timing_stereo_and_slice_positions(){
        for channels in [1,2]{let bytes=fixture(channels);let(s,m)=decode(&bytes).unwrap();
            assert_eq!(s.rate,44100);assert_eq!(s.frames(),88200);assert!((m.bpm-120.).abs()<0.01);assert!((m.beats-4.).abs()<0.001);
            assert_eq!(m.slices.len(),4);for (i,v) in m.slices.iter().enumerate(){assert!((v-i as f64*0.5).abs()<=1.01/44100.);}
            assert!(s.left.iter().any(|v|v.abs()>0.1));
            if channels==1{assert_eq!(s.left,s.right);}else{assert!(s.left.iter().zip(&s.right).all(|(l,r)|(l*0.5+r).abs()<0.0001));}
        }
    }
    #[test]
    fn rex_rejects_truncated_and_invalid_files(){let b=fixture(2);for n in [0,1,12,b.len()/2,b.len()-4]{assert!(decode(&b[..n]).is_err(),"accepted truncated file {n}");}assert!(decode(b"not a REX file").is_err());}

    #[test]
    #[ignore = "requires ML_REX_FIXTURES pointing to upstream compatibility fixtures"]
    fn rex_upstream_legacy_and_rx2_import() {
        let root=PathBuf::from(std::env::var_os("ML_REX_FIXTURES").expect("fixture folder"));
        for name in ["120FourBeats.rx2","120RexTest.rex","120RcyTest.rcy"] {
            let path=root.join(name);let sample=crate::sample::load(&path).unwrap();
            let meta=metadata(&path).unwrap();
            assert!((meta.bpm-120.).abs()<0.01);assert!(!meta.slices.is_empty());
            assert!((sample.frames() as f64/sample.rate as f64-meta.beats*0.5).abs()<2./sample.rate as f64);
            assert!(sample.left.iter().any(|v|v.abs()>0.001));
            eprintln!("{name}: {} Hz, {} frames, {} beats, {} slices",sample.rate,sample.frames(),meta.beats,meta.slices.len());
        }
    }
}
