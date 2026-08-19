# TS-050 track-commands Status

## Result

The command layer is implemented in:

```text
tools/track_studio/crates/track-commands/
```

It is independent of Tauri, Blender, Godot and SQLite. It provides a reversible
`Command` abstraction, an in-memory undo/redo `CommandHistory`, and batching so
that continuous mouse previews collapse into a single undo step.

## Implemented behavior

- `Command` trait with `apply`/`undo` and a label.
- `MoveControlPoint`, `MoveAsset`, `AddControlPoint`, `DeleteControlPoint`.
- `CommandHistory::execute`, `undo`, `redo`.
- `begin_batch` / `end_batch`: a sequence of preview edits becomes one undo step.
- `cancel_batch`: discards preview edits and restores the pre-batch state.
- A new committed edit clears the redo stack.

## Validation

```text
cargo test --workspace                         PASS: 20 tests
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
```

## Explicit limitations

- Commands mutate the `TrackDocument` in memory; persistence is still owned by
  `track-storage` and Git is still an explicit, separate user action.
- The command set covers centerline and asset moves plus point insert/delete;
  more domain commands will be added as editing tools land.
- Undo/redo is not yet surfaced through a Tauri IPC command surface.

The next step is to connect this history to the storage save path and to the
typed IPC command surface once `TS-120` is implemented.
