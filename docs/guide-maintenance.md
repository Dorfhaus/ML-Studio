# User Guide maintenance

The bundled source is `docs/user-guide.md`. The app reader and Helper share this build's content. `help::helper_text` expands the runtime main-menu references before the Helper starts; raw Markdown has placeholders for these entries. Never install a newer loose guide and assume an older executable will retain it.

## Current audit: 14 November 2026

The corpus expanded from 197 to 444 articles across the existing 13 chapters: 33 additional authored workflow articles and 214 generated reference articles. Earlier workflows remain. This is a substantial coverage expansion, not proof that every possible user question or third-party plugin is documented.

Reviewed workflows and primary evidence:

- Mixer selection, grouped gain, solo paths, route modifiers, insertion, locks and contextual availability: `crates/os-app/src/mixer.rs` and `views/mixer.rs`. The missing multi-selection answer is Ctrl-click strip bodies; Shift-click range selection and Ctrl-toggle-off must not be invented.
- Channel Strip assignment, independent bulk instances, free-slot/lock prerequisites, native processing replacement and paging: `rack_mixer.rs`, `docs/channel-strip-mixer.txt`, existing detailed fill/SSL articles.
- Audio destination picker and shared sample routing: `docs/mixer-routing-picker.md` and existing sampler routing workflow. Freeze/bounce/sidechain differences: `docs/track-freeze-and-bounce.md`, `docs/channel-rack-pitch-and-bounce.txt`, `docs/quick-sidechain.md`.
- Headphone correction and idle plugin behavior: `docs/headphone-correction.md`, `docs/smart-disable.md`. Do not confuse physical-output correction with rendered audio processing.
- Keyboard/mouse contexts, inheritance, sequences and finite scripts: `input_bindings.rs`, `shortcuts.rs`, `spec/ui/input-bindings.md`. Hover prerequisites verified against `app.rs::update_menu_hover` and its current tests.
- Piano Roll selections/types and audio-level links: `views/piano.rs`, `piano_note_mode.rs`, `peak_link.rs`. Plugin picker contexts/previews: `docs/plugin-picker.txt`.
- Wave modulation, workshop drafts/banks and recorded playback: current `plugins/lemon-wave/MODULATION.md`, `WAVETABLE-WORKSHOP.md`, `SOUND-LIBRARY.md`. Historical README stages must not override current controls; personal recording libraries are edition-dependent.
- Plasma editor, synthesis, routing, effects, links, phrases and preset import: pinned `plugins/ml-plasma/vendor/gridline/docs/plugins/plasma.md` plus the ML adapter README. Upstream Gridline host-level missing-Serum substitution and FL export were deliberately not presented as ML Studio features.
- Project formats, collaboration and Master Assistant controls: `docs/lmn-projects.md`, `docs/live-collaboration.md`, `docs/master-assistant-controls.txt` and existing guide workflows.
- Existing recording, arrangement, automation, MIDI, exports, Browser, theme and troubleshooting chapters retained and sampled in the retrieval acceptance questions. Existing Mixer menu prose was corrected where it advertised unimplemented placeholders.

## Updating and checking

1. Verify the actual handler/engine behavior, including selection, modifiers, prerequisites, cancellation, save/reopen and limitations. Update the authored workflow, not just a menu name. Avoid inferred FL Studio behavior.
2. Run `python packaging/expand-guide-reference.py`. It preserves authored articles before its marker and refreshes source-backed setting defaults/choices, literal control hints/menu labels and declared parameter names. Generated source locations and exclusions are in `docs/guide-source-audit.json`.
3. Run the generator with `--check`; run Helper `test_guide.py` with `ML_HELPER_GUIDE` set. Add a practical question for newly documented behavior. Run the native guide-catalog and Helper-export tests when updating bundled content/export behavior.
4. Test representative answers with the installed local model, including normal conversation and multi-step prerequisites. Retrieval passing alone does not prove the answer is correct. No private project, microphone or GUI control is needed for text-only checks.
5. Install the rebuilt guide in Development and matching Python modules. Preserve the profile, model ownership, bounded chat history and shutdown. Public publication remains a separate authorized workflow.

## Coverage limits to keep visible

The source catalog reports 543 nonliteral/dynamic hint calls, 200 dynamic menu labels and 16 historical setting comments requiring behavioral review. These are parser exclusions, not counts of confirmed missing features: many are dynamic device/preset names or covered in authored articles. Static labels do not establish every parameter's DSP meaning. The generator does not turn stale developer comments into instructions. Native main-menu entries are exported from the working registry rather than guessed from Rust text.

Dynamic user menus, third-party plugin controls, custom scripts, external hardware and every combination of settings cannot be certified by a label scan. The Helper must admit a missing answer. Keep expanding reviewed workflows as omissions are found; do not claim guaranteed universal coverage based on the article count.
