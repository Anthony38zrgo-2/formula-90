---
name: track-reconstruction
description: Deterministic real-circuit reconstruction and procedural Blender racetrack/environment generation.
---

# Skill: Track Reconstruction

Use for real circuit geometry, Blender racetracks, curbs, terrain, guardrails, procedural materials/vegetation, or "ejecuta el renderizado procedural".

## Gate

Base:
```powershell
.\scripts\run_track_pipeline.ps1 -Mode Base -Track <track> -Seed <seed>
```
Human-test Base before Procedural.

Procedural:
```powershell
.\scripts\run_track_pipeline.ps1 -Mode Procedural -Track <track> `
  -TreesDensity <density> -BushesDensity <density> `
  -GrassDensity <density> -BuildingsDensity <density> -Seed <seed>
```

## Hard rules

- Same config + seed must reproduce centerline, texture bank and placement.
- Never decorate `track_base.blend` in place; reopen it for every procedural run.
- Back up previous `.blend` outputs and atomically replace runtime GLBs.
- Terrain collision must be a single-valued heightfield/grid or another topology proven not to self-intersect. Do not use wide normal-offset shoulder ribbons on tight curves.
- Collision terrain must meet the road edge continuously; visual-only sink may be millimetric.
- Smooth low curbs only; never rectangular launch ramps.
- Guardrail visual mesh never acts as vehicle collision; use simple `-colonly` proxies.
- Procedural biome is `continent + longitude band + altitude band`.
- South America must expose four variants for vegetation/structure categories in every west/center/east × low/medium/high combination.
- Trees use 3 crossed planes, bushes 2, grass 1. Do not replace them with full 3D crowns unless explicitly requested.
- Buildings may be simple 3D shells with a basic top and no underside.
- Keep base texture count small; reuse deterministic textures instead of per-instance files.
- Validate overlaps and track clearances before Blender export.

## Failure

If track/terrain validation fails, do not run Blender. If environment validation fails, do not publish the decorated GLB. Preserve the last known-good canonical GLB on export failure.
