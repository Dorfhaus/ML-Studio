# Build the Windows x64 video renderer

ML Studio uses a separate executable to render AudioVisualizer scenes. The upstream source is AudioVisualizer v0.77, commit 18758d720c1d08cdd982352604e30e6260d146fe, licensed under AGPL-3.0-or-later. The patch and entry point in this directory are part of the corresponding source of the worker.

Open a Visual Studio 2022 developer shell with CMake and Git. Set `$packagingDirectory` to this directory and run the following commands from a new working folder.

```powershell
git clone https://github.com/mtyas/AudioVisualizer.git AudioVisualizer
git -C AudioVisualizer checkout 18758d720c1d08cdd982352604e30e6260d146fe
git -C AudioVisualizer apply --unidiff-zero "$packagingDirectory/offline-render.patch"
Copy-Item "$packagingDirectory/OfflineRender.cpp" AudioVisualizer/src/OfflineRender.cpp
cmake -S AudioVisualizer -B worker-build -G "Visual Studio 17 2022" -A x64
cmake --build worker-build --config Release --target MLVisualizerRender
```

The renderer is produced at `worker-build/MLVisualizerRender_artefacts/Release/MLVisualizerRender.exe`. It takes the scene JSON, stereo WAV, progress file, width, height, frame rate, tick tempo, and start beat, in that order. It writes raw bottom-up RGBA frames to its standard output. The application passes these frames directly to FFmpeg and supplies the same Master WAV for the audio track.

Set `LEMON_VISUALIZER_WORKER` to the built executable before building ML Studio on Windows x64. The renderer is embedded in the DAW and extracted on first use to the user's video tools directory. The app checks its pinned size and SHA-256 before running it.

For a public build, also set `LEMON_VISUALIZER_SOURCE` to an archive made by running `python archive-worker-source.py AudioVisualizer worker-build/_deps AudioVisualizer-worker-source.zip`. The release package includes this archive in the corresponding source download. Update the worker and source archive hashes in `video-tools.json` if rebuilding either artifact.

To build from the release source archive, run the same CMake command with the archived `AudioVisualizer` directory as the source and add `-DFETCHCONTENT_SOURCE_DIR_JUCE=<archive>/deps/juce -DFETCHCONTENT_SOURCE_DIR_CLAP-JUCE-EXTENSIONS=<archive>/deps/clap-juce-extensions -DFETCHCONTENT_SOURCE_DIR_TINYGLTF=<archive>/deps/tinygltf`. The archived worker source already has the patch applied.

The FFmpeg 9.0.2 essentials archive is fetched from Gyan's GitHub release with consent when first setting up Playlist video or Visualizer export. The manifest pins the archive and the extracted executable individually. FFmpeg is a separate GPL executable. Its release page includes the license and links to the source. The built-in Visualizer workspace and its MP4 export are supported on Windows x64, including the public build. The AudioVisualizer plug-in on Windows x64 also offers the encoder with its initial setup. Existing installations can add export tools from Export MP4. macOS and Linux do not expose these MP4 commands.

Playlist video shares the Windows Visualizer's FFmpeg installation. On macOS and Linux, Playlist video can use FFmpeg on PATH or set up its own copy using the architecture-specific source links and integrity pins in `reference-video-tools.json`. These packages are downloaded directly from their publishers.

Use a month-end FFmpeg-Builds release when refreshing the Linux pin. The upstream retention policy keeps month-end releases for two years and daily releases for 14 days. Refresh the manifest before the pinned month-end release expires.
