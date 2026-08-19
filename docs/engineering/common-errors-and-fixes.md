# Formula90s — Common Errors and Fixes

This document records failure patterns already encountered during Formula90s development. It is intended to reduce repeated debugging, token waste and accidental regressions in AI-assisted development.

Use it before brute-forcing a problem that resembles a previous failure.

## 1. Debugging rule: identify the subsystem before changing parameters

A visible driving problem can originate in several independent layers:

```text
vehicle physics
track collision
surface classification
visual geometry
generated asset pipeline
Godot import
controller / assists
camera / HUD
```

Do not assume every bad driving event is a tire or suspension problem.

Recommended diagnosis order:

1. reproduce consistently;
2. inspect telemetry;
3. determine whether wheels are in contact;
4. inspect track/collision geometry if contact becomes impossible or discontinuous;
5. inspect surface groups;
6. only then change vehicle parameters if the evidence points to the vehicle.

A fix that changes unrelated physics until the symptom disappears is not considered a robust fix.

---

## 2. Vehicle falls through the world after leaving asphalt

### Symptom

Typical telemetry pattern:

```text
all four suspension compression values → 0
slip values → 0
vehicle speed continues changing
vehicle never regains ground contact
```

In-game, the car leaves the track and falls indefinitely.

### Causes encountered

Several track-generation issues can cause this class of failure:

1. terrain collision mesh contains a gap;
2. grass/terrain collision faces have incorrect winding after coordinate conversion;
3. independent road and terrain concave meshes do not overlap safely at the transition;
4. a generated terrain topology removes cells beneath the road and accidentally creates an escape path;
5. imported collision is missing or not generated from the expected `-colonly` object.

A particularly important conversion is:

```text
Godot/data:   (x, z, height)
Blender:      (x, -z, height)
```

The sign inversion changes triangle winding. A triangle ordering that is upward-facing before this conversion can become downward-facing in Blender.

### Correct solution

The current track pipeline uses:

- a continuous single-valued terrain heightfield;
- explicit upward-facing Blender winding validation;
- a controlled collision underlay beneath the road;
- short edge collision bridges where needed;
- a large invisible `GrassSafetyFloor-colonly` below the playable world as a final failsafe.

The safety floor is not the primary ground. It exists only to turn a catastrophic infinite fall into a recoverable/debuggable state.

### Do not

- increase suspension length to hide the hole;
- increase tire grip;
- change chassis mass;
- globally modify GEVP;
- assume a visual ground surface automatically has valid collision.

### Verification

Drive deliberately:

```text
Road → Grass
Grass → Road
full off-track excursion
off-track on both sides of several corners
```

Telemetry should show normal suspension contact on the terrain rather than four wheels remaining at zero.

---

## 3. Car becomes violently stuck or receives an invisible impact on grass

### Symptom

Possible telemetry signature:

```text
large instantaneous longitudinal/lateral G spike
one or more suspension values suddenly near maximum
speed collapses in a few frames
car remains trapped or solver pushes it unpredictably
```

### Root cause encountered

The original grass terrain used wide normal-offset ribbons following the track centerline.

For a tight corner:

```text
offset distance > local curve radius
```

can make the offset curve fold over itself. Triangles then cross and create invisible wedges/collision walls.

La Chutana exposed this because the old outer shoulder offset was substantially larger than some local sampled corner radii.

### Correct solution

Use a regular terrain heightfield/grid or another topology that is mathematically prevented from folding over itself.

The terrain validator must check:

- finite vertices;
- non-degenerate triangles;
- expected winding;
- collision continuity;
- reasonable triangle budget.

### Do not

Recreate broad grass collision using independently offset left/right track ribbons unless the topology is explicitly proven safe for every curve radius.

---

## 4. Grass visually clips through asphalt

### Symptom

Green/dry terrain polygons appear through portions of the road, especially around banking or curves.

### Why it happens

Visual and collision requirements are different.

