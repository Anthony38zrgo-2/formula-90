# Examples

All commands use the isolated interpreter at
`tools/asset_pipeline/.venv/Scripts/python.exe` on Windows.

```powershell
$python = "tools\asset_pipeline\.venv\Scripts\python.exe"
& $python .agents\skills\3d-asset-generation\scripts\analyze_mesh.py input\car.glb --reports-dir reports
& $python .agents\skills\3d-asset-generation\scripts\simplify_mesh.py input\car.glb output\car_lowpoly.glb --target-ratio 0.20
& $python .agents\skills\3d-asset-generation\scripts\validate_mesh.py output\car_lowpoly.glb --compare-to input\car.glb
```

The examples are intentionally parameterized. Keep source assets in `input/`,
intermediate files in `working/`, final files in `output/`, and reports in
`reports/`.
