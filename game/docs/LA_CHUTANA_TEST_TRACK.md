# La Chutana handling test track

Purpose: use a real Peruvian circuit shape as the Formula90s handling-development reference before Phase C steering/countersteer work.

## Active Jordan scene

`res://scenes/tracks/test_field/jordan_handling_test.tscn`

The Jordan scene now loads the **Blender-generated base racetrack**, not the earlier Godot CSG prototype.

Godot wrapper:

`res://scenes/tracks/test_field/la_chutana_generated.tscn`

Generated runtime asset consumed by that wrapper:

`res://assets/generated/tracks/la_chutana/la_chutana_base.glb`

The GLB is intentionally ignored by Git and must exist locally. Generate it with:

```powershell
.\scripts\run_track_pipeline.ps1 -Mode Base -Track la_chutana
```

The historical `la_chutana_track.tscn/.gd` prototype remains only as a development reference and is no longer the active Jordan track.

## Blender authority

Track geometry is authored by the deterministic pipeline under:

`blender/track_pipeline/`

The active Blender builder is:

`blender/track_pipeline/build_track_blender.py`

Source measurements/configuration remain under:

- `blender/track_pipeline/data/la_chutana_reference.json`
- `blender/track_pipeline/configs/la_chutana.json`

## Runtime collision/surface contract

The Blender export uses Godot import naming for generated collision meshes:

- `RoadCollision-colonly`
- `CurbCollision_<segment>-colonly`
- `GrassCollision-colonly`

`la_chutana_generated.tscn` attaches `generated_track_surface_groups.gd`, which recursively restores the Formula90s surface groups expected by GEVP:

- `Road`
- `Curb`
- `Grass`
- `Wall` for later guardrails

Visual meshes are not used as the source of detailed collision when a dedicated collision proxy exists.

## Reference data

Targets used by the deterministic reconstruction:

- lap length: approximately **2.420 km**;
- main straight: approximately **800 m**;
- reference turn count: **7**;
- current direction: **clockwise**.

The scene is a gameplay/physics reconstruction rather than survey-grade CAD.

## Surface

- 12 m development-track width.
- White edge lines define asphalt limits.
- Grass/run-off begins outside the track/curb surface.
- Guardrails are added only by the procedural/infrastructure stage where configured.

The 12 m width is a Formula90s development choice, not a claim about La Chutana's surveyed width.

## Curbs

The Blender pipeline uses a low crowned profile rather than rectangular blocks:

- width: ~0.58 m;
- road-side transition: ~+5 mm;
- maximum crown: ~+22 mm;
- outer transition: ~+12 mm then ~+2 mm;
- local apex/exit placement only.

This profile is intentionally conservative for a low Formula chassis and is not claimed to reproduce a specific homologated FIA curb drawing.

## Start / finish and spawn

The generated GLB contains the procedural black-and-white start/finish surface and a `PlayerSpawn` marker.

For immediate Phase B testing, `jordan_handling_test.tscn` keeps the Jordan at the equivalent deterministic position roughly 18 m before meta. Future regenerated base tracks now author `PlayerSpawn` using the same Godot-XZ-to-Blender coordinate conversion as the centerline, avoiding axis-sign drift.

## Validation before Phase C

1. Run the Base pipeline and confirm `la_chutana_base.glb` exists.
2. Start `jordan_handling_test.tscn` through `run_jordan_handling.ps1`.
3. Confirm the scene is rendering the Blender-generated track, not the old CSG prototype.
4. Start roughly 18 m before meta and cross it naturally with forward throttle.
5. Complete a full lap without hidden ramps, gaps or collision seams.
6. Touch each curb with two wheels at low, medium and higher speed; normal contact must not catapult the car.
7. Run two wheels onto grass and return to asphalt.
8. Run fully onto grass and recover.
9. Confirm Road/Curb/Grass still produce distinct GEVP behavior.
10. Only after the base racetrack is human-approved may the procedural vegetation/guardrail stage run.
