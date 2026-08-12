# Simplify

Classify as `SIMPLIFY`. The controlled low-poly sequence is clean, remove
degenerates, identify components, preserve important regions, decimate,
recalculate normals, validate silhouette, then prepare UV/colors.

Use one explicit PyMeshLab decimation filter:

```powershell
tools\asset_pipeline\.venv\Scripts\python.exe .agents\skills\3d-asset-generation\scripts\simplify_mesh.py working\clean.glb output\asset_lowpoly.glb --target-faces 5000
```

The script records the filter, parameters, input/output counts, input hash, and
output analysis. Review the report before adding another operation.
