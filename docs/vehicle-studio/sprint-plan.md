# Formula-90 Vehicle Studio Sprint Plan

Status: `PLANNING`

Architecture authority: `docs/vehicle-studio/architecture-spec.md`

Backlog authority until an approved backlog database seed mechanism is
restored: `docs/vehicle-studio/implementation-backlog.md`

## 1. Delivery protocol

Every sprint and every implementation item follows the repository pipeline:

```text
sprint planning
  -> backlog item
  -> implement one bounded delta
  -> read-only review
  -> automated evidence
  -> human gate
  -> sprint retrospective
  -> done
```

`READY_FOR_HUMAN_GATE` is the maximum automated status. Only the user may mark
visual work `ACCEPTED` or `REJECTED`.

At sprint start:

1. Record branch, HEAD and worktree status.
2. Inventory overlapping local changes.
3. Confirm the sprint's exact write ownership.
4. Capture or verify the smallest relevant baseline.
5. Select one backlog item at a time.

At sprint close:

1. Run the declared item validations.
2. Review explicit staged paths and provenance.
3. Produce structured evidence and known limitations.
4. Request the human gate when visual or product acceptance is required.
5. Record the retrospective before marking work done.

No sprint may publish into active Godot runtime assets unless its scope
explicitly includes the separate runtime integration gate.

## 2. Phase map

| Order | Sprint | Backlog | Objective | Exit evidence |
|---:|---|---|---|---|
| 0 | VS Sprint 0 | VS-000..VS-006 | Baseline, contracts, source provenance and Williams mapping plan | Approved docs, deterministic baseline reports, no source mutation |
| 1 | VS Sprint 1 | VS-010..VS-015 | VehicleDocument, scan adapters and semantic onboarding | Williams accepted mapping plus canonical document |
| 2 | VS Sprint 2 | VS-020..VS-025 | Local service, Vue/Tailwind shell and linked CAD/3D read-only views | Deterministic four-view UI and before/after baseline |
| 3 | VS Sprint 3 | VS-030..VS-036 | First editable vertical slice: global size, wheelbase and tires | Staged Blend/GLB variant, topology and ground-contact gates |
| 4 | VS Sprint 4 | VS-040..VS-044 | Nose, sidepods, engine cover and wing deformers | Component parameter acceptance matrix and visual gate |
| 5 | VS Sprint 5 | VS-050..VS-054 | Blender material library, livery and baking | Authoritative Blend plus validated PBR runtime preview |
| 6 | VS Sprint 6 | VS-060..VS-064 | Revisions, reproducibility, diagnostics and hardening | Clean rebuild parity and failure/rollback evidence |
| 7 | VS Sprint 7 | VS-070..VS-073 | Prove generic F1 onboarding with a second vehicle | Second-car mapping/build without Williams-specific code |
| 8 | VS Sprint 8 | VS-080..VS-083 | Optional Godot integration and final documentation | Separate runtime human gate and retrospective |

## 3. Sprint 0 — contracts and baseline

### Goal

Make the first code change falsifiable and safe. Close source-of-truth gaps in
the current Williams package before any editor or deformation logic exists.

### Planned work

- Register the epic and backlog when a supported seed/validation mechanism is
  available; do not edit `.agents/data/agents.sqlite` directly.
- Freeze architecture decisions and schema ownership.
- Capture current GLB hashes, structure, dimensions, topology evidence,
  materials, textures, datums and joints.
- Classify the current manifest discrepancy: its mesh summary represents an
  older 3,120-triangle state while the current assembled asset is 10,308
  triangles.
- Record missing refinement-script provenance and `/mnt/data` report paths.
- Define a human semantic-mapping worksheet for the Williams.
- Define canonical fixtures that may be generated in temporary directories.

### Exit gate

- Architecture, sprint plan and backlog are reviewed.
- Williams source hashes and current limitations are captured without changing
  the untracked source package.
- Schema owners and proposed implementation paths are accepted.
- The first implementation item has explicit tests and rollback.

### Explicit exclusions

- No Vue project creation.
- No Blender save/export.
- No asset normalization or manifest repair.
- No code or runtime changes.

## 4. Sprint 1 — scan and semantic authority

### Goal

Create a deterministic VehicleDocument from GLB and Blend inputs through an
explicit human mapping gate.

### Vertical slice

```text
Williams GLB
  -> structural scan
  -> evaluated scan
  -> semantic suggestions
  -> mapping confirmation fixture
  -> canonical VehicleDocument
  -> validate twice and compare bytes
```

### Exit gate

- GLB scan reuses the existing inspector and analysis suite.
- Blend scan opens source read-only with factory startup and never saves it.
- Ambiguous semantics remain diagnostics until accepted.
- Canonical documents reject non-finite values, duplicate IDs, invalid axes,
  unsupported units and incomplete wheel/axle frames.
- Equivalent inputs/mappings serialize identically.

## 5. Sprint 2 — read-only authoring shell

### Goal

Allow a user to inspect and confirm the semantic model before deformation.

### Vertical slice

```text
VehicleDocument
  -> top/side/front/rear canonical SVG
  -> local project API
  -> Vue/Tailwind view panels
  -> interactive GLB preview
  -> linked selection and diagnostics
```

### Exit gate

