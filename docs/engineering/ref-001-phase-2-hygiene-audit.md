# REF-001 Phase 2 — repository hygiene and reproducibility audit

## Scope and method

Audit commit: `6dcabceb2d3ad69b11313ec43ae9fec69307da4c`.

This is an evidence-only audit. It does not delete, untrack, regenerate, or
move assets. Every classification below was checked against Git tracking,
ignore rules, source consumers, producers, and current validation entry points.

## Inventory

- 1,013 tracked paths.
- 111 tracked Godot `.import` sidecars.
- 195 paths are both tracked and matched by an ignore rule:
  - 193 under `blender/generated/la_chutana/`.
  - 1 canonical runtime track GLB under `game/assets/generated/tracks/`.
  - 1 temporary text file under `tmp/`.
- No tracked `.obj`, `.dll`, `.exe`, or log artifact was found.
- `third_party/godot-cpp` is the only declared submodule and resolves to
  `7e18e40d7591429f915035a7de7cf79457d555cc`.
- Git LFS is not configured.

## Classification

| Class | Evidence | Decision in this phase |
| --- | --- | --- |
| Retain: canonical runtime track | `game/assets/generated/tracks/la_chutana/la_chutana.glb` is tracked and loaded by `game/scenes/tracks/test_field/la_chutana_generated.tscn`, then by `jordan_handling_test.tscn`. | Keep. It is required by a fresh checkout despite its generated provenance. |
| Retain pending policy: Texture Forge bank | 193 tracked outputs under `blender/generated/la_chutana/` have manifests, source manifests, a deterministic generator, and hash validation in `blender/track_pipeline/validate_texture_forge.py`. | Keep. Their overlap with `.gitignore` is a policy ambiguity, not evidence of debris. |
| Retain as local derived runtime input | Jordan scenes require three GLBs under ignored `game/assets/generated/jordan_1995/`. `scripts/run_jordan_handling.ps1` verifies the committed runtime ZIP hash and materializes exactly those three files. | Keep ignored. The test/bootstrap route must invoke the producer before loading the Jordan world. |
| Retain: build/cache outputs | `.tools/`, GDExtension DLLs, and native `.obj` files are ignored and untracked. | Keep ignored. No cleanup change is needed. |
| Candidate only: duplicated F1 2026 textures | Identical texture content appears in both `f1_2026` and `f1_2026_b`, including eight copies each of wheel normal and roughness maps. Both asset trees are consumed by distinct scenes and GLBs can contain local texture references. | Do not deduplicate yet. It needs an import/resource dependency migration and scene regression tests. |
| Confirmed temporary artifact, deletion deferred | `tmp/test_local_path_upload.txt` only contains `/mnt/data/veg_v2_hashes.json`, has no repository consumer, and was added by commit `5ce0508` for upload-path testing. | Do not delete in the audit. Schedule a small, separate cleanup commit after confirming no workflow still depends on it. |

## Reproducibility gaps confirmed by Phase 1

1. `scripts/test_windows.ps1` does not materialize the ignored Jordan runtime
   GLBs before running world/compositor smoke tests. A clean checkout therefore
   cannot execute that route even though the verified producer exists.
2. `native/tests/unit_tests.cpp` includes
   `formula90s/vehicle/physics_math.hpp`, but that header is absent from both
   `native/include/` and `native/src/`. The official test script stops at that
   stale or incomplete test dependency before it can run the rest of the suite.
3. The direct Godot smoke scripts depend on the editor import/class-cache pass
   that `scripts/test_windows.ps1` normally performs first. This is a valid
   ordering constraint and should be made explicit in the test contract.

## Policy decision needed before changing ignore rules

The repository currently uses two different delivery models:

1. Commit a canonical runtime output (La Chutana GLB) so a clone is runnable.
2. Keep generated Texture Forge outputs tracked but also matched by the broad
   `blender/generated/` ignore rule, while Jordan visual runtime GLBs are local
   outputs materialized from a committed bundle.

No ignore rule was changed because choosing between committed runtime outputs
and mandatory deterministic generation affects clone size, CI prerequisites,
and release reproducibility. The existing tracked files remain valid until that
policy is deliberately unified.

## Safe next increments

1. Add an explicit, non-destructive preflight to the official test route that
   validates the Jordan bundle and materializes its three canonical runtime
   GLBs when absent. Then rerun the world/HUD smoke suite from a clean worktree.
2. Trace the missing `physics_math.hpp` API and decide whether to restore a
   tested production header or remove/replace the stale unit test. Do not add
   a placeholder header merely to make the compiler pass.
3. Decide and document the source-of-truth/release policy for generated
   Texture Forge output before changing `.gitignore` or untracking any output.
4. Only after steps 1–3, evaluate the temporary upload-path file and F1 2026
   texture consolidation in focused commits.