A coarse terrain heightfield interpolates across grid cells. If the visual terrain meets the road at exactly the same height, triangles can cross or z-fight with the asphalt even when the analytical edge height is correct.

Banking increases the problem because the low road edge can be significantly below the centerline elevation.

### Correct solution

- collision terrain meets the road boundary correctly;
- visual terrain receives a small sink;
- visual terrain is pushed farther beneath the road inside the asphalt corridor;
- it blends back toward terrain height outside the road edge;
- road elevation and terrain behavior are generated together rather than translated independently in Godot.

### Important distinction

```text
visual sink ≠ collision step
```

Never copy a visual z-fighting workaround directly into the collision surface.

---

## 5. Grass cards appear on the asphalt

### Symptom

Individual vegetation cards visibly protrude through or sit inside the road even though the underlying terrain is correct.

### Root cause

Procedural placement was originally reasoned mainly from centerline distance. That does not guarantee enough clearance from the actual asphalt edge, especially when the card has non-zero width.

### Correct solution

Placement zones are expressed from the real road edge using explicit category clearances.

For example, grass may start close to the road while bushes, trees and buildings require progressively larger clearances.

Validation must consider:

```text
center distance
- road half width
- asset footprint/radius
>= minimum edge clearance
```

### Do not

Reduce the card scale globally just to stop asphalt intrusion. Fix the placement mask/clearance.

---

## 6. Procedural vegetation looks uniformly scattered

### Symptom

The scene looks computer-generated even with enough assets:

- objects are evenly distributed;
- every area has similar density;
- there are no natural visual gaps or clusters.

### Cause

Uniform independent sampling around the lap creates statistical coverage, not scenic composition.

### Correct solution

Use deterministic clustered placement:

- cluster centers derived from seed;
- local jitter around centers;
- some open regions intentionally retained;
- large objects placed first;
- spatial-hash overlap rejection retained.

The goal is not ecological simulation. The goal is readable late-1990s racing scenery.

---

## 7. `medium` density looks like `low`

### Symptom

A density label does not match the visual expectation. In the first procedural pass, `medium` still appeared sparse, especially for grass.

### Lesson

Density names are user-facing artistic controls, not sacred numeric values.

They must be calibrated visually.

Current direction intentionally makes grass much denser because one-plane grass cards are inexpensive and need mass to read properly.

### Correct process

1. generate a known circuit with a fixed seed;
2. capture the same camera positions;
3. compare `very_low`, `low`, `medium`, `high`;
4. remap counts until each label is visually meaningful;
5. keep seed and all other variables constant while calibrating.

---

## 8. Bushes look like tiny trees or have insufficient volume

### Symptom

Bushes are hard to distinguish from grass or look like miniature upright trees.

### Correct art contract

Bushes should be:

- two crossed planes;
- visibly wider than tall;
- broader silhouettes;
- more internally filled texture alpha;
- a palette offset from tree foliage;
- used in clusters more often than isolated tall vegetation.

Fix width, silhouette and texture mass before increasing geometry complexity.

---

## 9. Trees look thin despite having multiple planes

### Symptom

A three-plane tree still looks sparse or obviously made from cards.

### Cause

Adding planes does not compensate for a weak texture silhouette.

### Correct solution

The texture should carry:

- strong outer silhouette;
- filled canopy mass;
- fake internal shade;
- brighter lit facets;
- darker lower/contact region;
- enough trunk contrast to remain readable.

Formula90s Texture Forge applies deterministic fake lighting/AO and silhouette processing. Improve the base card/recipe before moving to a full 3D crown.

---

## 10. Procedural structures are visually insignificant

### Symptom

Buildings exist but are too small to contribute to skyline or depth.

### Cause

Nominal real-world-looking dimensions may not read strongly enough through the game's camera/FOV and retro scene composition.

### Correct solution

Treat structures as scenic forms. Calibrate their footprint and height against actual gameplay screenshots, not only meters in isolation.

Keep geometry simple while increasing readable scale.

