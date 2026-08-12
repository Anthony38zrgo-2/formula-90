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

## Safety and context

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