- Four SVG projections are byte-stable for unchanged input.
- SVG sanitization rejects scripts, external resources and undeclared controls.
- Selecting a component highlights it in all views and the 3D preview.
- Before and after panes use identical camera definitions.
- Save produces semantic revisions only and does not invoke Blender.

### Human gate

The user confirms that the Williams views, component mapping, dimensions and
interaction model are intelligible before editing is enabled.

## 6. Sprint 3 — editable dimensions vertical slice

### Goal

Prove the complete edit-to-variant architecture using only global dimensions,
wheelbase and tires.

### Required edits

- Absolute and percentage wheelbase.
- Absolute and percentage vehicle width.
- Front and rear tire radius.
- Front and rear tire width.
- Centered, inboard-fixed and outboard-fixed tire-width policies.

### Exit gate

- Wheel anchors remain the wheel centers.
- Axle ownership moves all dependent wheel and suspension frames together.
- All four tire contact points remain on the declared ground plane.
- Resulting chassis height/rake is explicit and measured.
- Vertex/triangle connectivity matches the source.
- UV values remain unchanged during deformation.
- A staged `.blend` reopens and the three runtime GLBs pass structural audit.
- Rebuilding with identical inputs produces equivalent geometry and report
  hashes from a clean staging directory.

### Human gate

The user reviews fixed before/after CAD views, interactive previews and Blender
renders. No runtime publication occurs.

## 7. Sprint 4 — component deformers

### Goal

Add bounded semantic editing for the requested body areas without changing
topology.

### Dependency order

1. Nose stations and transition boundary.
2. Sidepod paired profiles and coke-bottle region.
3. Engine-cover/airbox stations.
4. Existing front/rear wing element transforms.

### Exit gate

- Each deformer owns explicit vertex bindings and protected boundaries.
- Symmetry is on by default and can only be disabled deliberately.
- Deformer order is deterministic and recorded in BuildIR.
- Bounds, degeneracy, inversion, clearance and self-intersection diagnostics
  block invalid candidate builds.
- Existing datums and material regions remain traceable.

### Human gate

A parameter matrix covering minimum, baseline and maximum values is visually
reviewed for each component.

## 8. Sprint 5 — materials and livery

### Goal

Produce professional Blender authoring materials and a portable runtime PBR
representation.

### Dependency order

1. Versioned node-library contract.
2. Carbon, painted composite, metal, rubber and plastic recipes.
3. Material-region assignment UI.
4. High-resolution livery layer model and UV view.
5. Deterministic bake/export.
6. Browser PBR preview and authoritative Blender render comparison.

### Exit gate

- The `.blend` preserves editable materials and livery sources.
- Runtime GLBs reference complete baked PBR textures by relative path.
- Source hashes, node-library version, color-management settings and bake
  parameters are present in provenance.
- Missing maps, unassigned faces and unsupported recipes block candidate
  acceptance.
- Visual quality reaches `READY_FOR_HUMAN_GATE`, never automatic acceptance.

## 9. Sprint 6 — project store and hardening

### Goal

Make revisions, recovery and clean rebuilds trustworthy.

### Exit gate

- Revisions are immutable and content addressed.
- Undo/redo operates on semantic commands.
- Build and Save are separate.
- Concurrent/conflicting build requests are serialized safely.
- Interrupted builds leave no active partial variant.
- Source mismatch, Blender-version mismatch and node-library mismatch fail
  before materialization.
- Clean rebuilds never reuse foreign `.blend`, GLB, object or cache files.

## 10. Sprint 7 — generic F1 proof

### Goal

Onboard a second F1 open-wheel asset without adding vehicle-name conditionals to
the domain, scanner, compiler or UI.

### Exit gate

- The second asset completes the same assisted mapping flow.
- Optional/missing component roles are represented explicitly.
- Its VehicleDocument and views validate deterministically.
- At least one global, tire and body-component edit builds successfully.
- Any required adapter is semantic/configuration data, not a hard-coded model
  identity branch.

## 11. Sprint 8 — optional runtime integration

### Goal

Integrate an accepted variant only after authoring acceptance, using the
existing three-GLB vehicle contract and runtime parity rules.

### Exit gate

- Integration stages explicit paths only.
- Runtime package contains chassis, shared front wheel and shared rear wheel.
- Godot imports and loads the candidate headlessly.
- `run_f1_94.ps1` parity safeguards remain intact.
- Physics placement remains owned by the existing vehicle runtime.
- Human approval is recorded before activation.

## 12. Sprint metrics

Track evidence, not activity counts:

- deterministic fixture/pass count;
- unresolved diagnostics by stable code;
- canonical rebuild hash parity;
- source assets modified: must remain zero;
- topology violations: must remain zero for accepted builds;
- unsupported semantic ambiguity accepted silently: must remain zero;
- candidate build rollback success;
- human-gate result and requested corrections.

## 13. Stop conditions

Stop the current item and return to planning when:

- a requested edit requires topology change;
- wheel/ground/anchor constraints conflict without a declared policy;
- the source hash or current branch changes during a build;
- another local change overlaps owned files;
- Blender evaluation differs between repeated clean builds without an explained
  operational cause;
- the UI would need to become persistence or geometry authority;
- visual acceptance is required but has not been provided.

