# Handoff — Rename shared audio bank `v10_vehicle` → `commons` and prune to used non-engine / non-gearbox effects

Planning date: 2026-09-20. Repo root: `D:/Formula90s`.
This document supersedes the previous Grand Prix sampler handoff. Do not start
the modification from this file alone; it is the contract for the next
implementation sprint. Pipeline: plan sprint → select backlog item → implement →
review → human approval → done.

---

## 0. Mission

The shared runtime sample bank `game/sounds/banks/v10_vehicle/` must become
`game/sounds/banks/commons/`. `commons` contains **only** the samples that are
actually consumed by the current runtime and are **not** engine or gearbox
material (non-specific effects). Everything else is removed from disk (and from
git), not archived.

Human decisions taken (do not re-litigate):

1. `commons` is the single shared non-specific bank; engine/gearbox samples are
   removed, not moved to another bank.
2. `tc_cut` is removed. Verify it is not required by the backend before removal
   (verification result is in §5; it **is** currently wired, see the required
   handling).
3. `impact_fire` is **not** preserved.
4. `game/sounds/sound_mixer_config.json` is pruned to the `commons` keys.
5. `game/sounds/banks/v10-gp3/` remains the source root of the separate Grand
   Prix sampler bank and is untouched by this task.

---

## 1. Provenance and current state

Record before touching anything; never reset or rewrite the dirty branch.

- Branch: `main-clean`.
- HEAD: `ce768b53f5c0a068253b42f7489cd89f93433059`.
- `game/BUILD_SOURCE` currently equals HEAD (parity restored by a full rebuild
  on 2026-09-20).
- Full rebuild backup (pre-existing dirty binaries + BUILD_SOURCE + SHA list):
  `C:\Users\bill\AppData\Local\Temp\opencode\formula90s-backup-20260920-144736`.
- Index state: `game/sounds/banks/v10-gp3/` (13 WAVs) is staged (`git add`,
  uncommitted). Nothing else is staged. Do not commit unless the human asks.
- Pre-existing dirty tracked files that are **not** part of this task and must
  not be staged, reverted, or reformatted: `.agents/AGENTS.md`, `AGENTS.md`,
  `game/BUILD_SOURCE`, the nine DLLs under `game/addons/formula90s/bin/`,
  `game/audio/v10_f2002_experimental/manifest.json`,
  `scripts/audio/build_v10_f2002_experimental_bank.py`,
  `tools/audio/tests/test_bank_determinism.py`, the deletions under
  `implementation/`.
- Pre-existing untracked content that must be preserved: `docs/backlog/`,
  `game/scenes/showcases/`, `game/scripts/showcase/`,
  `source-assets/audio/replacements/f1_2030/`, `implementation/sfx/`,
  `reports/`, `instrucciones.md`, the showcase scripts, and the
  `tools/audio/promote_f1_2030_backfire_sounds.py` promotion tool.
- Prior work in this tree (uncommitted) that this task must not regress:
  the standalone Grand Prix sampler backend (builder
  `scripts/audio/build_grand_prix_sampler_bank.py` revision 4, bank
  `game/audio/formula_one_2030_grand_prix_sampler/`, bank SHA
  `1093f4d46da1e5d1d159ea407ef4198e692f947ebe03aa8a38530db2f9f379f0`, Rust
  modules `grand_prix_sample_bank.rs` / `grand_prix_sampler.rs`, mixer
  integration, profile `audio.continuous_source = "grand_prix_sampler"`, the
  offline renderer bin, the runtime probe
  `game/tests/probe_grand_prix_sampler_backend.gd`, and the listening package
  under `reports/audio-v10/grand-prix-sampler/`).

All 57 files under `game/sounds/banks/v10_vehicle/` are tracked (28 WAVs + 28
`.import` companions + `bank_manifest.json`).

---

## 2. Verified runtime usage analysis (evidence)

Resolution rules in the current code:

- One-shot voices are built in `VehicleAudioEngine::new`
  (`game/crates/vehicle-audio-engine/src/mixer.rs`, the trigger list at
  `Trigger::Hit1 .. Trigger::TcCut`) from `Trigger::bank_key()`
  (`game/crates/vehicle-audio-engine/src/state.rs`) and, additionally, every
  sample whose manifest `role == "engine_backfire"`.
