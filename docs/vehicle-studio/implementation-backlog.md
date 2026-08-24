# Formula-90 Vehicle Studio Implementation Backlog

Status: `PROPOSED`

Epic: `vehicle-studio`

Architecture: `docs/vehicle-studio/architecture-spec.md`

Sprint plan: `docs/vehicle-studio/sprint-plan.md`

## 1. Backlog contract

Items are implemented in dependency order and remain `PROPOSED` until sprint
planning selects one. Each item owns a bounded delta and must record actual
evidence before it can reach `READY_FOR_HUMAN_GATE` or `DONE`.

```text
PROPOSED -> PLANNED -> IN_PROGRESS -> REVIEWED
         -> READY_FOR_HUMAN_GATE -> ACCEPTED -> DONE
                                  -> REJECTED -> revision or rollback
```

The supported `agentdb` executable and the referenced
`.agents/backlog/backlog_seed.json` are absent in this checkout. This document
is therefore the temporary versioned authority. `VS-000` registers these items
only after a supported seed/validation mechanism is available. Direct edits to
`.agents/data/agents.sqlite` are forbidden.

Every implementation item must verify branch/HEAD/status, declare exact file
ownership, preserve unrelated work, stage explicit paths, avoid active runtime
publication unless authorized, and include risk, tests, acceptance and
rollback.

## 2. Epic and phase-zero items

### VS-000 — Register Vehicle Studio epic

- Status: `BLOCKED_BY_TOOLING`
- Depends on: none
- Scope/files: supported backlog seed only; no database edits.
- Objective: register this epic and all child items using the supported backlog
  mechanism.
- Risk: corrupting/diverging local agent data by guessing a schema.
- Tests: supported seed and validation commands pass; every VS ID is queryable
  exactly once.
- Acceptance: versioned seed and queryable epic agree with this document.
- Rollback: remove only new seed entries; never edit
  `.agents/data/agents.sqlite` manually.

### VS-001 — Capture Williams immutable baseline

- Status: `DONE`
- Depends on: none
- Scope/files: new generated reports; Williams source remains read-only.
- Objective: capture hashes, GLB structure, evaluated bounds, dimensions,
  topology, UV, normals, materials, textures, datums and joints.
- Risk: treating the current untracked package as stable without hashes.
- Tests: run structural inspection twice and compare canonical report bytes.
- Acceptance: all six GLBs and 39 texture paths are accounted for; source
  before/after hashes are identical.
- Rollback: delete only newly generated reports/staging.

### VS-002 — Reconcile Williams manifest evidence

- Status: `DONE`
- Depends on: VS-001
- Scope/files: new planning report; no source manifest mutation.
- Objective: explain the 3,120-triangle mesh summary versus the current 10,308
  assembled triangles and classify stale/accurate fields.
- Risk: normalizing evidence in place and losing provenance.
- Tests: computed metrics match inspector output and refinement reports.
- Acceptance: a discrepancy matrix identifies current value, owner and repair
  recommendation for every affected field.
- Rollback: remove the new report only.

### VS-003 — Refinement provenance recovery plan

- Status: `DONE`
- Depends on: VS-001
- Scope/files: documentation/reports only.
- Objective: classify missing transformation scripts and ephemeral `/mnt/data`
  references; define the minimum reproducible recipe.
- Risk: falsely implying reports alone reproduce the current asset.
- Tests: every reported operation maps to source, parameters, expected output
  and known/unknown implementation evidence.
- Acceptance: unknown provenance remains explicit and blocks claims of complete
  source reproducibility.
- Rollback: remove the new provenance document.

### VS-004 — Freeze VehicleDocument v1 contract

- Status: `DONE`
- Depends on: VS-001, VS-003
- Scope/files: schema, contract documentation and fixtures only.
- Objective: create JSON Schema and canonical serialization rules.
- Risk: embedding Williams object names in the generic domain.
- Tests: valid fixture passes; duplicate IDs, non-finite floats, invalid axes and
  incomplete wheel frames fail deterministically.
- Acceptance: schema represents a second unnamed F1 without model-name branches.
- Rollback: revert schema/fixture files only.

### VS-005 — Freeze VehicleView SVG v1 profile

- Status: `DONE`
- Depends on: VS-004
- Scope/files: SVG profile, sanitizer contract and fixtures.
- Objective: define allowed elements, roles, view transforms and parameter
  bindings.
