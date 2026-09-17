# EXH — Geometry-driven V10 exhaust voicing

Date: 2026-09-17 · Branch: `main-clean` · HEAD: `2ad2b115e2ece1af4de2f4a78b77951cbf0c7e02`
Source of scope: `instrucciones.md` (replaces previous handoff). Launched vehicle:
`f1_2030_v10` via `run_f1_94.ps1` → `f1_2030_v10_geometric.json` → GF509 continuous
source, manifest `audio/v10_f2002_experimental/manifest.json`, 44100 Hz.

## Objetivo

More perceived low-mid body (~150-600 Hz vs 1-4 kHz) in the simulated V10 without a
muddy drone, ringing, lost transients or a broken V10 identity. Approach is physical
exhaust geometry. Preserve pistons, bore, stroke, rods, compression, firing order,
RPM range, torque curve and vehicle dynamics. No pitch shift, fabricated subharmonics,
fake misfires, added oscillators, global EQ or lowered gas temperature.

## Constraints (from handoff)

- Firing repetition at 12000 RPM is 1000 Hz; geometry reshapes balance, not the order.
- `EngineConfig` has primary lengths only (no diameter/area). Near-port volume is
  `bore_area * header_length * 0.1` (surrogate coupling): length is not acoustically
  isolated from gas dynamics.
- Collector modal frequencies are hard-coded reduced-order acoustics, with bank offsets.
- Candidate geometry must be explicitly selected, reversible, defaults preserved.
- Runtime transport of geometry is not implemented; resolve before touching
  vehicle-specific settings (the geometric profile is the launched one).

## Plan

- EXH-01 Reproduce reference: fixed-RPM (3000/6000/9000/12000) + 4500→15000→6500 sweep,
  procedural (scene) and hybrid, 44100 Hz, seed 4035969040, identical telemetry.
- EXH-02 Primary length candidate: uniform scale of current length array, explicit flag;
  inspect acoustic output and runner pressure/temperature/mass flow.
- EXH-03 Only if needed: minimal physical collector model (effective length, volume,
  losses, outlet boundary) with derived resonances/time constants.
- EXH-04 Route selected candidate explicitly; keep sample assets/weights fixed.

## Acceptance

- Config dimensions finite/positive/bounded; defaults byte-compatible.
- No unstable feedback, clipping, DC growth or unbounded thermal state.
- Relevant tests pass; render path allocates only at construction.
- Loudness-matched listening artifacts (raw outputs retained) + compact spectral
  evidence; human listening gate is the final word.

## Evidence log

### EXH-01 reference (2026-09-17)

- Render tool built from this HEAD in isolated `CARGO_TARGET_DIR` (temp):
  `cargo build --release -p v10-engine-synth --bin v10_render` → 21.7 s.
- Capture script: `reports/audio-v10/exh01/capture_reference.ps1` (ignored).
- Outputs: `reports/audio-v10/exh01/reference/*.wav` + stems + metadata
  (`source_dirty:false`, `source_fingerprint: fnv1a64:219a5da771b68610`).
- Scene gains applied: under_seat 0.475, mount_monocoque 0.59375, engine_cover 0.4788,
  metal 0.855, airbox 0.75.
- Note: `steady_3000_proc` peaks at 1.105 (PCM16 clip) in the current baseline;
  recorded as pre-existing behavior, not introduced here.

### EXH-02 primary length (scale candidates 1.30/1.40/1.50)

- `EngineConfig.header_length_scale` (default 1.0) applied uniformly to
  `header_lengths_m`; consumed by both the runner waveguide and the near-port
  `ExhaustRunner` volume (`bore_area * L_eff * 0.1`), so length is not acoustically
  isolated from gas dynamics. Reported in render metadata.
- Scale 1.0 is bit-identical to the EXH-01 reference (3 files SHA-256 match).
- 6000 RPM, engine `master` stem: low-mid (150-600 Hz) +4.5 dB at scale 1.5
  (-26.39 → -21.85), 1k-4k -0.9 dB. Scene mix: 150-600 unchanged (≤0.5 dB),
  500 Hz band +3.6 dB, 1k-4k up to -3.1 dB at 12000. Balance (150-600 vs 1k-4k
  on scene_mix) at 12000 improves +3.5 dB (3.80 → 7.31); at 9000 it worsens
  (-1.3 dB) because scale moves the runner resonance off the 750 Hz firing tone.
- Gas coupling at 6000: mass-flow RMS +20% (0.0812 → 0.0978 kg/s), runner pressure
  -0.11 bar, runner temperature -12 K. Surrogate volume coupling documented.

### EXH-03 physical collector geometry (derived model)

- `CollectorGeometry {volume_l, outlet_length_m, outlet_diameter_m, gas_temperature_k,
  loss_fraction_per_cycle, mode_coupling}` (default `None` = legacy modal bank,
  bit-identical). `Collector::from_geometry` derives the Helmholtz fundamental of
  chamber + tailpipe mass (unflanged end correction 0.61·r), the odd quarter-wave
  tailpipe harmonics, mode decay `tau = 2/(loss·f)` and the chamber leak from the
  Helmholtz decay. Declared as design assumptions, not measured hardware.
- The derived collector works inside its stem (candidate A: +9.4 dB @250 Hz,
  +9.4 @400, -11.3 @500 on `collector_a`) but the scene mix does not follow:
  150-600 delta ≤0.5 dB, per-band changes only in the 500 Hz band (up to +3.3 dB).
