# F1 2026/2008 — rework acumulado: cockpit 2010 + carrocería ancha + ala trasera + vías

This package combines the **2010-ish cockpit placement** with successive geometry
passes. It is not a replica of any single car: the 2026 halo/body language,
3500 mm wheelbase and existing aero surfaces are retained.

## Estado final (verdad medida en los GLB)

- Cockpit/driver package **0.30 m rearward** from the pre-rework source.
- `GEO_CHASSIS_BODY` sidepod outer shell up to **0.8745 m half-width** (1.749 m max).
- `GEO_CHASSIS_FLOOR` flat floor up to **0.8745 m half-width** (1.749 m max).
- Widening keeps the centreline, nose, coke-bottle tail and diffuser tip fixed.
- Rear wing (`GEO_CHASSIS_REAR_WING` incl. integrated endplates + `GEO_CHASSIS_ACTIVEREAR`
  flap + `GEO_CHASSIS_AEROPART2` plane): **x1.32 chord** about the leading edge
  (main chord 0.326 -> 0.430 m, rear edge z = 2.324 m).
- Front-wing `GEO_CHASSIS_ENDPLATE`: **x0.90 chord** from the rear
  (0.544 -> 0.490 m, leading edge fixed).
- Front track **1.708 m** (wheels 0.365 m wide at +/-0.854 m, axle total 2.073 m);
  `GEO_CHASSIS_FRONT_SUSPENSION` lengthened +36.5 mm/side with chassis mounts fixed.
- Rear tyre **0.492 m** wide (+20%, axle total 2.082 m); rear track stays 1.59 m.
- Overall vehicle: 5.149 m long, 2.082 m wide over the rear tyres.
- `JNT_WHEEL_FL/FR` interface datums at +/-0.854 m (scene + metadata updated).

## Historial de pases

1. Cockpit 0.36 m rearward + RB6 width (+75/+70 mm per side).
2. Ajuste 2026-09-06: cockpit 0.30 m, sidepod +95 mm/side (1.544 m),
   floor +90 mm/side (1.565 m), endplates traseros x1.38 (superado abajo).
3. Ensanche pontones/fondo hasta plano central de rueda trasera (0.795 m).
4. +10% sobre ancho actual (0.8745 m).
5. Ala trasera completa +20% y +10% (x1.32 total, planos y endplates).
6. Endplates delanteros x0.90 (tras probar x0.80 y x0.85).
7. Goma trasera x1.20; tren delantero a via 1.708 m con goma 0.365.

## Modified files

- `f1_2026_2008_chassis.glb`: canonical chassis for Godot/project use (only tracked
  chassis binary; hierarchy, scenes, materials, UVs and embedded textures byte-exact
  vs the previous pass; only POSITION/NORMAL buffers + accessor min/max changed).
- `f1_2026_2008_wheel_rear.glb`: rear tyre 0.492 m + widened rim, face datums +/-0.246 m.
- `f1_2026_2008_wheel_front.glb`: unchanged (0.365 m; track lives in scene/physics).
- `game/data/vehicles/f1_2026_2008/f1_2026_2008_physics.json`: front_track 1.708,
  rear tyre width 0.492 / contact patch 0.432 / wheel mass 33.6 kg,
  rear-wing area 0.95 m2, underfloor lift area 1.62 m2.
- `game/scenes/vehicles/f1_2026_2008/f1_2026_2008_rust.tscn`: front wheels/rays at
  +/-0.854 m, rear rays spread to the 0.492 m tyre (±40% width proportion kept).

## Method note

Blender 5.2 is available and `source/apply_2010_cockpit_rework.py` parameters were kept
as the reproducible source of truth for the cockpit/RB6 pass. A trial Blender re-export
was rejected for the binary: it emits 15 scenes, drops the `world` root node and injects
`KHR_texture_transform` on the carbon material. The committed GLBs were instead patched
directly in their POSITION/NORMAL buffers with the same window math (smoothstep
sidepod/floor windows, affine chord scaling about fixed leading edges, symmetric
tyre widening about fixed wheel centres), verified reloadable with binary-identical
containers outside the edited accessors.

## Editable source

- `source/f1_2026_2008_source.blend`: editable source assembly (includes all passes).
- `source/apply_2010_cockpit_rework.py`: Blender reproducibility script for the
  cockpit/RB6 pass.

## Preserved

- 3500 mm wheelbase and both axle-center datums
- tyre radius 0.33 m, ride heights, raycast heights
- rear track 1.59 m, rear suspension mesh
- front wing main plane position and geometry
- topology/triangle counts
- materials, UVs and embedded textures
- vehicle coordinate contract

## Validation

See `validation_report.json` and `export_report.json` for live-measured dimensions,
changed-vertex checks and preserved interface checks. `rb6_2010_width_rework_validation.json`
and `cockpit_2010_rework_validation.json` document the earlier intermediate passes.
