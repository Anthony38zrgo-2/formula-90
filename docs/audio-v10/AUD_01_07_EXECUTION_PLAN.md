# AUD-01–07 execution plan

Baseline provenance: branch `codex/aud-01-07`, source HEAD
`e5c8a6d9825742745c553e56d78f78077c399039`. This worktree was created from
that commit to avoid modifying the dirty `main-clean` checkout. Build artifacts,
DLLs, object files, `target`, `.godot`, and ignored caches are not reused.

The work follows the repository gate sequence for each item: planning, backlog
item, implementation, review, human gate, retrospective, done. A technical
deliverable may be ready while its listening gate remains open; it must not be
reported as done before that gate passes.

## Order and dependencies

1. **AUD-01 reference corpus.** Inventory the available R25 captures, record
   hashes, perspective, operating state, RPM source and uncertainty, and define
   level-matched listening comparisons. Human listening is the acceptance gate.
2. **AUD-02 reproducible baseline.** Render GF509 and its stems with a frozen
   trace, seed, sample rate, block size, config hashes, asset hashes, HEAD and
   explicit confirmation that GF509 initialized instead of falling back.
3. **AUD-03 real-path benchmark.** Measure `Gf509Runtime`,
   `VehicleAudioEngine`, and the actual `formula90-core` `AudioModule`/facade
   route with identical telemetry/assets/build. Report
   thread CPU, per-block wall distribution, memory/allocation evidence and
   component variants. Do not claim improvements from unmatched runs.
4. **AUD-04 authoritative telemetry.** Preserve the existing physical meaning
   of torque, clutch, shift phase, traction control and limiter state across the
   facade. Begin with an integration test showing what reaches GF509. Signed
   torque/load must not be redefined as an audio-only estimate.
5. **AUD-05 invariant/control-rate work.** Profile first, then move invariant
   coefficient work out of the sample loop. Interpolate controls whose slower
   update rate is demonstrated safe. Retain validation at API boundaries.
6. **AUD-06 inactive sample zones.** Skip inaudible signal/filter work while
   advancing every loop cursor. Reactivation must either prepare filter state
   or prove by a transition test that the discontinuity remains below the
   agreed tolerance.
7. **AUD-07 resampling.** Evaluate two separate changes: tabulated/polyphase
   kernel computation for CPU, and bandwidth reduction for pitch ratios above
   one for antialiasing. Compare against the same sinc reference and trace.

## Frozen comparison dimensions

- Source HEAD, Rust release profile and compiler identity.
- GF509 manifest and every consumed asset SHA-256.
- Runtime config, seed, telemetry trace, sample rate, block partition and output
  length.
- Warmup and measured iteration counts.
- Same machine and thread CPU clock for before/after performance claims.
- Same loudness matching method for listening comparisons.

## Gates

- **Technical:** deterministic render identity; explicit GF509 activation;
  zero callback I/O; no callback allocation regressions; block sizes 128, 256,
  512 and 1024 at 44.1 and 48 kHz where supported; transient and steady traces.
- **Perceptual:** level-matched A/B against the R25 corpus for acceleration,
  held RPM, lift/coast and shifts. A human records the preferred version and
  artifacts heard. No automated metric substitutes for this gate.
- **Runtime:** BUILD/HEAD parity through the canonical launcher after isolated
  source builds. Full rebuilds clean only this worktree's own Cargo/SCons/game
  `.godot` outputs.

## Current blockers and non-blockers

- Source captures are ignored files in the original checkout. Their identities
  can be recorded read-only; copying them into this worktree is unnecessary for
  the first inventory because packaged derivatives already carry source hashes.
- AUD-01 and all later perceptual acceptance remain open until a human listens.
- Benchmark and telemetry work can proceed independently of that human gate.

## AUD-04 delivery note (2026-09-05)

The authoritative physics adapter is present in `formula90-core`: it forwards
signed torque, clutch engagement, smoothed traction-control cut, limiter state,
and deterministic shift cut/recovery phases to GF509 through the existing C ABI
V3 packet. The internal runtime validates the new mechanical fields, interpolates
only continuous clutch/TC controls, and applies discrete shift/limiter states at
sample boundaries. GF509 consumes these controls: negative torque contributes
retention energy, shift/limiter states shape combustion energy, and TC adds a
bounded texture modulation without applying its physical load cut a second time.

Evidence: `cargo test -p formula90_core aud04_integration` passes (3/3), and
`cargo test -p v10-engine-synth --lib` passes (87/87, 1 ignored). The complete
`vehicle_audio_engine` library run is 207/214: its seven failures are existing
C++ DSP tests blocked by missing `f90_audio_dsp.dll` (`DllNotFound`). The AUD-04
release benchmark, complete physical scenario matrix, BUILD/HEAD launcher gate,
and level-matched human listening remain open.

## AUD-07 technical delivery (2026-09-05)

The sample layer keeps the evaluated sinc and radius-4 phase table as
independently selectable references. The AA path builds a 64-level, 1025-phase,
32-tap table at layer initialization. Its cutoff is `1/ratio` in source-sample
frequency, represented by the arithmetic coordinate `1 - 1/ratio`; phase and
cutoff rows are linearly interpolated in the callback. The maximum ratio is
derived from each asset and output rate as
`25000/rpm_anchor*source_sample_rate/output_sample_rate`, reaching about 26.95x
for the 44.1 kHz lowest-anchor asset at 8 kHz. Loop taps wrap circularly, so
the pre-resampling filter has no loop seam state. The fixed Butterworth filter
that ran after resampling was removed because it could not prevent folding.

Measured tests cover separate passband and folded-alias components at 1.3x and
2.0x at 44.1 kHz, 10.0x and 26.9x at 8 kHz, fractional cursor positions during
an up/down ratio sweep across 1, and continuity at the exact maximum row. The
26.9x check measures rejection of content clearly above the reduced cutoff; it
does not establish full-band timbre quality for every ratio. The table is
8,528,000 bytes (8.528 MB decimal; 8.13 MiB) per layer and is allocated only
during initialization.
Release benchmark, BUILD/HEAD launcher parity, candidate provenance manifest,
and the human timbre gate remain open.
