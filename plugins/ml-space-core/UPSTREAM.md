# Native stock Convolver and Vocoder

FFTConvolver: https://github.com/HiFi-LoFi/FFTConvolver, revision f2cdeb04c42141d2caec19ca4f137398b2a76b85, MIT (HiFi-LoFi). Minimal engine vendored with original notices, including AudioFFT's embedded Ooura FFT attribution. ML adds allocation-free clearHistory methods retaining IR plans. Rust uses two-stage 128/2048 partitioning without algorithmic latency. Preparation, resampling, file IO and replacement destruction run outside audio processing.

Worker ownership: pending belongs to the worker until atomic exchange; active/fading engines belong only to audio; retired belongs to the worker. A new swap waits for retirement to finish. Closing joins the worker before reclaiming shared slots.

mda effects: https://github.com/getdunne/mda-effects, revision 66d0536c142af567bb2fe4f547aa61379faaca00. Rust adaptation of mdaVocoder.cpp filter bank/envelopes. MIT, Paul Kellett and Shane Dunne; both licence texts retained. No VST SDK or original mutex-bearing shell included. ML adds stereo carrier sidechain, PolyBLEP internal saw, frequency bounds, sample-rate-scaled envelopes, smoothing and native editor.

WAV validation is adapted from the existing Lemon Wave decoder. The saved IR embeds original-rate float32 samples as base64. Offline windowed-sinc resampling compensates discrete impulse gain. Mono IR applies to both channels; stereo IR is independent L/R, not four-channel true stereo. Maximum 10 seconds and 16 MiB WAV. Small Room is original synthetic data; no third-party IRs bundled.

Shared stock controls, visibility, host frame and native-state undo apply. External carrier accepts polyphonic synth audio; internal carrier is one note, not a MIDI synth. Both report zero latency. Convolver pre-delay is intentional effect timing. Windows verification does not establish Mac/Linux readiness.
