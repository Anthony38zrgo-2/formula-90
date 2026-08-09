# Formula90s deterministic racetrack pipeline

This pipeline separates **track authority** from **procedural decoration**.

## Stage 1 — deterministic base track

```powershell
.\scripts\setup_track_pipeline.ps1
.\scripts\run_track_pipeline.ps1 -Mode Base -Track la_chutana
```

The base stage:

1. converts the stored reference trace to metric coordinates;
2. fits a periodic spline;
3. resamples at fixed metric spacing;
4. validates length, self-intersections and curb limits;
5. builds road, smooth low curbs, terrain and start/finish in Blender;
6. exports a base GLB.

The base track must be tested by a human before environment generation.

## Stage 2 — procedural environment

After the user explicitly accepts the base track:

```powershell
.\scripts\run_track_pipeline.ps1 -Mode Procedural -Track la_chutana `
  -TreesDensity medium -BushesDensity low -GrassDensity medium -Seed 1995
```

This stage:

1. inspects assets in `blender/vegetation` and `blender/infrastructure/guardrails`;
2. rejects implausible asset dimensions;
3. creates reproducible placement from the requested densities and seed;
4. rejects objects that violate track clearance;
5. prevents object overlaps using a spatial hash;
6. independently validates the final placement;
7. instances the assets in Blender without duplicating mesh/material data;
8. builds simple guardrail collision proxy geometry;
9. exports the decorated GLB.

## Generated files

Generated working files live under:

`blender/generated/<track>/`

Runtime GLBs live under:

`game/assets/generated/tracks/<track>/`

Both are generated artifacts and should not become source-of-truth.

## Agent contract

"Execute procedural rendering" means run `Procedural` mode with the densities and seed requested by the user. Never infer `high` density when density is omitted; use the config defaults.

Do not run Procedural mode until the user has explicitly accepted the Base result.
