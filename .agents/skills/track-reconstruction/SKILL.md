---
name: track-reconstruction
description: Deterministic real-circuit reconstruction and procedural Blender environment generation.
---

# Skill: Track Reconstruction

Use this skill for real circuit geometry, Blender racetrack generation, curbs, guardrails, vegetation, or the phrase "ejecuta el renderizado procedural".

## Authority order

1. geospatial/vector measurements if available;
2. georeferenced imagery;
3. orthographic/top-down reference;
4. perspective-corrected imagery;
5. manual approximation only as fallback.

Never replace measurable geometry with guessed `Vector3` values when a deterministic data source exists.

## Two-stage gate

### Base
Run:

```powershell
.\scripts\run_track_pipeline.ps1 -Mode Base -Track <track>
```

A Base result must be human-tested. Do not proceed because the script merely exited successfully.

### Procedural
Only after explicit user acceptance:

```powershell
.\scripts\run_track_pipeline.ps1 -Mode Procedural -Track <track> `
  -TreesDensity <density> -BushesDensity <density> -GrassDensity <density> -Seed <seed>
```

Allowed density values: `none`, `very_low`, `low`, `medium`, `high`.

## Hard constraints

- Seeded output must be reproducible.
- Track geometry is independent from procedural vegetation.
- No two placed environment objects may overlap their declared bounding radii.
- Vegetation must respect per-category minimum distance from every part of the track.
- Asset scale is restricted to catalog/config bounds.
- Implausible source dimensions are rejected, not silently normalized.
- Grass and bushes occupy the near environment; trees occupy the farther environment unless track config explicitly says otherwise.
- Guardrail collision uses simplified proxy geometry, never the detailed visual mesh.
- Curbs must use the configured low smooth profile; do not replace them with rectangular blocks.
- Raw `.blend` assets remain outside runtime packaging.
- Do not duplicate meshes/materials merely to create color variation.

## Failure protocol

If asset inspection finds no valid asset for a requested non-zero density, STOP and report the missing category. Do not synthesize substitute assets.

If Base validation fails, do not run Blender.

If procedural validation reports overlaps or clearance violations, do not export or claim success.
