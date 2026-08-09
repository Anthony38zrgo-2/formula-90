# Formula90s deterministic Blender racetrack pipeline

The pipeline has two gated stages and now requires **no external environment assets** for its default workflow.

## Stage 1 — deterministic base track

```powershell
.\scripts\setup_track_pipeline.ps1
.\scripts\run_track_pipeline.ps1 -Mode Base -Track la_chutana -Seed 1995
```

Base mode:

1. reconstructs/resamples the stored circuit reference;
2. validates length, intersections, road/curb limits;
3. generates low-resolution procedural textures;
4. builds road, smooth curbs, edge lines and bank-aware grass shoulders in Blender;
5. keeps a small vertical separation between drivable geometry and terrain;
6. backs up an existing `track_base.blend`;
7. atomically replaces the base GLB and publishes the canonical runtime GLB.

The generated base must be tested in Godot by a human before procedural decoration.

## Stage 2 — procedural environment

After explicit acceptance of the base:

```powershell
.\scripts\run_track_pipeline.ps1 -Mode Procedural -Track la_chutana `
  -TreesDensity low `
  -BushesDensity low `
  -GrassDensity medium `
  -BuildingsDensity very_low `
  -Seed 1995
```

Procedural mode does **not** require tree/building source files. For `south_america` it creates a deliberately small reusable set of low-poly prototypes:

- broadleaf tree;
- dry-canopy tree;
- palm;
- crossed 2D grass cards;
- crossed 2D bush cards;
- distant fake-building billboards;
- modular low-poly guardrails plus simplified collision boxes.

Placement is seed-based, respects configured track-distance zones and uses a spatial hash to reject overlaps.

## Region authority

Current track config:

```json
"procedural_environment": {
  "region": "south_america"
}
```

Region profiles intentionally contain only a few canonical variants. Diversity should come primarily from scale, rotation, placement and small material variation—not from dozens of duplicated assets.

## Textures/materials

`generate_procedural_textures.py` creates small deterministic textures under:

`blender/generated/<track>/textures/`

These include asphalt, dry ground, bark, guardrail metal, grass/bush cards and distant-building facades.

Generated textures are source inputs for the `.blend` and are embedded by the GLB export. They are generated artifacts, not hand-authored source-of-truth.

## Anti-clipping terrain rule

Road surface elevation is configured separately from terrain clearance. Banked sections can have a road edge lower than the centerline, so merely placing a flat grass plane a few millimetres below the centerline is insufficient.

The Blender builder therefore:

1. keeps the road centerline slightly elevated;
2. creates grass shoulders that start below each banked road edge;
3. transitions those shoulders toward a lower far-ground plane;
4. automatically lowers the far-ground plane enough to stay below the lowest banked road edge.

Do not fix this by applying a Y translation to the Godot track scene.

## Backups and atomic runtime replacement

Base output:

`blender/generated/<track>/track_base.blend`

Base backups:

`blender/generated/<track>/backups/base/`

Decorated output:

`blender/generated/<track>/track_environment.blend`

Decorated backups:

`blender/generated/<track>/backups/environment/`

Runtime debug exports:

- `<track>_base.glb`
- `<track>_environment.glb`

Godot loads the canonical:

`<track>.glb`

Base mode publishes the base GLB there. Procedural mode replaces it with the decorated GLB.

The scripts do **not** delete the live export before a new one exists. They export to a temporary file and use an atomic replace, which is safer than delete-then-write.

## Determinism contract

Same config + same seed => same textures and placement.

A procedural run must fail rather than silently continue when validation detects overlap/clearance violations.

`track_base.blend` remains the validated clean source. Re-running Procedural always starts from it, then backs up and replaces `track_environment.blend`. This prevents old procedural objects from accumulating across runs.
