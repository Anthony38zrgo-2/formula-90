# La Chutana handling test track

The active Jordan handling scene consumes the canonical Blender export:

`res://assets/generated/tracks/la_chutana/la_chutana.glb`

Generate Base with:

```powershell
.\scripts\run_track_pipeline.ps1 -Mode Base -Track la_chutana -Seed 1995
```

## Geometry authority

The stored reference targets approximately 2.420 km, ~800 m main straight and seven reference turns. This remains a gameplay reconstruction rather than survey CAD.

Road width is a 12 m Formula90s development choice. Curbs remain the low crowned ~0.58 m profile with ~22 mm maximum rise.

## Off-track collision fix

Telemetry from the prior shoulder implementation showed a sudden extreme deceleration followed by saturated suspension after leaving asphalt. The old grass collision used wide normal-offset ribbons; at tight-radius corners those offsets could self-intersect and create invisible collision wedges.

The Blender builder now creates a regular heightfield terrain grid instead. Cells fully below the asphalt corridor are omitted. Boundary terrain collision follows the banked road edge continuously, while only the visual grass mesh receives a tiny sink to prevent z-fighting. `validate_track.py` checks terrain topology, triangle budget and road/grass collision seam before Base export.

## Active art profile

La Chutana uses:

```text
continent = south_america
longitude = west
altitude = low
terrain palette = balanced green/dry/dirt
structures = mixed residential + industrial
```

This is an artistic environment profile rather than an ecological or historical simulation.

The generated South America bank contains four tree cards, four bush cards, four grass cards and four structure facades for every west/center/east × low/medium/high combination. Trees use three crossed planes, bushes two, grass one. Structures remain simple low-poly 3D shells with a basic roof.

## Validation before Phase C

1. Regenerate Base and run the Jordan handling scene.
2. Leave asphalt at several corners and confirm there is no invisible impact, clipping or trapped chassis.
3. Re-enter asphalt slowly and at moderate speed; there must be no hard collision step at the seam.
4. Confirm Road/Grass/Curb still produce distinct GEVP surface behavior.
5. Test selected curbs with two wheels at low/medium/high speed.
6. Human-approve Base before running Procedural mode.