Do not respond by adding many more tiny structures; fewer stronger shapes fit the art direction better.

---

## 11. Right-side wheels are mirrored/flipped incorrectly

### Historical symptom

Vehicle wheel visuals could become misaligned, inverted or require fragile per-wheel fixes.

### Stable contract

A car should normally use only:

```text
chassis.glb
wheel_front.glb
wheel_rear.glb
```

The same front mesh is reused left/right. The same rear mesh is reused left/right.

Physics still has four distinct RayCast wheels.

For the opposite side, use a dedicated orientation node/rotation rather than negative scale.

### Why negative scale is avoided

Negative scale can introduce:

- reversed winding/normals;
- confusing inherited transforms;
- exporter/importer differences;
- debugging ambiguity.

### Do not

Create four permanently duplicated wheel assets solely to solve side orientation.

---

## 12. Wheel visual position and wheel physics position drift apart

### Symptom

The tire mesh appears correct while the physical RayCast is elsewhere, or a calibration script temporarily hides the mismatch.

### Correct solution

Physics wheel positions are canonical. Visual hierarchy must follow the canonical positions instead of introducing persistent compensating offsets.

Avoid leaving temporary calibrators in the final hierarchy once the contract has been established.

---

## 13. Curbs launch the Formula car

### Symptom

Touching a curb produces unrealistic vertical launch or violent chassis contact.

### Cause

Common causes include:

- rectangular curb blocks;
- excessive curb height;
- abrupt profile transitions;
- collision geometry different from visual geometry;
- chassis/raycast interaction with a sharp leading edge.

### Current contract

Use a conservative low crowned profile with gradual transitions. The current La Chutana development profile is roughly 0.58 m wide with a maximum rise around 22 mm.

The exact values may evolve, but the principle does not:

> Formula90s curbs must be smooth enough to test tire/suspension behavior rather than behave as ramps.

### Do not

Globally soften the vehicle suspension merely because one curb mesh is malformed.

---

## 14. Guardrail visual geometry causes bad collision

### Symptom

Vehicle catches on posts, corrugation or tiny visual details.

### Cause

Using the detailed visual mesh directly as collision.

### Correct solution

Separate:

```text
guardrail visual module
+
simple box-like collision proxy
```

Generated collision objects use the `-colonly` naming contract so Godot imports them as collision-only geometry.

The collision should follow the guardrail route but not reproduce every post or corrugation.

---

## 15. Surface physics are wrong even though geometry looks correct

### Symptom

Grass behaves like asphalt, curb behaves like road, or guardrail does not behave as wall despite visible geometry being present.

### Cause

GEVP behavior depends on surface/group identification. Generated Blender collision nodes must be restored to the expected Formula90s groups after import.

Expected categories include:

```text
Road
Curb
Grass
Wall
```

The project-owned generated-track surface tagger performs this mapping based on imported names.

### Diagnostic

Inspect the imported `StaticBody3D` names and groups before changing friction values.

---

## 16. Spawn position changes sign after Blender generation

### Historical cause

Godot track data and Blender scene coordinates use different planar conventions.

The conversion currently follows:

```text
Godot/data x → Blender x
Godot/data z → Blender -y
height       → Blender z
```

A hardcoded spawn that bypasses the same conversion can appear on the wrong side of start/finish.

### Correct solution

Generate markers through the same coordinate conversion utility as the centerline/track geometry.

Never independently reinterpret axes for spawn, guardrails or procedural placement.

---

## 17. Procedural run accumulates duplicate trees and scenery

### Symptom

Each regeneration makes the scene denser even with the same seed.

### Cause

Decorating the previous `track_environment.blend` instead of starting from the clean Base.

### Correct solution

Every Procedural run must:

```text
open track_base.blend
→ apply current placements
→ save new track_environment.blend
```

The previous environment blend may be backed up but must not be used as the source for a fresh generation.

Same config + same seed should not accumulate state.

---

## 18. A failed export destroys the working runtime track

### Dangerous pattern