- Surface beds are resolved by `surface_key()` (`state.rs`) and read in
  `mix_aux_voices` (`mixer.rs`) via `self.bank.get(bed_key)`; asphalt has no bed.
- `tyre_scrub` and the sustained underfloor scrape are read by key in
  `mix_aux_voices` (`mixer.rs`).
- Collision triggers originate in `native/src/core/f90_core.cpp` (barrier,
  scrape, cone, hit1..4 by impact severity). No GDScript triggers impacts.
- The Grand Prix sampler owns upshift/downshift/backfire/limiter but **not**
  `TcCut` (`grand_prix_event_kind` in `mixer.rs` maps only ShiftUp, ShiftDown,
  Backfire, Limiter).

Classification result:

| Sample | Role | Runtime consumer | Verdict |
|---|---|---|---|
| `surf_rumble.wav` | surf_rumble | `surface_key("rumble")` → bed loop | KEEP |
| `surf_grass.wav` | surf_grass | `surface_key("grass")` → bed loop | KEEP |
| `surf_sand.wav` | surf_sand | `surface_key("sand")` → bed loop | KEEP |
| `impact_hit_1.wav` | impact_hit | collision `impact_hit_1` (normal impact 3–8) | KEEP |
| `impact_hit_2.wav` | impact_hit | collision `impact_hit_2` (8–15) | KEEP |
| `impact_hit_3.wav` | impact_hit | collision `impact_hit_3` (15–25) | KEEP |
| `impact_hit_4.wav` | impact_hit | collision `impact_hit_4` (>25) | KEEP |
| `impact_barrier.wav` | impact_barrier | barrier/wall/armco collision | KEEP |
| `impact_cone.wav` | impact_cone | cone/prop/dynamic-obstacle collision | KEEP |
| `impact_scrape.wav` | impact_scrape | barrier scrape one-shot + sustained underfloor scrape body | KEEP |
| `tyre_scrub.wav` | tyre_scrub | continuous tire scrub (`set_tire_scrub_state`) | KEEP |
| `impact_fire.wav` | impact_fire | loaded as `Trigger::Fire` one-shot, **no caller** in native/GD/Rust | REMOVE |
| `backfire_3..7.wav` | engine_backfire | legacy/GF509 backfire; sampler uses its own bank | REMOVE |
| `int_backfire.wav`, `int_backfire_2.wav` | engine_backfire | legacy/GF509 backfire | REMOVE |
| `engine_starter.wav` | engine_starter | no caller anywhere | REMOVE |
| `engine_start_backfire.wav` | engine_start_backfire | no caller anywhere | REMOVE |
| `limiter_hit.wav` | limiter_hit | legacy/GF509 limiter edge; sampler uses its own | REMOVE |
| `tc_cut.wav` | tc_cut | mixer TC-cut edge one-shot (all sources; see §5) | REMOVE |
| `shift_up.wav`, `shift_down.wav`, `shift_3.wav`, `shift_up_delayed.wav`, `shift_down_delayed.wav` | shift_* | no runtime lookup by key; GF509 synthesizes shifts and the sampler uses `v10-gp3` | REMOVE |

Removal set = 17 files. `commons` set = 11 files.

---

## 3. Target inventory — `game/sounds/banks/commons/`

Format contract (unchanged): mono, signed PCM16, 44,100 Hz. Keep the existing
bytes exactly; hashes are pinned.

