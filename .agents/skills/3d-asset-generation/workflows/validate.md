# Validate

Classify as `VALIDATE` after every relevant geometry or attribute change:

```powershell
tools\asset_pipeline\.venv\Scripts\python.exe .agents\skills\3d-asset-generation\scripts\validate_mesh.py output\asset.glb --compare-to input\asset.glb --json-out reports\validation.json
```

Check counts, components, bounds, dimensions, normals, degenerates, duplicate
vertices, non-manifold edges, UV integrity, and material assignments. The
default status `PASS_WITH_WARNINGS` is acceptable only when each warning is
understood; use `--strict` for a hard topology gate.
