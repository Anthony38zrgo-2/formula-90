# V10-012 — Optimize expensive chamber/source calculations

Date: 2026-09-08. Scope: one reviewable computation family: slider-crank
instantaneous chamber volume. The firing order, ten cylinder paths, chamber
phase contract, valve model, runner model, and audio gains are unchanged.

The worktree remained on `main-clean` at HEAD
`7feb50128a7ba1573c1e04d466cd5bd71e7aa7d6`. Existing HUD, project,
instruction, mixer-config, `third_party/godot-cpp`, and V10-007–011 changes
were preserved. `game/BUILD_SOURCE` remains a separate parity gate and is not
claimed by this record.

## Profile and chosen delta

Source inspection located repeated `sin`, `cos`, and `sqrt` work in
`SliderCrank::instantaneous_volume_m3`; it is called by the corrected chamber
for every cylinder and audio sample. The other candidates (`powf`/`exp` in
Wiebe and polytropic state, and valve/runner calculations) were intentionally
left for separate reviewable deltas.

The pre-change measurement was captured before modifying `geometry.rs`; the
post-change values are the median of three release runs. Both use the same
48 kHz, 48,000-sample warmup, 192,000-sample measurement window and the same
`V10Engine::render_sample` path. The geometry row is a two-million-call
microkernel measurement.

| Case | Before ns | After ns | Change |
| --- | ---: | ---: | ---: |
| Geometry volume call | 36.091 | 28.349 | -21.4% |
| Steady 9,000 rpm source sample | 1,691.210 | 1,463.216 | -13.5% |
| Steady 18,000 rpm source sample | 1,719.353 | 1,460.596 | -15.1% |
| 5,000→18,000 rpm transient | 1,623.184 | 1,485.100 | -8.5% |
| 18,000→1,800 rpm coast | 1,617.504 | 1,465.087 | -9.4% |

The result is a scoped source-kernel saving, not an inferred game-wide CPU
percentage. The V10-004 callback measurements remain a separate approval gate;
the published historical repeat spread was 2.1362% versus 2.0320% on the
legacy Far/sweep row. The minimum source-kernel reduction above is larger than
that repeat difference, while no claim is made that these isolated 48 kHz
numbers replace the 44.1 kHz callback benchmark.

## Implementation

`SliderCrank` now builds a 720-degree table at configuration construction:

- 1,024 intervals and 1,025 `f32` volume values per configured cylinder;
- about 4.0 KiB per cylinder, about 40 KiB for ten cylinders;
- linear interpolation in the real-time path, with exact phase wrap at 720°;
- `area_m2` is stored once instead of recomputed per sample;
- every supported bore, stroke, rod, and compression-ratio configuration builds
  its own table, so no stale static-configuration cache is possible;
- construction-time work is outside the audio callback and the render path adds
  no allocations.

The exact analytic formula is retained as an internal reference only. No
base-plus-energy table was introduced. The corrected chamber remains affine in
energy at fixed crank angle (tested at expansion, exhaust, intake, compression,
and cycle-wrap points), but the downstream valve/runner flow contains
pressure-dependent nonlinear operations, so a broader energy table would not be
an equivalent source optimization.

## Equivalence and safety evidence

The declared geometry bound is absolute volume error `<= 1.0e-8 m³`. Tests
cover the full cycle, wrap at 720°/0°, compact-rod clamping, and geometry
extremes. The corrected chamber comparison covers steady and transient points
with exact phase, burn, and heat-release agreement, plus relative bounds of
`2.0e-3` for pressure and `2.0e-4` for temperature.

The release source benchmark reported finite output for all scenarios. The
post-change boundary regression passed semantic, numerical, and runtime-matrix
checks, including event order and phase stepping:

`reports/audio-v10/v10-012/boundary-regression.json`

The listening capture is:

`reports/audio-v10/v10-012/v10_5000_to_18000_then_lift_coast_10s.wav`

It is 10 seconds at 48 kHz, with a 5-second 5,000→18,000 rpm sweep followed
by lift/coast to 1,800 rpm. The renderer reported peak `0.628528`, RMS
`0.126915`, and zero limiter reduction. Human listening remains a separate
gate; this optimization is not marked acoustically accepted by measurement
alone.

## Reproduction

```powershell
cargo test --release --manifest-path game/crates/Cargo.toml --package v10-engine-synth --lib
cargo run --release --manifest-path game/crates/Cargo.toml --package v10-engine-synth --bin v10_source_bench
cargo build --release --manifest-path game/crates/Cargo.toml --package v10-engine-synth --bin v10_boundary_regression
& .\game\crates\target\release\v10_boundary_regression.exe reports/audio-v10/v10-012/boundary-regression.json
cargo run --release --manifest-path game/crates/Cargo.toml --package v10-engine-synth --bin v10_render -- `
  --rpm 5000 --sweep-end-rpm 18000 --coast-end-rpm 1800 --seconds 10 `
  --accel-seconds 5 --warmup 1 --throttle 0.95 --load 0.90 `
  --sample-rate 48000 --seed 4035969040 --acoustic-scene `
  --sample-layer-dir game/audio/v10_gf509 `
  --out reports/audio-v10/v10-012/v10_5000_to_18000_then_lift_coast_10s.wav `
  --stems-dir reports/audio-v10/v10-012/v10_5000_to_18000_then_lift_coast_10s_stems `
  --physical-telemetry-csv reports/audio-v10/v10-012/v10_5000_to_18000_then_lift_coast_10s.physical.csv
```

Status: `READY_FOR_REVIEW`; the source optimization is uncommitted, human
listening is pending, and runtime BUILD/HEAD parity remains separate.
