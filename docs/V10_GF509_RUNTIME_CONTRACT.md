# GF509 baseline and runtime contract

Recorded 2026-09-01 for `INT-00` and `INT-01` on branch
`fix/v10-audio-physical-boundary`, observed HEAD
`4c6854081c21dc8cdc6ea1f761935120848090ea`.

## Frozen offline baseline

The accepted artifact is
`reports/audio/rust-greenfield/gf509_5000_14500_parameter_corrected_sweep.wav`
with SHA-256
`c1fb8db483fab04e4f8d8f64d6928cce8659e7210dcae37afa7080fc849c580a`.
Its metadata identifies source commit
`92ffdd01e55e1725f5a869208186e1db1f250a58`, dirty-source fingerprint
`fnv1a64:4468729e6ac3e2b6`, sample rate 44,100 Hz, seed `4035964944`,
5000–14500 RPM acceleration for 7 seconds followed by coast to 6500 RPM,
10 seconds total, 1 second warmup, throttle 0.92 and load 0.88. The fixed
7499 RPM reference and this lift/coast sweep remain the perceptual references.

The relevant current GF509 source snapshot is frozen by these SHA-256 values:

| Source | SHA-256 |
|---|---|
| `engine.rs` | `04c1ca2b9e4c1f0ae81cbce4dff7c34b9e40ab857f5ab50c8c9580f5e1bc67d6` |
| `config.rs` | `6157cb6b3857f5b8b7f9c51cc2dbebeec56a6bfe34270d5c5703f17588396090` |
| `crank.rs` | `51c0fdd2339017d337e02b85d3b84e91cf6ffb30129f2bb02e366526df474832` |
| `cylinder.rs` | `4618b263ba5e1a27a790a2ac5a6c5b7f9bb6e61add62358b47b7893fd6dfafd2` |
| `acoustics.rs` | `b04fed82f2066b75fa3f7900a1bfd74b8cfe034b21731c01cb72315f9258c957` |
| `scene.rs` | `f352935a5101624a64da1c235399376fe4ac9c49a01531f3e4483588cae209ee` |
| `sample_layer.rs` | `9e3bd428fbadf761e6452de22c80caaceca23fd1c1f1cb7bd23cc92e687d32fb` |
| `v10_render.rs` | `6d5cd2d1a02f7817953bc26ab89fadfef3c41e45ad9d0624b07c787403b40ba8` |

GF509 uses the default `EngineConfig` and `AcousticSceneConfig`, the default
`ThreeZoneSampleLayerConfig`, and a final static hybrid gain of 0.61. Exact
parameter values and the offline reproduction command remain in
`V10_GF509_HYBRID_AUDIO_ARCHITECTURE.md`.

## Prepared sample identity

All prepared samples are mono PCM16 at 44,100 Hz. The medium anchor is an
estimate and is not contractual.

| Zone | Anchor / effective RPM | Source SHA-256 | Metadata SHA-256 | Loop tonal SHA-256 | Loop residual SHA-256 |
|---|---:|---|---|---|---|
| low | 7499 / 7496.81 | `2eac0ef746c818ecac50fe77ca0099894f4762d55428a3cddfeb3007a6092cc5` | `316b0f68f945c680e1dfccb79816aa6025a73262183e6035b9e475aa65208338` | `09b52292fae91e3c4ae4c2805de1f5f418799f737a6fd999b6207d67203e071f` | `c454c232a0d8e274200495420948aa74666c944d5911bde413e41d5bb3e0c635` |
| med | 8202 estimated / 8195.12 | `d1a527d5ddf3ec6d4f38127d1c6a3b00d7b7771d5473f68315e4352c98db4007` | `cd3c8fc3c7a04fa9518e8b353ed668a1cd5ae51b7731eb87104baa1f45a541f7` | `55f57a3ec2e7caf9eca8edeb01dc18f3dedca074cc27a038ce129f9c0fe3fff4` | `50e8e51c719d76feff5b5cba4237cfbaf05bb8323af524c8e5fe6ad040cbf14d` |
| max | 15327 / 15320.63 | `f6b6cd4389fee32cb5f4b41a550e7aa146d4a5d0912fda86551dabfd598aa50b` | `905b5adff8ab568abbf5a22e4a96807757e050dcc73a87fa619f96a1d2050ab8` | `657f55a1e34b154d948009f20e6ae78fcad9a4904b5bfc3bb34638a23079082e` | `544a22367793c8111ce182e54438164f70895c6a39b2cf9616cb101470187b48` |

