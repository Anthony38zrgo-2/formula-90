---
name: track-reconstruction
description: Deterministic real-circuit reconstruction and procedural Blender racetrack/environment generation.
---

# Skill: Track Reconstruction

Use this skill for real circuit geometry, Blender racetrack generation, curbs, guardrails, procedural materials, vegetation, distant scenery, or the phrase "ejecuta el renderizado procedural".

## Authority order

1. geospatial/vector measurements if available;
2. georeferenced imagery;
3. orthographic/top-down reference;
4. perspective-corrected imagery;
5. manual approximation only as fallback.

Never replace measurable geometry with guessed `Vector3` values when a deterministic data source exists.

## Two-stage gate

### Base

```powershell
.\scripts\run_track_pipeline.ps1 -Mode Base -Track <track> -Seed <seed>
```

Base mode owns track geometry, smooth curbs, terrain shoulders, start/finish and the canonical runtime GLB. A Base result must be human-tested.

### Procedural

Only after explicit user acceptance:

```powershell
.\scripts\run_track_pipeline.ps1 -Mode Procedural -Track <track> `
  -TreesDensity <density> `
  -BushesDensity <density> `
  -GrassDensity <density> `
  -BuildingsDensity <density> `
  -Seed <seed>
```

Allowed densities: `none`, `very_low`, `low`, `medium`, `high`.

The default environment path must not require external assets. Region-specific canonical prototypes are generated in Blender and intentionally kept few in number.

## Hard constraints

- Same config + seed must reproduce placement and textures.
- Track geometry is independent from procedural decoration.
- `track_base.blend` is the human-validated clean source and must not be decorated in-place.
- Procedural runs start from `track_base.blend`, back up any previous `track_environment.blend`, then replace `track_environment.blend`.
- Never delete the live runtime GLB before a replacement exists; export to a temporary path and atomically replace.
- No two placed environment objects may overlap declared bounding radii.
- All categories must respect configured distance zones from the circuit.
- Grass and bushes are near-field 2D crossed cards.
- Trees are a small low-poly region-specific prototype set.
- Distant generic buildings are fake billboard scenery with no collision.
- Guardrails are modular procedural geometry and use simplified `-colonly` collision boxes.
- Curbs use the configured low smooth profile, never rectangular steps.
- Banked road geometry must not intersect a flat terrain plane. Use bank-aware grass shoulders and a lower far-ground plane.
- Do not fix road/terrain clipping by translating the whole Godot track scene.
- Generated textures should remain low-resolution and reusable; do not generate a unique texture per instance.
- Do not create persistent mesh/material duplicates merely for color variation.

## Region policy

Current region: `south_america`.

A region should have only a small number of canonical vegetation variants. Add a new region profile instead of continuously adding one-off models.

## Failure protocol

If Base validation fails, do not run Blender.

If procedural placement validation reports overlaps or clearance violations, do not export or claim success.

If the canonical runtime GLB cannot be atomically replaced after a successful export, leave the previous canonical asset intact and report failure.
