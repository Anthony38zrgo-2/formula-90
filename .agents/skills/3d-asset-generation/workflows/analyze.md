# Analyze

Classify as `ANALYZE` before editing. Preserve the source and run:

```powershell
tools\asset_pipeline\.venv\Scripts\python.exe .agents\skills\3d-asset-generation\scripts\analyze_mesh.py input\asset.glb --reports-dir reports --json-out reports\asset.json
```

The report includes counts, bounds, dimensions, center, PCA orientation,
materials, UVs, vertex colors, normals, connected components, and topology
signals. If the SHA-256 is already present, the cached report is reused.

Follow with `mesh_components.py` when a region decision is required. Do not
modify or classify from object names alone.
