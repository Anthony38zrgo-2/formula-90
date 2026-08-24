# Formula-90 Vehicle Studio Architecture Specification

Status: `PLANNING`

Initial reference asset: `blender/williams94_wheels_retextured/`

Target vehicle class: Formula 1 open-wheel cars.

## 1. Purpose

Vehicle Studio is a local authoring application for inspecting an existing F1
open-wheel asset, generating semantic CAD-style orthographic views, editing a
bounded set of vehicle proportions without changing mesh topology, authoring
professional Blender materials and liveries, and exporting an immutable
variant.

The first vertical slice uses the current Williams 1994 asset. The domain and
import contracts must not contain Williams-specific assumptions that prevent a
second F1 open-wheel car from being onboarded.

## 2. Product decisions

The following decisions are approved for v1:

- Inputs are `.glb` and `.blend` only.
- The editable authority is semantic JSON. SVG is a generated, restricted and
  round-trippable view of declared semantic controls; arbitrary SVG paths are
  not a 3D reconstruction source.
- Mesh topology changes are forbidden.
- Primary editable areas are nose, front and rear wings, engine cover and
  sidepods.
- Global dimensions include absolute values and percentage sliders, including
  wheelbase and vehicle width.
- Tire radius and width are editable independently by axle.
- Tire width supports three policies: centered growth, fixed inboard face and
  fixed outboard face.
- Changing tire radius preserves ground contact and recomputes axle/chassis
  height.
- Wheels remain centered on their own axle/wheel anchor frames.
- Era presets such as 1994 and 1997 express configurable proportions; they do
  not claim historical fidelity to a specific chassis.
- The authoritative variant includes `.blend`; runtime/review outputs include
  GLB and baked PBR textures.
- The UI is a local Vue application styled with Tailwind, with interactive
  before/after 3D and linked SVG views.
- Source assets are immutable. Save and Build are separate human actions.

## 3. Scope

### 3.1 In scope for v1

- Deterministic structural and geometric scanning of GLB and Blend inputs.
- Assisted semantic onboarding for F1 open-wheel components.
- Top, side, front and rear orthographic semantic SVG views.
- Optional section and UV views generated from the same semantic document.
- Persistent dimensions, datums, joints, regions and deformation controls.
- Constrained, topology-preserving geometry deformation.
- Absolute and percentage parameter editing.
- Interactive browser preview and authoritative Blender preview renders.
- Blender-native material recipes, high-resolution PBR maps and vector/raster
  livery layers.
- Immutable variants with provenance, validation and review artifacts.
- Export compatible with the repository vehicle import standard.

### 3.2 Non-goals for v1

- Adding, deleting or reconnecting vertices, edges, faces or mesh primitives.
- Arbitrary free-form reconstruction of 3D geometry from independent 2D paths.
- Automatic conversion of triangle meshes into B-Rep, NURBS or engineering CAD.
- Historical certification of an era preset.
- Physics, collision or aerodynamic simulation tuning.
- Editing non-open-wheel road cars, prototypes or closed-wheel race cars.
- OBJ or FBX import.
- Exact browser reproduction of arbitrary Blender shader node graphs.
- Publishing directly into active Godot runtime assets.

## 4. Existing repository boundaries

Vehicle Studio must extend rather than bypass these contracts:

- `docs/vehicles/vehicle-import-standard.md` owns axes, units, names, required
  datums/joints and runtime package shape.
- `docs/vehicles/vehicle-visual-asset-contract.md` owns the three-GLB runtime
  contract and the separation between physical wheel placement and visuals.
- `blender/analysis_suite/` remains a read-only evidence producer and is the
  preferred home for generic scan predicates and vehicle measurements.
- `blender/vehicle_pipeline/inspect_glb.py` remains the structural GLB
  inspector rather than being reimplemented in the UI.
- Blender is the only authority allowed to evaluate `.blend`, modifiers,
  Geometry Nodes and Blender material graphs.
