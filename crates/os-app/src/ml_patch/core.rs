// SPDX-License-Identifier: GPL-3.0-or-later
//! A bounded, acyclic stereo graph built on ML Studio's existing plugin host.
//! Construction, state and delay allocation belong to the UI thread. Rendering
//! only tries locks, uses prepared buffers and never loads a plugin or allocates.
use crate::plugins::Available;
use os_host::{Descriptor, Format, HostedEffect, Instrument, Kind, Library, Plugin, SharedPlugin};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, Mutex, OnceLock},
};
pub const ID: &str = "ml.patch";
pub const NAME: &str = "ML Patch";
const BLOCK: usize = 1024;
const MAX_NODES: usize = 32;
const MAX_EDGES: usize = 96;
const MAX_DELAY: usize = 262144;
const MAX_STATE: usize = 64 * 1024 * 1024;
type Catalog = Vec<Available>;
fn catalog() -> &'static Mutex<Catalog> {
    static C: OnceLock<Mutex<Catalog>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(Vec::new()))
}
pub fn set_catalog(v: &[Available]) {
    if let Ok(mut c) = catalog().lock() {
        *c = v.iter().filter(|a| a.desc.id != ID).cloned().collect();
    }
}
pub fn descriptor() -> Descriptor {
    Descriptor {
        format: Format::Native,
        id: ID.into(),
        name: NAME.into(),
        vendor: "ML Studio".into(),
        version: "1".into(),
        features: vec!["audio-effect".into()],
        kind: Kind::Effect,
        path: std::env::current_exe().unwrap_or_default(),
    }
}
pub struct PatchLibrary {
    desc: [Descriptor; 1],
}
pub fn available() -> Available {
    let d = descriptor();
    Available {
        desc: d.clone(),
        lib: Arc::new(PatchLibrary { desc: [d] }),
    }
}
impl Library for PatchLibrary {
    fn path(&self) -> &Path {
        &self.desc[0].path
    }
    fn plugins(&self) -> &[Descriptor] {
        &self.desc
    }
    fn create(self: Arc<Self>, id: &str) -> Result<SharedPlugin, String> {
        if id != ID {
            return Err("Unknown patch plugin".into());
        }
        Ok(Arc::new(Mutex::new(PatchPlugin {
            desc: descriptor(),
            graph: Arc::new(Mutex::new(Graph::empty())),
            latency: 0,
        })))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: u32,
    pub uid: String,
    pub name: String,
    pub state: Vec<u8>,
    pub x: f32,
    pub y: f32,
    pub bypass: bool,
    pub gain: f32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Edge {
    pub from: u32,
    pub to: u32,
    #[serde(default)]
    pub key: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Macro {
    pub name: String,
    pub node: u32,
    pub param: u32,
    pub value: f64,
    pub low: f64,
    pub high: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub version: u32,
    pub next_id: u32,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub macros: Vec<Macro>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 2,
            nodes: vec![],
            edges: vec![Edge {
                from: 0,
                to: 1,
                key: false,
            }],
            macros: (0..8)
                .map(|i| Macro {
                    name: format!("Macro {}", i + 1),
                    node: 0,
                    param: 0,
                    value: 0.5,
                    low: 0.,
                    high: 1.,
                })
                .collect(),
        }
    }
}
impl Document {
    pub fn validate(&self) -> Result<Vec<u32>, String> {
        if self.next_id > 1_000_000
            || self.next_id < 2
            || self.version != 1
            || self.nodes.len() > MAX_NODES
            || self.edges.len() > MAX_EDGES
            || self.macros.len() != 8
        {
            return Err("Unsupported or oversized ML Patch preset".into());
        }
        let mut ids = std::collections::BTreeSet::from([0, 1]);
        for n in &self.nodes {
            if n.id < 2
                || n.id >= self.next_id
                || !ids.insert(n.id)
                || !n.x.is_finite()
                || !n.y.is_finite()
                || !n.gain.is_finite()
                || !(0.0..=2.).contains(&n.gain)
                || n.uid == "native:ml.patch"
            {
                return Err("Invalid patch node".into());
            }
        }
        for (i, e) in self.edges.iter().enumerate() {
            if e.from == 1
                || e.to == 0
                || e.from == e.to
                || !ids.contains(&e.from)
                || !ids.contains(&e.to)
                || (e.key && e.to == 1)
                || self.edges[..i].contains(e)
            {
                return Err("Invalid or duplicate cable".into());
            }
        }
        for m in &self.macros {
            if !m.value.is_finite()
                || !m.low.is_finite()
                || !m.high.is_finite()
                || !(0.0..=1.).contains(&m.value)
                || !(0.0..=1.).contains(&m.low)
                || !(0.0..=1.).contains(&m.high)
            {
                return Err("Invalid macro value".into());
            }
        }
        let mut order = vec![0];
        while order.len() < ids.len() {
            let Some(&id) = ids.iter().find(|id| {
                !order.contains(id)
                    && self
                        .edges
                        .iter()
                        .filter(|e| e.to == **id)
                        .all(|e| order.contains(&e.from))
            }) else {
                return Err(
                    "That cable would create feedback. ML Patch keeps routing one-way.".into(),
                );
            };
            order.push(id);
        }
        Ok(order)
    }
}
struct Delay {
    data: Vec<[f32; 2]>,
    at: usize,
}
impl Delay {
    fn new(n: usize) -> Self {
        Self {
            data: vec![[0.; 2]; n],
            at: 0,
        }
    }
    fn tick(&mut self, v: [f32; 2]) -> [f32; 2] {
        if self.data.is_empty() {
            return v;
        }
        let old = self.data[self.at];
        self.data[self.at] = v;
        self.at = (self.at + 1) % self.data.len();
        old
    }
    fn reset(&mut self) {
        self.data.fill([0.; 2]);
        self.at = 0;
    }
}
struct Runtime {
    id: u32,
    plugin: Option<SharedPlugin>,
    left: Vec<f32>,
    right: Vec<f32>,
    key_l: Vec<f32>,
    key_r: Vec<f32>,
    delay: u32,
    dry_l: Vec<f32>,
    dry_r: Vec<f32>,
    mix: f32,
    gain: f32,
    bypass: Delay,
    error: Option<String>,
}
struct Cable {
    edge: Edge,
    source: usize,
    dest: usize,
    delay: Delay,
}
pub struct Graph {
    pub doc: Document,
    nodes: Vec<Runtime>,
    cables: Vec<Cable>,
    order: Vec<usize>,
    pub rate: u32,
    pub latency: u32,
    pub revision: u64,
    pub pending: bool,
    pub message: Option<String>,
    fade: usize,
    pub edits: Vec<String>,
    transport: os_engine::TransportInfo,
    macro_indices: [Option<(u32, u32)>; 8],
    events: Vec<(u32, u8, f64)>,
}
impl Graph {
    pub fn empty() -> Self {
        let mut g = Self {
            doc: Document::default(),
            nodes: vec![],
            cables: vec![],
            order: vec![],
            rate: 48000,
            latency: 0,
            revision: 0,
            pending: false,
            message: None,
            fade: 0,
            edits: vec![],
            macro_indices: [None; 8],
            events: Vec::with_capacity(256),
            transport: os_engine::TransportInfo {
                playing: false,
                tempo: 120.,
                frame: 0,
                beats: 0.,
                bar_start: 0.,
                bar: 0,
                numerator: 4,
                denominator: 4,
                looping: None,
            },
        };
        g.replace(Document::default()).unwrap();
        g
    }
    pub fn plugin(&self, id: u32) -> Option<SharedPlugin> {
        self.nodes.iter().find(|n| n.id == id)?.plugin.clone()
    }
    pub fn error(&self, id: u32) -> Option<&str> {
        self.nodes.iter().find(|n| n.id == id)?.error.as_deref()
    }
    pub fn snapshot(&self) -> Result<Document, String> {
        let mut d = self.doc.clone();
        for n in &mut d.nodes {
            if let Some(p) = self.plugin(n.id) {
                n.state = p
                    .lock()
                    .map_err(|_| "Plugin unavailable")?
                    .save_state()
                    .ok_or_else(|| format!("{} could not save its settings", n.name))?;
            }
        }
        Ok(d)
    }
    pub fn bytes(&self) -> Option<Vec<u8>> {
        let bytes =
            serde_json::to_vec(&serde_json::json!({"plugin":{"patch":self.snapshot().ok()?}}))
                .ok()?;
        (bytes.len() <= MAX_STATE).then_some(bytes)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Document, String> {
        if bytes.len() > MAX_STATE {
            return Err("Patch preset is too large".into());
        }
        let v: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let d: Document =
            serde_json::from_value(v["plugin"]["patch"].clone()).map_err(|e| e.to_string())?;
        d.validate()?;
        Ok(d)
    }
    /// Transactional: keep the old graph if any present plugin rejects its state.
    pub fn replace(&mut self, d: Document) -> Result<(), String> {
        let order = d.validate()?;
        let available = catalog()
            .lock()
            .map_err(|_| "Plugin catalog unavailable")?
            .clone();
        let mut next = Vec::new();
        for id in [0, 1].into_iter().chain(d.nodes.iter().map(|n| n.id)) {
            let mut plugin = None;
            let mut error = None;
            if let Some(n) = d.nodes.iter().find(|n| n.id == id) {
                // Layout/cable edits retain the exact running instance and its tail.
                if let Some(old) = self.doc.nodes.iter().find(|o| o.id == id && o.uid == n.uid) {
                    if old.state == n.state {
                        plugin = self.plugin(id);
                    }
                }
                if plugin.is_none() {
                    if let Some(a) = available.iter().find(|a| a.desc.uid() == n.uid) {
                        let p = crate::plugins::create_nested(a)?;
                        {
                            let mut h = p.lock().map_err(|_| "Plugin unavailable")?;
                            if !n.state.is_empty() && !h.load_state(&n.state) {
                                return Err(format!("{} rejected its saved settings", n.name));
                            }
                            if !h.activate(self.rate) {
                                return Err(format!("{} could not start", n.name));
                            }
                        }
                        plugin = Some(p);
                    } else {
                        error = Some(format!(
                            "{} is missing. Its settings are retained; this branch is silent.",
                            n.name
                        ));
                    }
                }
            }
            let delay = plugin
                .as_ref()
                .and_then(|p| p.lock().ok().map(|p| p.latency()))
                .unwrap_or(0);
            if delay as usize > MAX_DELAY {
                return Err("A plugin exceeds ML Patch's delay limit".into());
            }
            next.push(Runtime {
                id,
                plugin,
                left: vec![0.; BLOCK],
                right: vec![0.; BLOCK],
                key_l: vec![0.; BLOCK],
                key_r: vec![0.; BLOCK],
                delay,
                dry_l: vec![0.; BLOCK],
                dry_r: vec![0.; BLOCK],
                mix: if d.nodes.iter().any(|n| n.id == id && n.bypass) {
                    0.
                } else {
                    1.
                },
                gain: d.nodes.iter().find(|n| n.id == id).map_or(1., |n| n.gain),
                bypass: Delay::new(delay as usize),
                error,
            });
        }
        let indices: Vec<usize> = order
            .iter()
            .map(|id| next.iter().position(|n| n.id == *id).unwrap())
            .collect();
        let (cables, latency) = Self::plan(&d, &next, &indices)?;
        self.macro_indices = std::array::from_fn(|i| {
            let m = &d.macros[i];
            let p = next
                .iter()
                .find(|n| n.id == m.node)?
                .plugin
                .as_ref()?
                .lock()
                .ok()?;
            let index =
                (0..p.param_count()).find(|&i| p.param_info(i).is_some_and(|p| p.id == m.param))?;
            Some((m.node, index as u32))
        });
        self.doc = d;
        self.nodes = next;
        self.order = indices;
        self.cables = cables;
        self.latency = latency;
        self.pending = false;
        self.fade = 128;
        self.revision = self.revision.wrapping_add(1);
        self.message = self.nodes.iter().find_map(|n| n.error.clone());
        Ok(())
    }
    fn plan(d: &Document, nodes: &[Runtime], order: &[usize]) -> Result<(Vec<Cable>, u32), String> {
        let mut times = [0u32; MAX_NODES + 2];
        let mut input = [0u32; MAX_NODES + 2];
        for &i in order {
            let arrival = d
                .edges
                .iter()
                .filter(|e| e.to == nodes[i].id)
                .map(|e| times[nodes.iter().position(|n| n.id == e.from).unwrap()])
                .max()
                .unwrap_or(0);
            input[i] = arrival;
            times[i] = arrival.saturating_add(nodes[i].delay);
            if times[i] as usize > MAX_DELAY {
                return Err("Combined patch delay exceeds the supported limit".into());
            }
        }
        let mut memory = 0usize;
        let mut cables = vec![];
        for &edge in &d.edges {
            let source = nodes.iter().position(|n| n.id == edge.from).unwrap();
            let dest = nodes.iter().position(|n| n.id == edge.to).unwrap();
            let delay = (input[dest] - times[source]) as usize;
            memory += delay;
            if memory > 2 * 1024 * 1024 {
                return Err("This patch needs too much delay compensation memory".into());
            }
            cables.push(Cable {
                edge,
                source,
                dest,
                delay: Delay::new(delay),
            });
        }
        Ok((cables, times[1]))
    }
    pub fn idle(&mut self) {
        let mut changed = false;
        for n in &mut self.nodes {
            if let Some(p) = &n.plugin {
                if let Ok(mut p) = p.try_lock() {
                    p.idle();
                    let delay = p.latency();
                    if delay != n.delay {
                        n.delay = delay;
                        changed = true;
                    }
                }
            }
        }
        if changed || self.pending {
            match Self::plan(&self.doc, &self.nodes, &self.order) {
                Ok((cables, latency)) => {
                    self.cables = cables;
                    self.latency = latency;
                    for n in &mut self.nodes {
                        n.bypass = Delay::new(n.delay as usize);
                    }
                    self.pending = false;
                    self.fade = 128;
                }
                Err(e) => {
                    self.pending = true;
                    self.message = Some(e);
                }
            }
        }
    }
    pub fn process(&mut self, rate: u32, left: &mut [f32], right: &mut [f32]) {
        if self.pending || rate != self.rate {
            left.fill(0.);
            right.fill(0.);
            return;
        }
        for start in (0..left.len()).step_by(BLOCK) {
            let count = (left.len() - start).min(BLOCK);
            let l = &mut left[start..start + count];
            let r = &mut right[start..start + count];
            for n in &mut self.nodes {
                n.left[..count].fill(0.);
                n.right[..count].fill(0.);
                n.key_l[..count].fill(0.);
                n.key_r[..count].fill(0.);
            }
            self.nodes[0].left[..count].copy_from_slice(l);
            self.nodes[0].right[..count].copy_from_slice(r);
            for &(offset, id, value) in &self.events {
                if (start as u32..(start + count) as u32).contains(&offset) {
                    if let Some((node, index)) = self.macro_indices[id as usize] {
                        let m = &self.doc.macros[id as usize];
                        if let Some(p) = self
                            .nodes
                            .iter()
                            .find(|n| n.id == node)
                            .and_then(|n| n.plugin.as_ref())
                        {
                            if let Ok(mut p) = p.try_lock() {
                                p.param_fx(
                                    offset - start as u32,
                                    index,
                                    m.low + (m.high - m.low) * value,
                                );
                            }
                        }
                    }
                }
            }
            for &at in &self.order {
                for c in self.cables.iter_mut().filter(|c| c.dest == at) {
                    for i in 0..count {
                        let v = c
                            .delay
                            .tick([self.nodes[c.source].left[i], self.nodes[c.source].right[i]]);
                        let n = &mut self.nodes[at];
                        if c.edge.key {
                            n.key_l[i] += v[0];
                            n.key_r[i] += v[1];
                        } else {
                            n.left[i] += v[0];
                            n.right[i] += v[1];
                        }
                    }
                }
                let n = &mut self.nodes[at];
                if let Some(spec) = self.doc.nodes.iter().find(|s| s.id == n.id) {
                    for i in 0..count {
                        n.left[i] = finite(n.left[i]);
                        n.right[i] = finite(n.right[i]);
                        let dry = n.bypass.tick([n.left[i], n.right[i]]);
                        n.dry_l[i] = dry[0];
                        n.dry_r[i] = dry[1];
                    }
                    // Keep the dry delay primed even while wet, so bypass does not
                    // refill a delay or change the parallel arrival time.
                    if let Some(p) = &n.plugin {
                        if let Ok(mut p) = p.try_lock() {
                            let t = self.transport.advanced(start as u64, rate);
                            p.transport_fx(&t);
                            p.process_fx_keyed(
                                rate,
                                &mut n.left[..count],
                                &mut n.right[..count],
                                (&n.key_l[..count], &n.key_r[..count]),
                            );
                            if p.reported_delay_fx() != n.delay {
                                self.pending = true;
                            }
                        } else {
                            n.left[..count].fill(0.);
                            n.right[..count].fill(0.);
                        }
                    } else {
                        n.left[..count].fill(0.);
                        n.right[..count].fill(0.);
                    }
                    for i in 0..count {
                        let target = if spec.bypass { 0. } else { 1. };
                        n.mix += (target - n.mix).clamp(-1. / 128., 1. / 128.);
                        n.gain += (spec.gain - n.gain).clamp(-1. / 128., 1. / 128.);
                        n.left[i] = finite(
                            (finite(n.left[i]) * n.mix + n.dry_l[i] * (1. - n.mix)) * n.gain,
                        );
                        n.right[i] = finite(
                            (finite(n.right[i]) * n.mix + n.dry_r[i] * (1. - n.mix)) * n.gain,
                        );
                    }
                }
            }
            if self.pending {
                l.fill(0.);
                r.fill(0.);
            } else {
                for i in 0..count {
                    let gain = if self.fade > 0 {
                        self.fade -= 1;
                        1. - self.fade as f32 / 128.
                    } else {
                        1.
                    };
                    l[i] = finite(self.nodes[1].left[i]) * gain;
                    r[i] = finite(self.nodes[1].right[i]) * gain;
                }
            }
        }
        self.events.clear();
    }
    pub fn reset(&mut self) {
        self.events.clear();
        for n in &mut self.nodes {
            n.bypass.reset();
            if let Some(p) = &n.plugin {
                if let Ok(mut p) = p.try_lock() {
                    p.reset_fx();
                }
            }
        }
        for c in &mut self.cables {
            c.delay.reset();
        }
    }
    pub fn set_macro(&mut self, id: usize, value: f64) -> bool {
        let Some(m) = self.doc.macros.get_mut(id) else {
            return false;
        };
        if !value.is_finite() {
            return false;
        }
        m.value = value.clamp(0., 1.);
        let (node, param, v) = (m.node, m.param, m.low + (m.high - m.low) * m.value);
        if let Some(p) = self.plugin(node) {
            if let Ok(mut p) = p.try_lock() {
                p.set_param(param, v);
            }
        }
        true
    }
}
fn finite(v: f32) -> f32 {
    if v.is_finite() { v } else { 0. }
}
pub struct PatchPlugin {
    desc: Descriptor,
    pub graph: Arc<Mutex<Graph>>,
    latency: u32,
}
impl Instrument for PatchPlugin {
    fn note_on(&mut self, _: u32, _: u8, _: f32) {}
    fn note_off(&mut self, _: u32, _: u8) {}
    fn process(&mut self, _: u32, l: &mut [f32], r: &mut [f32]) {
        l.fill(0.);
        r.fill(0.);
    }
}
impl HostedEffect for PatchPlugin {
    fn process_fx(&mut self, rate: u32, l: &mut [f32], r: &mut [f32]) {
        if let Ok(mut g) = self.graph.try_lock() {
            g.process(rate, l, r);
            self.latency = g.latency;
        } else {
            l.fill(0.);
            r.fill(0.);
        }
    }
    fn reset_fx(&mut self) {
        if let Ok(mut g) = self.graph.try_lock() {
            g.reset();
        }
    }
    fn reported_delay_fx(&self) -> u32 {
        self.latency
    }
    fn transport_fx(&mut self, t: &os_engine::TransportInfo) {
        if let Ok(mut g) = self.graph.try_lock() {
            g.transport = *t;
        }
    }
    fn param_fx(&mut self, offset: u32, index: u32, value: f64) {
        if index < 8 && value.is_finite() {
            if let Ok(mut g) = self.graph.try_lock() {
                g.doc.macros[index as usize].value = value.clamp(0., 1.);
                if g.events.len() < 256 {
                    g.events.push((offset, index as u8, value.clamp(0., 1.)));
                }
            }
        }
    }
}
impl Plugin for PatchPlugin {
    fn desc(&self) -> &Descriptor {
        &self.desc
    }
    fn host_extension(&self) -> Option<Arc<dyn std::any::Any + Send + Sync>> {
        Some(self.graph.clone())
    }
    fn activate(&mut self, rate: u32) -> bool {
        let Ok(mut g) = self.graph.lock() else {
            return false;
        };
        if !(8000..=384000).contains(&rate) {
            return false;
        }
        g.rate = rate;
        for n in &g.nodes {
            if let Some(p) = &n.plugin {
                if !p.lock().is_ok_and(|mut p| p.activate(rate)) {
                    return false;
                }
            }
        }
        g.pending = true;
        g.idle();
        self.latency = g.latency;
        !g.pending
    }
    fn idle(&mut self) {
        if let Ok(mut g) = self.graph.try_lock() {
            g.idle();
            self.latency = g.latency;
        }
    }
    fn state_undo_supported(&self) -> bool {
        true
    }
    fn take_state_edits(&mut self) -> Vec<String> {
        self.graph
            .lock()
            .map(|mut g| std::mem::take(&mut g.edits))
            .unwrap_or_default()
    }
    fn take_state_changed(&mut self) -> bool {
        let Ok(mut g) = self.graph.try_lock() else {
            return false;
        };
        let Graph { nodes, doc, .. } = &mut *g;
        let mut changed = false;
        for n in nodes {
            if let Some(p) = &n.plugin {
                if let Ok(mut p) = p.try_lock() {
                    changed |= p.take_state_changed();
                    for e in p.take_param_events() {
                        match e {
                            os_host::ParamEvent::Value(id, v)
                            | os_host::ParamEvent::Sync(id, v) => {
                                changed |= matches!(e, os_host::ParamEvent::Value(..));
                                for m in &mut doc.macros {
                                    if m.node == n.id && m.param == id && m.high != m.low {
                                        m.value = ((v - m.low) / (m.high - m.low)).clamp(0., 1.);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        changed
    }
    fn has_editor(&self) -> bool {
        false
    }
    fn editor_open(&self) -> bool {
        false
    }
    fn open_editor(&mut self, _: *mut std::ffi::c_void, _: f32) -> Result<(u32, u32), String> {
        Err("Open ML Patch from its mixer slot".into())
    }
    fn editor_size(&self) -> Option<(u32, u32)> {
        None
    }
    fn close_editor(&mut self) {}
    fn save_state(&self) -> Option<Vec<u8>> {
        self.graph.lock().ok()?.bytes()
    }
    fn load_state(&mut self, b: &[u8]) -> bool {
        let Ok(d) = Graph::from_bytes(b) else {
            return false;
        };
        self.graph.lock().is_ok_and(|mut g| g.replace(d).is_ok())
    }
    fn latency(&self) -> u32 {
        self.latency
    }
    fn set_offline(&mut self, v: bool) {
        if let Ok(g) = self.graph.lock() {
            for n in &g.nodes {
                if let Some(p) = &n.plugin {
                    if let Ok(mut p) = p.lock() {
                        p.set_offline(v);
                    }
                }
            }
        }
    }
    fn param_count(&self) -> usize {
        8
    }
    fn param_info(&self, i: usize) -> Option<os_host::ParamInfo> {
        let g = self.graph.lock().ok()?;
        let m = g.doc.macros.get(i)?;
        Some(os_host::ParamInfo {
            id: i as u32,
            name: m.name.clone(),
            unit: "%".into(),
            group: "Macros".into(),
            min: 0.,
            max: 100.,
            default: 0.5,
            steps: 0,
            automatable: true,
            read_only: false,
            hidden: false,
            bypass: false,
            enumerated: false,
        })
    }
    fn param_value(&self, id: u32) -> Option<f64> {
        self.graph
            .lock()
            .ok()?
            .doc
            .macros
            .get(id as usize)
            .map(|m| m.value)
    }
    fn set_param(&mut self, id: u32, v: f64) -> bool {
        self.graph
            .lock()
            .is_ok_and(|mut g| g.set_macro(id as usize, v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    struct Lib {
        desc: [Descriptor; 1],
        delay: Arc<AtomicU32>,
    }
    impl Library for Lib {
        fn path(&self) -> &Path {
            &self.desc[0].path
        }
        fn plugins(&self) -> &[Descriptor] {
            &self.desc
        }
        fn create(self: Arc<Self>, _: &str) -> Result<SharedPlugin, String> {
            Ok(Arc::new(Mutex::new(Mock {
                desc: self.desc[0].clone(),
                delay: self.delay.clone(),
                ring: vec![[0.; 2]; 1024],
                at: 0,
                gain: 1.,
            })))
        }
    }
    struct Mock {
        desc: Descriptor,
        delay: Arc<AtomicU32>,
        ring: Vec<[f32; 2]>,
        at: usize,
        gain: f32,
    }
    impl Instrument for Mock {
        fn note_on(&mut self, _: u32, _: u8, _: f32) {}
        fn note_off(&mut self, _: u32, _: u8) {}
        fn process(&mut self, r: u32, l: &mut [f32], rr: &mut [f32]) {
            self.process_fx(r, l, rr)
        }
    }
    impl HostedEffect for Mock {
        fn process_fx(&mut self, _: u32, l: &mut [f32], r: &mut [f32]) {
            let d = self.delay.load(Ordering::SeqCst) as usize;
            for i in 0..l.len() {
                self.ring[self.at] = [l[i], r[i]];
                let v = self.ring[(self.at + 1024 - d) % 1024];
                l[i] = v[0] * self.gain;
                r[i] = v[1] * self.gain;
                self.at = (self.at + 1) % 1024;
            }
        }
        fn reported_delay_fx(&self) -> u32 {
            self.delay.load(Ordering::SeqCst)
        }
        fn reset_fx(&mut self) {
            self.ring.fill([0.; 2]);
            self.at = 0;
        }
        fn param_fx(&mut self, _: u32, i: u32, v: f64) {
            if i == 0 {
                self.gain = v as f32;
            }
        }
    }
    impl Plugin for Mock {
        fn desc(&self) -> &Descriptor {
            &self.desc
        }
        fn activate(&mut self, _: u32) -> bool {
            true
        }
        fn idle(&mut self) {}
        fn has_editor(&self) -> bool {
            false
        }
        fn editor_open(&self) -> bool {
            false
        }
        fn open_editor(&mut self, _: *mut std::ffi::c_void, _: f32) -> Result<(u32, u32), String> {
            Err("No editor".into())
        }
        fn editor_size(&self) -> Option<(u32, u32)> {
            None
        }
        fn close_editor(&mut self) {}
        fn save_state(&self) -> Option<Vec<u8>> {
            Some(self.gain.to_le_bytes().to_vec())
        }
        fn load_state(&mut self, s: &[u8]) -> bool {
            if s.len() != 4 {
                return false;
            }
            self.gain = f32::from_le_bytes(s.try_into().unwrap());
            self.gain.is_finite()
        }
        fn latency(&self) -> u32 {
            self.delay.load(Ordering::SeqCst)
        }
        fn param_count(&self) -> usize {
            1
        }
        fn param_info(&self, i: usize) -> Option<os_host::ParamInfo> {
            (i == 0).then(|| os_host::ParamInfo {
                id: 42,
                name: "Gain".into(),
                unit: "".into(),
                group: "".into(),
                min: 0.,
                max: 1.,
                default: 1.,
                steps: 0,
                automatable: true,
                read_only: false,
                hidden: false,
                bypass: false,
                enumerated: false,
            })
        }
        fn param_value(&self, id: u32) -> Option<f64> {
            (id == 42).then_some(self.gain as f64)
        }
        fn set_param(&mut self, id: u32, v: f64) -> bool {
            if id != 42 {
                return false;
            }
            self.gain = v as f32;
            true
        }
    }
    fn setup() -> (Graph, Arc<AtomicU32>) {
        let mut d = descriptor();
        d.id = "patch-test-delay".into();
        d.name = "Test delay".into();
        let delay = Arc::new(AtomicU32::new(32));
        set_catalog(&[Available {
            desc: d.clone(),
            lib: Arc::new(Lib {
                desc: [d.clone()],
                delay: delay.clone(),
            }),
        }]);
        let mut g = Graph::empty();
        let mut doc = Document::default();
        doc.next_id = 3;
        doc.nodes.push(Node {
            id: 2,
            uid: d.uid(),
            name: d.name,
            state: vec![],
            x: 200.,
            y: 40.,
            gain: 1.,
            bypass: false,
        });
        doc.edges = vec![
            Edge {
                from: 0,
                to: 1,
                key: false,
            },
            Edge {
                from: 0,
                to: 2,
                key: false,
            },
            Edge {
                from: 2,
                to: 1,
                key: false,
            },
        ];
        g.replace(doc).unwrap();
        (g, delay)
    }
    fn warm(g: &mut Graph) {
        let mut l = vec![0.; 2048];
        let mut r = l.clone();
        g.process(48000, &mut l, &mut r);
        g.reset();
    }
    fn impulse(g: &mut Graph, at: usize, value: f32) {
        let mut l = vec![0.; 256];
        let mut r = l.clone();
        l[0] = 1.;
        r[0] = 1.;
        g.process(48000, &mut l, &mut r);
        for i in 0..256 {
            assert!(
                (l[i] - if i == at { value } else { 0. }).abs() < 1e-5,
                "sample {i}: {}",
                l[i]
            );
            assert_eq!(l[i], r[i]);
        }
    }
    #[test]
    fn ml_patch_compensates_parallel_paths_and_live_bypass() {
        let (mut g, _) = setup();
        assert_eq!(g.latency, 32);
        warm(&mut g);
        impulse(&mut g, 32, 2.);
        let mut d = g.snapshot().unwrap();
        d.nodes[0].bypass = true;
        g.replace(d).unwrap();
        warm(&mut g);
        impulse(&mut g, 32, 2.);
    }
    #[test]
    fn ml_patch_rebuilds_dynamic_latency_without_transport_restart() {
        let (mut g, d) = setup();
        warm(&mut g);
        d.store(64, Ordering::SeqCst);
        let mut l = [1.; 128];
        let mut r = l;
        g.process(48000, &mut l, &mut r);
        assert!(g.pending);
        assert_eq!(l, [0.; 128]);
        g.idle();
        assert!(!g.pending);
        assert_eq!(g.latency, 64);
        warm(&mut g);
        impulse(&mut g, 64, 2.);
    }
    #[test]
    fn ml_patch_cycle_and_invalid_state_are_transactional() {
        let (mut g, _) = setup();
        let before = g.bytes().unwrap();
        let mut d = g.snapshot().unwrap();
        d.edges.push(Edge {
            from: 2,
            to: 2,
            key: false,
        });
        assert!(g.replace(d).is_err());
        assert_eq!(g.bytes().unwrap(), before);
        assert!(Graph::from_bytes(b"{}").is_err());
        let mut d = g.snapshot().unwrap();
        d.nodes[0].state = vec![1, 2];
        assert!(g.replace(d).is_err());
        assert_eq!(g.bytes().unwrap(), before);
    }
    #[test]
    fn ml_patch_missing_plugin_keeps_state_and_silences_only_its_branch() {
        let (mut g, _) = setup();
        let saved = g.bytes().unwrap();
        set_catalog(&[]);
        let mut absent = Graph::empty();
        absent.replace(Graph::from_bytes(&saved).unwrap()).unwrap();
        assert!(absent.error(2).is_some());
        assert_eq!(absent.bytes().unwrap(), saved);
        warm(&mut absent);
        impulse(&mut absent, 0, 1.);
    }
    #[test]
    fn ml_patch_macros_map_ids_to_parameter_indices_and_roundtrip() {
        let (mut g, _) = setup();
        let mut d = g.snapshot().unwrap();
        d.macros[0] = Macro {
            name: "Wet gain".into(),
            node: 2,
            param: 42,
            value: 0.5,
            low: 0.,
            high: 1.,
        };
        g.replace(d).unwrap();
        g.set_macro(0, 0.5);
        warm(&mut g);
        impulse(&mut g, 32, 1.5);
        let saved = g.bytes().unwrap();
        let mut other = Graph::empty();
        other.replace(Graph::from_bytes(&saved).unwrap()).unwrap();
        assert_eq!(other.bytes().unwrap(), saved);
        let mut plugin = PatchPlugin {
            desc: descriptor(),
            graph: Arc::new(Mutex::new(other)),
            latency: 32,
        };
        plugin.param_fx(0, 0, 0.25);
        let mut g = plugin.graph.lock().unwrap();
        warm(&mut g);
        impulse(&mut g, 32, 1.25);
    }
}
