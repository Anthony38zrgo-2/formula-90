# Asset Pipeline Environment

This directory owns the isolated Python environment for deterministic 3D asset
work. It is not the project Python and is not installed globally.

Create or recreate it on Windows with Python 3.12 (Open3D provides the most
reliable Windows wheel there):

```powershell
python -m venv tools/asset_pipeline/.venv
tools\asset_pipeline\.venv\Scripts\python.exe -m pip install --upgrade pip setuptools wheel
tools\asset_pipeline\.venv\Scripts\python.exe -m pip install -r tools/asset_pipeline/requirements.txt
```

The pinned environment has been verified with `pip check` and imports for
NumPy, SciPy, PyMeshLab, Trimesh, Open3D, Pillow, OpenCV, NetworkX, Shapely,
Rtree, and Pyglet. If an optional package cannot be installed on a platform,
keep the core pipeline usable and only disable the feature that needs it:

| Package | Degraded feature |
| --- | --- |
| Open3D | point-cloud registration and ICP |
| OpenCV | advanced texture masks and image morphology |
| Rtree | accelerated Trimesh proximity queries |
| Pyglet | optional Trimesh visualization |

The reusable commands live in
`.agents/skills/3d-asset-generation/scripts/`. Keep asset stages separate as
`input/`, `working/`, `output/`, and `reports/`; these are pipeline conventions,
not locations for the virtual environment itself.

## GLB normal gate

Every triangle primitive in a canonical vehicle source and its runtime exports
must contain a `NORMAL` accessor. A glTF without normals is rendered with flat
face normals, which causes polygon-shaped lighting changes and makes generated
LOD transitions conspicuous while the vehicle moves.

Repair a source candidate without changing its pre-existing buffers, geometry,
UVs, materials, images, nodes, or transforms:

```powershell
tools\asset_pipeline\.venv\Scripts\python.exe tools\asset_pipeline\add_gltf_normals.py input\vehicle.glb output\vehicle.glb --crease-angle 45 --report reports\vehicle_normals.json
```

The vehicle runtime generator rejects a source with missing normals and exports
all split chassis/wheel scenes with `include_normals=True`. Validate the repaired
source before promotion and retain the previous source/runtime bundle until the
isolated Godot smoke passes.

## Vehicle assembly equivalence gate

The fully assembled source GLB is the geometric golden reference. Generate a
Jordan candidate in staging with one `T_vehicle` and its mandatory equivalence
artifacts:

```powershell
tools\asset_pipeline\.venv\Scripts\python.exe tools\asset_pipeline\generate_jordan_197_runtime.py --output-dir .codex-staging\jordan_197_candidate
```

Or validate an installed runtime directly:

```powershell
tools\asset_pipeline\.venv\Scripts\python.exe tools\asset_pipeline\validate_assembly_equivalence.py --source assets-lowpoly-python\vehicles\canonical\formula_reference_livery_contract_annotated.glb --runtime-dir game\assets\models\vehicles\jordan_197 --manifest game\assets\models\vehicles\jordan_197\vehicle_runtime_manifest.json --source-assembly-out game\assets\models\vehicles\jordan_197\source_assembly.json --report-out game\assets\models\vehicles\jordan_197\assembly_equivalence_report.json --overlay-out game\assets\models\vehicles\jordan_197\assembly_equivalence_overlay.glb
```

Publication is blocked unless source/runtime anchors, pairwise distances,
hub-to-suspension distances, ground clearance and per-node geometry are all
within 2 mm. The manifest declares `T_vehicle` and actual placements but never
supplies the expected geometry. Review the generated overlay in
`res://scenes/tracks/test_field/jordan_197_assembly_equivalence_overlay.tscn`
before running GEVP tests.
