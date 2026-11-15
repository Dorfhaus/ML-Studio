
# Score Editor engraving engine

ML Studio uses the Verovio 6.3.0 CLI as a separate, local engraving process. It
sends generated MEI and receives SVG. The existing ML Studio project, editing
history and audio engine own the notes and playback. resvg rasterizes the SVG.

Build the CLI from [Verovio revision 425dd7b](https://github.com/rism-digital/verovio/tree/425dd7b6ba26edcecbe345c10b15586d095743d9)
with CMake on the target platform. Use the source's cmake directory as the
CMake source directory. A native Windows x64 build can use the Visual Studio
2022 x64 generator with CMAKE_MSVC_RUNTIME_LIBRARY set to MultiThreaded.
Keep NO_EDIT_SUPPORT off for this revision. The CLI was built with
NO_HUMDRUM_SUPPORT, NO_ABC_SUPPORT, NO_PAE_SUPPORT and NO_GABC_SUPPORT enabled.

Build Release, then install it into a temporary prefix using CMake. Run
python packaging/score/bundle.py SOURCE INSTALL OUTPUT.zip and set
LEMON_SCORE_BUNDLE to the resulting target-specific ZIP before building
ML Studio. Each build must use the bundle matching its platform and CPU.
An externally built CLI can be used for testing through LEMON_VEROVIO_DIR,
pointing at a folder containing verovio (verovio.exe on Windows) and data/.

Verovio source and licenses are linked in the top-level third-party notices.
For distribution keep a corresponding source copy of the exact Verovio
revision alongside the ML Studio release and preserve the bundle notices.
