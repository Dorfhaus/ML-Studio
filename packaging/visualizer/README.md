# Build the optional Windows video renderer

ML Studio Development uses a separate executable to render AudioVisualizer scenes. The upstream source is AudioVisualizer v0.77, commit 18758d720c1d08cdd982352604e30e6260d146fe, licensed under AGPL-3.0-or-later. The patch and entry point in this directory are part of the corresponding source of the worker.

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

Place the verified worker beside the Development Profile folder in `ML Visualizer Video Tools`. Keep the upstream license and instructions for obtaining corresponding source there. Update the renderer size and SHA-256 in `video-tools.json` when rebuilding with a different binary. The Development executable verifies both values before use.

The FFmpeg 9.0.2 essentials archive is fetched from Gyan's GitHub release with consent at first export. The manifest pins the archive and the extracted executable individually. FFmpeg is a separate GPL executable. Its release page includes the license and links to the source. The built-in Visualizer workspace and its MP4 export are supported on Windows x64, including the public build. The upstream AudioVisualizer plug-in’s separate MP4 command still requires the Development renderer. macOS and Linux do not expose these MP4 commands.
