//! Exercise the same stock descriptors and parameter entry points as the rack and mixer.
use super::*;
use std::time::Instant;

#[test]
#[ignore = "full stock parameter sweep and per-plugin timing report"]
fn stock_live_control_sweep() {
    let stock = Plugins::builtins();
    for (instrument, item) in stock.available.iter().map(|p| (true, p))
        .chain(stock.effects.iter().map(|p| (false, p))) {
        let shared = item.lib.clone().create(&item.desc.id).expect(&item.desc.name);
        let mut plugin = shared.lock().unwrap();
        assert!(plugin.activate(48000), "{}", item.desc.name);
        let sample = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../os-engine/tests/data/click_sine.mp3");
        let key = if item.desc.name == "ML Drums" || item.desc.name == "ML Chop" { 36 } else { 48 };
        if item.desc.name == LEMON_SAMPLER || item.desc.name == "ML Chop" {
            let state = plugin.save_state().unwrap();
            let path = sample.to_str().unwrap();
            let loaded = if item.desc.name == LEMON_SAMPLER { with_sample_path(&state, path) }
                else { with_chop_sample(&state, path) };
            assert!(plugin.load_state(&loaded.unwrap()));
        }
        let params: Vec<_> = (0..plugin.param_count()).filter_map(|i| plugin.param_info(i))
            .filter(|p| p.automatable && !p.read_only && !p.hidden && !p.bypass).collect();
        let mut clock = 0_u32;
        let mut last = [[0.0_f32; 2]; 2];
        let mut timings = Vec::new();
        let mut edit_times = Vec::new();
        let mut worst = (0.0_f32, String::new());
        let mut peak = 0.0_f32;
        let mut changes = 0;
        let mut render = |p: &mut dyn os_host::Plugin, edit: Option<&str>| {
            let mut l = [0.0; 64];
            let mut r = [0.0; 64];
            if instrument {
                if clock % 8192 == 0 {
                    p.note_off(30, key);
                    p.note_on(32, key, 0.64);
                }
            } else {
                for i in 0..64 {
                    let phase = ((clock + i as u32) % 48000) as f32 * std::f32::consts::TAU / 48000.;
                    l[i] = 0.19 * (phase * 93.).sin();
                    r[i] = 0.15 * (phase * 117.).cos();
                }
            }
            let start = Instant::now();
            if instrument { p.process(48000, &mut l, &mut r); }
            else { p.process_fx(48000, &mut l, &mut r); }
            let elapsed = start.elapsed().as_nanos();
            timings.push(elapsed);
            if edit.is_some() { edit_times.push(elapsed); }
            for (prev, channel) in last.iter_mut().zip([l, r]) {
                assert!(channel.iter().all(|x| x.is_finite()), "{} {edit:?}", item.desc.name);
                if let Some(label) = edit {
                    let adjacent = (prev[1]-prev[0]).abs().max((channel[1]-channel[0]).abs());
                    let jump = (channel[0]-prev[1]).abs() - 2.0*adjacent;
                    if jump > worst.0 { worst = (jump, label.to_owned()); }
                }
                *prev = [channel[62], channel[63]];
                peak = channel.iter().fold(peak, |a, x| a.max(x.abs()));
            }
            clock += 64;
        };
        for _ in 0..128 { render(&mut *plugin, None); }
        for param in &params {
            let original = plugin.param_value(param.id).unwrap_or(param.default);
            for next in [0.2, 0.8, original] {
                if !plugin.set_param(param.id, next) { continue; }
                changes += 1;
                render(&mut *plugin, Some(&param.name));
                for _ in 0..7 { render(&mut *plugin, None); }
            }
        }
        timings.sort_unstable();
        edit_times.sort_unstable();
        println!("AUDIT | {} | {} | {} | {:.4} | {} | {:.5} {} | edited p99 {} us",
            item.desc.name, changes, plugin.latency(), peak,
            timings[timings.len()*99/100]/1000, worst.0, worst.1,
            edit_times.get(edit_times.len()*99/100).copied().unwrap_or(0)/1000);
    }
}
