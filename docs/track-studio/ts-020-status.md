# TS-020 track-storage Status

## Result

The first SQLite persistence slice is implemented in:

```text
tools/track_studio/crates/track-storage/
```

It uses rusqlite with bundled SQLite and a versioned `0001_initial.sql`
migration. The storage API owns the connection and exposes document save/load;
callers do not execute SQL directly.

## Implemented behavior

- File-backed and in-memory SQLite opening.
- Versioned initial migration.
- Storage schema metadata.
- Project row with schema, project and compiler versions.
- Canonical TrackDocument JSON persistence.
- Transactional insert/update save.
- Validation rejection before persistence.
- Load and deserialize round trip.

## Validation

```text
cargo test --workspace                         PASS: 6 tests
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
```

## Explicit limitations

- The migration uses one canonical JSON document column as the vertical-slice
  persistence boundary. Normalized relational tables are deferred until the
  approved schema contract is expanded.
- Persistent revision rows, recovery backups and project locking are not yet
  implemented.
- Tauri is not connected; this crate is the only SQLite owner.
- The storage API currently accepts a validated full document rather than
  partial spatial queries.

The next storage change must be contract-first: add migration tests and decide
which high-frequency entities become relational columns before splitting the
document JSON into tables.
