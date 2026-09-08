# V10-001 through V10-006 execution record

Date: 2026-09-07
Scope: `V10_AUDIO_REALISM_AND_OPTIMIZATION_BACKLOG.md`, from V10-001 through V10-006.

## Scope and provenance

The worktree was recorded before implementation on branch `main-clean`, at HEAD
`a2d94f7ec7996690269d577728752fd7ec80f741`, with `origin/main-clean` at the same
revision. Existing HUD, project, instruction, mixer-config, and
`third_party/godot-cpp` changes were treated as unrelated and preserved.

The durable evidence package is written by:

```powershell
.\scripts\audio\write_v10_001_006_manifest.ps1
```

It records branch/HEAD/status, `BUILD_SOURCE`, toolchain, lockfile and source
hashes, selected audio/config hashes, rendered binary hashes, and copies only
the original audit's source evidence (`Cargo.lock`, `Cargo.toml`,
`evidencia.txt`, `informe.md`, and source harness files) into
`reports/audio-v10/v10-001-006/original-audit`. Target artifacts are not copied.

At the time of this execution, `game/BUILD_SOURCE` does not match HEAD. This is
recorded as a provenance/parity warning; it is not silently promoted to a
runtime release claim.

## V10-001 — Preserve evidence and establish implementation provenance

The original temporary V10 audit was identified and its source evidence was
preserved in the durable report directory. The release renderer and the new
provenance probe are hashed in the manifest. The renderer's source fingerprint
now includes chamber thermodynamics, runner/valve/gas/polytrope code, runtime,
and the renderer itself.

The retained-charge finding remains the motivating evidence: the previous
chamber model could carry a hot state across the nominal cycle boundary. The
new capture records the corrected model and its diagnostic fields rather than
overwriting that historical evidence.

## V10-002 — Deterministic captures and diagnostic taps

The authoritative runtime paths are `Gf509Runtime` and `VehicleAudioEngine`.
`v10_provenance_probe` creates two identical GF509 runtime instances and two
identical mixer instances, renders paired blocks, and reports maximum paired
sample difference, bit-exact status, output hashes, configuration reload
values, LOD labels, and settled RMS. The probe is deliberately outside the
real-time callback; file I/O is evidence collection only.

The greenfield renderer now emits physical stems for chamber pressure,
temperature, heat-release rate, chamber phase, valve area, mass flow, runner,
collector, and final output. The physical telemetry CSV is the row-level trace
for the chamber contract.

The updated probe demonstrates the three control contracts with counters:
`EventsOnly` still increments GF509 render calls (output-only mute), switching to
the legacy source produces zero GF509 calls (graph excision), and the diagnostic
bypass produces zero GF509 calls plus one bypass block before one call after
resume (processing bypass with frozen state). The paired runtime probe remains
bit-exact with maximum difference 0.0.

## V10-003 — Configuration and control ownership

The effective signal ownership is:

`physics telemetry -> vehicle-audio adapter -> VehicleAudioEngine ->
Gf509Runtime -> V10Engine (10 cylinders) -> acoustic scene -> sample layer ->
mid duck -> headroom -> mixer low-pass -> reverb/master limiter -> Godot`.

| Concern | Authority | Result |
| --- | --- | --- |
| Physical engine defaults | `EngineConfig` and GF509 runtime defaults | Used by the Rust renderer/runtime |
| Vehicle-specific values | Vehicle JSON and adapter initialization | Must be captured with the runtime probe |
| Master/reverb settings | `game/sounds/sound_mixer_config.json` | Master saturation is currently not applied by the Rust reload path; this is recorded, not changed in V10-001..006 |
| Sample-layer configuration | GF509 sample-layer manifest/config | Captured by renderer metadata and binary/source hashes |
| LOD selection | `VehicleAudioEngine` listener-distance/Lod state | Current GF509 path still renders at Far/Virtual; optimization is deferred |
| Diagnostic capture | `v10_render` and `v10_provenance_probe` | Output-only and outside the callback |

The control-mode contract is explicit:

1. Output-only mute: retain graph/state, suppress the selected continuous
   contribution; downstream event paths may remain audible.
2. Graph excision: remove the branch including downstream sends.
3. Processing bypass: skip DSP while declaring whether state is frozen, reset,
   or advanced by policy.

No mute or LOD behavior was broadened in this implementation. The probe makes
the current ownership and processing behavior observable for the next backlog
item.

## V10-004 — Performance budget and measurement gate

The measurement contract is fixed at 44.1 kHz with 256-sample callback blocks
for the direct runtime path (5.805 ms callback deadline). The long-window
Windows gate is six 10-second scenario windows plus three release runs, using
aggregate `GetThreadTimes`; per-block `GetThreadTimes` is not accepted as a
measurement method. The capture separates runtime, sample layer, vehicle mixer,
and callback-path timings where the harness exposes them.

