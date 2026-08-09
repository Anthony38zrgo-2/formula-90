# Formula90s vegetation

The default racetrack pipeline no longer requires external vegetation assets.

Vegetation is generated from the deterministic biome + Texture Forge system under `blender/track_pipeline/`.

Current visual contract:

- tree = 3 crossed textured planes;
- bush = 2 crossed textured planes;
- grass = 1 textured plane;
- no gameplay collision on vegetation cards;
- silhouettes and fake prerendered shading are carried primarily by the texture;
- four variants are generated per vegetation category for every supported South America biome combination.

Optional hand-authored source assets may be added later, but they must still pass through the same deterministic style/validation contract before becoming runtime authority.