```text
delete old GLB
→ attempt export
→ Blender fails
→ no playable track remains
```

### Correct solution

Use atomic publication:

```text
export temporary/new file
→ confirm export completed
→ replace canonical GLB
```

The last known-good canonical asset must survive a failed build.

The same principle applies to `.blend` sources: back up an existing source before replacing it.

---

## 19. Generated `.blend` or GLB appears missing from Git

### Symptom

A local track exists but GitHub does not contain the `.blend` or generated runtime asset.

### Explanation

This can be intentional. Raw/generated Blender files and generated runtime outputs are excluded to keep the repository small and reproducible.

The repository versions:

- source reference data;
- deterministic scripts;
- configuration;
- recipes;
- validators.

The local pipeline generates large binary artifacts.

### Correct action

Run the appropriate pipeline command locally rather than attempting to commit every generated binary.

---

## 20. `bpy` import fails in normal Python

### Symptom

```text
ModuleNotFoundError: bpy
```

### Explanation

Blender-building scripts are intended to be executed by Blender's Python runtime, not the regular project virtual environment.

Normal Python handles:

- NumPy/SciPy/OpenCV/Pillow transforms;
- track preparation;
- placement;
- validation;
- Texture Forge.

Blender executes scripts that import `bpy`.

### Correct command path

Use `scripts/run_track_pipeline.ps1`, which invokes the correct runtime for each stage.

---

## 21. Texture style changes between regenerations

### Symptom

Same conceptual asset looks different every time an agent regenerates it.

### Root problem

Allowing AI/image generation to act as final texture authority without a deterministic post-process.

### Correct solution

Formula90s Texture Forge makes the final texture a deterministic build output.

Authority becomes:

```text
source/base
+ biome
+ recipe
+ seed
+ forge version
= final PNG
```

The Texture Forge manifest records recipe and file hashes. Validation should fail if the generated texture bank does not match the declared outputs.

### Do not

Manually edit one generated runtime PNG and rely on it remaining correct after the next regeneration.

---

## 22. AI spends repeated iterations solving an already-known class of problem

### Symptom

An agent attempts several parameter changes without improving the issue, or a previously solved bug class reappears under a new visual symptom.

### Required response

Stop brute-force iteration and classify the failure.

Ask:

1. Has this symptom appeared before?
2. Is there a relevant entry in this document?
3. Is the current telemetry consistent with a geometry, contact, surface or physics failure?
4. What is the smallest subsystem that can own the fix?
5. Is a validator missing that would prevent this regression automatically?

If the same bug can recur, the preferred solution is:

```text
fix
+ validator/guard
+ documentation
```

not only `fix`.

---

## 23. Physics tuning changes several families simultaneously

### Symptom

After a change the car feels different, but it is impossible to know whether the cause was tires, steering, suspension, differential or aero.

### Correct development rule

Tune one behavioral family per phase.

Current sequence:

```text
Phase A geometry                       complete
Phase B mechanical grip/brakes/diff    baseline complete
Track validation                       current gate
Phase C steering/countersteer
Phase D suspension
Phase E V10/powertrain
Phase F aerodynamics
Phase G assists / true no-assists
```

Do not combine Phase C steering changes with tire or aerodynamic tuning.

This is especially important in AI-assisted development because a multi-variable patch can appear successful while concealing a regression.

---

## 24. Vehicle physics are retuned to compensate for track defects

### Symptom

Examples:

- adding grip because grass collision throws the car sideways;
- increasing suspension travel because the terrain has holes;
- adding stability assistance because a curb collision creates a spin;
- reducing chassis sensitivity because guardrail collision catches the vehicle.

### Rule

Fix the owning subsystem.

```text
broken track geometry → track fix
wrong surface tag     → import/group fix
bad curb mesh         → curb fix
bad guardrail proxy   → collision fix
actual handling issue → vehicle tuning
```

A track must be mechanically trustworthy before it is used as evidence for vehicle tuning.

---

## 25. Vendor GEVP edits solve a local Formula90s issue but create global uncertainty

