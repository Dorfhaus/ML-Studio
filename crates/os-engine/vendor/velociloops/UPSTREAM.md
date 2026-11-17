# VelociLoops

Source: https://github.com/kunitoki/VelociLoops
Revision: f77609ec70b7bb6db362ba97030bd9703539fa01
Licence: Unlicense (LICENSE retained). The upstream fft8g implementation is retained with its source comments.

Used as a statically linked, cross-platform C++17 REX/RX2/RCY decoder. No external SDK, downloaded runtime or DLL is required. ML Studio's bridge limits files to 64 MiB and rendered loops to 16,777,216 stereo frames. The vendored VLHeapArray allocation paths are additionally capped at 33,554,432 elements, including legacy decoding. The bridge uses the implementation's PPQ scale of 15360 per quarter note; the upstream public-header wording incorrectly describes 15360 per bar.

The bridge catches native exceptions, owns handles with RAII, reconstructs the loop using decoded slices at their musical positions, and preserves tempo and slice positions. The UI imports audio and slice metadata; it does not write REX files. Tests generate their own synthetic files with the upstream writer.