- Godot is a consumer and validator, never the vehicle-authoring authority.

The current Track Studio editor is architectural precedent for sanitization,
normalization, undo/redo, immutable revisions and explicit build. Its
track-specific SVG coordinate contract must not be reused as a vehicle schema.

## 5. Authority and ownership

```text
source GLB/Blend        immutable input geometry and authoring evidence
VehicleDocument        editable semantic authority
VehicleView SVG        restricted projection/editing surface
VehicleBuildIR         generated compiler-facing deformation/material plan
Blender materializer   evaluated mesh, materials, baking and export
variant manifest       immutable provenance and artifact hashes
Godot package          validated runtime consumer output
```

| Concern | Owner | Explicitly not owned by |
|---|---|---|
| Semantic vehicle state | VehicleDocument | SVG DOM, Vue state, Blender scene |
| Orthographic presentation | VehicleView compiler | Hand-edited free-form SVG |
| UI selection and transient camera | Vue client | VehicleDocument |
| Deformation rules | Vehicle compiler/BuildIR | Blender operator history |
| Mesh evaluation | Blender materializer | Browser renderer |
| Professional material graph | Versioned Blender node library | GLB preview |
| Runtime PBR representation | Bake/export stage | Authoritative `.blend` |
| Runtime activation | Existing validated launch/build pipeline | Vehicle Studio |

## 6. End-to-end flow

```text
Import source
  -> hash and structural inspection
  -> evaluated geometry scan
  -> semantic classification suggestions
  -> human mapping gate
  -> VehicleDocument creation
  -> CAD/UV SVG compilation
  -> edit commands and validation
  -> explicit Build request
  -> VehicleBuildIR compilation
  -> Blender headless materialization
  -> structural, geometric, material and visual validation
  -> human review gate
  -> immutable variant activation inside the Vehicle Studio project
  -> optional, separately approved Godot integration
```

An import may report ambiguity but may not silently invent component ownership,
wheel axes, datums or deformation regions. A VehicleDocument becomes editable
only after the onboarding mapping is explicitly accepted.

## 7. VehicleDocument v1

VehicleDocument is deterministic, review-oriented JSON. It preserves author
intent and references the immutable source by hash.

```text
VehicleDocument
  schema_version
  project_id
  revision_id
  source
    kind: glb | blend
    path
    sha256
    blender_version: optional
    evaluated_geometry_sha256
  coordinate_system
    unit: meter
    right_axis: +X
    up_axis: +Y
    forward_axis: -Z
    ground_y
  component_map[]
    component_id
    semantic_role
    source_object_ids[]
    source_primitive_ids[]
    deform_region_id: optional
    material_region_ids[]
  frames[]
    frame_id
    semantic_role
    parent_frame_id: optional
    transform
    source_binding
  dimensions
    wheelbase
    front_track
    rear_track
    body_width
    overall_width
    length
    height
    front_overhang
    rear_overhang
    tire_front
    tire_rear
  stations[]
    station_id
    longitudinal_position
    semantic_role
    profile_controls
  deform_regions[]
    region_id
    semantic_role
    vertex_bindings
    protected_boundaries
    symmetry_policy
    falloff
  parameters[]
    parameter_id
    kind
    absolute_value
    baseline_value
    percentage
    unit
    bounds
    dependencies[]
  material_regions[]
  material_recipes[]
  livery_layers[]
  view_definitions[]
  validation_policy
  metadata
```

### 7.1 Persistent identity

- IDs are stable strings and independent of array order, display names or
  current projection coordinates.
- Source objects and primitives are addressed by stable scan identifiers that
  include their source path and geometry hash.
- Collections are serialized by documented semantic key or persistent ID.
- Floats use a fixed canonical representation; non-finite values are rejected.
- Canonical bytes contain no timestamps or machine-specific absolute paths.

### 7.2 Required semantic roles