- Risk: arbitrary SVG becoming geometry authority or an injection surface.
- Tests: safe fixtures canonicalize; scripts, external URLs, event handlers and
  undeclared controls fail.
- Acceptance: unchanged input compiles to byte-identical four-view SVG.
- Rollback: revert profile and fixtures only.

### VS-006 — Williams semantic mapping worksheet

- Status: `DONE`
- Depends on: VS-001, VS-004
- Scope/files: mapping fixture/documentation only.
- Objective: map meshes, primitives, datums and joints to generic roles and
  mark protected/deformable regions.
- Risk: accepting inferred mappings without human confirmation.
- Tests: every primitive is mapped, ignored with reason or unresolved.
- Acceptance: reaches `READY_FOR_HUMAN_GATE`; user acceptance alone may create
  the initial document.
- Rollback: remove the proposed mapping fixture.

## 3. Domain and scan items

### VS-010 — VehicleDocument domain library

- Status: `DONE`
- Depends on: VS-004
- Scope/files: isolated pure-Python package and unit tests.
- Objective: typed loading, validation, canonical ordering and serialization.
- Risk: mixing filesystem/process behavior into the domain.
- Tests: IDs, floats, axes, frames, parameters, sorting and byte parity.
- Acceptance: no Blender, Vue, HTTP or runtime dependency.
- Rollback: remove only the new package/tests.

### VS-011 — GLB scan adapter

- Status: `DONE`
- Depends on: VS-001, VS-010
- Scope/files: adapter reusing `inspect_glb.py` and `analysis_suite`.
- Objective: normalize existing structural evidence without duplicate parsing.
- Risk: inconsistent recomputation of GLB facts.
- Tests: adapter evidence equals existing inspector on fixtures and Williams.
- Acceptance: source is read-only, hashed and reported deterministically.
- Rollback: remove adapter/tests; existing inspector remains unchanged.

### VS-012 — Blend read-only scan adapter

- Status: `DONE`
- Depends on: VS-010
- Scope/files: isolated Blender probe and adapter tests.
- Objective: inspect evaluated objects, modifiers, materials, UVs and custom
  properties without saving the source.
- Risk: Blender startup handlers or source mutation.
- Tests: temporary Blend hash unchanged; factory-startup scan repeats;
  corrupt/unsupported files return diagnostics.
- Acceptance: scan never saves and records Blender version.
- Rollback: remove probe/adapter/tests.

### VS-013 — Semantic suggestion engine

- Status: `DONE`
- Depends on: VS-011, VS-012
- Scope/files: pure suggestion/evidence module and fixtures.
- Objective: propose F1 roles/datums from names, hierarchy, geometry and
  symmetry without creating authority.
- Risk: false-confidence classification.
- Tests: confidence/evidence goldens; ambiguity remains unresolved.
- Acceptance: every suggestion exposes evidence and is rejectable/overridable.
- Rollback: remove suggestions; manual mapping remains.

### VS-014 — Assisted semantic onboarding

- Status: `DONE`
- Depends on: VS-005, VS-006, VS-013, VS-021
- Scope/files: onboarding API/UI and command tests.
- Objective: confirm components, axes, ground, wheels, frames, regions and
  symmetry before document creation.
- Risk: incomplete mapping saved as valid.
- Tests: unresolved required roles block; optional roles remain explicit;
  accepted commands generate canonical bytes.
- Acceptance: Williams reaches human gate without manual JSON editing.
- Rollback: remove onboarding surface; scan remains read-only.

### VS-015 — Initial revision compiler

- Status: `DONE`
- Depends on: VS-010, VS-014
- Scope/files: mapping-to-document compiler and deterministic fixtures.
- Objective: create revision zero only from accepted mapping/source hash.
- Risk: paths/timestamps entering canonical bytes.
- Tests: compile twice in distinct temp paths and compare bytes/hashes.
- Acceptance: revision zero validates and references exact source hash.
- Rollback: remove compiler and temporary revisions.

## 4. Application and visualization items

### VS-020 — Local project service

- Status: `DONE`
- Depends on: VS-010
- Scope/files: isolated service, API contract and tests.
- Objective: own projects, commands, diagnostics and build requests.
- Risk: client-controlled paths or subprocess injection.
- Tests: root confinement, malformed payloads, concurrency and
  no-build-on-save.
