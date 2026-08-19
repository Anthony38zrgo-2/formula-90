# TS-060 track-assets Status

## Result

The semantic asset registry is implemented in:

```text
tools/track_studio/crates/track-assets/
```

It owns asset metadata and validation, mirroring the existing Python
`asset_registry.py`, and returns stable `Diagnostic` records.

## Implemented behavior

- `AssetSpec` with semantic id, kind, category, source path, source SHA-256,
  dimensions, preview, collision class, budget and metadata.
- `AssetRegistry` indexed by semantic id with `lookup`.
- Validation for duplicate/invalid ids, invalid kinds and collision classes,
  invalid/negative dimensions, budget ordering, missing sources (non-procedural
  kinds), missing previews and source SHA-256 mismatch.
- Procedural kinds (flag) are allowed without a source.
- Source/preview existence and SHA-256 are verified when a base directory is
  provided.

## Validation

```text
cargo test --workspace                         PASS: 31 tests
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
```

## Explicit limitations

- Binary assets (GLB/PNG) are not embedded; only paths, hashes and metadata are
  held.
- The registry is not yet connected to the `track-storage` project database or
  to the build gate; that wiring lands with the import/build phases.
- Asset-based validations in `TS-040` (missing asset, asset inside road
  envelope, vegetation in exclusion zones) can now be added since the registry
  exists.

The next step is to wire this registry into the import path and the `TS-040`
validator so a track cannot reference an unknown, out-of-budget or hash-mismatched
asset.
