# Procedural vegetation

`blender/vegetation/` is reserved for optional future art overrides. The active Formula90s racetrack pipeline no longer requires external vegetation assets.

Default procedural environment generation creates a deliberately small reusable set for the configured region (`south_america` currently):

- low-poly broadleaf tree;
- low-poly dry-canopy tree;
- low-poly palm;
- crossed 2D grass cards;
- crossed 2D bush cards.

The number of canonical tree types is intentionally small. Variety comes from deterministic placement, scale and rotation rather than dozens of unique meshes.

Generated textures are small and live under `blender/generated/<track>/textures/`; they are embedded into runtime GLBs and are not source-of-truth.

If hand-authored vegetation is reintroduced later, treat files in this folder as optional overrides rather than a required dependency of Base or Procedural mode.
