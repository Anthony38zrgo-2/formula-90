# Formula90s Analysis Suite (ANL-001)

Deterministic, headless 3D geometry analysis for the Formula90s repository.
The suite turns a geometric question into machine-readable evidence: JSON to
stdout, an identical report file on request, and a documented exit contract.

It is an **analysis and validation** tool: it never mutates a source asset,
scene, texture, tracked GLB, OBJ or `.blend` file. It writes only requested
JSON reports and temporary test fixtures.

## Scope and non-goals

In scope:

- structural GLB audit (delegated to the existing vehicle pipeline inspector);
- transform-aware per-vertex predicate evidence over GLB geometry;
- read-only OBJ validation (delegated to the existing validator);
- headless Blender scene/object mesh and transform facts;
- opt-in, config-driven vehicle measurements (pass / fail / not_measurable).

Non-goals (by design):

- no rendering, screenshot capture or visual acceptance of any kind;
- no asset mutation, import of external assets, or `.blend` saving;
- no FIA limits, legal-compliance claims or compliance validation;
- no OBJ parsing or repair logic (that belongs to `blender/obj_validator/`);
- no GUIs, interactive prompts, network requests, package installation or
  virtual environments (zero third-party runtime dependencies).

## Source-preserving guarantee

Every command reads its inputs and writes only the requested report. The
existing inspectors/validators are invoked read-only: `validate_obj.py` is
always called without any repair flag and with `--logs-dir` redirected to a
temporary directory, so the repository is never written to. Nothing in this
suite invokes `fix_obj.py` or any repair path (guarded by a unit test).

## Architecture

```text
run_analysis.py  (scripts/)                thin entry point, stdlib only
        |
        v
analysis_suite/cli.py                      one stable CLI, JSON + exit contract
        |  audit-glb   |  probe-geometry   |  validate-obj  |  probe-blend  |  vehicle-measure
        v              v                   v                v               v
glb_adapter.py    predicates.py      obj_adapter.py    blender_headless.py  policies/vehicle.py
        |          (world-space      |  subprocess ->   |  subprocess ->     |  composes generic
        |           vertex evidence) |  obj_validator/  |  Blender 5.2 head- |  predicate evidence;
        v                            |  validate_obj.py |  less: scripts/    |  semantics stay generic
vehicle_pipeline/inspect_glb.py       |  (read-only,    |  blender_scene_    |
(reused, never recomputed)            |   temp logs)    |  probe.py (bpy)    v
                                                                        predicates.py (generic)

common plumbing: paths.py (repo-root discovery), hashing.py (SHA-256),
                 contracts.py (exit codes, config validation),
                 reporting.py (deterministic JSON envelope)
```

## Commands

All commands accept `--config <repo-relative-json>` (required) and
`--report <repo-relative-json>` (optional). Config and report paths are
resolved against the repository root discovered by walking upward from the
executing script, never against the current working directory. Input paths in
configs are repository-relative (absolute paths are also accepted).

| Command | Purpose |
|---|---|
| `audit-glb` | structural GLB inspection via the existing vehicle inspector |
| `probe-geometry` | transform-aware per-vertex predicate evidence for GLB geometry |
| `validate-obj` | read-only adapter to `blender/obj_validator` |
| `probe-blend` | headless Blender scene/object mesh and transform facts |
| `vehicle-measure` | Formula90s vehicle-specific, config-driven measurements |

Primary invocation (Blender 5.2 bundled Python; `--help` works without
Blender):

```powershell
& "C:\Program Files\Blender Foundation\Blender 5.2\5.2\python\bin\python.exe" `
  blender\analysis_suite\scripts\run_analysis.py <subcommand> `
  --config <repo-relative-json> --report <repo-relative-json>
```

### JSON and exit-code contract

Every command emits exactly one JSON object to stdout with sorted keys:

