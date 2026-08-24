# Formula-90 barrier asset contract

The machine-readable source of truth is
`manifests/barrier_construction_manifest.json`. Recipes may select dimensions
and variants, but may not override that construction contract.

Barrier modules are authored in metres using glTF's Y-up coordinate system.
Local X is the two-metre module axis, +Z faces the track, and the origin is at
ground centre. Visual and collision meshes are separate GLB files.

Tyres are laid flat with their axles on Y and stacked into columns. The
track-facing view sees tread; circular openings are visible only from above.
`depth_layers` means rows of tyre columns behind the track-facing row. It never
multiplies barrier height. Published heights must remain between 0.45 m and
1.50 m.

Painted Formula-90 barriers use the circuit palette's navy blue and white.
Red/white barrier variants are outside this library contract.
Each tyre uses at least 12 major and 8 minor sections (192 triangles), double
the original 96-triangle prototype, so its sidewall reads as a tyre rather
than a faceted block.

Every visual GLB carries deterministic procedural vertex-colour shading. It
combines object-space diffuse light, a subtle ground-contact gradient, and
material-specific variation: tread bands for tyres, grain for concrete,
highlights for metal, and a height gradient for plastic. This adds readable
volume without external texture files. Directional cast shadows are not baked;
the runtime renderer remains responsible for them.

## Build

```powershell
python game/resources/environment/tools/build_barrier_3d_library.py `
  --recipes game/resources/environment/recipes/barrier_library.json `
  --repo .
python -m unittest game.resources.environment.tests.test_barrier_3d_library -v
```

The builder stages a complete library, validates every mesh, and only then
publishes it under `game/resources/environment/assets/barriers`. The manifest
records hashes for recipe, palette, visual GLB, collision GLB, and review strip.

## Human gate

Review `barrier_library_catalog.png`, then both the individual and five-module
PNGs for every manifest asset in the versioned `review` directory. Confirm
silhouette, colours, scale, seams, front orientation, and collision alignment
in Godot before accepting a regenerated track. A human gate is incomplete if
any barrier asset is omitted from the review set.
