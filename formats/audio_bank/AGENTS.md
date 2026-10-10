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
  ascending; transition windows use `smoothstep` and `equal_power`, never
  overlap, and each loop's active coverage must match its neighboring
  transition windows.
- Events are mono16 as well and keep their recorded preparation recipes and
  SHA-256 values; engine events are triggered by the sampler, not by the shared
  commons bank.

- `limiter_cut_window` is an additional sampler trigger value. Its group uses
  one fixed-pitch limiter variant and zero timing offset. The decoded recording
  is a gated looping layer driven by finite physical cut intervals; event
  cooldown and retrigger settings do not apply. `limiter_entry_edge` retains
  its historical event behavior.
