# ML Drums

An ML Studio stock instrument adapted from [Autokit v0.5.3 by REXIST](https://github.com/Hornfisk/autokit/tree/76629e4c4832d46fab8f6f82bba72f3904d58511), GPL-3.0-or-later. The kit, sequencer, effects, sample analysis and editor derive from Autokit. The original synthesis and licensing notice for the drum sounds is in [resources/default_samples/README.md](resources/default_samples/README.md). The GPL text is in the workspace LICENSE.

The DAW links the CLAP factory directly on every target. The separate identity is `de.michaellane.ml-drums`. Project state captures live kit edits and the pattern bank. Factory sounds are identified by a stable key. Custom pad audio travels in the project as lossless mono PCM with its sample rate. Scanning an optional user library leaves the active kit in place. Presets, library configuration and MIDI exports use the active ML Studio profile.

On Linux the editor uses X11 and OpenGL. Native builds require pkg-config and the development packages for GL, X11, X11-XCB and XCB. The Windows cross-builder needs the matching x86-64 link libraries and pkg-config entries in its Debian sysroot. Playback and offline export require no display or open editor.