```json
{
  "suite": {"name": "formula90s-analysis-suite", "version": "1.0.0"},
  "command": "audit-glb",
  "metadata": {"python": "3.13.x"},
  "inputs": [{"path": "repo/relative/path.glb", "sha256": "…", "size_bytes": 0}],
  "findings": [{"level": "info|warning|error", "code": "…", "message": "…"}],
  "evidence": { /* command-specific */ },
  "summary": {"status": "pass|fail|error|not_measurable", "exit_code": 0},
  "exit_code_explanation": "0: PASS — …"
}
```

- `--report` writes an identical JSON document (same bytes as stdout plus a
  trailing newline) to the requested repository-relative path.
- Findings are sorted deterministically by `(code, level, message)`. Reports
  contain no timestamps, random identifiers or machine-specific absolute
  paths.
- Inputs carry SHA-256 hashes (uppercase). When a config declares
  `expected_sha256`, a mismatch is a `warning` finding; it never silently
  passes.

Exit codes:

| Code | Meaning | Triggering commands |
|---|---|---|
| `0` | PASS | all commands on success |
| `1` | FAIL | deterministic validation failure (`vehicle-measure`, `validate-obj`) |
| `2` | CONFIG | invalid config, precondition or input (missing path, unsupported extension, Blender/Python unavailable, `not_measurable` outcome, hash precondition) |
| `3` | CORRUPT | corrupt or unsupported asset (malformed GLB, unopenable `.blend`) |

`not_measurable` is never silently treated as `pass`: any `not_measurable`
measurement makes `vehicle-measure` exit `2`, and a `fail` always takes
precedence over `not_measurable` (exit `1`). Invalid configs and unsupported
inputs always produce a JSON error envelope and never a traceback.

## Configuration examples

### audit-glb

```json
{
  "source": "game/assets/models/vehicles/f1_90s_canonical_1997/source/jordan_191_1995/jordan_191_1995_chassis_source.glb",
  "expected_sha256": "68A97592AE978E875D4978E90D158479D724E43E88A6F656A757980B99F2A9E4",
  "full": true,
  "normal_tolerance": 0.01
}
```

### probe-geometry

Predicate names, axes, centres, thresholds, sections and tolerances live only
in config (see `configs/example_probe_geometry.json`). Supported predicate
types: `above_axis`, `below_axis`, `ahead_of_axis`, `behind_axis`,
`inside_band`, `outside_band`, `radial_distance`. Axes may be signed (`-Z`
inverts the comparison direction, which is how "forward = -Z" conventions are
expressed). Sections are named axis-bound ranges. Samples are bounded and
lexicographically ordered by `(vertex_index, position)`; a full vertex dump is
never produced by default.

### vehicle-measure

Opt-in policy layer (see `configs/example_vehicle_measurement.json`). Sources
carry `expected_sha256`; datums may be planes (`axis` + `value`) or points;
measurements output `pass`, `fail` or `not_measurable` per requested
measurement. Measurement kinds: `datum_distance`, `max_extent`,
`max_abs_extent`, `forward_extent_beyond_datum`,
`behind_extent_beyond_datum`, `radial_about`.

### validate-obj

```json
{
  "source": "blender/obj_validator/fixed/F1-97_front_wing_symmetric_fixed.obj",
  "python_executable": "C:\\Program Files\\Blender Foundation\\Blender 5.2\\5.2\\python\\bin\\python.exe"
}
```

The exact validator command is documented in the report in repository-relative
form. The validator's human-readable, non-JSON output is classified
(`pass` / `fail` / `precondition`) and never machine-parsed; its timestamped
log files are confined to a temporary directory.

### probe-blend

```json
{
  "blend_file": "blender/F1-97_front_wing_symmetric.blend",
  "objects": ["aleron trasero", "aleron delantero"],
  "blender_executable": "C:\\Program Files\\Blender Foundation\\Blender 5.2\\blender.exe"
}
```

