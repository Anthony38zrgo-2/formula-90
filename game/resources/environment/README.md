# Semantic environment cards — phase 1

This laboratory turns authored semantic SVG silhouettes into deterministic RGBA PNG review
cards. It does not generate meshes or publish runtime assets.

Trees and bushes receive the deterministic `arboreal_stipple_v1` finish after SVG
rasterization. It adds palette-locked 1–3 px leaf mottling, sparse transparent canopy gaps,
and a coarse light field while keeping trunks and branches crisp. Its seed is derived from
the asset ID and biome, so authors do not supply randomness.

The same pass classifies exposed woody pixels independently and applies palette-locked bark
variation without transparency. Realistic sources should describe tapered curved trunks,
primary and secondary branches, separated canopy clusters, and deliberate negative space;
the post-process is a finish, not a substitute for authored structure.

## Contract

The root must declare `data-schema-version="1"`, `data-asset-id`, and one of
`data-asset-kind="tree|bush|grass"`. Drawable elements inherit `data-role` and `data-tone`
from groups. Supported tones are `deep-shadow`, `shadow`, `base`, `light`, `highlight`, and
`accent`.

Allowed drawable elements are `path`, `circle`, `ellipse`, `rect`, `polygon`, and `polyline`.
Scripts, styles, text, images, external references, filters, masks, and foreign objects are
rejected. Authors should use `fill="currentColor"`; the pigmenter replaces it from the biome
palette without changing geometry or alpha.

## Run

```powershell
python game/resources/environment/tools/render_semantic_cards.py batch `
  --input-dir game/resources/environment/examples `
  --palette game/resources/environment/palettes/south_america_west_low.json `
  --output-dir game/resources/environment/generated
```

Outputs include pigment SVGs, 512 px transparent PNGs, per-asset JSON reports, and a review
sheet. Repeated runs with identical inputs must produce identical PNG hashes.

## Recursive semantic tree source

`generate_semantic_tree.py` adapts the useful geometric core of the complex-tree reference:
seeded quadratic Bézier branching, decreasing length and width, controlled 2–3 way forks,
terminal twigs, knots, and rotated leaf ellipses. It emits only contract-safe semantic SVG.

```powershell
python game/resources/environment/tools/generate_semantic_tree.py `
  --output game/resources/environment/examples/tree_recursive_01.svg `
  --asset-id tree_recursive_01 --seed 73 --depth 7 --leaf-density 2.4
```

Seven levels are the review default. Eight or more can exceed the 2,000 drawable safety
limit depending on branching and leaf density.

Generated trees also contain six `data-role="root"` Bézier paths. Root variation uses a
separate seed stream, so changing roots cannot alter the established trunk or canopy.

## Proposed final fake-shadow pass

Keep shadows out of the semantic SVG and palette pass. A later `build_card_shadow.py` should
read the final RGBA alpha, isolate the contact point around the roots, project a compressed
and sheared copy of the silhouette onto the ground, apply a fixed Gaussian blur, and tint it
from a biome `shadow` color. It should emit a separate `<asset>_shadow.png` plus an optional
review composite; the canonical card remains unchanged. Direction, length, opacity, blur,
and seed belong in a small JSON recipe and in the output report. This separation permits the
runtime to use, omit, rotate, or replace the fake shadow without regenerating vegetation.

## Semantic voxel prototype

`voxelize_semantic_tree.py` samples root, trunk, and branch Béziers into connected voxel
tubes. Leaves remain 2D ellipse sprites: they receive seeded front/back placement but no
voxel thickness or cube faces. A continuous depth field keeps shared wooden joints connected.
The default audit grid is 96×96×48. It writes compressed wood-only voxel data (`.npz`), a
JSON report, and a transparent hybrid isometric PNG.

```powershell
python game/resources/environment/tools/voxelize_semantic_tree.py `
  --svg game/resources/environment/examples/tree_recursive_01.svg `
  --palette game/resources/environment/palettes/south_america_west_low.json `
  --output-dir game/resources/environment/generated/voxels
```