- Acceptance: API never executes arbitrary client strings.
- Rollback: remove service; domain remains usable from CLI.

### VS-021 — Vue and Tailwind application shell

- Status: `DONE`
- Depends on: VS-020
- Scope/files: new Vehicle Studio frontend only.
- Objective: component tree, inspector, diagnostics and project state.
- Risk: dependency/cache files entering commits.
- Tests: frontend unit/build checks; ignored caches stay unstaged.
- Acceptance: app loads a read-only canonical project snapshot.
- Rollback: remove scoped frontend and dependency manifest changes.

### VS-022 — Deterministic orthographic SVG compiler

- Status: `DONE`
- Depends on: VS-005, VS-011, VS-015
- Scope/files: projection compiler and golden SVG fixtures.
- Objective: generate top, side, front and rear semantic CAD views.
- Risk: floating-point or contour-order instability.
- Tests: repeat/golden tests, explicit ordering and fixed formatting.
- Acceptance: Williams views are byte-stable and carry persistent IDs/hashes.
- Rollback: remove compiler/fixtures.

### VS-023 — Linked SVG view panels

- Status: `DONE`
- Depends on: VS-021, VS-022
- Scope/files: frontend SVG panels and selection tests.
- Objective: synchronize selection across four views.
- Risk: persisting mutated SVG DOM as authority.
- Tests: selection/command tests and sanitizer round trip.
- Acceptance: edits emit semantic commands; server state re-renders views.
- Rollback: revert panels; retain exported SVG.

### VS-024 — Before/after interactive 3D preview

- Status: `DONE`
- Depends on: VS-020, VS-021
- Scope/files: browser renderer, camera definitions and preview API.
- Objective: compare source/variant with identical cameras.
- Risk: browser PBR mistaken for authoritative Blender output.
- Tests: loading/error paths, camera parity and clear labeling.
- Acceptance: interactive and Blender-authoritative previews are distinct.
- Rollback: remove 3D panel; SVG inspection remains.

### VS-025 — Authoritative Blender preview renders

- Status: `PROPOSED`
- Depends on: VS-012, VS-020
- Scope/files: fixed render recipe and staged visual manifest.
- Objective: repeatable before/after images for human review.
- Risk: uncontrolled color, camera or lighting changes.
- Tests: setting/camera/light hashes and output dimensions.
- Acceptance: pairs differ only by vehicle artifact.
- Rollback: remove staged renders; never edit source Blend.

## 5. Build and global deformation items

### VS-030 — VehicleBuildIR compiler

- Status: `DONE`
- Depends on: VS-010, VS-015
- Scope/files: pure compiler, schema and fixtures.
- Objective: compile semantic parameters into ordered frame, deformation,
  material and export operations.
- Risk: Blender reinterpreting undeclared semantics.
- Tests: canonical bytes, dependency order and invalid-state rejection.
- Acceptance: BuildIR contains every operation/postcondition.
- Rollback: remove compiler/schema/fixtures.

### VS-031 — Blender materializer scaffold

- Status: `DONE`
- Depends on: VS-012, VS-030
- Scope/files: isolated `blender/vehicle_studio/` worker and fixtures.
- Objective: apply BuildIR in factory-startup Blender, writing only to staging.
- Risk: source mutation, implicit operator state or cache reuse.
- Tests: source hash unchanged; isolated repeat; hash mismatch fails pre-write.
- Acceptance: Blender performs no semantic selection.
- Rollback: remove worker/staging.

### VS-032 — Wheelbase and global-width deformer

- Status: `DONE`
- Depends on: VS-030, VS-031
- Scope/files: compiler operations, Blender application and fixtures.
- Objective: move axle frames and apply protected piecewise body deformation;
  support absolute/percentage values.
- Risk: suspension disconnects or cockpit/wing distortion.
- Tests: frame ownership, measured dimensions, protected stations, topology and
  UV parity.
- Acceptance: Williams min/base/max matrix passes.
- Rollback: disable/remove operation; prior revision stays active.

### VS-033 — Tire radius and ground-contact deformer

- Status: `DONE`
- Depends on: VS-030, VS-031
- Scope/files: wheel-local operation, chassis height/rake policy and tests.
- Objective: change axle radii while preserving ground and anchors.
- Risk: conflicting front/rear deltas or hidden rake.
- Tests: contact equation, anchor equality, reported height/rake and clearance.
- Acceptance: all wheels satisfy contact tolerance over the matrix.
- Rollback: revert operation; retain source/last accepted revision.

