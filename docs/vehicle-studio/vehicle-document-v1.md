# VehicleDocument v1 Contract

Backlog item: `VS-004`

Status: `REVIEWED`

JSON Schema:
`tools/vehicle_studio/schemas/vehicle_document_v1.schema.json`

Positive fixture:
`tools/vehicle_studio/fixtures/vehicle_document_valid_minimal.json`

Fixture matrix:
`tools/vehicle_studio/fixtures/vehicle_document_fixture_cases.json`

## 1. Contract boundary

VehicleDocument is the editable semantic authority. It references immutable
source geometry by hash and stores confirmed components, frames, dimensions,
regions, parameters, material intent, livery intent and view definitions.

It does not contain evaluated mesh blobs, Blender operator history, generated
SVG paths, BuildIR or runtime GLB bytes.

## 2. Canonical requirements

- Schema version is exactly `1`.
- Paths are repository/project relative and contain no parent traversal.
- Hashes are uppercase SHA-256.
- Axes are fixed to `+X` right, `+Y` up and `-Z` forward in metres.
- Topology changes and source mutation are false constants.
- Ground contact and UV preservation are true constants.
- IDs use lowercase ASCII with stable separators.
- Every canonical collection is sorted by its documented persistent ID or
  semantic ordering before serialization.
- Canonical JSON uses UTF-8, LF, sorted object keys and fixed finite-float
  formatting owned by the future domain serializer.

## 3. Semantic validation beyond JSON Schema

JSON Schema owns structural shape and local field constraints. The domain
validator must additionally enforce:

- global ID uniqueness across entity collections;
- required unique frame roles;
- valid parent-frame references and an acyclic frame tree;
- valid component/region/material/parameter references;
- non-conflicting source primitive ownership;
- view axes that are orthogonal and non-duplicated;
- parameter baseline/value/bound consistency;
- source/evaluated hash preconditions;
- front/rear axle ordering under the `-Z` forward convention;
- left/right symmetry ownership where enabled;
- tire/anchor/contact consistency.

The fixture matrix explicitly distinguishes schema failures from semantic
failures such as duplicate IDs and missing required frame roles.

## 4. Validation evidence

PowerShell `Test-Json` validates the positive fixture against the Draft 2020-12
schema. The fixture matrix currently produces:

| Case | Expected | Actual | Boundary |
|---|---|---|---|
| valid minimal | PASS | PASS | schema + semantic |
| invalid forward axis | FAIL | FAIL | schema |
| absolute source path | FAIL | FAIL | schema |
| topology allowed | FAIL | FAIL | schema |
| duplicate component ID | FAIL | FAIL | semantic |
| missing wheel frame | FAIL | FAIL | semantic |
| ground contact disabled | FAIL | FAIL | schema |

Executable canonical serialization and the complete semantic validator belong
to `VS-010`; this contract freezes their required behavior.

