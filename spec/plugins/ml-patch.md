# ML Patch

ML Patch is a host-native stereo effect container, offered in the ordinary Mixer
effect picker. Its document is stored in the native slot state under
`plugin.patch`, version 1. Node IDs survive layout changes and deletion; macro
indices 0–7 never move. Missing child plugins retain opaque state and silence only
their branch. Loading a malformed document or a plugin rejecting state preserves
the previous graph. The graph itself is not exported as a standalone VST.

## Reuse decision

Reviewed the staged Carla source (`CarlaHost.h`, `CarlaEngineGraph.hpp`, and the
Python/Qt patchcanvas), GPL-2.0-or-later. Carla supplies a useful complete host and
patchbay, but ML Studio's present Carla adapter is an optional Windows x86 VST2
bridge. Embedding a second full host and Python/Qt frontend would duplicate the
existing format loaders, stock-plugin registry, crash containment, editor frames,
and platform setup. Consequently ML Patch reuses ML Studio's existing GPL-3.0-or-
later format-neutral host, native UI, project history, presets and native plugin
windows. No Carla code was copied and Carla is not a new runtime dependency.
Existing upstream attribution in those components is retained.

The workflow reference is Image-Line's public Patcher manual:
https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/plugins/Patcher.htm
This implements the effect-map workflow, not Patcher preset compatibility or full
feature parity. Instruments/MIDI, custom control surfaces, nested patches and
feedback are not supported. Each macro controls one exposed child parameter.

## Processing

Topologically ordered stereo main/sidechain edges, 32 effects and 96 cables.
Parallel inputs sum without automatic normalization. Compensation is the maximum
upstream arrival time at each main/sidechain junction. The whole patch reports its
output latency; bypassed nodes retain delay. Dynamic child latency changes mute
the graph until UI-thread re-planning completes. Graph edits fade in over 128
samples. Combined latency is limited to 262,144 samples; compensation edges to
2,097,152 stereo frames. These limits reject excessive graphs before installation.

The audio callback uses prepared 1024-frame scratch blocks, fixed graph topology,
preallocated delay lines, bounded automation events and `try_lock`. File access,
plugin instantiation, state serialization and delay allocation happen on the UI
thread. A busy/missing child produces silence, never an unaligned dry fallback.

## UI and persistence

Cable click or drag, move nodes, cancel with Escape, remove, bypass, gain trim,
searchable installed-effect list, native child editor, exposed parameter controls,
eight macro assignments, project Undo/Redo, normal preset save/load and project
reopen. The existing Workspace Visibility system hides the menu entry only;
existing saved graphs continue to run and can be reopened from their slots.

The user performs native-window and hardware-audio testing. Automated checks use
synthetic signals and the app's offscreen renderer, not GUI automation.