Near/Mid/Far/Virtual component budgets remain review-controlled values rather
than invented thresholds in this change. The current report therefore records
the measured harness output and marks the optimization gate pending product
approval of the per-component shares. A successful compile or unit test is not
treated as a CPU-budget pass.

The direct release path measured at 44.1 kHz/256 samples reported these useful
reference points: `Gf509Runtime` with sample layer p50 932.8 us, p95 1603.5 us,
p99 3933.9 us, and diagnostic worst 8612.5 us; `VehicleAudioEngineRenderOnly`
p50 881.7 us, p95 1460.7 us, p99 2417.2 us, and diagnostic worst 7401.6 us.
The callback route made zero allocations during its 4096 measured blocks. These
are measurements, not an approval against a final product budget.

The existing long-window `va_bench` was also run three times. Its six 10-second
scenario repetitions use the existing Near 5.00% and Far 2.00% profile limits.
Run 1 passed; runs 2 and 3 failed only the Far/sweep row at 2.1362% and
2.0320%, respectively. That harness exercises the legacy procedural mixer
benchmark, so this is recorded as a performance follow-up and not conflated
with the GF509 physical-model acceptance.

## V10-005 — Corrected 720-degree chamber contract

The implemented model is a bounded, phase-aware real-time approximation. It is
not a CFD or fully conserved mass/energy solver, which keeps the contract
explicit and testable at the audio callback rate.

| Crank angle | Phase | State contract |
| --- | --- | --- |
| 0..180 deg | Expansion/combustion | Fresh charge state plus bounded Wiebe heat release; ideal-gas pressure/temperature |
| 180..360 deg | Exhaust | Smooth transition to bounded exhaust pressure/temperature boundary |
| 360..540 deg | Intake | Smooth transition to bounded intake-manifold/ambient charge |
| 540..720 deg | Compression | Fresh-charge polytropic compression; no retained hot combustion state |

The cycle boundary uses the same fresh-charge compression state at 720/0 deg,
so the old second-hot-compression pattern is removed. Chamber pressure and
temperature feed the runner/valve path, while flow remains bounded by the
existing valve contract. The 10-cylinder firing order remains
`[0, 5, 1, 6, 2, 7, 3, 8, 4, 9]` with 72-degree spacing.

Independent expectations are: finite bounded pressure/temperature/heat values,
pressure release before exhaust, intake renewal before compression, continuity
at phase boundaries, and no hot recompression on the next cycle.

## V10-006 — Implementation and verification

The source implementation is in `v10-engine-synth`:

- `thermodynamics/combustion.rs`: `ChamberPhase`, fresh-charge renewal, bounded
  four-phase state, and chamber diagnostics.
- `cylinder.rs`: one chamber evaluation per physical cylinder frame and exposed
  pressure/temperature/heat/phase fields.
- `engine.rs`, `thermodynamics/mod.rs`, and `lib.rs`: diagnostic propagation and
  public phase export.
- `vehicle-audio-engine/src/mixer.rs`: diagnostic-only GF509 call counters and
  frozen-state processing bypass used by the V10-003 probe.
- `v10_render.rs`: provenance fingerprint, diagnostic stems/CSV, and the
  `--hold-before-lift` capture mode.

Verification completed: `cargo test --manifest-path
game/crates/v10-engine-synth/Cargo.toml --lib` — 89 passed, 0 failed, 1
ignored. Release builds for `v10_render` and `v10_provenance_probe` complete.

The implementation is ready for the human listening gate, not marked as audio
accepted. The required listening artifact is:

`reports/audio-v10/v10-001-006/v10_5000_to_1800_lift_coast_10s.wav`

Its intended shape is five seconds held at 5000 rpm, followed by five seconds
of lifted/coasting descent to 1800 rpm, with matching stems, telemetry CSV, and
metadata beside it. Human review should compare the audible transition and
listen specifically for the removed hot second-compression character.

## Reproduction commands

```powershell
.\scripts\audio\write_v10_001_006_manifest.ps1
& .\game\crates\target\release\v10_provenance_probe.exe `
  game/sounds/banks/v10_vehicle game/audio/v10_gf509 `
  reports/audio-v10/v10-001-006/provenance-probes.json

& .\game\crates\target\release\v10_render.exe `
  --rpm 5000 --sweep-end-rpm 1800 --hold-before-lift `
  --seconds 10 --accel-seconds 5 --warmup 1 `
  --throttle 0.95 --load 0.90 --sample-rate 48000 `
  --seed 4035969040 --acoustic-scene `
  --sample-layer-dir game/audio/v10_gf509 `
  --out reports/audio-v10/v10-001-006/v10_5000_to_1800_lift_coast_10s.wav `
  --stems-dir reports/audio-v10/v10-001-006/v10_5000_to_1800_lift_coast_10s_stems `
  --physical-telemetry-csv reports/audio-v10/v10-001-006/v10_5000_to_1800_lift_coast_10s.physical.csv
```

Status: `READY_FOR_HUMAN_GATE`; runtime parity and human listening acceptance
remain separate gates.
