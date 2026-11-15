// SPDX-License-Identifier: GPL-3.0-or-later
//! Mixer inventory and bus ordering. The inventory only reads project data.
use super::*;
use os_model::{ChannelKind, ClipSource};

pub(super) fn inventory(state: &crate::app::State, project: &Project) -> Vec<Track> {
    let mut unique = std::collections::HashSet::new();
    let mut placed = Vec::new();
    let mut playlist_roles: BTreeMap<u16, Vec<Role>> = BTreeMap::new();
    if let Some(arrangement) = project.arrangement() {
        for clip in &arrangement.clips {
            let lane = arrangement.tracks.get(usize::from(clip.track));
            if clip.muted || !lane.is_none_or(|row| row.enabled) {
                continue;
            }
            if unique.insert(clip.source) {
                placed.push(clip.clone());
            }
            if let ClipSource::Channel(id) = clip.source {
                for label in [
                    clip.name.as_deref(),
                    lane.and_then(|row| row.name.as_deref()),
                ]
                .into_iter()
                .flatten()
                {
                    let role = Role::guess(label, false);
                    if role != Role::Unknown && !role.is_bus() {
                        let roles = playlist_roles.entry(id).or_default();
                        if !roles.contains(&role) {
                            roles.push(role);
                        }
                    }
                }
            }
        }
    }
    let placed_channels = project.clip_channels(&placed);
    let channels: Vec<_> = project
        .channels
        .iter()
        .filter(|ch| ch.enabled && !matches!(ch.kind, ChannelKind::Automation | ChannelKind::Layer))
        .collect();
    let sources: BTreeSet<_> = channels
        .iter()
        .filter(|ch| placed_channels.contains(&ch.index))
        .map(|ch| ch.mixer_track)
        .collect();
    let assigned: BTreeSet<_> = channels.iter().map(|ch| ch.mixer_track).collect();
    let graph = edges(project);
    let reachable = reach_master(&graph);
    let max = mixer::max_insert(project);
    let mut connected = sources.clone();
    for track in &project.mixer {
        if assigned.contains(&track.index)
            || track.name.as_ref().is_some_and(|s| !s.trim().is_empty())
            || track.slots.iter().any(Option::is_some)
        {
            connected.insert(track.index);
        }
    }
    let mut receivers = BTreeSet::new();
    let mut pending: Vec<_> = connected.iter().copied().collect();
    while let Some(id) = pending.pop() {
        for &destination in graph.get(&id).into_iter().flatten() {
            receivers.insert(destination);
            if connected.insert(destination) {
                pending.push(destination);
            }
        }
    }
    let mut rows = Vec::new();
    for mixer_track in project.mixer.iter().filter(|track| {
        track.index != 0 && track.index <= max && !track.clip_fx && connected.contains(&track.index)
    }) {
        let id = mixer_track.index;
        let name = mixer::name(state, id);
        let is_bus = receivers.contains(&id) && !sources.contains(&id);
        let mut role = Role::guess(&name, is_bus);
        if role == Role::Unknown {
            let mut suggestions = channels
                .iter()
                .filter(|ch| ch.mixer_track == id)
                .filter_map(|ch| {
                    if sources.contains(&id) && !placed_channels.contains(&ch.index) {
                        return None;
                    }
                    [
                        ch.name.as_deref(),
                        ch.sample_path
                            .as_deref()
                            .and_then(|s| s.rsplit(['/', '\\']).next()),
                        ch.plugin.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    .map(|label| Role::guess(label, false))
                    .find(|role| *role != Role::Unknown)
                    .or_else(|| {
                        match playlist_roles.get(&ch.index).map(Vec::as_slice) {
                            Some([role]) => Some(*role),
                            _ => None,
                        }
                    })
                });
            if let Some(first) = suggestions.next() {
                if suggestions.all(|other| other == first) {
                    role = first;
                }
            }
        }
        let protected = if !mixer_track.enabled {
            Some("Muted")
        } else if !reachable.contains(&id) {
            Some("Outside Master mix")
        } else if mixer::locked(state, id) {
            Some("Locked fader")
        } else if fader_controlled(project, id) {
            Some("Controlled fader")
        } else {
            None
        };
        let downstream = graph.get(&id).cloned().unwrap_or_default();
        let mut route = Vec::new();
        let mut seen = BTreeSet::from([id]);
        let mut destinations = downstream.as_slice();
        while !destinations.is_empty() {
            route.push(
                destinations
                    .iter()
                    .map(|&to| {
                        if to == 0 {
                            "Master".into()
                        } else {
                            mixer::name(state, to)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" + "),
            );
            let [next] = destinations else { break };
            if *next == 0 || !seen.insert(*next) {
                break;
            }
            destinations = graph.get(next).map(Vec::as_slice).unwrap_or_default();
        }
        rows.push(Track {
            id,
            name,
            role,
            level: Level::default(),
            protected,
            include: sources.contains(&id)
                && !role.is_bus()
                && role != Role::Return
                && protected.is_none(),
            downstream,
            route: route.join(" > "),
            confirmed: false,
            fx_choices: BTreeMap::new(),
        });
    }
    rows
}

/// Match downstream processing to the audio from earlier opted-in buses, including
/// paths through excluded buses. Parallel buses share a measurement render.
pub(super) fn bus_layers(
    tracks: &[Track],
    form: &Form,
) -> Result<VecDeque<Vec<u16>>, &'static str> {
    let graph: BTreeMap<_, _> = form
        .rows
        .iter()
        .map(|track| (track.id, &track.downstream))
        .collect();
    let selected: BTreeSet<_> = tracks
        .iter()
        .filter(|track| {
            track.role.is_bus() && processing_eligible(track, &form.selected, form.selection_only)
        })
        .map(|track| track.id)
        .collect();
    let mut requires: BTreeMap<_, _> = selected.iter().map(|&id| (id, BTreeSet::new())).collect();
    for &id in &selected {
        let mut visits = BTreeSet::new();
        let mut pending = vec![id];
        while let Some(current) = pending.pop() {
            for &destination in graph.get(&current).into_iter().flat_map(|v| v.iter()) {
                if visits.insert(destination) {
                    if let Some(upstream) = requires.get_mut(&destination) {
                        upstream.insert(id);
                    }
                    pending.push(destination);
                }
            }
        }
    }
    let mut layers = VecDeque::new();
    while !requires.is_empty() {
        let layer: Vec<_> = requires
            .iter()
            .filter(|(_, upstream)| upstream.is_empty())
            .map(|(&id, _)| id)
            .collect();
        if layer.is_empty() {
            return Err("Included buses have a circular route. Exclude a bus to continue.");
        }
        for id in &layer {
            requires.remove(id);
            for upstream in requires.values_mut() {
                upstream.remove(id);
            }
        }
        layers.push_back(layer);
    }
    Ok(layers)
}
