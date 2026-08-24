# Williams 1994 Manifest Discrepancy Report

Backlog item: `VS-002`

Status: `REVIEWED`

Baseline authority: `docs/vehicle-studio/williams94-baseline.md`

Source manifest: `blender/williams94_wheels_retextured/manifest.json`

Source-manifest SHA-256:
`AFB6EAF3B6B53F05E1F6F9C40CADA7DF8314707C42D385F8F5F12AEB882B333F`

## 1. Result

The manifest mixes three different kinds of information without an explicit
effective-state boundary:

1. an original 3,120-triangle mesh inventory;
2. chronological refinement-stage summaries;
3. current runtime paths.

The current GLBs are structurally valid enough to inspect, but the top-level
mesh inventory is not an authoritative description of their current geometry.
No source file is repaired by this report.

## 2. Discrepancy matrix

| Field/evidence | Manifest value | Measured current value | Classification | Required owner/action |
|---|---|---|---|---|
| `geometry_assets.assembly` | `geometry/F1_94_geometry.glb` | File is absent | `STALE_PATH` | Future manifest compiler must omit it or materialize/hash a declared authoring assembly |
| `meshes[].count` | 35 entries | 19 chassis meshes plus two canonical 4-mesh wheel resources; 35 instantiated runtime meshes with four wheels | `HISTORICAL_LAYOUT` | Separate source-resource inventory from instantiated-runtime inventory |
| Sum `meshes[].vertices` | 9,360 | 22,668 positions in full chassis; 30,924 positions with four wheel instances | `STALE_GEOMETRY` | Regenerate from inspected artifacts and state whether counts are resource or instance counts |
| Sum `meshes[].triangles` | 3,120 | 7,556 full chassis; 10,308 with four wheel instances | `STALE_GEOMETRY` | Regenerate final effective inventory from current GLBs |
| Core triangles in `wing_split_report.json` | 1,140 | 3,930 | `HISTORICAL_STAGE` | Keep the report immutable but label its output hash/stage; final manifest must not use it as current count |
| Front wing + nose implied split stage | Earlier modular result | 972 current triangles | `HISTORICAL_STAGE` | Record stage lineage and current artifact hash |
| Rear wing implied split stage | Earlier modular result | 2,654 current triangles | `HISTORICAL_STAGE` | Record stage lineage and current artifact hash |
| `body_refinement.assembled_triangles` | 5,736 | Later stages supersede it | `HISTORICAL_STAGE` | Rename/represent as stage output, not effective runtime total |
| `sidepods_coke_bottle_refinement.assembled_triangles` | 8,244 | Later wheel stage reports 10,308 | `HISTORICAL_STAGE` | Preserve chronology explicitly in provenance graph |
| `wheel_refinement.assembled_triangles` | 10,308 | 10,308 | `CURRENT_MATCH` | Retain, but compute from artifact hashes rather than trust report text |
| `wheel_refinement.bitmap_uv_policy` | Linear midpoint interpolation | Later `wheel_uv_remap` uses barycentric mapping from original triangles | `SUPERSEDED_POLICY` | Effective manifest must point to the last applied UV policy |
| `decoupling_strategy.albedo` | One external PNG per mesh; original 64x64 layout | 39 files but only five unique bitmap hashes | `TRUE_BUT_REDUNDANT` | Represent content-addressed texture sources separately from per-role assignments |
| GLB artifact hashes | Not authoritative at top level | Six measured hashes exist in baseline | `MISSING_PROVENANCE` | Final manifest requires size/hash for every source and output artifact |
| Normal coverage | Not declared | Full chassis 15/19; core 1/16 | `MISSING_VALIDATION` | Export gate must deterministically recompute/validate normals |
| Report source/target paths | Several `/mnt/data/...` values | Paths are unavailable in repository | `NON_PORTABLE_PROVENANCE` | Replace with repository-relative source IDs/hashes in reproducible recipes |

## 3. Counts with explicit semantics

These count domains must not be conflated:

| Count domain | Meshes/resources | Positions | Triangles |
|---|---:|---:|---:|
| Current full-chassis GLB resource | 19 meshes | 22,668 | 7,556 |
| Current canonical front-wheel GLB resource | 4 meshes | 2,064 | 688 |
| Current canonical rear-wheel GLB resource | 4 meshes | 2,064 | 688 |
| Unique runtime resources combined | 27 meshes | 26,796 | 8,932 |
| Runtime visual instances: chassis + FL/FR/RL/RR | 35 instantiated meshes | 30,924 | 10,308 |
| Historical top-level manifest mesh table | 35 entries | 9,360 | 3,120 |

The current `10,308` total is independently reproduced by structural
inspection and agrees with the final wheel-refinement summary. This agreement
does not make the earlier per-mesh table current.

## 4. Effective-state rules for the future manifest compiler

The replacement manifest contract must:

- identify every artifact by repository-relative path, byte size and SHA-256;
- distinguish immutable input, intermediate stage, authoring output and runtime
  output;
- distinguish resource counts from instance-expanded counts;
- represent operation order and superseded policies explicitly;
- compute current metrics from current artifacts, never copy them from a prior
  report;
- preserve historical reports by hash rather than rewriting them;
- reject declared paths that do not exist;
- record normal/tangent, UV, topology and semantic-frame validation state;
- use content-addressed texture sources plus semantic assignment records;
- contain no `/mnt/data` or other machine-specific path in canonical fields.

## 5. Repair boundary

Repairing `manifest.json` now would create a new source claim without the
missing transformation scripts and stage hashes. The safe sequence is:

1. preserve this discrepancy report;
2. complete the provenance recovery contract (`VS-003`);
3. define VehicleDocument and BuildIR artifact ownership;
4. implement a deterministic manifest compiler;
5. generate a candidate manifest beside the source;
6. compare and review;
7. replace the source manifest only under a separate approved backlog item.

VS-002 result: `REVIEWED`; source repair intentionally not performed.