### Risk

Editing vendor code makes it difficult to determine whether a behavior belongs to upstream GEVP or Formula90s.

### Preferred order

1. project-owned configuration;
2. project-owned subclass/controller;
3. adapter/wrapper;
4. scene composition;
5. vendor modification only when the issue is genuinely in the vendor layer and the change is intentionally maintained as a fork.

Do not modify a known upstream-restored vendor file to solve an art, track or UI problem.

---

## 26. Fast telemetry interpretation reference

Telemetry does not prove every cause, but some signatures are useful.

### Four suspension compressions at zero for sustained time

Likely:

```text
vehicle airborne
or
no valid wheel ray contact
or
falling through world
```

If it happens after leaving the circuit and persists indefinitely, inspect terrain collision first.

### One or more compressions near maximum with abrupt G spike

Likely:

```text
hard geometry impact
collision wedge
bottom-out
sharp curb/terrain discontinuity
```

Inspect geometry before changing grip.

### High slip while suspension contact remains normal

More likely to be a genuine tire/handling event than a missing collider.

### Sudden behavior change exactly at a surface boundary

Check surface group/classification and physical transition geometry.

---

## 27. Regression checklist before accepting a track pipeline change

For Base generation:

- track length validation passes;
- centerline has no self-intersection;
- terrain vertices are finite;
- terrain triangles are non-degenerate;
- Blender-facing winding is correct;
- collision terrain is continuous;
- safety floor configuration is valid;
- road/grass transition is drivable;
- curbs do not launch the car;
- Road/Curb/Grass groups behave distinctly;
- start/spawn is correct;
- canonical GLB remains valid if generation fails.

For Procedural generation:

- Texture Forge validates hashes/recipes;
- placement uses expected biome;
- no object overlaps;
- no vegetation invades road clearance;
- vegetation has no unintended collision;
- guardrail proxies exist where visual guardrails exist;
- same config + seed reproduces placement;
- procedural run starts from Base;
- visual density matches its semantic label.

---

## 28. When to add a new entry to this document

Add a troubleshooting entry when at least one condition is true:

- the problem consumed significant debugging time;
- the symptom could easily be misclassified;
- an AI agent is likely to repeat the wrong fix;
- the failure can recur in another track/vehicle;
- a deterministic validator can prevent it;
- the correct solution depends on a project-specific contract that is not obvious from general Godot/Blender knowledge.

Record:

```text
symptom
observable evidence
root cause
correct fix
what not to do
validation method
```

The purpose is not to archive every bug. It is to preserve reusable engineering knowledge.

---

## 29. `#` comments corrupt a Godot `.tscn` scene

### Symptom

A scene loads without a parser error, but a child script or property appears on
the root node. Later nodes may be absent at runtime even though their text is
visible in the `.tscn` file.

### Root cause

Godot text scenes use `;` for comments. A line beginning with `#` is parsed as
scene data, and subsequent node headers can be consumed as malformed root-node
properties instead of beginning a new node.

### Correct solution

Use `;` for explanatory and `FUTURE_*` comments in `.tscn` files, then run a
scene-load/smoke test that verifies the expected child hierarchy.

### Do not

Do not assume a successful text diff or a lack of an immediate parser error
means the scene hierarchy was preserved.

### Validation

`game/tests/smoke_test_arcade_hud_scene.gd` loads the HUD and verifies its map,
speed gauge, and notification Label exist as distinct child nodes.

---

## 30. HUD controls extracted from a world scene lose their runtime references

### Symptom

After the world/HUD compositor moves HUD controls from `WorldViewport` into a
root `CanvasLayer`, minimap/vehicle or aid bindings are null at runtime, or a
feature works in one track scene but not on the default bootstrap route.

### Root cause

Controls extracted from a world scene into a root CanvasLayer require explicit
runtime references. Cross-tree relative `NodePath`s are fallback-only: once a
Control is reparented out of the world tree, paths that were valid in the
original scene may resolve to nothing. In particular, a track scene without a
`DrivingAids` controller silently breaks every path that consumes aid state,
because the compositor treats a missing controller as "no aids" instead of
failing loudly.

