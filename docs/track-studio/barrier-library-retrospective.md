# Full-circuit safety environment sprint retrospective

## Outcome

The full circuit now consumes an authoritative sector safety manifest, a
deterministic staged asset generator, validated visual/collision GLBs, the two
Peruvian house assets, and sector-aware trackside props. The accepted
Formula-90 painted palette is navy blue and white. The legacy barrier generator
is no longer an active runtime authority.

## What failed in the prototype

- RGB channels were decoded incorrectly, producing neon colours.
- Tyres were oriented or simplified in ways that did not read as stacked tyres.
- Generated library assets were not actually consumed by the track pipeline.
- Open meshes and weak assertions allowed invalid geometry to pass.
- Review images did not guarantee coverage of every barrier asset.
- Trackside props used one legacy 12.8 m offset instead of the active barrier's
  outer face, which placed spectators inside wider safety sectors.
- `track_flag` had no catalog source and silently used a bright-red fallback.
- Coplanar reverse faces on trackside cards caused invalid-mesh export warnings.

## Safeguards added

- `barrier_construction_manifest.json` owns the coordinate, tyre, colour,
  shading, geometry, publication, and human-gate contracts.
- The generator stages all outputs and publishes only after validation.
- Every output and every construction input is hashed.
- Tyres use at least 12 by 8 torus sections, lie flat with the axle on Y, and
  use depth layers that cannot alter barrier height.
- Visual and collision GLBs are separate, watertight, winding-consistent, and
  checked for finite vertices, degenerate faces, bounds, and triangle budgets.
- The track GLB validator compares declared and exported barrier instance IDs.
- The building and trackside validators compare every placement with the
  exported GLB and require all non-safety scenery to remain non-collidable.
- Trackside placement resolves the exact active sector and sequence prototype,
  then adds an asset-specific clearance outside its collision envelope.
- Missing catalog materials fail the build; the track flag is an explicit
  procedural navy/white asset and has no red fallback.
- Runtime BUILD schema 3 rejects changes to the barrier and building contracts,
  generators, asset manifests, integration validators, config, builder, or GLB.

## Verification completed

- Eight barrier assets generated and reviewed.
- Seven deterministic library tests pass.
- Full-circuit integration contains 2,428 matching barrier instances and zero
  legacy barrier nodes.
- Seven source-backed houses are present: four `house_peru_01` and three
  `house_peru_02`, all non-collidable.
- All 124 trackside instances match their placement records, remain outside the
  active barrier envelope, and have collision disabled. The 33 track flags use
  the procedural navy/white material; zero legacy red flag materials remain.
- Trackside cards export without invalid-mesh warnings.
- Canonical `la_chutana.glb` was published from the approved source state; the
  ignored local BUILD schema 3 is regenerated against the final commit HEAD.
- Godot import, BUILD/HEAD/hash validation, HUD/minimap/sensor smoke, and
  Mountains3D background smoke pass.

## Follow-up

Blender still reports pre-existing validity warnings for two grass prototypes
and sampler-selection warnings for some textured cards. They do not affect the
validated barrier, building, or trackside contracts and should be addressed as
a separate atomic maintenance item.
