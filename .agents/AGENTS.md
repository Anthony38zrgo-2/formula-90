# Formula-90 Agent Guide

## Canonical principles

1. Fail hypotheses before implementations.
2. Prefer falsification over confirmation.
3. Never patch when a cheaper experiment can reject the hypothesis.
4. One experiment normally changes one causal dimension.
5. Same hypothesis plus same failure signature means stop implementation.
6. Every failure must reject a hypothesis, verify a fact, narrow the search,
   change the failure signature, or identify ownership.
7. Capture a baseline before meaningful behavioral changes.
8. Validate immediately after the smallest change.
9. Roll back failed candidates.
10. Remove falsified assumptions from active context.
11. Escalate on evidence stagnation, not frustration.
12. Understand enough to experiment; do not analyze the whole repository by default.

Fail Fast is not "code fast". Adapt Fast is not random strategy switching. Adaptation
must follow evidence learned from the previous experiment.

## Required workflow

### Preflight

Before a risky or behavioral change, determine the task type, affected subsystem,
risk, required skill, acceptance criterion, baseline, ownership, cheapest
falsification test, immediate validation command, and rollback strategy.

Trace who owns, computes, mutates, consumes, and serializes the relevant state.
Patch the owning subsystem, not merely the visible symptom.

### Hypothesis protocol

```text
HYPOTHESIS
EVIDENCE
FALSIFICATION TEST
EXPECTED SIGNAL
FAILURE SIGNATURE
```

States are `UNTESTED`, `SUPPORTED`, `FALSIFIED`, `INCONCLUSIVE`, and
`SUPERSEDED`. Parameter sweeps within one causal explanation remain one hypothesis.

### Attempt budget

- **Attempt 0 — no implementation:** observe, capture baseline, identify ownership,
  form a hypothesis, and run the cheapest falsification test.
- **Attempt 1 — micro-patch:** allowed only when the hypothesis survives Attempt 0.
  Change one causal variable or tightly coupled group and validate immediately.
- **Attempt 2 — second hypothesis/final local iteration:** allowed only with new
  evidence, a materially changed hypothesis, or a changed failure signature.

If the same failure signature survives Attempt 2, stop implementation and enter
Diagnostic Mode. A third blind implementation is forbidden.

### Validation and rollback

Validate the smallest affected surface immediately. A failed candidate must produce
information, then be rolled back unless independently useful and justified.

```text
BASELINE -> HYPOTHESIS -> EXPERIMENT -> MICRO-PATCH -> CANDIDATE
         -> DELTA -> PASS | FAIL | INCONCLUSIVE
```

A behavioral candidate without a relevant baseline is normally `INCONCLUSIVE`.
Structural or binary fixes may use direct pass/fail evidence.

## Stop conditions and Diagnostic Mode

Stop production implementation when the same signature survives two implementation
attempts, no measurable acceptance criterion exists, ownership is unknown, the next
change cannot be isolated, evidence contradicts the patch, a workaround would be
stacked, baseline noise hides the regression, validation tooling is broken, or
active context is contradictory.

Diagnostic Mode means **no production patches**. Inspect code, configuration,
history, known incidents and telemetry; run parsers, offline models, minimal
reproductions and isolated tests. Return to implementation only after evidence
supports a causal micro-patch.

## Domain routing

- handling/chassis/tires/suspension/brakes: `vehicle-physics`
- physics failure diagnosis: `physics-diagnostics`
- aerodynamics: `aerodynamics`
- engine/transmission: `powertrain`
- telemetry: `telemetry`
- `.tscn`: `scene-safety`
- known failures: `problem-lookup`
- uncertain APIs: `knowledge-query`
- baseline/candidate comparison: `regression-validation`
- unclear ownership/cross-system flow: `repo-analysis`
- repeated signature/workaround pressure: `problem-solving-guardrails`
- delegation: `context-handoff`
- falsified hypothesis/stale context: `context-garbage-collection`

## 3D Asset Tooling

Use `3d-asset-generation` for any 3D asset task. Classify the task before
selecting a tool:

