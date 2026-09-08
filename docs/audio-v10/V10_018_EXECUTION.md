# V10-018 — Evaluate load-dependent saturation

Date: 2026-09-08. Scope: isolated pruning experiment for
`LoadDependentSaturation` in the corrected acoustic scene. The master
saturation, linked limiter, parallel compressor, source engine, samples, and
all unrelated scene gains were held unchanged.

The worktree remained on `main-clean` at HEAD
`7feb50128a7ba1573c1e04d466cd5bd71e7aa7d6`. Existing HUD, project,
instruction, mixer-config, `third_party/godot-cpp`, and V10-007–012 changes
were preserved. `game/BUILD_SOURCE` remains a separate parity gate.

## Experiment design

The full path uses the corrected `AcousticScene` with its default
`load_saturation_gain = 0.10`. The excised path uses a diagnostic graph
switch that skips `LoadDependentSaturation::process` and returns zero for that
branch. `LowMidParallelCompressor::process` still runs in both paths.

Both variants use the same 48 kHz renderer, seed `4035969040`, one-second
warmup, identical engine inputs, unchanged output gain, and no sample layer.
The paired cases are:

- 9,000 rpm, partial load: throttle `0.55`, load `0.50`;
- 9,000 rpm, full load: throttle `1.00`, load `1.00`;
- 9,000 rpm coast: throttle/load `0.00`;
- 5,000→18,000 rpm sweep followed by lift/coast to 1,800 rpm.

The render switch is diagnostic-only and defaults to the existing full path:

```text
--excise-load-saturation
```

The unchanged parallel-compressor result is verified by identical SHA-256
stems in every full/excised pair.

## Cost and output results

The isolated scene benchmark uses 48,000 warmup and 192,000 measured samples.
The end-to-end row includes `V10Engine::render_sample` and is reported only as
a local comparison, not as a game-wide CPU percentage.

| Case | Scene saving | Engine + scene saving | Full scene RMS | Excised RMS | Paired delta / full |
| --- | ---: | ---: | ---: | ---: | ---: |
| Partial load | 4.839% | 2.628% | 0.193807 | 0.191253 | 9.08% |
| Full load | 5.863% | 2.414% | 0.293857 | 0.288360 | 10.52% |
| Coast | 4.752% | 0.912% | 0.123590 | 0.122989 | 5.20% |
| 5,000→18,000 + coast | — | — | 0.175832 | 0.173078 | 12.16% |

The saturation stem itself is non-negligible: RMS `0.063947` at partial load,
`0.111738` at full load, and `0.023257` at coast. The 9,000 rpm full-load
low-mid saturation stem has stronger third/fifth-order indicators than the
partial-load case (`6028.1/4138.2` versus `3419.0/2411.2` in the fixed-window
FFT proxy). This confirms load contrast rather than a purely constant color.

The low-mid crest-factor inspection is:

| Case | Parallel compressor crest | Load saturation crest | Saturation 80–500 Hz FFT RMS |
| --- | ---: | ---: | ---: |
| Partial load | 2.929 | 2.347 | 144.255 |
| Full load | 2.669 | 1.877 | 246.337 |
| Coast | 3.235 | 3.094 | 52.465 |
| Sweep/coast | 3.878 | 2.650 | 216.153 |

The renderer reported no limiter reduction and no clipped samples in any
variant. Full/excised peaks after warmup were:

| Case | Full peak | Excised peak |
| --- | ---: | ---: |
| Partial load | 0.622375 | 0.601013 |
| Full load | 0.847473 | 0.846222 |
| Coast | 0.418671 | 0.417450 |
| Sweep/coast | 0.866608 | 0.818970 |

The roughness metric is explicitly a diagnostic proxy: first difference of a
1 ms-smoothed absolute envelope divided by envelope RMS. It changed only
slightly in the steady cases (partial `0.01937` full versus `0.01992`
excised; full load `0.01976` versus `0.02036`; coast `0.01866` versus
`0.01872`). This suggests the module primarily changes low-mid body/density,
not a large increase in the measured envelope roughness. Human listening is
required to decide whether that body is useful or masks source dynamics.

Raw paired data:

`reports/audio-v10/v10-018-saturation/analysis.json`

Cost data:

`reports/audio-v10/v10-018-saturation/scene-cost.json`

## Disposition

Technical disposition: **KEEP — pending human listening**.

The module is not a safe removal candidate based on low RMS alone: it produces
a reproducible load-dependent contribution, saves only about 5% of isolated
scene cost, and changes the paired output by 5–12% RMS relative to the full
scene. The experiment therefore retains the candidate for the human gate. No
reduced or simpler transfer was tested or promoted; that is a separate
iteration only if listening identifies excess density while preserving useful
load contrast.

The result is not marked acoustically accepted. The human should compare the
following full/excised pairs at matched playback level, focusing on partial
load body, full-load density, coast cleanliness, low-mid masking, and whether
the sweep loses source definition:

```text
reports/audio-v10/v10-018-saturation/renders/steady_partial_load/full/out.wav
reports/audio-v10/v10-018-saturation/renders/steady_partial_load/excised/out.wav
reports/audio-v10/v10-018-saturation/renders/steady_full_load/full/out.wav
reports/audio-v10/v10-018-saturation/renders/steady_full_load/excised/out.wav
reports/audio-v10/v10-018-saturation/renders/lift_coast_5000_to_18000/full/out.wav
reports/audio-v10/v10-018-saturation/renders/lift_coast_5000_to_18000/excised/out.wav
```

## Reproduction

```powershell
cargo test --release --manifest-path game/crates/Cargo.toml --package v10-engine-synth --lib
cargo build --release --manifest-path game/crates/Cargo.toml --package v10-engine-synth --bin v10_saturation_bench --bin v10_render
& .\game\crates\target\release\v10_saturation_bench.exe `
  --output reports/audio-v10/v10-018-saturation/scene-cost.json
.\scripts\audio\render_v10_018_saturation.ps1 `
  -OutputRoot reports/audio-v10/v10-018-saturation/renders
python scripts/audio/analyze_v10_018_saturation.py `
  reports/audio-v10/v10-018-saturation/renders `
  reports/audio-v10/v10-018-saturation/analysis.json
```

Status: `READY_FOR_HUMAN_LISTENING`; no master saturation retune or production
disposition was bundled, and no commit was created.