The preparation manifest records an Automobilista mod asset tree as the
immediate extraction path. On 2026-09-01, the project owner clarified that the
underlying samples originate from Geoff Crammond's GP2/GP3 abandonware and
explicitly approved their use and packaging in this project. This is the human
provenance/license gate required by `INT-00`; it records the project owner's
authorization rather than an independent legal assessment. `INT-02` is no
longer blocked on sample provenance.

## Runtime contract v1

The embedding API is `Gf509Runtime` in `v10-engine-synth::runtime`.

- Construction receives `Gf509RuntimeConfig`. Asset loading and validation
  happen during construction, never in `render_block`.
- Control input is `RuntimeTelemetry { rpm, throttle, load, gear, dt_seconds }`.
  RPM is 0–25,000, throttle and load are 0–1, gear is -1–12, and `dt_seconds`
  is finite and 0–1. Crank phase remains sample-accurate internal state.
- Output is planar stereo `f32` at `EngineConfig.sample_rate`. Both slices must
  have equal lengths no greater than `max_block_frames`. The current continuous
  source is centered, so left and right are identical.
- `render_block` performs no file I/O, logging, temporary-buffer allocation or
  global-state access. It preserves phase across arbitrary block boundaries.
- `reset` reconstructs deterministic post-create state and retains the config.
  It may reload assets and is forbidden in the audio callback.
- Offline CLI/WAV/stem generation remains in `src/bin/v10_render.rs` and is not
  used by the runtime facade.

For `INT-01`, `sample_layer_directory = None` deliberately selects the
procedural `V10Engine + AcousticScene` path. Passing a prepared directory
enables the complete GF509 hybrid path. Runtime-relative asset packaging and
manifest policy belong to `INT-02`.

## INT-02 / INT-03 implementation status

The runtime package is `game/audio/v10_gf509`. It contains only the six loop
WAV files consumed by `ThreeZoneSampleLayer`, three runtime-only metadata files
without source-machine paths, and `manifest.json`. Initialization validates the
manifest key/schema, all three zones, sample rate, channel count, loop shape,
local filenames and SHA-256 hashes before any render can begin.

`VehicleAudioEngine` exposes `ContinuousSourceKind::{Legacy, V10Gf509}`.
GF509 is rendered exactly at the former continuous-engine mix point, before
beds, tyre scrub, scraping, one-shots, reverbs and final protection. A failed
GF509 initialization explicitly leaves `Legacy` selected. A render failure is
reported through `gf509_render_failed` and does not silently change source.

## INT-04 telemetry and reset

The active vehicle profile selects `continuous_source: "v10_gf509"`.
`formula90-core` derives the asset root from the loaded bank's `game/` root and
feeds GF509 from the same physics tick that publishes RPM, throttle and gear.
Load is the positive effective engine torque divided by the torque currently
available from the configured curve, clamped to 0–1. `dt_seconds` is the fixed
physics delta accepted by the facade.

`Gf509Runtime` interpolates RPM, throttle and load over the corresponding number
of audio samples, independently of audio block partitioning. Crank phase remains
internal and sample-accurate. Facade reset reconstructs GF509 deterministically
and stops active one-shots, continuous beds, scrape and tyre cursors. If physics
ticks pause, no new target is published and audio holds the last coherent state;
respawn/reset clears it before the next tick.

## INT-05 / INT-06 diagnostics and realtime budget

The versioned vehicle audio configuration records source, manifest, gain,
initialization fallback and diagnostic mode. Diagnostic modes are `mix`,
`v10_only` and `events_only`; they change routing without recompilation.
`ContinuousDiagnostics` publishes source, received/rendered RPM, pre-protection
peak, estimated reduction, last/worst render time, block count, deadline misses
and asset/render errors. The callback updates scalar counters only and performs
no logging.

Release benchmark command:

```powershell
cargo run --release -p vehicle_audio_engine --bin gf509_rt_bench
```

Warm-run results on 2026-09-01 at 44.1 kHz, 300 measured blocks per size:

| Frames | Median | p95 | Worst | Deadline | Worst/deadline |
|---:|---:|---:|---:|---:|---:|
| 128 | 343.6 us | 535.8 us | 1319.4 us | 2902.5 us | 45.46% |
| 256 | 710.7 us | 918.5 us | 1818.0 us | 5805.0 us | 31.32% |
| 512 | 1305.1 us | 1710.9 us | 3261.3 us | 11610.0 us | 28.09% |
| 1024 | 2623.0 us | 3119.0 us | 4351.5 us | 23220.0 us | 18.74% |

The warm run recorded zero deadline misses and zero render/asset errors. An
immediately preceding cold run recorded seven scheduler outliers, with worst
cases above deadline at 512 and 1024 frames; both results are retained here so
the realtime gate is not represented by typical latency alone. The existing
allocation probe confirms zero Rust allocations/frees in the GF509 callback.