The onboarding contract recognizes at least:

```text
vehicle_root
chassis
nose
front_wing
rear_wing
sidepod_left
sidepod_right
engine_cover
cockpit
floor
diffuser
suspension_fl/fr/rl/rr
wheel_front
wheel_rear
driver
```

Optional or absent roles are represented explicitly. Their absence is not an
error unless an enabled parameter or export policy requires them.

### 7.3 Required frames

The normalized vehicle must provide:

- vehicle origin;
- front and rear axle centers;
- four wheel anchors;
- front and rear wing mounts;
- wheel local frames and inboard/outboard face datums;
- ground-contact datums for front and rear tires.

Existing source datums are preserved. Missing datums may be suggested from
geometry, but require human confirmation before becoming authoritative.

## 8. VehicleView SVG profile

Each view is a deterministic projection of a VehicleDocument revision and its
evaluated preview geometry.

Allowed semantic groups include:

```text
outline
feature-line
hidden-line
section
datum
dimension
component-region
deformation-control
material-region
livery-layer
annotation
```

Editable SVG nodes must reference a declared parameter or livery object:

```xml
<circle
  data-role="deformation-control"
  data-parameter-id="sidepods.station-04.half-width"
  data-axis="X"
  cx="0.61"
  cy="0.42" />
```

Rules:

- SVG coordinates declare units and an explicit view transform.
- Every editable element references a persistent VehicleDocument ID.
- Unsupported elements, scripts, external URLs, CSS and event handlers are
  rejected by a dedicated sanitizer.
- Moving a control emits a semantic command; the SVG DOM is never persisted as
  independent geometry authority.
- Recompiling an unchanged VehicleDocument and geometry hash produces
  byte-identical canonical SVG.
- Conflicting simultaneous edits from different views are resolved by the
  parameter model, not by merging SVG paths.

## 9. Scan and onboarding architecture

### 9.1 GLB scan

The GLB path uses the stdlib inspector and analysis suite for structure,
transforms, attributes, bounds and hashes. Blender headless is then used only
when evaluated meshes, material graphs or projection renders are required.

### 9.2 Blend scan

Blend inputs are opened with the supported Blender executable using
`--background --factory-startup`. The scan reads evaluated objects, modifiers,
collections, materials, UV maps and custom properties without saving the
source file.

### 9.3 Semantic suggestions

Suggestions may use names, hierarchy, symmetry, location, bounds, wheel-like
radial geometry, material membership and proximity to known datums. Every
suggestion includes confidence and evidence. Low confidence is not converted
into semantic truth.

### 9.4 Human mapping gate

The onboarding UI must allow the user to:

- assign source objects/primitives to semantic components;
- confirm axes, scale and ground plane;
- confirm wheel centers, axles and mounting faces;
- mark protected and deformable regions;
- confirm symmetry pairs;
- preview required runtime component separation.

Only an accepted mapping creates the initial VehicleDocument revision.

## 10. Deformation architecture

Vehicle Studio compiles parameters into ordered, explicit deformation
operations. Blender consumes operations; it does not infer product semantics.

### 10.1 Topology invariant

For every topology-preserving build:

- object/primitive membership is unchanged unless the export packaging step
  creates declared copies;
- index count and triangle connectivity hash are unchanged;
- no vertex, edge or face is created or removed;
- UV indices and UV values remain unchanged during geometry deformation;
- material-region membership remains stable;
- normals and tangents may be deterministically recomputed.

### 10.2 Global proportions

Global scaling is expressed as region-aware transforms, not a single object
scale. Cockpit, axle frames, overhangs and deformable body intervals have
separate ownership. The compiler rejects an underconstrained wheelbase or width
operation rather than stretching unrelated geometry silently.

### 10.3 Wheelbase

