# Williams 1994 Provenance Recovery Contract

Backlog item: `VS-003`

Status: `REVIEWED`

## 1. Finding

The current Williams package contains useful chronological reports but not the
scripts, immutable intermediate artifacts or complete source hashes required to
reproduce its present geometry from the earliest reported input.

The following claims are therefore distinct:

- `CURRENT_ARTIFACT_VERIFIED`: current bytes and structural facts can be
  measured and hashed.
- `OPERATION_REPORTED`: a report describes an operation and expected result.
- `OPERATION_REPRODUCIBLE`: code, input hashes, parameters, tool versions and
  output hashes can reproduce the result from clean staging.

The first two are available. The third is not yet proven for the complete
refinement chain.

## 2. Reported lineage

```text
/mnt/data/williams94_extract
  -> sanitation
/mnt/data/williams94_sanitized
  -> wing split/modularization
/mnt/data/williams94_modular
  -> rear refinement
  -> body refinement
/mnt/data/williams94_body_refined
  -> sidepod/coke-bottle refinement
/mnt/data/williams94_sidepods_refined
  -> wheel refinement
/mnt/data/williams94_wheels_refined
  -> wheel UV barycentric remap
/mnt/data/williams94_wheels_uvfixed
  -> wheel retexture
/mnt/data/williams94_wheels_retextured
```

The `/mnt/data` nodes are report labels, not resolvable repository sources.

## 3. Operation recovery matrix

| Stage | Evidence file | Reported behavior | Reproducible implementation present? | Recovery requirement |
|---|---|---|---|---|
| Sanitation | `sanitation_report.json` | Plane/roundtrip cleanup and normalized output | No | Identify original extract hash, exact cleanup operations and export settings |
| Modular split | `wing_split_report.json` | Core, front/nose and rear-wing separation with retained coordinates | No | Recover deterministic primitive/object selection and stage-output hashes |
| Rear refinement | `rear_refinement_report.json` | Rear lower wing/diffuser refinement | No | Recover selected faces, weights/transforms, subdivision and UV rules |
| Body refinement | `body_refinement_report.json` | Nose and airbox/engine-cover refinement | No | Recover station/control values, region membership, deformation order and output hashes |
| Sidepod refinement | `sidepods_refinement_report.json` | 836 selected original faces; conforming subdivision and displacement | No | Persist exact face/vertex bindings, falloff, symmetry and protected regions |
| Wheel refinement | `wheel_refinement_report.json` | 40 circumference and 24 hub segments; preserved anchors | No | Recover topology-generation/refinement code and exact per-axle parameters |
| Wheel UV remap | `wheel_uv_remap_report.json` | Barycentric UV transfer from modular source triangles | No | Recover source-triangle correspondence algorithm and source artifact hash |
| Wheel retexture | `wheel_retexture_report.json` | Wheel texture reassignment/copy | No | Recover semantic assignment table and content hashes |

No repository Python or PowerShell source currently references these Williams
stage names or implements the reported transformations.

## 4. Minimum reproducible operation recipe

Every recovered or future operation must serialize:

```text
operation_id
operation_version
tool_id
tool_version
blender_version: optional
input_artifacts[]
  role
  relative_path_or_content_id
  size_bytes
  sha256
selection_bindings[]
  source_object_id
  source_primitive_id
  vertex_or_face_ids
parameters
coordinate_system
ordered_steps[]
expected_invariants
output_artifacts[]
  role
  size_bytes
  sha256
validation_report_sha256
```

Selections must be persistent bindings. A proximity query or object-name
wildcard evaluated at build time is not sufficient provenance.

## 5. Recovery policy

### 5.1 What may be asserted now

- Current GLB/PNG/report bytes match the VS-001 fingerprint.
- Current structural counts, bounds and semantic-node names are measured.
- Reports are historical evidence of intent and intermediate metrics.

### 5.2 What may not be asserted now

- That the current asset can be recreated from `williams94_extract`.
- That a report's source and output are still available.
- That UV or topology transformations are identical to an absent script.
- That the top-level manifest is a current artifact inventory.

### 5.3 Forward-only recovery

Vehicle Studio does not need to reverse engineer every absent historical
script before creating a variant. It must instead:

1. treat the current 55-file VS-001 fingerprint as the immutable imported
   baseline;
2. record a human-confirmed semantic mapping against the current GLB hashes;
3. express every new change as VehicleDocument parameters and BuildIR;
4. retain complete recipe/tool/output hashes from that point forward;
5. never describe the imported baseline as reproducible from its earlier
   unverified lineage.

This forward-only boundary resolves the missing history without mutating or
laundering it.

## 6. Promotion gate

The current package may serve as the initial Vehicle Studio source when:

- its VS-001 fingerprint matches;
- the provenance status is shown as `IMPORTED_BASELINE`;
- the manifest discrepancy is attached as a diagnostic;
- the user accepts the semantic mapping;
- all new variants record complete forward provenance.

Promotion does not repair or overwrite the source package. It creates a new
Vehicle Studio project whose revision zero references the immutable baseline.

VS-003 result: `REVIEWED`; full historical reproducibility remains
`NOT_PROVEN`, forward variant reproducibility is the required contract.

