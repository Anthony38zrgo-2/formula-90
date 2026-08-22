# La Chutana Formula90s handling track

La Chutana is the current real-circuit handling test before the next Jordan physics-tuning phase.

Active runtime resources:

`res://assets/generated/tracks/la_chutana/la_chutana.glb`

`res://assets/generated/tracks/la_chutana/la_chutana_vegetation.glb`

The first resource owns the physical environment, barrier visuals and collision.
The second owns collision-free vegetation. They remain separate because Godot 4.7.1
cannot reliably import the combined scene at its current node count.

Generate Base:

```powershell
.\scripts\run_track_pipeline.ps1 -Mode Base -Track la_chutana -Seed 1995
```

Then test through the Jordan handling scene. Only after human approval should Procedural mode run.

## Current geometric targets

- approximately 2.420 km lap;
- approximately 800 m main straight reference;
- 7-turn public reference;
- 12 m Formula90s development width;
- conservative ~0.58 m smooth curb profile with ~22 mm maximum crown.

This is a gameplay reconstruction, not survey-grade CAD.

## Off-track collision fix

Telemetry from the previous generated track showed a world-escape/freefall signature: all four suspension compression values reached zero while vehicle speed continued rising with zero throttle. The pipeline diagnosis found a terrain-generation problem rather than a Jordan physics problem.

The corrected Base generation now uses:

1. a continuous terrain collision grid with no holes deleted under the road;
2. an under-road collision layer below the dedicated Road collider;
3. corrected terrain triangle winding after Godot-XZ -> Blender conversion;
4. exact narrow Grass collision ribbons at both road edges;
5. a large `GrassSafetyFloor-colonly` several metres below the world as a failsafe;
6. visual terrain pushed below the road to prevent grass/asphalt clipping.

The safety floor must not normally be contacted. If telemetry shows it being reached, the primary terrain collision still needs investigation.

## Art direction

Current procedural art target:

```text
late-90s PS1 rally/racing
painted/prerendered texture character
strong card silhouettes
clean retro readability
macro terrain variation
```

La Chutana biome:

```text
South America / west / low
balanced dry + green palette
mixed simple residential + industrial medium/far structures
```

Texture Forge generates asymmetric terrain zones containing greener vegetation pigment, dry grass pigment and smaller soil-exposure patches. La Chutana uses a hybrid 50/50 ground-cover contract: 4,660 collision-free grass cards remain concentrated near the asphalt while `grassg1`-`grassg3` detail covers deterministic regions of the terrain texture. Vegetation cards receive deterministic fake lighting, AO, posterization and subtle Bayer dithering.

Geometry:

- trees: 2 crossed planes, with the three canonical variants plus `tree3`, `tree4`, `tree7` and `tree8`;
- bushes: 2 crossed planes, with the four canonical variants plus `forest1`-`forest3`;
- grass: 1 plane, reduced from 9,320 to 4,660 instances and complemented by terrain detail;
- buildings: simple 3D boxes with a basic top face;
- guardrails: modular visual geometry with separate simplified collision.

## Validation checklist

1. Regenerate Base after pulling pipeline changes.
2. Drive over both road edges repeatedly at low/medium/high speed.
3. Test Road -> Grass -> Road re-entry with steering angle.
4. Confirm no grass visual triangles clip through asphalt.
5. Confirm no sustained four-wheel `Comp = 0` freefall state occurs after leaving the road.
6. Verify curbs do not launch the Jordan under ordinary contact.
7. Confirm the procedural Base remains visually readable without environment decoration.
8. Only then run Procedural mode.
