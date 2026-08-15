# TS-151 & TS-150 Terrain and Vegetation in BuildIR Status

## Result

Deterministic terrain and vegetation are now owned by Rust and emitted as
explicit data in BuildIR (`track-build`), so Blender does not decide where trees
go or how the heightfield is sampled.

## Implemented behavior (TS-151 terrain)

- `BuildIR.terrain_heightfield`: a regular grid over the road bounds at
  `TERRAIN_CELL_M` (6 m) with a margin. Each `TerrainCell` records x, z and a
  deterministic height from the elevation of the nearest road sample, so the
  terrain matches the road surface near the track.
- `BuildIR.terrain_cell_m` records the grid size.
- `terrain_hash` is now computed over the generated heightfield, so a change in
  terrain sampling changes the hash.

## Implemented behavior (TS-150 vegetation)

- `BuildIR.vegetation_instances`: explicit placements expanded from each
  `VegetationRegion` using a seeded xorshift64* PRNG and a jittered grid.
- Points outside the region polygon or inside the `track_exclusion` band of the
  road are discarded.
- Asset is selected from `asset_set`; scale and yaw come from the configured
  ranges, all deterministically.
- `vegetation_hash` is computed over the generated instances.
- Same region + seed always produces the same instances.

## Validation

```text
cargo test --workspace                         PASS: 51 tests
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
```

## Explicit limitations

- The heightfield model is a new, documented semantic model; it has not yet
  been compared for visual parity against the production `terrain_grid.py`.
- Vegetation uses a jittered-grid model; parity against the production
  `vegetation_regions.py` placement is pending.
- The Blender backend terrain/vegetation builders that consume these arrays are
  still pending in `TS-110`.

The next step is to add the Blender `terrain_builder`/`vegetation_builder` that
materialize these explicit arrays, and to run parity comparisons against the
current La Chutana output.
