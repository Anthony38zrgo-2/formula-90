# Formula-90 Game Canon

This file is binding for every change under `game/`. It specializes the root
`D:\Formula90s\AGENTS.md` protocol for runtime audio assets.

## Sound bank canon

- The canonical V10 engine bank is `D:\Formula90s\game\sounds\banks\v10-v2-bank`
  (`game/sounds/banks/v10-v2-bank` in the repository).
- Every runtime sound bank lives under `game/sounds/banks/`. Never place runtime
  banks under `game/audio/`; that tree is historical and is not consumed anymore.
- The shared commons bank stays at `game/sounds/banks/commons` because it is not
  V10-specific. Do not move it into `v10-v2-bank` and do not rename its
  `bank_manifest.json`.
- Engine runtime assets declared by `v10-v2-bank/manifest.json` must sit next to
  that manifest inside `v10-v2-bank`. Raw recordings and derived runtime WAVs
  coexist in the same folder; raw file names (`engine_*.wav`, `backfire_*.wav`,
  `gearup.wav`, `geardn.wav`, `limiter.wav`) are sources, derived file names
  (`*_loop.wav`, `*_event.wav`, `backfire_burst_*.wav`) are runtime assets.
- The F1 2030 V10 vehicle profile must point at the canon through
  `audio.grand_prix_sampler.manifest = "sounds/banks/v10-v2-bank/manifest.json"`
  in `game/data/vehicles/f1_2030/f1_2030_v10_geometric.json`. The runtime
  resolves this path relative to the Godot project root.
- This folder carries a `.gdignore`: Godot must not import runtime bank WAVs.
  The Rust runtime loads them from the filesystem, so bank assets are never
  referenced through `res://` paths.

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

The engine bank uses manifest schema 1 and adds these invariants:

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

## Regeneration rules

- Rebuild the canon only with
  `python -m tools.audio.build_canonical_v10_engine_bank` from the repository
  root. Run `python -m tools.audio.build_canonical_v10_engine_bank --check` to
  verify that the shipped bank still matches its sources byte for byte, and
  `--audit` to inspect formats and measurements.
- Never hand-edit derived runtime WAVs, `manifest.json` or
  `source_inventory.json`. Change the raw source or the preparation tool and
  regenerate.
- The preparation tool is deterministic. Any intentional audio change must
  update the raw sources, the tool revision, the derived hashes and the tests in
  `tools/audio/tests/test_canonical_v10_engine_bank.py` in the same change.
