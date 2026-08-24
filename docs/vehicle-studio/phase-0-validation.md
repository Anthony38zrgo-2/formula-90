# Vehicle Studio Phase 0 Validation

Status: `ACCEPTED`

Backlog scope: `VS-001` through `VS-006`

Branch: `f1-94`

Planning HEAD: `506386ca5d08bd646d38437ce6e1b1ee3c96a922`

## 1. Scope result

| Item | Result | Evidence |
|---|---|---|
| VS-001 Williams baseline | `REVIEWED` | 55-file fingerprint, GLB/texture/report inventory and repeat probe |
| VS-002 manifest discrepancy | `REVIEWED` | Historical/current count matrix and safe repair boundary |
| VS-003 provenance recovery | `REVIEWED` | Missing-script classification and forward-only provenance contract |
| VS-004 VehicleDocument v1 | `REVIEWED` | Draft 2020-12 schema, positive fixture and seven-case matrix |
| VS-005 VehicleView SVG v1 | `REVIEWED` | Machine profile, safe fixture and four negative security fixtures |
| VS-006 semantic mapping | `ACCEPTED` | 27 mapped + 19 ignored primitive occurrences, zero unclassified |

No Blender write, asset repair, application scaffold, runtime publication or
Godot mutation occurred in Phase 0.

## 2. Source preservation

Williams source fingerprint before and after Phase 0:

`3C7B996CFAB2EDF263E223A06846BF84C6822CD343D21E03561CD9AD19FB90A9`

Covered source files: `55`

Result: `PASS`; source hash set unchanged.

## 3. Contract validation

### VehicleDocument

- JSON files parse: `PASS`.
- Schema definitions: `29`.
- Referenced definitions: `29`.
- Unresolved schema references: `0`.
- Positive fixture schema validation: `PASS`.
- Positive fixture duplicate IDs: `0`.
- Missing required frame roles: `0`.
- Fixture outcome matches: `7/7`.

### VehicleView SVG

- XML parse: `5/5` fixtures.
- Positive fixture profile result: `PASS`.
- Script fixture: `FAIL` as required.
- External image fixture: `FAIL` as required.
- Event attribute fixture: `FAIL` as required.
- Unbound editable control fixture: `FAIL` as required.

The negative event fixture also violates axis/binding rules. Multiple stable
diagnostics are allowed; the security failure remains explicit.

## 4. Semantic mapping validation

| Evidence | Result |
|---|---:|
| Actual canonical authoring primitives | 27 |
| Declared canonical mappings | 27 |
| Missing mappings | 0 |
| Extra mappings | 0 |
| Actual compatibility-assembly primitives | 19 |
| Declared ignored duplicates | 19 |
| Unclassified primitive occurrences | 0 |
| Maximum mapped-frame translation delta | `4.29e-11 m` |

The numerical delta is below the declared `1e-6 m` document tolerance.

## 5. Known limitations retained intentionally

- The current Williams source package is untracked and imported by fingerprint.
- Historical refinement scripts/intermediates remain unavailable.
- Normals are incomplete on some chassis primitives.
- Sidepod, engine-cover and protected `GEO_CHASSIS` vertex bindings are not
  present in historical reports and must be confirmed interactively later.
- Nose interface bindings require confirmation.
- Wings support whole-object editing until element-level bindings are confirmed.
- `agentdb` and the documented backlog seed path are absent; the Markdown
  backlog remains the temporary versioned authority.
- Visual acceptance is `NOT_OBSERVED`; Phase 0 did not render or deform.

## 6. Human gate

The Phase 0 gate asks for acceptance of the semantic-authority boundary, not of
visual output. Acceptance approves:

1. modular core/front/rear/front-wheel/rear-wheel GLBs as authoring sources;
2. full chassis GLB as an ignored comparison/compatibility duplicate;
3. source-explicit component and frame mappings;
4. imported-baseline provenance rather than reconstructed historical claims;
5. mandatory later interactive confirmation of persistent deform/protected
   regions;
6. whole-object wing editing until element groups are confirmed.

On acceptance, the next backlog item is `VS-010` and no revision-zero vehicle
project is created until `VS-014` records the interactive mapping decisions.
