---
name: track-reconstruction
description: Deterministic real-circuit reconstruction, collision-safe Blender generation, Texture Forge and procedural environment generation.
---

# Track reconstruction

Use this skill for real circuit geometry, Blender racetracks, curbs, terrain, guardrails, procedural vegetation or Texture Forge work.

## Stage gate

Run Base first and stop for human testing:

```powershell
.\scripts\run_track_pipeline.ps1 -Mode Base -Track <track> -Seed <seed>
```

Only after explicit human acceptance may Procedural run.

## Collision hard rules

- Never reintroduce wide centerline-offset Grass collision ribbons.
- Terrain collision must be a continuous grid/heightfield with no deleted road cells.
- Terrain faces must point Blender +Z after coordinate conversion.
- The road edge must have an exact narrow Grass collision bridge.
- `GrassSafetyFloor-colonly` must remain a last-resort failsafe below the playable terrain.
- Grass/bush/tree visual cards do not receive gameplay collision.
- Guardrails use simplified collision proxies, never detailed visual geometry.
- Do not change Jordan/GEVP physics to hide a track collision bug.

## Texture Forge hard rules

Final procedural textures are compiled deterministically by Python.

Inputs are:

```text
source recipe + biome + seed + forge preset
```

Current preset: `ps1_rally_clean`.

The generator must preserve provenance hashes. Do not hand-edit generated PNG files and commit the result as authority.

Art direction:

- late-90s PS1 rally/racing readability;
- strong silhouettes;
- subtle fake prerendered lighting/shadows;
- low effective color count;
- subtle ordered dithering;
- terrain uses large asymmetric green/dry/soil zones rather than uniform noise.

## Vegetation contract

- trees: 3 crossed planes;
- bushes: 2 crossed planes;
- grass: 1 plane;
- buildings: simple 3D with basic roof/top;
- four base variants per category per South America longitude/altitude combination;
- placement must account for bounding radius when enforcing road-edge clearance;
- clustered placement is preferred over uniform scatter.

## Failure protocol

If track, terrain, Texture Forge or environment validation fails, STOP before Blender export. Never claim success because Blender merely opened or exported.