### VS-034 — Tire-width policies

- Status: `DONE`
- Depends on: VS-030, VS-031
- Scope/files: wheel-local operations and clearance tests.
- Objective: centered, inboard-fixed and outboard-fixed width.
- Risk: reversed local axis or suspension/chassis penetration.
- Tests: face-datum invariants for both axles/sides and collisions.
- Acceptance: wheel center/anchor invariant under every policy.
- Rollback: disable a failing policy independently.

### VS-035 — Variant Blend and GLB exporter

- Status: `READY_FOR_HUMAN_GATE`
- Depends on: VS-031, VS-032, VS-033, VS-034
- Scope/files: staging exporter and manifest tests.
- Objective: save variant Blend and chassis/front-wheel/rear-wheel GLBs without
  active publication.
- Risk: partial output or wrong component packaging.
- Tests: Blend reopen, GLB audit, relative textures and artifact hashes.
- Acceptance: staged package satisfies vehicle import standard.
- Rollback: delete candidate staging only.

### VS-036 — Global-dimensions vertical-slice gate

- Status: `PROPOSED`
- Depends on: VS-022, VS-024, VS-025, VS-032, VS-033, VS-034, VS-035, VS-060
- Scope/files: evidence manifest and review report only.
- Objective: assemble automated/visual evidence for edit-to-variant flow.
- Risk: proceeding without product validation.
- Tests: clean rebuild, topology/UV/frame/contact audit and paired render.
- Acceptance: reaches `READY_FOR_HUMAN_GATE`; user accepts/rejects.
- Rollback: candidate remains inactive and removable.

## 6. Body component deformer items

### VS-040 — Nose station deformer

- Status: `PROPOSED`
- Depends on: VS-036 accepted
- Scope/files: bindings, parameters, compiler/materializer and fixtures.
- Objective: edit length, height, width, inclination and transition.
- Risk: inverted faces or broken front-wing boundary.
- Tests: min/base/max, topology, symmetry, normals and continuity.
- Acceptance: protected transition/wing mount remain within tolerance.
- Rollback: remove nose operation/parameters only.

### VS-041 — Sidepod/coke-bottle deformer

- Status: `PROPOSED`
- Depends on: VS-036 accepted
- Scope/files: paired station bindings and tests.
- Objective: edit intake, width, height, undercut and rear taper.
- Risk: floor/cockpit intrusion or asymmetric binding.
- Tests: symmetry, protected regions, self-intersection and continuity.
- Acceptance: both sides pass min/base/max with zero topology changes.
- Rollback: remove sidepod operation/bindings only.

### VS-042 — Engine-cover/airbox deformer

- Status: `PROPOSED`
- Depends on: VS-041
- Scope/files: cover stations, protected regions and tests.
- Objective: edit airbox height/length, spine width and rear falloff.
- Risk: intersecting driver, sidepods or rear-wing mount.
- Tests: clearance, continuity, symmetry and bounds.
- Acceptance: protected regions/mounts remain valid.
- Rollback: remove engine-cover operation only.

### VS-043 — Existing wing-element deformer

- Status: `PROPOSED`
- Depends on: VS-036 accepted
- Scope/files: front/rear element bindings and transform tests.
- Objective: edit span, chord, height, incidence and Z position of existing
  elements.
- Risk: implying new elements or separating connected topology.
- Tests: ownership, mounts, symmetry, normals and clearance.
- Acceptance: only declared elements move; topology remains unchanged.
- Rollback: remove wing operations only.

### VS-044 — Body-deformer visual acceptance matrix

- Status: `PROPOSED`
- Depends on: VS-040, VS-041, VS-042, VS-043, VS-060
- Scope/files: evidence and paired renders only.
- Objective: review minimum/baseline/maximum plus combined presets.
- Risk: accepting isolated values while combinations fail.
- Tests: individual/combined presets and clean rebuild parity.
- Acceptance: reaches `READY_FOR_HUMAN_GATE`; user decision recorded.
- Rollback: reject recipes; retain accepted global slice.

## 7. Materials and livery items

### VS-050 — Versioned Blender node-library contract

