# audio_bank — Bank manifest contract

Scope: the versioned manifest schema of a built runtime sound bank and its companion contract notes.
Consumers: `tools/audio/` producers, the vehicle audio Rust crates, and the banks under `game/sounds/banks/`.
Rules: the executable contract is `manifest.schema.json`. Adding an optional field keeps the version; making a field required or changing meaning mints a new version with a coordinated cutover.

## Sampling format

Every WAV referenced by a runtime bank manifest must be exactly:

- RIFF/WAVE container.
- PCM, uncompressed.
- Mono (1 channel).
- Signed 16-bit little-endian samples.
- 44100 Hz sample rate.
- Byte count, frame count and SHA-256 must match the manifest entry exactly.
  The loader rejects any mismatch (`read_wav_mono16` in
  `game/crates/vehicle-audio-engine/src/bank.rs`).

## Engine-bank schema-1 invariants

- `loop_start_frame = 0` and `loop_end_frame_exclusive = derived_frames`.
- `loop_crossfade_frames` greater than zero and at most one quarter of the loop
  length.
- `reference_revolutions_per_minute = integer_cycle_count * 44100 / derived_frames * 120`;
  every loop must contain an integer number of engine cycles so loop wraps stay
  phase-coherent across zones.
- `coverage` spans 4500 to 18000 revolutions per minute; loop zones are strictly
  ascending; transition windows use `smoothstep` and `equal_power` and never
  overlap. Sequential banks match each loop's active coverage to its neighboring
  transition windows.
- Powered loops may provide the optional `presence_curve` array, with ordered
  `revolutions_per_minute` and `relative_weight` points. All powered loops must
  provide curves together. Weights are finite within 0 to 1, curves start and
  end at their declared active coverage, and interior coverage boundaries have
  zero weight so entry and exit remain continuous. Combined curves must cover
  the entire engine range without silent boundaries. The sampler interpolates
  each curve with smoothstep and normalizes the squared weight sum to one.
  Sequential transition metadata remains ordered for compatibility, but curves
  determine powered weights and active coverage. Coast loops remain sequential
  and must not carry presence curves. This optional schema-1 extension requires
  a rebuilt runtime; older loaders reject the new active coverage.
- Events are mono16 as well and keep their recorded preparation recipes and
  SHA-256 values; engine events are triggered by the sampler, not by the shared
  commons bank.

- Schema-1 sampler manifests may provide `transmission_whine` as a separate
  continuous recording. It declares source and derived hashes, frame count,
  full-file loop bounds, preparation crossfade frames, authored mesh frequency
  in hertz, calibrated gain, and minimum and maximum playback rates. Engine
  cycle and revolutions coverage invariants do not apply to this recording.
  It obeys the same mono16 44100 hertz WAV contract. The maximum playback rate
  is at most 4.0. Invalid declared assets fail initialization; an absent entry
  preserves the legacy whine. Prepared loop crossfading is baked into the WAV.

- `limiter_cut_window` is an additional sampler trigger value. Its group uses
  one fixed-pitch limiter variant and zero timing offset. The decoded recording
  is a gated looping layer driven by finite physical cut intervals; event
  cooldown and retrigger settings do not apply. `limiter_entry_edge` retains
  its historical event behavior.