### Correct solution

- The compositor must inject runtime references (`minimap.set_target(vehicle)`)
  after reparenting, keeping relative `NodePath`s only as isolated-scene
  fallback.
- Every default/gameplay route scene must expose the full HUD contract
  (VehicleRigidBody + DrivingAids), not only the primary handling-test scene.
- Cover the default route with a bootstrap smoke test and a visual capture
  test; a passing isolated-scene test is not evidence for the default route.

### Do not

Do not assume a HUD bug is fixed because source inspection looks correct or
because the handling-test scene passes. Run the default-route tests.

### Validation

`smoke_test_bootstrap_world_hud_compositor.gd` (default route movement test)
and `visual_test_arcade_hud_capture.gd` (1280x720 capture with all HUD
elements) must pass together.

---

## 31. Promoting a single-GLB vehicle to canonical 3-GLB (Jordan197)

### Symptom

A new `*_LOD0_Historical.glb` appears as a single 70-object scene (931 components, 7050 verts) and must replace the `jordan_191` 4-GLB canonical set without breaking `test_field.tscn`, `run_jordan_handling.ps1`, `WorldHudCompositor` or smoke tests.

### Difficulties encountered

1. **931 tiny components** — `analyze_mesh` reports `components 931`, many `1-face` decoratives. `mesh_components` classifier `confidence 0.0` for most; destructive split by face-count loses material.
2. **Hardcoded `VehicleRigidBody` paths** in 6 places (`world_hud_compositor.gd:6`, `handling_tuning_panel.gd:6`, `smoke_*`, `run_jordan_handling.ps1:10`). Next promotion would re-edit 4 files.
3. **GLB import cache** — `Trimesh` export produces `*.glb` + embedded `*.png` without `.import` descriptors; `godot --headless --import` must run before `smoke_test` else `No loader found` `PackedScene`.
4. **Track/wheelbase faithful vs frozen** — `197` `front_track 1.762 / rear 1.748 / wheelbase 3.073 / radius 0.324` differs `+19%/+6.7%/-6.7%` from `191` `1.478/1.437/2.878/0.3473`. Choosing faithful changes handling; preserving frozen hides asset.
5. **Z-forward convention** — the canonical asset is front `+Z`, while GEVP and the chase camera use front `-Z`. Godot import does not prove or own this semantic conversion. The vehicle scene must apply a proper yaw 180 (`diag(-1,1,-1)`, determinant `+1`) to `ChassisVisual`; this is not negative-scale mirroring. Tyre datums remain in asset space and must be transformed before comparing them with RayCast positions.
6. **HandlingTuningPanel export path** — default `Jordan191` breaks `Jordan197` smoke `vehicle_path` check, but fallback `_find_vehicle_in_scene` `handling_tuning_panel.gd:234` still finds vehicle; test strict equality is the failure, not runtime.

### Correct solution (this iteration)

*   Skill `3d-asset-generation` `ANALYZE → mesh_components (filter geometry, not name) → spatial_query --surface → vertex_regions → validate` — chassis = 29 non-wheel geometries, wheels = single-sided `LP_TYRE_LF (+spokes/rim/hub)` merged and centered `centroid 0.881` subtracted, not `LF+RF` double-wheel.
*   `REF-002 VehiclePathResolver` `game/addons/formula90s/scripts/vehicle_path_resolver.gd:1` with `CANDIDATE_PATHS [Jordan197,Jordan191,VehicleController]` + recursive `Vehicle` search; `WorldHudCompositor` delegates, `run_jordan_handling.ps1:2` parametrized `VehicleId=jordan_197` default, manifest-driven assets.
*   After `trimesh` export, run `godot --headless --import` to generate `*.glb.import` + `*.png.import` before any `smoke_test`.
*   Decision `J197-001`: faithful `track/wheelbase/radius` (asset truth) over frozen `191`; document both in `PROJECT_STATE.md §5.1` `vehicle_manifest.json`.
*   Declare `coordinate_contract`: asset front `+Z`, runtime front `-Z`, conversion owner `vehicle_scene_visual_yaw_180`.
*   Rotate `ChassisVisual` 180 degrees around Y while preserving its vertical offset; do not rotate/re-export the canonical GLB to hide scene ownership.
*   Rotate opposite-side wheel `Visual` `Transform3D(-1,0,1)` not `scale -1` (`COMMON_ERRORS.md:352`).
*   Relax smoke `vehicle_path` strict check to allow `Jordan197` or fallback `_vehicle != null`.
*   Extend the smoke beyond presence/material checks: require forward dot `>=0.99`, transformed axle datums within `0.01 m`, and correct front/rear wheel PackedScenes.