| File | Role | loop | duration_s | peak | loudness_dbfs | sha256 |
|---|---|---:|---:|---:|---:|---|
| `surf_rumble.wav` | surf_rumble | true | 2.000000 | 0.890 | -8.28 | `2bd139d9eb8e00473b111877ab33213db23c094507240f36c07c38d4ea70f74f` |
| `surf_grass.wav` | surf_grass | true | 1.547846 | 0.890 | -11.21 | `eb0295ab5d3ccf630e5cef7c869a509c7dcc6ae2dc27becc7e26dfd057ded57b` |
| `surf_sand.wav` | surf_sand | true | 2.174966 | 0.890 | -15.61 | `ba5020c2f6eb9e33b6665209d44ccef147610bc636cd23bf80f3a5291bc1b8c6` |
| `impact_hit_1.wav` | impact_hit | false | 0.802812 | 0.890 | -13.60 | `4f0fa91a560a835fbe2c10822bea10a74f1d4395505f6ea74959cffa2030f6b9` |
| `impact_hit_2.wav` | impact_hit | false | 3.791315 | 0.890 | -15.86 | `9b0a8d71b74308be22c0c36c9ffd4e97fa3410bb49b6e16d6f488492aacbdc30` |
| `impact_hit_3.wav` | impact_hit | false | 2.010816 | 0.890 | -17.36 | `646853434ff983ed3e80ed19a11a541ffaff0fb0b0c36800f4ecf8114e2add1f` |
| `impact_hit_4.wav` | impact_hit | false | 0.802812 | 0.890 | -13.86 | `ab73167de4c68c5fe87a334f774ee07bedbfe50b1d94684a639e1aa45da2fb0d` |
| `impact_barrier.wav` | impact_barrier | false | 2.443220 | 0.920 | -9.72 | `846e3156f14cfd869d780ebb2792199bb3289144edb30d621a0317e45292b4da` |
| `impact_cone.wav` | impact_cone | false | 0.351519 | 0.920 | -15.32 | `0e9cd6cb035dba091be061c216feda098669aa8a67d27e64e640e77c45da478a` |
| `impact_scrape.wav` | impact_scrape | false | 0.360000 | 0.920 | -29.59 | `fc668e217139129988fe3f85d27d35232e87ddfc76736d375b6c95ab656562e8` |
| `tyre_scrub.wav` | tyre_scrub | true | 2.300000 | 0.890 | -9.84 | `c1b40033fbe07638ffe8f99fffbb3ce47b56d62a72ddf033c08d8e2a971e4a03` |

Loop entries keep their existing `loop_start_s` / `loop_end_s` values verbatim.

`bank_manifest.json` for `commons`:

- `bank_name`: `"commons"`.
- `schema_version`: 1, `sample_rate`: 44100, `pcm_bits`: 16, `channels`: 1,
  `seed`: 0.
- `generator`: replace with an honest string such as
  `"formula90s commons bank (non-specific effects pruned 2026-09-20)"`.
- `files`: exactly the 11 entries above, copied verbatim from the current
  manifest (role, loop, loop bounds, duration, loudness, peak, sha256,
  dc_offset, provenance and any per-entry `synthesis` block retained).
- No engine/gearbox entries. `bank.rs` rejects `RETIRED_KEYS`; `commons` contains
  none of them, so no change to the retired-key prohibition is allowed.

---

## 4. `tc_cut` verification (required before removal)

`tc_cut` **is currently wired** into the backend. Exact sites:

- `game/crates/vehicle-audio-engine/src/state.rs`: `Trigger::TcCut => "tc_cut"`.
- `game/crates/vehicle-audio-engine/src/mixer.rs`: `Trigger::TcCut` is in the
  one-shot construction list, and the TC-cut rising edge fires
  `self.trigger(Trigger::TcCut)` in `set_telemetry_timed` (with the existing
  400 ms cooldown). The Grand Prix sampler does **not** own `TcCut`.
- `game/crates/vehicle-audio-engine/src/ffi.rs`: `trigger_from_code` maps 11 →
  `TcCut`.
- `game/crates/formula90-core/src/audio.rs`: `code_from_bank_key("tc_cut")` → 11
  and `trigger_from_code(11)` → `TcCut`.
- `native/src/core/f90_core.cpp`: no named `tc_cut` trigger; the TC cut reaches
  the mixer through `set_audio_ambient(tc_cut_ratio)` telemetry, and the mixer
  derives the edge.
- `game/crates/vehicle-audio-engine/src/state.rs` test
  `trigger_bank_keys_are_stable` asserts `Trigger::TcCut.bank_key() == "tc_cut"`.
- `game/sounds/sound_mixer_config.json` has a `tc_cut` sound entry.

Required handling when removing `tc_cut.wav`:

