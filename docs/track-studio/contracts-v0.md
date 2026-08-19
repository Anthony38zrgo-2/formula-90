# Track Studio Contracts v0

This is a planning contract for the first Rust implementation. It is not yet
an implementation schema and must be converted into Rust types and tests by
`TS-010` and `TS-002`.

## Ownership

```text
TrackDocument -> Rust domain
SQLite        -> Rust storage
BuildIR       -> Rust compiler
Blender       -> Python materializer
Runtime       -> Godot consumer/validator
```

TypeScript may hold viewport, selection, input and preview state. It may not
own persistence, domain geometry, validation rules or SQL.

## TrackDocument v1

The editable document is semantic and preserves author intent:

```text
TrackDocument
  schema_version
  project_version
  compiler_version
  track_id
  metadata
  reference_layers[]
  centerline
  profiles
    width_left
    width_right
    elevation
    banking
  track_sections[]
    surface
    kerb
    runoff
    barrier
  terrain_regions[]
  vegetation_regions[]
  asset_instances[]
  gameplay
    start_finish
    spawn_points
    grid
    checkpoints
    sectors
    pit_lane
    marshal_posts
```

Required invariants:

- IDs are persistent and independent from array order or visual position.
- Centerline coordinates use an explicit project unit and axis convention.
- Station values are distances from Start/Finish.
- Profiles reject non-finite values and invalid ordering at the domain boundary.
- Regions and instances retain semantic type and ownership.
- External assets are referenced by `asset_id`, never by an editable mesh blob.

## BuildIR v1

BuildIR is generated, explicit and compiler-facing. It may destructively sample
or flatten the editable model:

```text
BuildIR
  build_ir_version
  source_project_hash
  compiler_version
  geometry_hash
  terrain_hash
  vegetation_hash
  objects_hash
  gameplay_hash
  road_samples[]
    station
    position
    tangent
    normal
    width_left
    width_right
    elevation
    bank
  terrain_samples[]
  barrier_segments[]
  explicit_asset_instances[]
  explicit_vegetation_instances[]
  runtime_metadata
```

Blender must be able to build the required mesh and instances without
reinterpreting SVG, selecting vegetation, calculating station or deciding
semantic terrain behavior.

## Diagnostic v1

Every validation boundary returns records with this shape:

```text
Diagnostic
  code
  severity: INFO | WARNING | ERROR | FATAL
  object_id: optional persistent ID
  station: optional distance in meters
  message
  metadata: deterministic key/value object
```

Diagnostics are stable API data. Human-readable wording may improve, but codes
and ownership fields must remain compatible within a schema version.

## Generated JSON

`track.export.json` is canonical, deterministic and review-oriented. It keeps
semantic intent and is safe to diff.

`track.build.json` is generated BuildIR serialization. It is not manually
editable and must include source, compiler and subsystem hashes.

`track_runtime_metadata.json` is generated from the same validated document and
contains at least bounds, start/finish, spawn, grid, checkpoints, sectors,
minimap polyline, marshal posts and pit lane.

## IPC command families

The first typed Tauri surface should cover:

- `project_create`, `project_open`, `project_save`, `project_close`;
- `document_snapshot`, `document_validate`;
- command apply, undo, redo and revision restore;
- `export_review`, `compile_build_ir`;
- `build_start`, `build_cancel`, `build_status`;
- asset registry query and diagnostic query.

Long-running build output is delivered through Tauri events or channels. The UI
receives structured progress and diagnostics, not direct process ownership.

## Determinism rules

- Sort all persisted collections by persistent ID or an explicitly documented
  semantic key before serialization.
- Use fixed float formatting and reject non-finite values.
- Persist all procedural seeds.
- Include schema, compiler and relevant external asset hashes in provenance.
- Do not include timestamps in canonical export or BuildIR bytes.
- Runtime activation timestamps may exist in operational manifests but are not
  inputs to source or BuildIR hashes.

## Compatibility rule

Legacy SVG/JSON import may report ambiguity or loss. It must never silently
invent domain data. The importer produces a `TrackDocument` plus diagnostics;
only a successful explicit save creates the `.f90track` authority.