### Antipattern remediation (high priority)

*   Centralize `VehiclePathResolver` / `ProjectSettings vehicles/canonical_id` or `Group "vehicle"` — no new promotion should edit `>1` file (`REF-002 priority 98`).
*   Generate `assets-lowpoly-python/manifest.json` from script, include `Jordan197` (currently missing `manifest.json:1` 9 assets).
*   Keep `input/working/output/reports` separation `SKILL.md:137`; never overwrite `Historical.glb`.

### Validation

*   `analyze_mesh` `chassis 1.567/4.621 / wheel_front 0.307/0.648` matches `0.324*2`.
*   `smoke_test_jordan_197_handling_scene PASS`, `smoke_test_jordan_191 PASS`, `smoke_test_bootstrap_world_hud_compositor PASS` (now `Jordan197`).
*   `godot --headless --editor --quit DONE`.
*   Visual capture from the chase camera shows the rear wing nearest the camera, nose toward the track, and wider rear tyres on the rear axle.

### Follow-up retrospective

The complete `J197-VIS-001` root-cause analysis, rejected hypotheses, validation
environment failures and permanent agent guardrails are documented in
`docs/troubleshooting/jordan-197-orientation-retrospective.md`.

---

## 32. AgentDB unavailable because the execution sandbox cannot write SQLite state

### Symptom

An agent cannot run `agentdb problem`, `agentdb knowledge` or the bootstrap
scripts. The resolver reports `agentdb no disponible`, even though
`.agents/runtime/target/release/agentdb.exe` exists. A direct binary invocation
may report `unable to open database file`, while the fallback Cargo build fails
with `target/release/.cargo-lock: Access denied`.

### Root cause

The current execution identity can read `.agents`, but cannot write
`.agents/data/agents.db` or its SQLite journal/WAL files. `Test-AgentDbBinary`
therefore rejects an otherwise valid release binary and `Resolve-AgentDb` falls
through to an unnecessary `cargo build --release`, which requires write access
to the protected Cargo target directory.

### Correct solution

1. Stop the feature or debugging task; AgentDB is a prerequisite for agent work.
2. Run the existing release binary and preflight with an execution context that
   has write access to the configured AgentDB database, or repair the ACL for
   the actual execution identity.
3. Validate in order:

   ```text
   agentdb stats
   agentdb validate
   .agents/scripts/08-smoke-test.ps1
   ```

4. Continue only after `PREFLIGHT: PASS`, `HEALTHCHECK: PASS` and
   `SMOKE TEST: PASS`.

### Do not

- Do not rebuild Rust blindly when a valid release binary already exists.
- Do not delete, recreate or silently switch the AgentDB database to hide an
  access failure.
- Do not report “no known problem” when the problem lookup itself could not run.
- Do not proceed with vehicle, physics or gameplay changes while AgentDB remains
  unavailable.

### Validation

The required evidence is a successful `00-preflight.ps1`, `01-health.ps1` and
`08-smoke-test.ps1`. A problem query returning an empty result is valid only
after the command has executed successfully with `{"mode":"indexed"}`.
