# AUD-02 reproducible baseline

Source baseline: `codex/aud-01-07` at
`e5c8a6d9825742745c553e56d78f78077c399039`. The release binaries were built
from this worktree after confirming that `game/crates/target` did not exist.
Compiler: `rustc 1.95.0 (59807616e 2026-04-14)`, MSVC x86-64, LLVM 22.1.2.

## Runtime-path capture

`aud_runtime_baseline` freezes one exact 264704-frame trace at 44100 Hz in
256-frame blocks. It accelerates linearly from 4500 to 15000 RPM at throttle
0.92/load 0.88, changes gear 3 to 4 with an explicit upshift-cut phase, then
lifts and coasts to 6500 RPM at throttle 0.035/load 0.10 with negative torque.
The emitted `trace.csv` contains every input field for every block; its SHA-256
is `a29b723fe31eeaa2e3bec5e458b166d92264e6142c09aa58f4ac2fe13429739c`.

The harness instantiates both `Gf509Runtime` and `VehicleAudioEngine`, verifies
that the mixer reports `ContinuousSourceKind::V10Gf509`, rejects non-finite or
out-of-range output before PCM conversion, and writes both stereo channels.
Two independent runs produced identical files:

| Route | Peak | Stereo WAV SHA-256 |
|---|---:|---|
| `Gf509Runtime` | 0.76722455 | `84c36644bca5b6106340cf54fb1c1180d950c15b610a18b0ebef7141297fbd70` |
| `VehicleAudioEngine` | 0.62682182 | `22d36a126b300de6f01af307883af86f8cf07b0257a645e2b8203a6f6533d559` |

The capture metadata includes the exact trace, harness, bank manifest, all
bank/GF509 asset hashes, and hashes of the complete `v10-engine-synth` Rust
source plus the mixer/config/powertrain source used by the second route. It also
records the current route difference:
`Gf509Runtime` receives the trace `dt_seconds`, while
`VehicleAudioEngine::set_telemetry` currently forwards `dt_seconds = 0.0` to
GF509. Therefore the two route hashes are separate baselines, not a parity
claim. AUD-04 is responsible for correcting this telemetry loss.

Reproduction from `game/crates`:

```powershell
cargo build --release --manifest-path Cargo.toml -p vehicle_audio_engine --bin aud_runtime_baseline
./target/release/aud_runtime_baseline.exe ../sounds/banks/v10_vehicle ../audio/v10_gf509 ../../reports/audio-v10/aud02/runtime-v2/run1
./target/release/aud_runtime_baseline.exe ../sounds/banks/v10_vehicle ../audio/v10_gf509 ../../reports/audio-v10/aud02/runtime-v2/run2
```

## Diagnostic stems capture

The existing `v10_render` was also built from the clean source baseline and run
twice at 48000 Hz with seed 4035969040, a 4500→15000→6500 RPM
acceleration/lift trace, acoustic scene, and packaged sample layer. Both master
files have SHA-256
`b2f547127ad03f227b93b01a7fe8c8d22dcfaaa9cc972763f016014a16cd4060`.
Its stems remain the frozen component reference for later comparisons.

`v10_render` constructs `V10Engine`, `AcousticScene`, and
`ThreeZoneSampleLayer` directly. It does not exercise `Gf509Runtime` telemetry
smoothing or `VehicleAudioEngine` mixing, so this stem capture is diagnostic
and does not establish runtime parity.

The exact direct/stems command, run from the repository root, was:

```powershell
./game/crates/target/release/v10_render.exe --rpm 4500 --sweep-end-rpm 15000 --coast-end-rpm 6500 --seconds 12 --warmup 1 --accel-seconds 8 --throttle 0.92 --load 0.88 --sample-rate 48000 --seed 4035969040 --acoustic-scene --sample-layer-dir game/audio/v10_gf509 --out reports/audio-v10/aud02/direct/run1/master.wav --stems-dir reports/audio-v10/aud02/direct/run1/stems
```