- Root cause measured from stems: the shipped scene mix at 150-600 Hz is dominated
  by the structural routes (mount_monocoque ≈ -26.6 dB, under_seat ≈ -26.2 dB,
  airbox/metal ≈ -32 dB pre-output-gain) while engine_air is ≈ -37.6 dB. Exhaust
  geometry therefore has little authority over the band that carries the body.
- Combination candidates (scale 1.5 + collector) reproduced the same masking plus
  a narrow 500 Hz coincidence at 6000 RPM (+11.8 dB) that risks a fixed drone;
  excluded from the gate set.

### EXH-04 explicit routing (mechanism, defaults preserved)

- Transport: `audio.gf509.header_length_scale` and `audio.gf509.collector_geometry`
  → `V10LayerTuning` → `Gf509RuntimeConfig.engine`. Absent keys keep the declared
  baseline geometry, so no other GF509 vehicle is retuned. The launched profile
  (`f1_2030_v10_geometric.json`) is NOT modified pending the listening gate.
- Gate set: `reports/audio-v10/exh04/gate/{baseline,primary,primary_collector}/{proc,hybrid}`
  with identical telemetry/seed/sample rate; loudness-matched copies (BS.1770,
  gains recorded, peak-capped at 0.999 on two hybrid excerpts) in
  `reports/audio-v10/exh04/auditions/`.
- Test status: v10-engine-synth 104 passed (1 ignored); vehicle_audio_engine 225
  passed; formula90_core lib 5 passed; `facade_parity` 3 failures reproduced at
  pristine HEAD `2ad2b115` (pre-existing, unrelated).

## Result — gate passed, candidate activated (2026-09-17)

- Human listening gate: user selected **primary_collector** (header scale 1.5 +
  physical collector V3.5 L / tailpipe 0.45 m / d 0.09 m / 1000 K / loss 0.4 /
  coupling 0.6).
- Activated in the launched profile `f1_2030_v10_geometric.json` (`audio.gf509`),
  only that section; no other GF509 vehicle touched.
- `physics_sha256` refreshed in `f1-2030/manifest.json`
  (`174e801b…`, computed over the updated profile).
- End-to-end runtime validation without Godot: `aud_path_bench`
  (`CoreFacade → AudioModule → profile parse → V10LayerTuning → Gf509Runtime`)
  exit 0, output `reports/audio-v10/exh04/aud_path_bench.json`.
- `run_f1_94.ps1 -ValidateRuntimeOnly` green (manifest, BUILD/HEAD 2ad2b115,
  Fuji package, import).
- In-game parity note: the installed DLLs were built from `2ad2b115` and do not
  contain this uncommitted transport, so the game still renders baseline until
  `scripts/build_windows.ps1` rebuilds from the new source; old binaries ignore
  the new optional keys, so the updated profile is backward compatible.

## Revert for in-game comparison (2026-09-17)

- User requested reverting the profile to the previous (pre-candidate) state to
  test in-game. Both keys (`header_length_scale`, `collector_geometry`) removed
  from `audio.gf509`; the file is now byte-identical to committed HEAD.
- `physics_sha256` refreshed to the reverted profile hash
  (`33969b67…`, also clears the stale hash left by the suspension commit).
- Revalidated: `aud_path_bench` exit 0 with the baseline profile and
  `run_f1_94.ps1 -ValidateRuntimeOnly` green. Baseline in-game testing needs no
  rebuild (installed DLLs are from `2ad2b115`); re-enabling the candidate later
  only requires the two keys again plus a runtime rebuild from the new source.

## Stable scene base (2026-09-17)

Persisted in the launched profile `f1_2030_v10_geometric.json` (`audio.gf509`) as
the reference scene balance, after the scene audition sweep:

- `scene_gains`: `under_seat 0.0`, `mount_monocoque 0.713`,
  `engine_cover 0.9156`, `metal 0.0`, `airbox 0.0`, `engine_air 0.841`.
- `metal_panel_lowpass_hz 1_000_000`, `cover_radiation_lowpass_hz 1_000_000`
  (at/above `0.48 * sample_rate` the stage is a true bypass).
- Dry gains (0.18/0.46/0.14) and `output_gain` 2.9 stay at their defaults; the
  values above are transported by `V10LayerTuning` → `AcousticSceneConfig`.

Reference audition: `reports/audio-v10/exh06/scene_config_sweep/sweep_mount_minus1db_cover_plus1db.wav`
(sweep 5000→18000 RPM, lift & coast, 10 s, 44100 Hz). Re-rendering the same
parameters reproduces that file bit-for-bit (SHA-256 match).

Validation: v10-engine-synth 106 tests, vehicle_audio_engine 225 tests,
formula90_core lib 26 tests; `aud_path_bench` exit 0 through the real profile
path; `run_f1_94.ps1 -ValidateRuntimeOnly` green with the refreshed
`physics_sha256`. In-game still needs a runtime rebuild from this source: the
installed DLLs predate the transport.

## Retrospective

- Worked: explicit scale/geometry knobs with bit-identical defaults; isolated
  target dir and capture scripts; stems exposed the masking root cause instead of
  guessing; profile transport gated behind optional keys.
- Cost: two rounds of collector scans before the structural masking was visible;
  next time measure stem contribution ratios before tuning candidates.
- Follow-ups (not authorized here): rebuild runtime to hear in-game; if the
  effect still reads as too subtle, the measured limit points at the scene
  structural balance (mount_monocoque/under_seat) or the sample layer, which are
  outside this item's scope.


