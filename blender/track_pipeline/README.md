# Formula90s deterministic Blender racetrack pipeline

Two gated stages: **Base** builds a clean playable circuit; **Procedural** decorates only after human approval.

## Base

```powershell
.\scripts\setup_track_pipeline.ps1
.\scripts\run_track_pipeline.ps1 -Mode Base -Track la_chutana -Seed 1995
```

Base reconstructs/validates the centerline, generates deterministic low-resolution texture banks, builds the Blender track, backs up the previous `track_base.blend`, exports through a temporary GLB and atomically publishes `<track>.glb`.

### Terrain collision contract

Wide normal-offset grass ribbons are forbidden: on tight corners they can fold over themselves and create invisible collision wedges. The current builder uses a regular deterministic heightfield grid. Cells fully beneath the asphalt corridor are omitted; boundary terrain collision is continuous with the banked road edge. Only the visual terrain gets the configured millimetric sink.

`validate_track.py` checks finite/non-degenerate terrain triangles, seam error and triangle budget before Blender runs.

## Procedural

```powershell
.\scripts\run_track_pipeline.ps1 -Mode Procedural -Track la_chutana `
  -TreesDensity low -BushesDensity low -GrassDensity medium `
  -BuildingsDensity very_low -Seed 1995
```

Every procedural run reopens the clean `track_base.blend`; it never stacks new decoration over an old result. Previous `.blend` outputs are backed up and runtime GLBs are replaced atomically.

## Biome model

Environment art is selected by:

```text
continent: south_america
longitude: west | center | east
altitude:  low | medium | high
```

That yields nine South America combinations. `generate_procedural_textures.py` creates **4 trees, 4 bushes, 4 grass cards and 4 building facades per combination**. La Chutana uses `south_america / west / low`, a balanced green/dry/dirt terrain palette and mixed residential/industrial distant structures. This is an artistic model, not ecological or historical simulation.

## Retro geometry rules

- tree: 3 crossed double-sided planes = 6 directional faces;
- bush: 2 crossed planes = 4 directional faces;
- grass: 1 double-sided card;
- distant/medium buildings: low-poly 3D shell, four walls + basic roof, no underside;
- guardrail: low-poly visual module + separate `-colonly` box collision.

Cards use a small bank of clean PS1-style textures with fake/pre-rendered self-shadow facets. Instance variety comes from deterministic selection, rotation and scale rather than unique per-instance textures.

## Texture bank

Generated under `blender/generated/<track>/textures/`:

- shared asphalt, guardrail and start/finish;
- per-biome terrain, bark and 16 cards/facades;
- `bank_manifest.json` for all nine combinations;
- `active_manifest.json` consumed by Blender.

Terrain pigmentation uses asymmetric low-frequency masks so some broad regions are greener, drier or slightly earthy. Large world-space UV scaling keeps those changes subtle instead of visibly tiled every few metres.

## Density

Grass is cheap and intentionally more abundant. La Chutana `very_low` now starts around the previous low-density order of magnitude; higher presets scale from there. Spatial-hash overlap and distance-zone validation remain mandatory.

## Outputs

```text
blender/generated/<track>/track_base.blend
blender/generated/<track>/track_environment.blend
blender/generated/<track>/backups/base/
blender/generated/<track>/backups/environment/
game/assets/generated/tracks/<track>/<track>_base.glb
game/assets/generated/tracks/<track>/<track>_environment.glb
game/assets/generated/tracks/<track>/<track>.glb
```

The last file is the canonical asset consumed by Godot. Never delete it before a replacement has exported successfully.
