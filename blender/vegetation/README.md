# Vegetation asset drop zone

Place local low-poly vegetation source assets here.

Recommended structure:

```text
blender/vegetation/
├── trees/
├── bushes/
└── grass/
```

Supported source formats for the procedural tooling:

- `.blend`
- `.glb`
- `.gltf`

Raw `.blend` files are intentionally ignored by Git. They are source assets, not runtime packaging.

The asset inspector measures each model before placement. Assets outside the plausible dimensional ranges in the track config are rejected unless a sidecar explicitly overrides the rule.

Optional sidecar next to an asset:

`tree_a.blend.asset.json`

```json
{
  "id": "tree_a",
  "category": "trees",
  "scale_min": 0.85,
  "scale_max": 1.20,
  "weight": 1.0
}
```

Do not create persistent color duplicates of the same tree. The placement manifest stores small tint metadata while all instances reuse the same geometry/material data.
