---
name: track-reconstruction
description: Deterministic real-circuit reconstruction, collision-safe Blender generation, Texture Forge and procedural environment generation.
---

# Track reconstruction

Use this skill for real circuit geometry, Blender racetracks, curbs, terrain, guardrails, procedural vegetation or Texture Forge work.

## Geometry tooling

Use PyMeshLab as the default non-visual 3D geometry tool. Inspect topology,
face/vertex counts, bounds, normals, transforms and format conversion with it
before opening Blender when possible. Use Blender for authored finishing,
materials, UVs, scene composition and final exports. CAD-derived props follow
`CadQuery -> PyMeshLab -> Blender`.

## Stage gate

Run Base first and stop for human testing:

```powershell
.\scripts\run_track_pipeline.ps1 -Mode Base -Track <track> -Seed <seed>
```

Only after explicit human acceptance may Procedural run.

## Visual validation gate

DeepSeek V4 cannot inspect screenshots, Blender viewports or game renders. It may build
the track, run deterministic validation and capture named proof images, but it cannot
approve visual acceptance.

For any art-direction, asset, vegetation, skybox, barrier or composition change, split
the work as: DeepSeek execution -> GPT image-capable or manual human review -> explicit
user confirmation. Do not proceed to the next visual stage, or report visual success,
without that confirmation. Automated checks cover geometry, counts, transforms, collision,
paths and deterministic output only; they do not certify appearance.

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

### Vegetation source and stylization contract

- New vegetation sources are pre-cut RGBA PNGs with transparent backgrounds.
- `migrate_vegetation_source_keys.py` is not part of the active source-backed pipeline.
- `recut_vegetation_textures.py` is read-only and validates only the last visible alpha row.
- Magenta `#FF00FF` is legacy compatibility only; it is never the default for new sources.
- `vegetation_texture_stylizer.py` applies the documented biome palette, deterministic shadow/occlusion masks, fixed Bayer dithering and palette quantization while preserving alpha and the bottom anchor.
- The stylizer cache is deterministic: source hash, palette, recipe and algorithm version define the result.

## Vegetation contract

- La Chutana trees: 2 crossed planes (4 directional faces);
- bushes: 2 crossed planes;
- grass: 1 plane;
- buildings: simple 3D with basic roof/top;
- four base variants per category per South America longitude/altitude combination;
- placement must account for bounding radius when enforcing road-edge clearance;
- clustered placement is preferred over uniform scatter.

## Trackside perimeter contract

- The preferred La Chutana layout is: asphalt -> curb when present -> grass/gravel -> continuous tire-stack perimeter.
- Tire stacks use a 5 m separation from the road edge as the default escape margin.
- Tire visuals are continuous low-poly modules; their gameplay collision is a separate continuous simplified wall, tall enough to prevent the car from leaving the playable perimeter.
- Spectators, marshals, photographers, flags, signs and rocks are visual-only and must not receive gameplay collision.
- Trackside 2D cards are single-face objects placed outside the tire perimeter and must remain within the configured visual-card budget.

## Failure protocol

If track, terrain, Texture Forge or environment validation fails, STOP before Blender export. Never claim success because Blender merely opened or exported.