- Status: `PROPOSED`
- Depends on: VS-035
- Scope/files: material-library Blend, manifest and validation.
- Objective: typed, hashed node groups for portable recipes.
- Risk: Blender-version coupling and untracked binary mutation.
- Tests: library opens; required groups/sockets and hashes match.
- Acceptance: library version/compatibility explicit.
- Rollback: remove new library/manifest.

### VS-051 — Core professional material recipes

- Status: `PROPOSED`
- Depends on: VS-050
- Scope/files: carbon, painted composite, metal, rubber and plastic recipes.
- Objective: bounded parameters and deterministic assignments.
- Risk: overstating browser/runtime parity.
- Tests: schema, node binding and fixed-scene renders.
- Acceptance: every recipe materializes without manual Blender edits.
- Rollback: remove failing recipe independently.

### VS-052 — Material-region assignment workflow

- Status: `PROPOSED`
- Depends on: VS-014, VS-021, VS-051
- Scope/files: semantic commands/API/UI and coverage validator.
- Objective: assign recipes to stable regions with source provenance.
- Risk: face ownership changing after deformation/export.
- Tests: coverage, stable membership and undo/redo.
- Acceptance: no face silently unassigned or multiply owned.
- Rollback: restore prior semantic revision.

### VS-053 — High-resolution livery and UV editor

- Status: `PROPOSED`
- Depends on: VS-023, VS-052
- Scope/files: UV SVG compiler, livery domain/API/UI and fixtures.
- Objective: ordered vector/raster layers at selectable resolution.
- Risk: destructive atlas replacement or unstable rasterization.
- Tests: order, clipping, transforms, hashes and deterministic raster output.
- Acceptance: original textures stay available and unmodified.
- Rollback: revert livery revision; delete generated bake.

### VS-054 — Deterministic PBR bake and preview parity

- Status: `PROPOSED`
- Depends on: VS-025, VS-051, VS-052, VS-053
- Scope/files: bake BuildIR, worker, reports and preview assets.
- Objective: bake Blender materials/livery to glTF PBR maps.
- Risk: hidden color/render settings or incomplete maps.
- Tests: fixed settings, map coverage/dimensions, relative paths, clean repeat.
- Acceptance: Blend remains authority; baked GLB reaches visual human gate.
- Rollback: delete baked candidate only.

## 8. Validation, revisions and hardening items

### VS-060 — Unified Vehicle Studio diagnostics

- Status: `PROPOSED`
- Depends on: VS-010, VS-030
- Scope/files: diagnostic domain and validators.
- Objective: stable codes for every authoring/build boundary.
- Risk: UI string matching or warnings treated as pass.
- Tests: severity precedence, owner context and deterministic order.
- Acceptance: build-blocking codes are explicit API data.
- Rollback: revert adapter while retaining checks.

### VS-061 — Immutable revision and project store

- Status: `PROPOSED`
- Depends on: VS-010, VS-020
- Scope/files: content-addressed store and recovery tests.
- Objective: separate Save from Build with additive history.
- Risk: partial writes, unsafe paths or mutable accepted revisions.
- Tests: atomic save, reopen, undo/redo, conflict and interruption.
- Acceptance: accepted revisions immutable and hash-addressable.
- Rollback: remove isolated test store; never delete user projects.

### VS-062 — Build isolation and provenance enforcement

- Status: `PROPOSED`
- Depends on: VS-031, VS-061
- Scope/files: build coordinator, staging policy and validator.
- Objective: block cache/output reuse across source/document/branch/tool hashes.
- Risk: foreign binary passing as current.
- Tests: deliberate source/document/Blender/library mismatch failures.
- Acceptance: mismatch fails before materialization/publication.
- Rollback: disable build entry; preserve revisions/sources.

### VS-063 — Clean rebuild parity suite

- Status: `PROPOSED`
- Depends on: VS-035, VS-054, VS-062
- Scope/files: temporary integration fixtures and reports.
- Objective: prove equivalent geometry, GLB and canonical reports from separate
  clean staging roots.
- Risk: comparing reused caches.
- Tests: two verified-empty roots and negative foreign-cache fixture.
- Acceptance: parity passes or classified differences block release.
- Rollback: remove temporary outputs only.

### VS-064 — Failure and rollback drills

- Status: `PROPOSED`
- Depends on: VS-061, VS-062
- Scope/files: integration tests/evidence only.
- Objective: recover from scan, compile, Blender, bake and export failures.
- Risk: partial candidate activation.
- Tests: injected failures at every publication boundary.
- Acceptance: active revision/artifacts never change after failure.
- Rollback: clean injected temporary candidates only.

