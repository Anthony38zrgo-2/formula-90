# Export

Classify as `EXPORT`. Validate the Python-produced asset before export. Use
Blender headless only when a final GLB/glTF conversion, complex material,
baking, scene assembly, or visual review is genuinely required:

```powershell
blender --background --python export_asset.py -- input\asset.glb output\asset.glb
```

The Blender script and parameters are part of the reproducible provenance.
Analyze and validate the exported file again. Automated geometry checks do not
replace visual acceptance by a human or image-capable reviewer.
