# Vegetation color policy

## Source contract

New vegetation sources must be delivered as pre-cut RGBA images:

- transparent background in the alpha channel;
- no opaque studio background;
- no chroma-key recut in the normal source-backed path;
- bottom of the visible subject aligned by the deterministic pipeline.

The canonical compatibility color remains documented as `electric_cyan` / `#00FFFF` / `[0, 255, 255]`.
It may be used only when an external generator cannot emit transparency and a temporary local extraction is explicitly recorded.

Legacy magenta (`#FF00FF`) is compatibility-only for old assets. It must never be the default key for new sources.

## Palette lock

The current procedural vegetation palette is snapshotted in `vegetation_palette_reference.json`.
Use that file as the canonical reference to avoid drift in tree, bush, grass, and terrain colors across biomes.

## Deterministic stylization

`vegetation_texture_stylizer.py` maps visible RGB to this catalog, applies fixed Bayer dithering and derives fake occlusion from luminance and alpha topology. Alpha, bounding box and placement semantics are preserved.
