# La Chutana handling test track

Purpose: use a real Peruvian circuit shape as the Formula90s handling-development reference before Phase C steering/countersteer work.

## Active Jordan scene

`res://scenes/tracks/test_field/jordan_handling_test.tscn`

Godot wrapper:

`res://scenes/tracks/test_field/la_chutana_generated.tscn`

The wrapper loads the canonical generated racetrack:

`res://assets/generated/tracks/la_chutana/la_chutana.glb`

The canonical file is generated locally and ignored by Git.

Base mode publishes the clean track to this path. Procedural mode later replaces the same canonical path with the decorated version, so the Jordan scene does not need to change between stages.

## Base generation

```powershell
.\scripts\run_track_pipeline.ps1 -Mode Base -Track la_chutana -Seed 1995
```

Base outputs:

- `blender/generated/la_chutana/track_base.blend`
- `game/assets/generated/tracks/la_chutana/la_chutana_base.glb`
- canonical `game/assets/generated/tracks/la_chutana/la_chutana.glb`

If `track_base.blend` already exists it is backed up under:

`blender/generated/la_chutana/backups/base/`

The new GLB is exported to a temporary file and atomically replaces the old runtime export. The pipeline intentionally does not use delete-then-write.

## Geometry authority

Track geometry is generated from:

- `blender/track_pipeline/data/la_chutana_reference.json`
- `blender/track_pipeline/configs/la_chutana.json`

Targets:

- lap length: approximately **2.420 km**;
- main straight: approximately **800 m**;
- reference turn count: **7**;
- current direction: **clockwise**.

This remains a gameplay/physics reconstruction rather than survey-grade CAD.

## Road / grass clipping correction

The road center surface now has a small configured elevation (`surface_elevation_m`) instead of sharing a nearly coplanar grass surface.

A flat grass plane alone is not sufficient in banked sections: the low road edge can be substantially below the centerline.

The Blender builder therefore generates:

1. road surface at the configured small elevation;
2. bank-aware grass shoulders beginning just below each road edge;
3. a gradual transition across the shoulder;
4. a far-ground plane automatically lowered below the lowest banked road edge.

This removes grass/asphalt clipping without translating the whole Godot scene and without removing the configured banking.

## Curbs

The Blender pipeline uses the existing conservative low crowned profile:

- width: ~0.58 m;
- road-side transition: ~+5 mm;
- maximum crown: ~+22 mm;
- outer transition: ~+12 mm then ~+2 mm;
- local apex/exit placement only.

Red/white sections are generated deterministically along the curb length.

## Procedural environment

Only after the Base track is human-approved:

```powershell
.\scripts\run_track_pipeline.ps1 -Mode Procedural -Track la_chutana `
  -TreesDensity low `
  -BushesDensity low `
  -GrassDensity medium `
  -BuildingsDensity very_low `
  -Seed 1995
```

Current environment region:

`south_america`

No external vegetation/building/guardrail asset is required by the default pipeline.

It generates:

- a small set of reusable low-poly regional tree prototypes;
- crossed 2D grass cards;
- crossed 2D bush cards;
- distant fake-building billboards;
- modular low-poly guardrails;
- simplified guardrail collision boxes;
- low-resolution procedural textures/materials.

Placement is deterministic and rejects object overlaps.

Procedural output:

- `blender/generated/la_chutana/track_environment.blend`
- `game/assets/generated/tracks/la_chutana/la_chutana_environment.glb`
- canonical `game/assets/generated/tracks/la_chutana/la_chutana.glb`

An existing `track_environment.blend` is backed up under:

`blender/generated/la_chutana/backups/environment/`

Every procedural run starts from the clean validated `track_base.blend`; it does not decorate the previous procedural result. This prevents accumulated stale objects.

## Runtime collision/surface contract

Generated collision meshes use Godot import naming:

- `RoadCollision-colonly`
- `CurbCollision_<segment>-colonly`
- `GrassShoulderCollision...-colonly`
- `GrassFarCollision-colonly`
- `GuardrailCollision_...-colonly`

`generated_track_surface_groups.gd` restores the expected Formula90s/GEVP groups:

- `Road`
- `Curb`
- `Grass`
- `Wall`

## Validation before Phase C

1. Regenerate Base.
2. Run the Jordan handling scene.
3. Confirm asphalt no longer clips through grass in banked sections.
4. Complete a full lap without hidden ramps, gaps or collision seams.
5. Touch curbs at several speeds; normal contact must not catapult the car.
6. Run two wheels and then the whole car onto grass and recover.
7. Confirm Road/Curb/Grass still produce distinct GEVP behavior.
8. Confirm start/spawn remains correct.
9. Human-approve the base.
10. Only then run Procedural mode.
