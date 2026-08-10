# Formula90s deterministic racetrack pipeline

## Semantic layout and indexed objects

La Chutana has an editable deterministic authoring layer:

- `layouts/la_chutana/semantic_layout.png` contains exact-color procedural zones;
- `layouts/la_chutana/object_markers.png` contains machine-readable indexed markers;
- `layouts/la_chutana/layout_preview.png` displays the same markers with human-readable labels;
- `layouts/la_chutana/object_catalog.json` maps every index to an asset and placement constraint;
- `compiled_layout.json` is the Blender placement authority.

The marker text is never parsed with OCR. Marker index `N` is encoded as exact RGB
`[224, high_byte(N), low_byte(N)]`. Repeated indices instantiate the same reusable asset.

Build the initial editable maps once:

```powershell
blender\track_pipeline\.venv\Scripts\python.exe blender\track_pipeline\bootstrap_semantic_layout.py `
  --layout-config blender\track_pipeline\layouts\la_chutana\layout_config.json
```

After editing either PNG, compile and validate before Blender:

```powershell
blender\track_pipeline\.venv\Scripts\python.exe blender\track_pipeline\compile_semantic_layout.py `
  --layout-config blender\track_pipeline\layouts\la_chutana\layout_config.json

blender\track_pipeline\.venv\Scripts\python.exe blender\track_pipeline\validate_semantic_layout.py `
  --layout-config blender\track_pipeline\layouts\la_chutana\layout_config.json `
  --raw-config blender\track_pipeline\configs\la_chutana_raw.json
```

Do not run the bootstrap command after hand-editing the maps: it intentionally recreates the initial template.

The complete guarded workflow is available as one command:

```powershell
.\scripts\run_semantic_track_pipeline.ps1 -Track la_chutana -Publish
```

Use `-Bootstrap` only to recreate the initial maps from the legacy placement data.

The racetrack toolchain has two human-gated stages:

1. **Base**: deterministic track geometry, collision, curbs, terrain and Texture Forge output.
2. **Procedural**: vegetation, medium/far buildings and guardrails on top of the validated base `.blend`.

Do not decorate an unvalidated Base track.

## Commands

```powershell
.\scripts\setup_track_pipeline.ps1

.\scripts\run_track_pipeline.ps1 `
  -Mode Base `
  -Track la_chutana `
  -Seed 1995
```

After human validation:

```powershell
.\scripts\run_track_pipeline.ps1 `
  -Mode Procedural `
  -Track la_chutana `
  -TreesDensity medium `
  -BushesDensity medium `
  -GrassDensity medium `
  -BuildingsDensity medium `
  -Seed 1995
```

## Collision contract

The terrain collision is a continuous regular heightfield. It is **not** produced by wide centerline-offset ribbons.

Important rules:

- the terrain collision grid never deletes road cells;
- inside the road it becomes a lower underlay, while the dedicated Road collision remains authoritative;
- a narrow exact Grass collision ribbon bridges each road edge to the terrain grid;
- terrain triangle winding is explicitly authored for Blender +Z after Godot-XZ -> Blender conversion;
- `validate_track.py` rejects downward terrain winding;
- a large `GrassSafetyFloor-colonly` sits well below the circuit as a last-resort world-escape guard;
- the safety floor is a fallback, not a substitute for valid terrain collision.

The previous bug where the generated terrain faces became downward-facing after `(x,z,h) -> (x,-z,h)` conversion could make one-sided imported concave collision unreliable from above. That winding is now validated explicitly.

## Visual road/runoff transition

Visual terrain is intentionally lower beneath asphalt, so coarse grid triangles cannot clip through the road. A narrow dirt/dry shoulder ribbon is placed immediately outside the road for the late-90s racing/rally look. It is visual-only; collision comes from the exact edge bridge + continuous terrain grid.

## Formula90s Texture Forge

`texture_forge.py` is the deterministic art compiler between procedural/source artwork and Blender.

Authority:

```text
seed + biome + source recipe + forge preset
                 ↓
        deterministic pixels
                 ↓
              Blender
                 ↓
               Godot
```

Current style preset:

`ps1_rally_clean`

Texture Forge applies deterministic operations such as:

- alpha/silhouette cleanup at controlled low resolution;
- palette-constrained source colors;
- fake upper-left prerendered lighting;
- fake interior AO / lower shadow;
- edge darkening;
- posterization;
- subtle Bayer ordered dithering;
- large asymmetric terrain pigment masks;
- SHA-256 provenance for every generated PNG.

`validate_texture_forge.py` verifies the generated bank and hashes before Blender runs.

Generated images are intentionally not versioned. The reproducible recipe and source code are versioned instead.

## Biome model

Current artistic biome key:

```text
continent = south_america
longitude = west | center | east
altitude = low | medium | high
```

There are 9 South America combinations. Each combination generates:

- 4 tree cards;
- 4 bush cards;
- 4 grass cards;
- 4 building facades;
- terrain, shoulder and bark textures.

La Chutana currently uses:

```text
south_america / west / low
```

This is an art-direction system, not a scientific vegetation classifier.

## Retro geometry contract

- Trees: exactly **3 crossed planes** (6 directional faces), tall silhouette-driven cards.
- Bushes: exactly **2 crossed planes** (4 directional faces), lower and wider than trees.
- Grass: exactly **1 plane/card**.
- Buildings: low-poly 3D, four walls plus one very simple top face and no bottom face.
- Vegetation has no gameplay collision by default.
- Guardrail visuals are independent from simplified box collision.

## Placement

Placement is seeded and reproducible. It uses clustered composition rather than uniform scatter, while a spatial hash rejects overlap.

Minimum placement distance is measured from the actual road edge plus the asset bounding radius, preventing rotated vegetation cards from entering asphalt.

The same config + seed must produce the same `placements.json` and the same texture hashes.
