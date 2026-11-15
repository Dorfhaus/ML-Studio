# Mix Assistant genre starting points

Research checked 15 November 2026. The native assistant uses the existing GPL stock processors and MIT ebur128 meter. The built-in genre choices work offline and independently of the optional mix reference pack.

## Sources and design decisions

- Pop favors lead vocal consistency, while Jazz and Classical favor dynamic contrast. Hip-hop and Electronic/Dance emphasize kick and bass. Rock and Metal preserve drum attack in dense arrangements. Genre never changes routing.
- [Waves: Sean Divine's R&B vocal mix](https://www.waves.com/mixing-smooth-female-vocals-studiorack) and [Kori Anders on beats and vocals](https://www.waves.com/mixing-hard-beats-soft-vocals-kori-anders): R&B can combine controlled vocals, low-end weight and ambience. The authored R&B profile favors vocals/bass and slightly increases optional ambience.
- Country keeps a small lead-vocal priority. Acoustic/Folk, Jazz and Classical use progressively lighter control and EQ.
- [Shure: recording and mixing vocals](https://www.shure.com/en-EU/insights/how-to-record-and-mix-vocals) and [Ableton's compressor reference](https://www.ableton.com/en/manual/live-audio-effect-reference/#compressor): threshold must follow the input, and attack/release change the envelope. Recipes derive threshold from measured RMS, use bounded timing and no automatic makeup, then render the real processor to check the result.

These sources support tendencies, not universal numerical presets. The gates, ratios, offsets and approximate gain-reduction goals in `genre.rs` are deliberately conservative product tuning. They are not measured genre distributions, listening-validated mastering targets, or a claim that a label identifies the song's ideal sound. Subgenres vary. Balanced remains available and explicit effect choices override suggestions.

## Implementation and limits

Genre changes affect dynamics eligibility, target gain reduction, ratios, drum attack preservation, relative source balance, the amount of corrective EQ and optional wet levels. Tonal cuts still require measurement evidence. Genre never forces a compressor onto a steady signal or disables the full-range safety check. Warmth and space remain opt-in. Existing instances, excluded buses, routing and project metadata are preserved.

The profile remembers the last genre. Choosing or cancelling a genre does not change the project; changing genre invalidates the audition until Update Preview. The final effect count and effect-review reasons distinguish no automatic suggestion, an existing processor, a user exclusion and processing discarded during safety review. Apply remains one undoable operation.

Calibration now carries the original strength-scaled reduction target with each recipe. Previously the retry used a fixed 2.2 dB source / 1.2 dB bus target regardless of strength. That would override a deliberately gentle genre setting.

The separate open-source [Keel](https://github.com/fcarvajalbrown/Keel) was considered: its repository describes a Python stem/mix/master workflow and AGPL plus additional licensing. Replacing the existing graph-preserving native preview/apply engine with a separate renderer would not improve this targeted feature. No external code was copied.

Synthetic regressions check automatic processing of bare tracks, quieter genre choices, strength through calibration, preview invalidation, Levels only, cancellation, Undo and remembered choice. Musical results and uncommon routing still require listening on real projects; a profile is a starting point, not an automatic guarantee.