`objects` may be empty (all mesh objects, sorted by name). Reports existence,
type, world transform, determinant, evaluated vertex/triangle counts,
local/world bounds, material slots and finite-vertex evidence per object.

## Pure Python vs Blender headless

- Pure Python (no `bpy`): `audit-glb`, `probe-geometry`, `validate-obj`,
  `vehicle-measure`, and `probe-blend`'s argument handling. Run with Blender
  5.2's bundled Python as shown above.
- Blender headless: `probe-blend` additionally spawns
  `blender.exe --background --python scripts/blender_scene_probe.py -- …`.
  The `.blend` is opened without saving, evaluated meshes are cleared, nothing
  is rendered or imported. If Blender is unavailable the command returns a
  deterministic JSON precondition error (exit `2`).

No GUI, interactive prompt, network request, package installation or current
working-directory dependency is used. `blender/track_pipeline/.venv` is never
used.

## How K3's exploration informed the policy

`blender/vehicle_pipeline/examples/deep_analysis_k3.py` proved the useful
analysis concepts: named front/rear axle datums, height/width/overhang
sections, wheel-centre radial measurement and investigation thresholds. Its
hard-coded paths, datums, thresholds and semantic assumptions were removed;
every one of those values now lives in config:

| K3 hard-code | Now lives in |
|---|---|
| `FRONT_AXLE_Z = -1.4394`, `REAR_AXLE_Z = 1.4906` | `configs/example_vehicle_measurement.json` → `datums` |
| `HEIGHT_THRESHOLD_GLB_Y = 0.6198` | measurement `expected` bounds |
| `WIDE_FWD_LIMIT = 0.70`, `REAR_ZONE_WIDTH_LIMIT = 0.50` | measurement `expected` bounds |
| `SRC = r'D:\Formula90s\…'` | `sources[].path` (repository-relative) |
| wheel radius in the YZ plane | `radial_about` + `plane_axis` |
| forward = negative Z | `coordinate_convention: {"forward": "-Z", …}` |

The example thresholds are investigation values with documented margins, not
FIA limits or compliance claims. Measured values on the promoted Jordan 1995
sources: wheelbase 2.93 m, cockpit height 0.650 m, half-width 0.718 m,
front overhang 0.906 m, rear overhang 0.754 m, front wheel radius 0.354 m.

## Reuse boundaries

- `blender/vehicle_pipeline/inspect_glb.py` is imported and wrapped
  (`glb_adapter.py`); `audit-glb` returns its evidence instead of recomputing
  it. The directory is never modified.
- `blender/obj_validator/` owns OBJ/MTL validation (and repair). This suite
  only calls `validate_obj.py` in read-only mode and never invokes
  `fix_obj.py` or any repair flag. Pre-existing limitations are recorded in
  the report, not fixed here:
  - the validator requires `trimesh`, which is not installed in Blender 5.2's
    bundled Python (and the suite must not install packages) → classified as
    `precondition` / `not_measurable` (exit `2`);
  - its output is human-readable text plus timestamped log files, not JSON →
    classified, not machine-parsed.
- `deep_analysis_k3.py` remains an example only, not a production API.

## Deterministic tests

```powershell
& "C:\Program Files\Blender Foundation\Blender 5.2\5.2\python\bin\python.exe" `
  -m unittest discover -s blender\analysis_suite\tests -v
```

`unittest` only. All GLB fixtures are generated in temporary directories and
leave no files behind. Unit tests never require `bpy`; `probe-blend` coverage
exercises the deterministic adapter paths (missing Blender executable, missing
blend file) so they skip cleanly when Blender is unavailable.

## Visual-review authority

This suite produces numeric, machine-readable evidence only. It never claims
visual acceptance. Any later visual validation stage gate (renders,
screenshots, viewport output) requires, per repository policy: an
image-capable reviewer (GPT with image capability or manual human review) and
explicit user confirmation before downstream visual work may proceed.