```text
ANALYZE  MODIFY  SIMPLIFY  SEPARATE  GENERATE
TEXTURE  UV  VALIDATE  EXPORT
```

The default deterministic hierarchy is:

```text
NumPy / SciPy
        -> Trimesh
        -> PyMeshLab
        -> Pillow / OpenCV
        -> Open3D when a concrete advantage is demonstrated
        -> Blender headless only for final operations
```

Responsibilities are intentionally separated:

- NumPy is the intermediate representation for vertices, faces, masks,
  transforms, vectors, and numerical classification.
- SciPy provides `scipy.spatial.cKDTree` for nearest-neighbor, radius queries,
  spatial label propagation, proximity clusters, and cross-mesh comparison.
  Do not write nested O(n^2) distance loops when a KD-tree applies.
- Trimesh handles scene understanding, connected components, bounds, centroids,
  area, volume, raycasting, surface proximity, intersections, transforms, and
  validation. For real distance to a surface, query triangles with Trimesh;
  the nearest vertex is only an approximation.
- PyMeshLab handles deterministic mesh changes: cleaning, repair, normals,
  decimation, remeshing, UV, vertex colors, and attribute transfer. Identify
  the filter and parameters first, run on a working copy, measure the delta,
  and validate immediately.
- Pillow handles simple procedural textures, palettes, masks, and PNG export.
  OpenCV is for advanced masks, segmentation, morphology, edges, and image
  transforms. Keep image logic separate from geometry logic.
- Open3D is reserved for point clouds, ICP/registration, advanced normal
  estimation, and geometry comparisons where NumPy/SciPy/Trimesh are not enough.
- Blender is not the primary geometry editor. Use Blender headless for complex
  materials, baking, visual review, scene assembly, format-specific conversion,
  or final GLB/glTF export when the Python mesh libraries cannot provide it.

Never modify unknown geometry blindly.

Before modifying an asset, produce a minimum report containing the file and
format, object/vertex/face/component counts, bounding box, X/Y/Z dimensions,
center, approximate orientation, materials, UV state, vertex colors, normals,
disconnected components, and manifold/non-manifold status where applicable.
Detect regions from geometry before changing them. Do not use hardcoded vertex
indices unless the asset was generated by this pipeline and the index contract
is explicit. Names are supporting evidence, never the sole classification truth.

Automatic classification must emit:

```text
region
classification
confidence
evidence
```

Low-confidence regions must not receive destructive modifications. Prefer
feature vectors containing centroid, bounding-box dimensions, area, volume,
vertex/face counts, distance to the vehicle center, height, orientation,
symmetry, circularity, and aspect ratio. Vehicle labels such as wheel, rim,
front wing, rear wing, floor, cockpit, body, suspension, and mirror are
hypotheses supported by those features.

After every relevant modification, compare BEFORE and AFTER and check vertex
and face counts, components, bounds, dimensions, normals, degenerate faces,
duplicate vertices, non-manifold edges, UV integrity, and material assignments.
Use `tools/asset_pipeline/requirements.txt` with the isolated interpreter at
`tools/asset_pipeline/.venv/`; never install this stack into global Python.

Use `input/`, `working/`, `output/`, and `reports/` stages. Never overwrite the
original asset. Cache expensive analysis by SHA-256 under
`reports/<asset_hash>/analysis.json`. Prefer reusable parameterized scripts in
`.agents/skills/3d-asset-generation/scripts/` over one-off fix scripts.

For low-poly assets, preserve this order where applicable:

```text
clean -> remove degenerates -> identify components -> preserve key regions
-> decimate -> recalculate normals -> validate silhouette -> UV
-> vertex colors/textures -> GLB
```

Prioritize silhouette, proportions, recognizable geometry, major aerodynamic
elements, wheel placement, and clean shading over microdetail. If a modification
fails twice or produces unexpected geometry, stop brute-force iteration and
inspect, re-analyze, determine ownership, and change strategy.

For CAD-derived assets, use `CadQuery -> PyMeshLab -> Blender`; for ordinary
assets, use the smallest applicable tool in the hierarchy above.