1. Removing the asset makes the one-shot a silent no-op: `trigger_at_rpm`
   returns early because no voice matches, and no error is raised. This is the
   accepted behaviour; do **not** add a fallback asset.
2. Preserve the public trigger ABI: keep `Trigger::TcCut`, code 11, and
   `code_from_bank_key("tc_cut")` stable. Do not renumber legacy codes.
3. Remove the `tc_cut` key from `sound_mixer_config.json` (§6).
4. Update the offline renderer `game/crates/formula90-core/src/bin/grand_prix_sampler_render.rs`:
   it currently triggers `Trigger::TcCut` at the 9.0 s mark of the acceptance
   sequence; drop that trigger (and its `code == 3 ? Hit1 : TcCut` branch) so
   the listening package does not claim a TC accent that no longer exists.
5. Update the listening-package documentation that lists `tc_cut` as preserved
   (`reports/audio-v10/grand-prix-sampler/listening_package.md`, §"Preserved
   effects" and the sequence description).
6. Verify with a grep after the change that the only remaining `tc_cut`
   references are the intended ABI/code-table mappings and the telemetry field
   (`tc_cut_ratio`), which is unrelated to the removed asset.

Consequence (accepted by the human): the TC-cut accent is silent in every
backend, including the Grand Prix sampler. GF509's engine envelope still reacts
to `tc_cut_ratio` telemetry internally; only the sampled accent disappears.

---

## 5. Required repository changes

### 5.1 Bank directory

1. Create `game/sounds/banks/commons/` with the 11 WAVs (byte-identical copies)
   and the new `bank_manifest.json`.
2. Delete `game/sounds/banks/v10_vehicle/` entirely: 17 removed WAVs, their 28
   `.import` companions, and the old `bank_manifest.json`. Deletion is
   intentional; recovery is possible from git history at HEAD `ce768b53...`.
3. Do not hand-edit `.import` files. After the move, run the Godot import step
   (`scripts/run_f1_94.ps1` runs `godot --headless --path game --import`) so
   `commons` gets fresh `.import` companions and the stale ones are gone.
4. Stage explicit paths only. `git add -A` is prohibited. Keep the move as a
   reviewable set: new `commons/` paths, deleted `v10_vehicle/` paths, and the
   edited source/config/test files listed below.

### 5.2 Reference update matrix (`v10_vehicle` → `commons`)

Update every runtime/config reference. Known sites (verify with a fresh
`git grep v10_vehicle` before finishing; the list may grow):

- `native/include/formula90s/core/f90_core.hpp`: default `bank_dir_res_`.
- `game/scenes/runtime/vehicle_test_session.tscn` and
  `game/scenes/runtime/vehicle_test_session_mp4_6_senna_1.tscn`: `bank_dir`.
- `game/crates/vehicle-audio-engine/src/lib.rs`: `DEFAULT_BANK_REL` and the
  module doc comment.
- `game/crates/vehicle-audio-engine/src/bank.rs`: module doc comment only
  (loader logic is path-agnostic).
- `game/crates/vehicle-audio-engine/src/ffi.rs`: test path (line ~318).
- `game/crates/vehicle-audio-engine/src/config.rs`: test path (line ~1510) and
  the module doc comment.
- `game/crates/vehicle-audio-engine/tests/bank_integration.rs`: bank path,
  `bank_name` assertion, expected-keys list, and header comment.
- `game/crates/formula90-core/src/lib.rs`: test/`CoreConfig` bank paths and the
  doc comment for the bank dir.
- `game/crates/formula90-core/src/audio_worker.rs`: test bank path (line ~570).
- `game/crates/formula90-core/tests/facade_audio.rs`: bank path.
- `game/crates/formula90-core/src/bin/v10_physics_transient_capture.rs` and
  `game/crates/formula90-core/src/bin/grand_prix_sampler_render.rs`: bank path.
- `tools/audio/render_audio_scenario.py`: `BANK_DIR` and header contract comment;
  `tools/audio/bank_validator.py`: CLI default; `tools/audio/bank_generator.py`
  and `tools/audio/bank_manifest.py`: `bank_name` default; `tools/audio/remaster_lib.py`
  and `tools/audio/preview_ab.py`: references.
- `scripts/audio/write_v10_001_006_manifest.ps1` and
  `scripts/audio/write_v10_007_011_reference_manifest.ps1`: bank manifest paths.
- `scripts/run_f1_94.ps1`: the audio smoke log string only (cosmetic).
- Documentation (optional but preferred): `docs/architecture/runtime-map.md`,
  `docs/audio-migration-map.md`, `docs/audio-tooling.md`,
  `docs/audio-v10/*`, `game/crates/vehicle-audio-engine/SOUND_CONFIG_DOCS.md`.
  Leave `archive/audio/v10_vehicle_continuous_backup_2026-08-29/` untouched.
- Do not touch `game/sounds/banks/v10-gp3/` or the Grand Prix sampler bank.

### 5.3 `sound_mixer_config.json`

Prune `game/sounds/sound_mixer_config.json`:

- `sounds`: keep exactly the 11 `commons` keys (`surf_rumble`, `surf_grass`,
  `surf_sand`, `impact_hit_1..4`, `impact_barrier`, `impact_cone`,
  `impact_scrape`, `tyre_scrub`). Remove `engine_*` (retired),
  `engine_starter`, `engine_start_backfire`, `engine_tc`, `exhaust-mic`,
  `int_backfire`, `int_backfire_2`, `backfire_3..7`, `limiter_hit`, `tc_cut`,
  `shift_up`, `shift_down`, `shift_3`, `shift_up_delayed`, `shift_down_delayed`,
  `impact_fire`.
- `reverb_buses`: keep `impact_short` (impacts), `surface_ambience` (surfaces)
  and `mechanical_small` (`tyre_scrub`); remove `exhaust_chamber`.
- Keep `defaults`, `exhaust`, `master`, `hot_reload` unchanged.
- The loader resolves this file from the bank directory's parent-parent
  (`game/sounds/banks/commons` → `game/sounds/sound_mixer_config.json`), so the
  file location does not move.
- Sanitization must still pass: `SoundMixerConfig` reports unknown/missing keys
  as warnings only; do not introduce new unknown keys.

---

## 6. Tests to update

- `game/crates/vehicle-audio-engine/tests/bank_integration.rs`:
  - bank path → `commons`;
  - `bank.bank_name == "commons"`;
  - expected-keys list → the 11 `commons` keys only;
  - keep the retired-key assertions and the loop/one-shot assertions;
  - `tyre_scrub_voice_activates_and_advances_on_loaded_bank` must still pass.
- `game/crates/vehicle-audio-engine/src/state.rs`: `trigger_bank_keys_are_stable`
  keeps asserting `Trigger::TcCut.bank_key() == "tc_cut"` (ABI mapping is
  intentionally retained even though the asset is gone).
- Any mixer tests that construct a dummy bank with removed keys must be checked;
  the dummy-bank tests are synthetic and should be unaffected, but the
  `impact_fire`/`tc_cut` expectations in real-bank tests must be removed.
- `tools/audio/tests/`: `render_audio_scenario`/`bank_validator` tests resolve
  `BANK_DIR`; update the constant and any expected-file lists. The two
  pre-existing failures (`test_bank_contract.py::test_retired_keys_are_absent`,
  `test_playback_metadata.py::test_runtime_manifest_declares_retired_keys_explicitly`)
  are unrelated (the clean HEAD manifest lacks `retired_keys`); do not "fix"
  them by editing the shared manifest unless the human asks.
- After the change, `cargo test -p vehicle_audio_engine --lib`,
  `cargo test -p formula90_core --lib`, and the Python audio tests must pass.

---

## 7. Build, runtime validation and acceptance

1. Rebuild the native extension because `f90_core.hpp` changes:
   `scripts/build_windows.ps1` (writes `game/BUILD_SOURCE = HEAD`, rebuilds
   SCons + Rust + DSP, runs ctest). Verify `BUILD_SOURCE == HEAD` afterwards.
2. Godot import runs inside `scripts/run_f1_94.ps1`; it regenerates the
   `commons` `.import` files.
3. Required runtime evidence with the active Grand Prix sampler profile:
   - `scripts/run_f1_94.ps1 -ValidateRuntimeOnly` → exit 0;
   - `scripts/run_f1_94.ps1 -SmokeAudio` → exit 0;
   - `scripts/run_f1_94.ps1 -Smoke` → exit 0;
   - `game/tests/probe_grand_prix_sampler_backend.gd` → exit 0,
     `audio_source_code=2` (proves the sampler bank still loads).
4. Acceptance criteria:
   - `commons` contains exactly the 11 listed files plus `bank_manifest.json`;
     `v10_vehicle` no longer exists;
   - every `commons` WAV hash matches §3;
   - `VehicleSoundBank::load("game/sounds/banks/commons")` succeeds and the
     integration test's expected keys all resolve;
   - surfaces, impacts (hit1..4, barrier, cone, scrape) and tyre scrub are
     audible in the runtime smoke and in the offline listening package;
   - `tc_cut`, `impact_fire`, backfire, starter and shift accents are gone and
     no code path errors on their absence;
   - `sound_mixer_config.json` contains only `commons` sound keys plus the
     retained buses/global blocks;
   - no stale `v10_vehicle` reference remains in source, scenes, config, tools
     or scripts (docs updated as far as practical).
5. Re-render the Grand Prix listening package after the `tc_cut` renderer edit
   so its sequence and documentation no longer claim the removed accents.

---

## 8. Constraints and protocol

- Follow `AGENTS.md`: fully self-explanatory names, no code comments, atomic
  staging with explicit paths, `git add -A` prohibited, never commit unless the
  human asks, never reset/switch/alter the dirty branch, verify
  `git diff --cached` before any commit.
- Do not modify the Grand Prix sampler bank, its builder, its profile section,
  or `v10-gp3`.
- Do not change public telemetry/trigger ABI layouts or legacy trigger codes.
- Do not weaken `RETIRED_KEYS` enforcement in `bank.rs`.
- If scope, intent or affected systems become unclear, stop and ask.

---

## 9. Risks and consequences (recorded)

- Removing `backfire_*`, `int_backfire*`, `limiter_hit`, `tc_cut`,
  `engine_starter` and `engine_start_backfire` means the legacy/GF509 backend
  loses its sampled backfire, limiter, TC-cut and starter accents. GF509 still
  synthesizes shifts and reacts to TC/limiter telemetry internally, but the
  sampled accents are gone. This is accepted; rollback to GF509 audio fidelity
  would require restoring those files from git history.
- `impact_fire` is removed even though the handoff that planned the Grand Prix
  sampler listed "fire" as preserved. The human decided not to preserve it; the
  one-shot voice simply will not exist.
- The five shift samples were already unused by the current mixer; removing
  them changes nothing at runtime.

---

## 10. Rollback

The entire pre-change state is recoverable from git at HEAD `ce768b53...`:
`git checkout ce768b53 -- game/sounds/banks/v10_vehicle game/sounds/sound_mixer_config.json`
(plus the edited reference files). Do not use destructive reset on the dirty
branch. Restoring the old bank also restores the removed accents for the GF509
rollback path.

---

## 11. Backlog for the next sprint

1. **COMMONS-01 Source inventory and removal plan.** Confirm the 11/17 split
   against a fresh `git grep`, record hashes, and freeze the `commons` manifest
   content.
2. **COMMONS-02 Create `game/sounds/banks/commons/`.** Copy the 11 WAVs
   byte-identically, write the new `bank_manifest.json`, delete
   `game/sounds/banks/v10_vehicle/` and its `.import` companions.
3. **COMMONS-03 Update runtime references.** Apply the §5.2 matrix; rebuild the
   native extension; reimport in Godot.
4. **COMMONS-04 Prune `sound_mixer_config.json`.** Keep only `commons` sound
   keys and the retained reverb buses/global blocks.
5. **COMMONS-05 Remove the dead accents and update consumers.** Handle
   `tc_cut` per §4 (no-op trigger, ABI preserved, renderer and docs updated);
   update `bank_integration.rs`, tools/tests and docs.
6. **COMMONS-06 Verification and evidence.** Run the §7 build/validation set,
   capture results under `reports/audio-v10/`, re-render the listening package,
   and prepare the review for human approval.