## 9. Generic F1 proof and integration items

### VS-070 — Second-car onboarding proof

- Status: `PROPOSED`
- Depends on: VS-014, VS-044 accepted
- Scope/files: second mapping fixture/project; no Williams code branches.
- Objective: prove generic F1-open-wheel behavior.
- Risk: model-name conditionals masquerading as generality.
- Tests: source search and the same scan/mapping/document/view pipeline.
- Acceptance: second asset builds a global, tire and component edit.
- Rollback: remove second fixture/project.

### VS-071 — Optional-role and ambiguity hardening

- Status: `PROPOSED`
- Depends on: VS-070
- Scope/files: compatibility changes and fixtures.
- Objective: represent missing/split/combined components without invented data.
- Risk: weakening required runtime frames.
- Tests: optional-role matrix; axle/wheel frames stay mandatory.
- Acceptance: ambiguity blocks only dependent features.
- Rollback: revert compatibility delta.

### VS-072 — Era proportion preset system

- Status: `PROPOSED`
- Depends on: VS-032, VS-033, VS-034, VS-040, VS-041, VS-042, VS-043
- Scope/files: preset schema, UI and fixtures.
- Objective: proportion sets such as 1994/1997 style without historical claims.
- Risk: unsafe values or implied factual accuracy.
- Tests: schema/bounds, labels and representative combined builds.
- Acceptance: preset resolves to ordinary editable parameters.
- Rollback: remove preset; parameters remain.

### VS-073 — Generic F1 human gate and retrospective

- Status: `PROPOSED`
- Depends on: VS-063, VS-070, VS-071, VS-072
- Scope/files: evidence, decision and retrospective.
- Objective: accept or reject genericity beyond Williams.
- Risk: declaring generality from automation alone.
- Tests: two-car matrix and clean rebuild evidence.
- Acceptance: reaches `READY_FOR_HUMAN_GATE`; user decision recorded.
- Rollback: keep product labeled Williams-only.

### VS-080 — Godot runtime-package validator adapter

- Status: `PROPOSED`
- Depends on: VS-035, VS-060, VS-073 accepted
- Scope/files: adapter to existing validators; no publication.
- Objective: validate three staged GLBs against runtime contracts.
- Risk: coupling authoring acceptance to active runtime.
- Tests: headless temporary import/load and expected failures.
- Acceptance: passes without modifying active generated assets.
- Rollback: remove adapter/temp outputs.

### VS-081 — Explicit runtime publication gateway

- Status: `PROPOSED`
- Depends on: VS-062, VS-080
- Scope/files: separately approved gateway and atomic manifests.
- Objective: publish only accepted candidate with BUILD/HEAD/source parity.
- Risk: overwriting runtime or mixing branch binaries.
- Tests: preconditions, atomic replacement and rollback drill.
- Acceptance: separate explicit human authorization required.
- Rollback: atomically restore prior hashed package.

### VS-082 — User and developer documentation

- Status: `PROPOSED`
- Depends on: VS-073
- Scope/files: `docs/vehicle-studio/` and in-app help.
- Objective: document onboarding, dimensions, materials, builds and recovery.
- Risk: documenting unimplemented behavior.
- Tests: commands/examples against clean setup.
- Acceptance: limitations/human gates are accurate and prominent.
- Rollback: revert inaccurate docs only.

### VS-083 — Final acceptance and retrospective

- Status: `PROPOSED`
- Depends on: VS-063, VS-073, VS-080, VS-082; VS-081 only if publication is
  requested.
- Scope/files: final evidence, retrospective and status updates.
- Objective: close the epic with technical, visual and rollback evidence.
- Risk: equating implemented with accepted.
- Tests: full checklist and unresolved-diagnostic inventory.
- Acceptance: human decision recorded; rejected work returns to backlog.
- Rollback: keep epic open and last accepted variant active.

## 10. First implementation recommendation

After planning approval, execute one item at a time:

```text
VS-001 Williams baseline
  -> VS-002 manifest evidence
  -> VS-003 provenance recovery plan
  -> VS-004 VehicleDocument contract
  -> VS-005 SVG profile
  -> VS-006 Williams semantic mapping human gate
```

No application scaffold or Blender mutation should begin before this Phase 0
sequence is complete.
