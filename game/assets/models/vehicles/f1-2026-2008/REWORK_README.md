# F1 2026/2008 — 2010 cockpit + RB6-width sidepod/floor rework

This package combines the existing **2010-ish cockpit placement** with a second geometry pass inspired by the **2010 Red Bull RB6 planform**. It is not a direct RB6 replica: the 2026 halo/body language, 3500 mm wheelbase, current wheel package and existing aero surfaces are retained.

## Geometry changes
- Cockpit/driver package remains **0.36 m rearward** from the pre-rework source.
- `GEO_CHASSIS_BODY` sidepod outer shell widened smoothly by up to **75 mm per side**.
- Resulting maximum sidepod/body width in the main radiator section: approximately **1.50 m**.
- `GEO_CHASSIS_FLOOR` widened smoothly by up to **70 mm per side**.
- Resulting maximum flat-floor width: approximately **1.525 m**.
- Widening starts progressively behind the front axle region, stays broad through the sidepod/radiator volume, then tapers before the rear axle to preserve the coke-bottle/rear-body transition.

## Modified chassis files
- `f1_2026_2008_chassis.glb`: canonical modified chassis for Godot/project use.
- `f1_2026_2008_chassis_cockpit2010.glb`: compatibility alias containing the same modified geometry.
- `f1_2026_2008_chassis_cockpit2010_rb6wide.glb`: explicit-name copy of the same modified geometry.
- Front/rear wheel GLBs and textures remain unchanged.

## Editable source
- `source/f1_2026_2008_source.blend`: original editable source assembly.
- `source/apply_2010_cockpit_rework.py`: Blender 4.x reproducibility script. It applies the cockpit rework plus the RB6-inspired sidepod/floor widening, saves `source/f1_2026_2008_source_cockpit2010_rb6wide.blend`, and exports the modified chassis GLBs.

Run in Blender 4.x:
```bash
blender -b source/f1_2026_2008_source.blend -P source/apply_2010_cockpit_rework.py
```

## Preserved
- 3500 mm wheelbase and both axle-center datums
- all wheel joints and wheel assets
- front/rear suspension meshes
- front/rear wing positions and geometry
- cockpit location from the previous 2010 pass
- topology/triangle counts
- materials, UVs and embedded textures
- vehicle coordinate contract

## Validation
See `rb6_2010_width_rework_validation.json` and `validation_report.json` for exported widths, changed-vertex counts and preserved interface checks.
