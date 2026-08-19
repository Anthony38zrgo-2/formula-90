# TS-010 TrackDomain v1 Status

## Result

The first Rust domain slice is implemented under:

```text
tools/track_studio/
  Cargo.toml
  crates/track-domain/
```

The crate is pure Rust and has no dependency on SQLite, Tauri, Blender or
Godot. It currently defines the minimal semantic document, profiles, sections,
regions, vegetation regions, asset instances, gameplay markers and diagnostics.

## Implemented behavior

- Persistent string IDs for domain objects.
- `TrackDocument` schema version 1.
- BTreeMap metadata/properties for deterministic object-key ordering.
- Validation for empty IDs, duplicate IDs, non-finite coordinates, invalid
  section station ranges, invalid polygons, empty vegetation asset sets and
  unordered profile stations.
- Canonical collection ordering by persistent ID or station.
- Deterministic JSON bytes for equivalent collection insertion order.
- Fixed six-decimal float formatting in canonical JSON.
- `Diagnostic` records with code, severity, optional object and station data.

## Validation

```text
cargo test --workspace                         PASS: 3 tests
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
```

## Explicit limitations

- This is the domain scaffold, not the complete TrackDocument contract.
- Closed-centerline topology, station derivation, geometry evaluation and
  profile range rules belong to `TS-030`.
- Asset existence and registry hashes belong to `TS-060`.
- SQLite persistence belongs to `TS-020`.
- BuildIR belongs to `TS-090`.

These limitations are intentional. The next boundary work is SQLite storage;
its migrations must persist the schema and compiler versions already emitted by
this domain crate.
