# Vegetation color policy

## Source-key background

- Canonical name: `electric_cyan`
- Canonical hex: `#00FFFF`
- Canonical RGB: `[0, 255, 255]`

All newly generated vegetation source images that require a solid chroma background **must** use this exact color.
Do not use legacy magenta (`#FF00FF`) for new vegetation generations.

Legacy source images that still use `#FF00FF` remain explicitly supported only by the fallback recut path.
They are not represented as newly regenerated electric-cyan sources.

## Why this key color

Vegetation art direction can contain pink/magenta-adjacent hues (autumn leaves, flowers, stylized highlights).
Electric cyan is less likely to collide with the vegetation palette while remaining easy to segment.

## Palette lock

The current procedural vegetation palette is snapshotted in `vegetation_palette_reference.json`.
Use that file as the canonical reference to avoid drift in tree, bush, grass, and terrain colors across biomes.
