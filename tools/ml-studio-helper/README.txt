ML Studio Helper (Melli) - optional offline companion

Choose Help > ML Studio Helper (Melli), then Download and install. The Qwen3.5-0.8B UD-Q3_K_XL model is 492 MB, downloaded directly from Hugging Face.
The separate runtime adds about 175-245 MB depending on platform; allow 2 GB free space.
A download panel shows real progress, downloaded MB and Cancel download.
Run in background lets you work; reopen this Help entry to view progress again. Windows x64, Linux x64
(glibc 2.36 or later with a desktop/X11 or Wayland environment), and macOS 14+
(Apple Silicon or Intel) are supported. Python, Qt and llama.cpp are included.
Linux requires the host desktop libraries; Debian/Ubuntu packages include
libxcb-cursor0, libxkbcommon-x11-0, libegl1 and libopengl0.
GPU acceleration uses Vulkan on Windows/Linux and Metal on Mac; CPU fallback
is available. Performance depends on available memory, drivers and hardware.

Click Melli to load the model and chat about the DAW, music or everyday topics.
Her personality is witty, warm and confident. She cannot browse, hear a project,
operate the DAW, or promise accurate answers to every question. Exact DAW
answers use this build's guide. General conversation is model-generated.

Minus collapses the bubble while retaining the model. X closes Melli and unloads
it. DAW exit/crash and Settings > Visibility hiding the companion also stop it.
Animation is idle/blink, thinking and reply feedback, not speech or lip sync.
The overlay follows the main window and hides when it is minimized/unfocused;
on Wayland, the compositor may choose window placement instead.

The ... menu offers New chat, history, Guide source, Memory, Remove downloaded
Helper files, and Close. Removing files asks for confirmation in ML Studio and
preserves saved notes. Clear notes first if you want to remove those too.

Chats are not saved. Recent context is capped at 12 messages / 12000 characters;
each message is capped at 2400 and input at 700. New chat/exit clears recent
context, but useful self-disclosed notes persist across sessions. Saved memory
is one replace-in-place file capped at 16 KiB / 24 short notes, with a bounded
temporary file during writes. Lower-priority/older notes are displaced when full; this is not
perfect recall. Memory can be viewed, edited, disabled or forgotten in ... >
Memory. Disabling stops both learning and use without deleting existing notes.
Do not rely on automatic memory filtering to protect secrets.

Logs rotate at 64 KiB plus two backups; model output is discarded. Packages
replace the previous runtime; one shared model is reused across DAW updates.
Cancelled downloads use fixed temporary paths and are cleaned up on retry.
Projects, presets and user recordings are never package-cleanup targets.
Files live in the ML Studio user folder's Melli directory by default. The
existing Windows Development installation retains its E: product location.
ML_HELPER_INSTALL_ROOT can override storage for managed installations.

Artwork is the approved generated adult cyberpunk companion. Four poses are
rendered from assets/companion-polished.png; Barlow font is bundled under OFL.
License files for CPython, Qt/PySide/Shiboken, llama.cpp and Qwen are included.

Guide coverage and answer grounding (14 November 2026)
The application exports its embedded guide plus the live main-menu registry at
Helper launch. Rebuild/install Development for guide changes; copying only the
loose Markdown is temporary because launch replaces it with the build's guide.

The index includes every article chunk instead of truncating articles at 3800
characters. Inflected search terms share aliases. Retrieval remains bounded to
three chunks / 7600 characters. Explicit editor questions consult the guide.
For guide answers the model selects numbered passages; the app returns the actual
text and includes surrounding workflow prerequisites for how-to questions. Model
paraphrases are never displayed as verified instructions. Invalid/empty choices
produce an honest missing-answer message. Exact guide passages remain English;
ordinary conversation/theory still uses the conversational model. This prevents
invented additions, not selecting an irrelevant or outdated documented passage.

Run packaging/expand-guide-reference.py after changing settings, control hints,
menus or declared plugin parameters; --check detects stale generated references.
Read docs/guide-maintenance.md for review scope and remaining coverage limitations.
With ML_HELPER_GUIDE pointing at the exported guide and TEMP on E:, run test_guide.py.
It checks 45 practical questions, full-article retrieval, links, prerequisite
retention and rejection of model-written extra advice. test_helper.py retains the
bounded-memory and real-model shutdown checks. Models are unchanged; no training.