## Vehicle visual coordinate contract

For every new or replaced 3D vehicle, prove the asset/runtime coordinate
relationship before editing physics or accepting a smoke test:

1. Read `coordinate_contract` and `validation_datums` from the manifest. If the
   contract is absent, derive and document it before integration.
2. Derive `asset_forward` from measured geometry (`nose - tail`) or from
   `front_axle - rear_axle`; object names alone are insufficient.
3. Derive `physical_forward` from the centers of the front and rear RayCast
   pairs. Do not assume that an importer changes semantic forward axes.
4. Declare exactly one conversion owner: export pipeline, visual scene, or a
   dedicated presentation node. Multiple compensating rotations are forbidden.
5. Require `normalized_visual_forward.dot(normalized_physical_forward) >= 0.99`
   and transformed axle datums within the documented tolerance before runtime
   acceptance.
6. Verify that front/rear wheel instances use their corresponding canonical GLB.
   Resource existence, surface counts and materials alone are not sufficient.
7. A proper 180-degree yaw has determinant `+1`; it is not the prohibited
   negative-scale mirroring used to fake side orientation.

If an orientation defect is visible at rest, in neutral, and at zero speed,
falsify visual assembly and coordinate ownership before invoking
`vehicle-physics`, suspension tuning, grip changes, or GEVP patches.

The canonical Formula-90 import contract is defined in
`docs/vehicles/VEHICLE_IMPORT_STANDARD.md`:

```text
asset/runtime: front -Z, up +Y, right +X
runtime:       chassis GLB + four independent FL/FR/RL/RR wheel GLBs
hierarchy:     Wheel -> {Corner}Wheel -> Visual
conversion:    applied before export; no runtime correction node
```

Any vehicle that intentionally differs must declare the exception in its
manifest and provide an equivalent semantic smoke test.

## Godot validation environment

Before launching Godot headless from a sandbox or automation:

- use the console executable so stdout/stderr are captured;
- ensure the effective `user://` log directory is writable, normally by setting
  task-local `APPDATA` and `LOCALAPPDATA` under the workspace, or use an approved
  unsandboxed execution when that is the established workflow;
- check for stale Godot processes before treating an access error as a test result;
- classify a crash before test assertions as `INCONCLUSIVE`, not candidate FAIL;
- if output contains `Failed to open user://logs`, fix the validation environment
  before investigating production code;
- distinguish read failure from value mismatch: never compare a null/absent hash
  after `Get-FileHash`, parser, or file-open failure.

If `agentdb` is unavailable, search `docs/troubleshooting/resolved-incidents.md`
and `game/docs/COMMON_ERRORS_AND_FIXES.md`, then record the lookup as unavailable.
Do not report “no known incident” when the lookup tool itself did not run.

## Safety and context

- Run every repository Git operation through Git Bash. On Windows use
  `C:\Program Files\Git\bin\bash.exe`; do not invoke Git from PowerShell.
- Preserve unrelated dirty-worktree changes.
- Never promote, escalate, hand off, delegate, or transfer any task to another
  agent, subagent, model, or a higher-capability model unless the user explicitly
  requests that specific action in the current task. This prohibition applies in
  every context, including planning, implementation, diagnostics, validation,
  reviews, retries, time pressure, or suspected task complexity. A prior approval
  for delegation, a prior model choice, completion of local attempts, or the
  availability of a stronger model does not constitute approval for a new
  escalation.
- Do not change vendor GEVP for Formula-90 tuning without evidence.
- Keep `.tscn` edits surgical and preserve `node_paths`, references, and owners.
- Validate RayCast origins, ground clearance, scene load, and referenced resources.
- Visual acceptance requires an image-capable reviewer or human confirmation.
- Never refactor a subsystem as a debugging tactic without evidence.

Before a complex handoff, run Context GC and use `context-handoff`. Preserve verified
facts, constraints, failure signature, hypothesis state, baseline, validation and
rollback conditions; discard raw tool noise and superseded reasoning.
