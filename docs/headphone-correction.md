# Headphone monitoring correction

The master strip's headphone icon opens a searchable catalog. Click a model to activate its parametric correction. AutoEq profiles download once; the bundled ADAM Audio H200 profile works offline on first use. Match the exact model, impedance, pads and ANC variant where listed. The PHONES rocker toggles correction; the compact mixer places the icon and rocker below M.

Fresh installs start OFF. The chosen model, validated EQ and on/off choice are app preferences, isolated between personal and development profiles. Downloads are cached in the user folder under Headphone profiles. The selected profile is also saved in settings.txt and works without a connection. Download failure leaves the current correction intact and displays status inside the selector. Arrow keys/Enter, wheel and Previous/Next browse the catalog. Esc closes it.

DSP runs once after summing playback, live notes and preview streams at the physical output device. ASIO, WASAPI and cpal/Core Audio/Linux outputs supply their actual sample rate. Engine exports, freeze, bounce and mixer recordings never use this device processor, so headphone compensation is never printed into files. It is independent of master slot FX, All FX, STRIP bypass and the miniature EQ. It affects ML Studio's output only, not other programs. Bypass it when listening through speakers or when another correction system is active.

Up to twelve validated parametric peak/low-shelf/high-shelf filters, AutoEq's negative preamp for headroom, fixed-size stereo filter state and 20ms transitions. No network, disk access, allocation or blocking lock in the audio callback. Bypass becomes bit exact after the transition. Filters above the device's usable range are limited to 45% of sample rate.

The catalog includes 6,033 AutoEq entries plus the bundled H200. These profiles target a listening balance for measured headphone models. Fit and individual variation remain.

Data source: https://github.com/jaakkopasanen/AutoEq at revision 7ae0f56d53074872b028649617a22bbb4232feb7, results/README.md. Each row retains its measurement provider; per-model ParametricEQ.txt comes from that pinned revision over HTTPS. See crates/os-app/assets/headphones/AutoEq-LICENSE.txt (MIT, Jaakko Pasanen). License also accessible from Credits in the selector. No executable download or external runtime.

The bundled H200 uses oratory1990's Harman Target preset from [OPRA](https://github.com/opra-project/OPRA), pinned to revision `0b88ecd4e2bef7cf69fd5d50f1d06fb586c10865`. The standalone EQ text records its exact source and [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license. All nine filter values are retained. The preamp is reduced from −2 to −6 dB to leave headroom for the filters' combined response.

Verification: focused synthetic filter-response/bypass tests across 44.1/48/96/192kHz, physical output summing and preference restoration, headless master-control hit testing at 435/660px mixer heights in both layouts, searchable selector, a real HTTPS profile download and bit-identical non-silent offline exports with correction enabled/disabled. User visual/audio review and Mac/Linux runtime testing remain separate.
