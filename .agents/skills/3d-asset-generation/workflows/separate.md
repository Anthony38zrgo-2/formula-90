# Separate

Classify as `SEPARATE`. Analyze connected components first, then export each
component into a new output directory without touching the source:

```powershell
tools\asset_pipeline\.venv\Scripts\python.exe .agents\skills\3d-asset-generation\scripts\mesh_components.py input\asset.glb --json-out reports\components.json
tools\asset_pipeline\.venv\Scripts\python.exe .agents\skills\3d-asset-generation\scripts\separate_components.py input\asset.glb working\classified
```

Use feature vectors and confidence/evidence to decide whether components are
wheels, wings, body panels, or other regions. A generic component export is not
itself a semantic classification.
