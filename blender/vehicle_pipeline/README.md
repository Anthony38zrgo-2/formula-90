# Canonical 1997 Source Audit

This Phase A.1 contract audits the promoted Jordan source GLBs only. The
promoted source directory is:

`game/assets/models/vehicles/f1_90s_canonical_1997/source/jordan_191_1995/`

Source files:

- `jordan_191_1995_chassis_source.glb`
- `jordan_191_1995_wheel_fl_source.glb`
- `jordan_191_1995_wheel_rl_source.glb`

## Hash Verification

Run this from the repository root and compare each result with the matching
`sha256` entry in `provenance.json`:

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath @(
  "game\assets\models\vehicles\f1_90s_canonical_1997\source\jordan_191_1995\jordan_191_1995_chassis_source.glb",
  "game\assets\models\vehicles\f1_90s_canonical_1997\source\jordan_191_1995\jordan_191_1995_wheel_fl_source.glb",
  "game\assets\models\vehicles\f1_90s_canonical_1997\source\jordan_191_1995\jordan_191_1995_wheel_rl_source.glb"
)
```

## Inspection

The inspector is stdlib-only and runs with Blender 5.2's bundled Python:

```powershell
& "C:\Program Files\Blender Foundation\Blender 5.2\5.2\python\bin\python.exe" `
  blender\vehicle_pipeline\inspect_glb.py `
  <source-file.glb> --full --pretty
```

Use `--normal-tolerance FLOAT` to configure the near-unit normal tolerance.
Evidence reports belong in:

`blender/generated/vehicles/f1_90s_canonical_1997/reports/source_audit/`

Generated reports are evidence only and are not tracked.

## Exit Contract

- `0`: valid inspection completed and JSON was emitted.
- `2`: input or tool precondition error; error JSON is still emitted.

This is a source audit, not legal, FIA, or other compliance validation. It
reports source structure, attributes, finite values, bounds, and transforms;
future geometry and compliance work is a separate phase.