Wheelbase changes move the complete front and/or rear axle frames along Z.
Wheel anchors, wheel geometry, suspension hub endpoints and axle-owned datums
follow their axle frame. Body regions between protected stations are warped by
an explicit piecewise mapping. Each wheel remains centered on its own anchor.

Suspension arm endpoints are re-evaluated from confirmed inner/outer frames.
If a required endpoint is missing or an arm cannot be adjusted without a
topology violation, the build is blocked with a diagnostic.

### 10.4 Tire radius and ground contact

For each axle, the compiled result must satisfy:

```text
wheel_center_y - effective_tire_radius == ground_y
```

Changing radius updates the axle/wheel-center height. The chassis ground
relationship is recomputed from the confirmed axle frames and ride-height/rake
policy. Different front and rear deltas may produce a declared rake change;
they may not leave either tire above or below the ground plane.

### 10.5 Tire width

Width is evaluated in the wheel local axial frame using one declared policy:

- `centered`: inboard and outboard faces move by half the width delta;
- `inboard_fixed`: mounting/inboard face is fixed and the outboard face moves;
- `outboard_fixed`: outboard face is fixed and the inboard face moves.

Wheel-center position and wheel-anchor transform are invariant for all three
policies. Clearance diagnostics cover suspension, chassis and paired wheels.

### 10.6 Body component deformers

Nose, sidepods and engine cover use station profiles and bounded control cages.
Wing elements use existing-component transforms and bounded profile controls.
Each deformer declares:

- owned vertex bindings;
- protected boundaries;
- symmetry behavior;
- falloff function;
- ordered dependencies;
- parameter bounds;
- postconditions and collision checks.

No deformer may select vertices at runtime using an undocumented proximity
threshold. Selection/weights are part of the accepted semantic document.

## 11. VehicleBuildIR v1

VehicleBuildIR is generated and compiler-facing. It is never edited manually.

```text
VehicleBuildIR
  build_ir_version
  source_sha256
  document_sha256
  compiler_version
  blender_version
  topology_contract
  ordered_frame_operations[]
  ordered_deformation_operations[]
  normal_tangent_policy
  material_assignments[]
  bake_operations[]
  export_operations[]
  expected_invariants
  artifact_plan
```

The same source hash, canonical VehicleDocument, compiler version, Blender
version and node-library hash must produce equivalent geometry and canonical
reports. Binary `.blend` byte identity is not assumed because Blender may
serialize operational metadata; evaluated geometry, material recipes and
exported GLB hashes are the reproducibility gates.

## 12. Materials and livery

### 12.1 Authoritative material representation

Professional materials are stored as typed recipes referencing versioned
Blender node groups. Initial families include:

- painted composite/clearcoat;
- exposed carbon fiber;
- metal;
- rubber/tire;
- plastic;
- glass/visor;
- emissive display.

Recipes expose bounded parameters and texture slots. Arbitrary node graphs may
remain in an imported Blend source but are not considered portable Vehicle
Studio recipes until explicitly wrapped or mapped.

### 12.2 Livery

Livery layers reference material regions and UV sets. Layers may be vector or
raster and declare transform, order, blend mode, opacity, clipping mask and
source hash. High-resolution authoring is supported independently from the
original 64x64 Williams textures.

### 12.3 Browser and runtime parity

The browser uses baked glTF-compatible PBR maps for interactive preview. An
authoritative Blender render uses fixed cameras, lighting, color management and
render settings. Browser similarity is a usability feature; Blender renders
and exported artifact validation own acceptance.

## 13. Application architecture

```text
Vue + Tailwind client
  linked CAD/UV SVG panels
  before/after Three.js preview
  semantic inspector and parameters
  diagnostics, revisions and build controls
             |
             v
local Python service
  project/revision store
  scan orchestration
  command validation
  SVG and BuildIR compiler
  build queue and structured progress
             |
             v
Blender headless worker
  evaluated scan
  deformation
  materials and bake
  `.blend` save and GLB export
```

