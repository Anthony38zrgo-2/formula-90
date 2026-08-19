# Formula-90 Track Studio Migration Plan

## Plan contract

Each item below corresponds to a backlog entry in the `track-studio` epic.
Implementation follows dependency order, not directory order. Every item must
record its own files, risk, tests, acceptance criteria and rollback condition
before work begins.

## Phase map

| Order | Backlog | Objective | Main validation |
|---:|---|---|---|
| 0 | TS-000, TS-001, TS-002, TS-003 | Inventory, backlog, baseline and contracts | Reports, hashes, parity and decision review |
| 1 | TS-010 | Pure Rust TrackDomain | Domain invariants and serialization |
| 2 | TS-020 | SQLite `.f90track` storage | Migrations, save/reopen and rollback |
| 3 | TS-120 | Tauri shell and typed IPC | Open/create/save/validate command path |
| 4 | TS-050 | Commands, undo/redo and revisions | Apply/undo/redo/recovery tests |
| 5 | TS-030 | Deterministic geometry | Golden samples, station and collision parity |
| 6 | TS-040 | Unified diagnostics | Invalid input blocks build and focuses ownership |
| 7 | TS-090 | TrackDocument to BuildIR | Versioned byte-stable compiler output |
| 8 | TS-110 | Blender materialization boundary | Same GLB and collision contracts |
| 9 | TS-100 | Incremental build graph | Selective invalidation and atomic activation |
| 10 | TS-080 | Gameplay/runtime metadata | Same source drives runtime data |
| 11 | TS-160 | Godot integration | Headless GLB and metadata validation |
| 12 | TS-180 | Cleanup and server retirement | Compatibility parity before removal |
| 13 | TS-200, TS-210 | Documentation reconstruction and final acceptance | Full checklist and Windows build |

## Phase 0 work packages

### P0-A: Backlog registration

Register `TS-000` first, then all child items through `TS-210` in
`.agents/backlog/backlog_seed.json`. Run seed and validation. Do not create a
second backlog database or edit `agents.db` directly.

### P0-B: Repository and ownership inventory

Use the current authoring, pipeline, tests, scripts, asset registry, track
revisions and Godot validator as evidence. Produce:

- `PHASE_0_INVENTORY.md`
- `CURRENT_FLOWS.md`
- `CONTRACTS_V0.md`
- an active/compatibility/historical/obsolete classification

### P0-C: Baseline and falsification

Run the smallest checks first:

1. Python unit tests.
2. Fixture compile twice and compare bytes.
3. La Chutana import with parity checks.
4. Temporary Blender build.
5. Temporary Godot GLB validation.

No baseline command may publish into `game/assets/generated/` or alter the
active runtime. Results must be `PASS`, `FAIL` or `INCONCLUSIVE`, never an
assumption.

### P0-D: Compatibility and contract map

Define mappings from current SVG/JSON/raster/revision forms into the target
TrackDocument. Draft the schemas and invariants for TrackDocument, BuildIR,
Diagnostic, runtime metadata and typed IPC. Record unresolved decisions in
`DECISIONS.md` before TS-010 starts.

### P0-E: Phase 1 readiness review

Phase 1 may start only when:

- backlog is seeded and queryable;
- current ownership is documented;
- fixture and La Chutana baselines are captured;
- source-of-truth conflicts are classified;
- TrackDocument and BuildIR contracts have one owner;
- the first Rust change has a bounded test and rollback;
- unrelated worktree changes remain untouched.

## Vertical migration slice

The first implementation slice after Phase 0 is intentionally narrow:

```text
minimal legacy SVG
  -> TrackDocument import
  -> .f90track save/reopen
  -> track.export.json
  -> BuildIR
  -> existing Blender adapter
  -> GLB pair
  -> Godot headless validation
```

It must pass before migrating the complete editor UI, vegetation rules or
La Chutana. This preserves the current Python pipeline while testing the new
authority boundary.

## Rollback strategy

- Planning changes are limited to backlog and `docs/track-studio/`.
- Rust candidates are isolated behind new crates and tests.
- Existing Python scripts remain runnable until parity is proven.
- Failed generated builds remain in temporary/staged directories.
- Active runtime files are replaced only through validated atomic activation.
- Failed candidates are reverted; useful tests and evidence are retained.
