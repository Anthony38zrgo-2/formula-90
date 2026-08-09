# Formula90s procedural vegetation

External vegetation assets are optional overrides; the default track pipeline generates its own retro vegetation.

Biome selection is deterministic:

```text
continent + longitude (west/center/east) + altitude (low/medium/high)
```

For every South America combination the generated texture bank contains at least:

- 4 tree cards;
- 4 bush cards;
- 4 grass cards;
- 4 facade textures used by distant/medium structures.

Geometry contract:

- trees: 3 crossed double-sided planes (6 directional faces), deliberately taller than the first prototype generation;
- bushes: 2 crossed planes, low and relatively wide;
- grass: one double-sided card, intended for high instance counts.

Textures use strong-clean PS1 styling with hard silhouettes and fake/pre-rendered self-shadow facets. Tree and bush palettes are intentionally offset so they do not collapse into one identical green mass.

`blender/vegetation/` remains available for future hand-authored overrides, but an empty folder must never break the default pipeline.