The client may own selection, camera and uncommitted form state. It may not own
canonical persistence, validation rules, Blender process lifetime or artifact
publication.

Builds run in isolated staging directories. The local service validates all
paths against the project root and never passes arbitrary client-provided
Python or Blender expressions to a subprocess.

## 14. Project and variant layout

Proposed layout, subject to the implementation backlog:

```text
tools/vehicle_studio/                 application and compilers
blender/vehicle_studio/               Blender-only adapters/materializer
blender/vehicle_studio/materials/     versioned node-library sources
docs/vehicle-studio/                  architecture and planning
vehicle_projects/<project_id>/        ignored working projects
  project.json
  revisions/<revision_sha>/
  builds/<build_sha>/
game/assets/generated/...             never written without separate approval
```

Canonical source/revision data and generated binaries must not be mixed. The
exact tracked/ignored policy is decided before implementation of persistence.

## 15. Variant artifacts

An accepted build contains:

```text
variant.manifest.json
vehicle.document.json
vehicle.build.json
views/
  top.svg
  side.svg
  front.svg
  rear.svg
  uv-*.svg
authoring/
  vehicle.variant.blend
runtime/
  vehicle_chassis.glb
  vehicle_wheel_front.glb
  vehicle_wheel_rear.glb
textures/
reports/
  scan.json
  geometry-validation.json
  material-validation.json
  export-validation.json
  visual-review.json
```

The manifest includes source, document, BuildIR, tool, Blender, node-library
and artifact hashes. Generated reports contain no machine-specific absolute
paths.

## 16. Validation and diagnostics

Every boundary returns structured diagnostics with stable code, severity,
owner ID, view/component/parameter context and deterministic metadata.

Required gates include:

- input hash and supported-version preconditions;
- finite evaluated geometry;
- topology/connectivity parity;
- normals, tangents, winding and degenerate faces;
- UV validity and material-region coverage;
- symmetry where enabled;
- wheel/anchor/frame invariants;
- ground contact for all four tires;
- suspension and body clearance;
- requested versus measured absolute dimensions;
- source and output GLB structural audit;
- canonical JSON/SVG/BuildIR determinism;
- `.blend` reopen validation;
- Godot headless import only during separately approved runtime integration;
- image-capable review plus explicit human acceptance for visual quality.

No absence of errors is treated as visual acceptance.

## 17. Save, Build and human gates

- Save creates a canonical semantic revision and never invokes Blender.
- Preview Build creates disposable staged artifacts.
- Candidate Build creates an immutable variant candidate after automated gates.
- Human review accepts or rejects the candidate using linked before/after CAD,
  3D and Blender-render evidence.
- Acceptance activates the candidate within the Vehicle Studio project only.
- Runtime publication is a separate backlog item and human gate.

## 18. Rebuild and provenance safety

- Record branch, HEAD and status before every implementation item.
- Never reuse Blender caches, `.blend` candidates, GLB outputs or ignored
  staging from another branch/source hash.
- Staging paths include source, document and build hashes.
- A build refuses a source hash mismatch.
- Active artifacts are never overwritten in place; publication is atomic.
- Commits stage explicit paths and remain atomic per backlog item.
- Launch/integration scripts must preserve BUILD/HEAD parity required by the
  repository protocol.

## 19. Architecture acceptance criteria

Architecture implementation is considered proven only when the vertical slice:

1. Scans the Williams source without modifying it.
2. Requires and records an accepted semantic mapping.
3. Generates deterministic VehicleDocument and four canonical SVG views.
4. Edits wheelbase, global width, tire radius and all tire-width policies.
5. Preserves topology and wheel-to-anchor relationships.
6. Preserves ground contact and reports resulting chassis height/rake.
7. Displays linked before/after browser previews.
8. Creates a staged `.blend` and the three required runtime GLBs.
9. Reopens/audits outputs and produces deterministic validation reports.
10. Passes an explicit human visual gate before any runtime integration.



