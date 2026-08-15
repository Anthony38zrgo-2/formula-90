# TS-120 Application Service Status

## Result

The Rust application service layer behind the typed IPC surface and the Tauri
shell are implemented:

```text
tools/track_studio/crates/track-service/   (application service)
tools/track_studio/app/src-tauri/          (Tauri shell)
```

The service owns the current `TrackDocument`, the storage connection, the
command history, the asset registry and the build graph, and is fully
unit-testable. The Tauri shell exposes thin `#[tauri::command]` wrappers over
it. The shell is a standalone crate (kept out of the core workspace so the
Tauri toolchain does not enter the core test/build loop).

## Implemented commands

- Project lifecycle: `project_open`, `project_save`.
- Snapshot: `document_snapshot` (canonical JSON + runtime metadata + diagnostics).
- Validation: `document_validate`.
- Commands: `document_undo`, `document_redo`.
- Build: `compile_build_ir`, `build_plan` (dirty subsystems).

## Validation

```text
cargo test --workspace                         PASS: 60 tests
cargo fmt --all -- --check                     PASS
cargo clippy --workspace --all-targets -- -D warnings  PASS
cargo check (app/src-tauri)                    PASS
cargo build (app/src-tauri)                    PASS -> track-studio-tauri.exe (~15.5 MB)
```

## Explicit limitations

- The frontend is a placeholder `index.html`; the TypeScript UI (`TS-130`)
  does not yet invoke these commands.
- Real (non-dry-run) Blender/Godot execution is wired through the `track-build`
  `Runner` trait but is not yet surfaced as a command.

The Tauri shell is complete and compiles to a Windows desktop binary. Next is
the frontend (`TS-130`) and real build/playtest wiring (`TS-160`).
