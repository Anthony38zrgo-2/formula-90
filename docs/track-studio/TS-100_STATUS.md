# TS-100 Build Graph and Orchestration Status

## Result

The incremental build graph and orchestration are implemented in
`track-build::build_graph`.

## Implemented behavior

- `BuildPlan`: deterministic per-subsystem hashes of a compiled BuildIR
  (geometry, terrain, vegetation, objects, gameplay).
- `BuildCache`: persisted record of the last successfully built hashes
  (`load`/`save` to JSON), so unchanged subsystems are not rebuilt.
- `BuildGraph::dirty`: lists subsystems whose hash changed since the cache.
- `BuildGraph::run`: compiles BuildIR, writes `track.build.json`, invokes the
  runner only for dirty subsystems, and updates the cache only on success.
- `Runner` trait: `ProcessRunner` (real `std::process::Command`) and a
  unit-test fake. A failed candidate never enters the cache.

## Validation

```text
cargo test --workspace                         PASS: 55 tests
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
```

## Explicit limitations

- Cache/persisted state is JSON in memory/file; a full disk-backed artifact
  store is not yet implemented.
- Build progress/cancellation surfacing to the UI requires the Tauri shell
  (`TS-120`).
- The runner currently invokes Blender headless with per-subsystem args; wiring
  the actual Blender materializer script arguments is part of the remainder.

The next step is the Tauri shell and typed IPC (`TS-120`) so the orchestrator
can be driven from the desktop UI, then Godot integration (`TS-160`).
