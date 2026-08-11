# Formula90s — Current Project State

> Canonical current-state handoff for humans and AI agents.
>
> This file is intentionally different from `game/docs/PROJECT_DIRECTION.md` and
> `game/docs/COMMON_ERRORS_AND_FIXES.md`.
>
> - `PROJECT_DIRECTION.md` defines where the project is going.
> - `COMMON_ERRORS_AND_FIXES.md` records reusable failure patterns and fixes.
> - `PROJECT_STATE.md` defines where the project is **right now**.
>
> Any agent that is about to modify vehicle physics, telemetry, track generation,
> environment generation, or generated content should read this file first.

---

## 0. Status notation

Every important statement should use one of these labels when ambiguity is possible.

| Label | Meaning |
|---|---|
| `[FROZEN]` | Known-good reference that must not be changed unless the task explicitly reopens it. |
| `[VALIDATED]` | Tested successfully and accepted as the current working state. |
| `[ACTIVE]` | Current implementation or current focus. |
| `[PLANNED]` | Approved next work, but not yet implemented or validated. |
| `[EXPERIMENTAL]` | Temporary or exploratory work. |
| `[DEPRECATED]` | Historical implementation that must not be used as current authority. |
| `[UNKNOWN]` | State is intentionally not assumed; inspect runtime/repository before acting. |

Do not silently convert `[PLANNED]` values into `[VALIDATED]` values.

Do not silently reopen `[FROZEN]` work.

---

# 1. Repository identity

- Project: **Formula90s**
- Repository: `Anthony38zrgo-2/formula-90`
- Active development branch: `refactor/gevp-clean-baseline`
- Current project goal: compact 1990s-inspired formula racing game with convincing,
  readable, mechanically expressive handling and a late-1990s console visual identity.
- Current development car: **Jordan 1995**
- Current handling-development circuit: **La Chutana**

### Current development order

```text
Phase A — physical geometry/layout                COMPLETE
Phase B — mechanical grip/brakes/differential    COMPLETE
Track validation / environment gate              COMPLETE ENOUGH FOR HANDLING WORK
Telemetry setup snapshot                         NEXT REQUIRED INFRASTRUCTURE
Phase C — steering / countersteer                 NEXT PHYSICS PHASE
Phase D — suspension
Phase E — V10 powertrain / transmission
Phase F — aerodynamics
Phase G — assists / true no-assists behavior
```

---

# 2. Mandatory reading order for agents

Before modifying the project:

1. `PROJECT_STATE.md`
2. `game/docs/PROJECT_DIRECTION.md`
3. `game/docs/COMMON_ERRORS_AND_FIXES.md`
4. Relevant local `AGENTS.md`
5. Relevant subsystem source files
6. Recent commits touching the same subsystem

Do not work from chat memory alone if the repository is available.

---

# 3. Source-of-truth hierarchy

When values disagree, use the following authority order.

## 3.1 Runtime physics

```text
runtime node/resource values
    >
current vehicle scene/resource
    >
project-owned Formula90s controller/config
    >
documentation
    >
chat history
```

A value observed in runtime is more authoritative than a manually written note.

## 3.2 Telemetry setup

```text
<telemetry_session>_setup.json
    >
runtime scene/resource values
    >
PROJECT_STATE.md
```

The setup snapshot is an immutable record of the configuration that produced that
specific telemetry capture.

## 3.3 Track geometry

```text
deterministic reference/config
    >
track pipeline output
    >
generated Blender source
    >
canonical generated GLB
    >
Godot wrapper/import
```

The runtime GLB is not the design source.

## 3.4 Generated textures

```text
source/generated base
    >
Texture Forge recipe/config
    >
manifest
    >
generated PNG output
```

Generated PNGs are outputs, not hand-edited source authority.

## 3.5 Vendor code

GEVP vendor code is upstream-owned.

Formula90s-specific behavior should normally be implemented through:

- configuration,
- subclassing,
- adapters,
- scene composition,
- project-owned controller code,
- project-owned surface logic.

---

# 4. Frozen GEVP baseline

## 4.1 Known-good baseline

`[FROZEN]`

Known-good baseline commit:

```text
feab26c3df1e7eaabb7674b2e40cfd12fa299b8e
```

Known-good baseline vehicle scene:

```text
game/scenes/vehicles/baseline_2026/baseline_2026.tscn
```

Known-good baseline test scene:

```text
game/scenes/tracks/test_field/gevp_baseline.tscn
```

The frozen baseline exists to answer:

> Does the underlying GEVP vehicle still work independently from Formula90s tuning?

It is not the place to implement Jordan-specific behavior.

## 4.2 Vendor controller

`[FROZEN]`

Vendor file:

```text
game/addons/gevp/scripts/vehicle_controllergd.gd
```

Known upstream commit:

```text
172cfc8536f02f2568d7e8f530a83e73ffbd7796
```

Do not modify this vendor file to solve Formula90s-specific tuning problems.

## 4.3 Other known vendor-related facts

- Physics tick target: **120 Hz**
- `vehicle.gd` is not treated as a pristine upstream copy because Formula90s retains
  project-specific engine configuration behavior there.
- `wheel.gd` contains safe surface fallbacks and debug behavior used during earlier
  wheel/contact debugging.

Changing these areas requires explicit justification.

---

# 5. Jordan 1995 — current physical state

# 5.1 Phase A — geometry and physical layout

Status: `[VALIDATED]`

Canonical scene:

```text
game/scenes/vehicles/jordan_1995/jordan_1995.tscn
```

## Chassis

```text
mass_kg               = 505
front_weight_ratio    = 0.45
cg_vertical_offset_m  = -0.20
inertia_multiplier    = 1.10
wheelbase_m           = 2.930
```

## Tire dimensions

```text
tire_radius_m         = 0.3473

front_tire_width_mm   = 335
rear_tire_width_mm    = 420

front_wheel_mass_kg   = 12
rear_wheel_mass_kg    = 16
```

## Physics wheel positions

Front left:

```text
(-0.739368, 0.0575, -1.4394)
```

Front right:

```text
(+0.739368, 0.0575, -1.4394)
```

Rear left:

```text
(-0.718479, 0.0525, 1.4906)
```

Rear right:

```text
(+0.718479, 0.0525, 1.4906)
```

These RayCast positions are physical authority.

Visual wheel hierarchy must follow physics, not the reverse.

## Collision

Current tub/nose/rear collision design from the validated Phase A state should not
be retuned during steering work.

---

# 5.2 Wheel visual architecture

Status: `[VALIDATED]`

A Formula90s car should normally use only:

```text
chassis.glb
wheel_front.glb
wheel_rear.glb
```

Physics still has four distinct wheel RayCasts.

The same front visual wheel is reused on both sides.

The same rear visual wheel is reused on both sides.

For the opposite side:

```text
Orientation node
→ 180 degree rotation
```

Do not solve wheel orientation with negative scale.

Do not create four permanent duplicated wheel meshes only for left/right orientation.

---

# 5.3 Phase B — mechanical grip, braking, differential

Status: `[VALIDATED]`

Canonical scene:

```text
game/scenes/vehicles/jordan_1995/jordan_1995_phase_b.tscn
```

Phase B inherits the Phase A physical layout.

## Brakes

```text
front_brake_bias = 0.57
```

## Differential

```text
rear_locking_diff_torque = 170
```

## Tire/contact behavior

```text
contact_patch               = 0.21
braking_grip_multiplier     = 1.08
```

## Surface stiffness

```text
Road     = 8.75
Curb     = 7.00
Gravel   = 0.50
Grass    = 0.50
```

## Surface friction

```text
Road     = 2.65
Curb     = 2.20
Gravel   = 1.10
Grass    = 0.90
```

## Rolling resistance / rolling modifier

```text
Road     = 1.0
Curb     = 1.5
Gravel   = 2.0
Grass    = 4.0
```

## Lateral assist

```text
Road     = 0.02
other surfaces = 0
```

## Longitudinal ratio

```text
Road     = 0.48
other surfaces = 0.45
```

### Phase B rule

`[FROZEN FOR PHASE C]`

Do not change these values during Phase C unless telemetry proves that a supposed
steering problem is actually a mechanical-grip defect and the phase boundary is
explicitly reopened.

---

# 6. Current test controls

Status: `[ACTIVE]`

Known Jordan test controls:

```text
1        automatic transmission
2        stability aid
3        steering aid
4        braking aid
5        grip aid

A        upshift
Z        downshift
C        clutch
Space    handbrake
R        reset

Arrow keys
         throttle / brake / steering
```

Current intended initial aid state:

```text
AUTO = ON

stability = OFF
steering  = OFF
braking   = OFF
grip      = OFF
```

Agents must inspect runtime before assuming that input or aid bindings have not changed.

---

# 7. Handling objective

Status: `[ACTIVE DIRECTION]`

The target is not a hardcore modern simulator and not a generic arcade vehicle.

Desired behavior:

- progressive steering;
- no immediate full lateral adhesion from a keyboard tap;
- understandable weight transfer;
- recoverable oversteer when corrected in time;
- late/excessive correction can still produce a spin;
- lifting throttle can help recover grip;
- braking behavior should remain readable;
- low rear stability should create intuitive oversteer;
- strong rear stability / forward brake bias should be capable of producing understeer;
- off-track surfaces should degrade behavior without becoming invisible walls;
- aerodynamics will later modify high-speed behavior but must not hide poor mechanical handling.

Historical inspiration:

```text
Monaco Grand Prix: Racing Simulation 2 / late-1990s formula handling feel
```

This is an inspiration target, not a requirement to duplicate proprietary physics.

---

# 8. Phase C — steering and countersteer

Status: `[PLANNED — NEXT PHYSICS PHASE]`

Only the steering/countersteer family should be modified.

## 8.1 Candidate starting values

These are **starting candidates**, not validated values.

```text
steering_speed              ≈ 3.7
countersteer_speed          ≈ 9.0
steering_decay              ≈ 0.26
slip_assist                 ≈ 0.11
countersteer_assist         ≈ 0.70
steering_exponent           ≈ 1.70
max_steering_angle_deg      ≈ 25
```

Do not record these later as final values unless telemetry and human testing accept them.

## 8.2 Allowed changes during Phase C

- steering input shaping;
- steering build-up rate;
- steering unwind/decay;
- countersteer response;
- steering exponent/nonlinearity;
- steering assistance directly related to countersteer;
- maximum steering angle if required by the steering model;
- instrumentation needed to measure steering behavior.

## 8.3 Forbidden simultaneous tuning

Do not retune these families during Phase C:

```text
tire friction
contact patch
brake bias
differential baseline
springs
dampers
anti-roll behavior
engine torque
engine braking
gear ratios
aerodynamics
track collision
surface classification
```

If one of these must change, stop and explicitly reclassify the task.

---

# 9. Phase D — suspension

Status: `[PLANNED]`

Target family:

- springs;
- damping;
- anti-roll behavior if used;
- transient weight transfer;
- braking pitch;
- direction-change behavior;
- curb/bump response.

Do not increase tire grip to conceal poor suspension behavior.

---

# 10. Phase E — V10 powertrain and transmission

Status: `[PLANNED]`

Target family:

- intended 1990s V10 torque delivery;
- RPM range;
- engine braking;
- throttle response;
- gear ratios;
- differential interaction.

Powertrain tuning must not begin until mechanical handling and steering are understood.

---

# 11. Phase F — aerodynamics

Status: `[PLANNED]`

Target family:

- front downforce;
- rear downforce;
- aero balance;
- drag;
- speed-dependent behavior.

Mechanical handling must already work before aero is used to shape balance.

---

# 12. Phase G — assists and true no-assists behavior

Status: `[PLANNED]`

The project must clearly separate:

```text
natural vehicle physics
GEVP baseline stabilization
Formula90s optional aids
```

Eventually `OFF` must have a precise behavioral meaning.

---

# 13. Telemetry contract

Status: `[ACTIVE — MUST BE EXTENDED BEFORE PHASE C]`

Telemetry is the primary evidence source for handling decisions.

Human comments such as:

```text
"the car feels too nervous"
"countersteer feels late"
"rear grip disappears too quickly"
```

are valid observations, but physics changes should be correlated with telemetry where possible.

---

# 14. Telemetry file pairing

Status: `[PLANNED — REQUIRED BEFORE PHASE C]`

Every telemetry capture must produce exactly one immutable setup snapshot.

Example:

```text
telemetry_20260809_112500_001.csv
telemetry_20260809_112500_001_setup.json
```

Naming rule:

```text
<telemetry-basename>.csv
<telemetry-basename>_setup.json
```

The two files are inseparable diagnostic artifacts.

---

# 15. Setup snapshot rule

The setup JSON must be generated automatically from **actual runtime values** when
the telemetry session starts.

Do not:

- manually type the JSON after testing;
- reconstruct the setup from memory;
- infer values from documentation;
- copy values from another run.

The setup file must represent what the running vehicle actually used.

---

# 16. Proposed setup JSON schema

Status: `[PLANNED]`

Recommended structure:

```json
{
  "schema_version": 1,

  "session": {
    "telemetry_file": "telemetry_20260809_112500_001.csv",
    "timestamp": "2026-08-09T11:25:00-05:00",
    "development_phase": "phase_c",
    "physics_hz": 120
  },

  "provenance": {
    "git_commit": "<runtime commit sha>",
    "git_branch": "refactor/gevp-clean-baseline",
    "vehicle_scene": "res://...",
    "track_scene": "res://..."
  },

  "vehicle": {
    "id": "jordan_1995",
    "configuration": "phase_c"
  },

  "chassis": {},
  "tires": {},
  "steering": {},
  "brakes": {},
  "differential": {},
  "suspension": {},
  "engine": {},
  "transmission": {},
  "aerodynamics": {},
  "assists": {},
  "surfaces": {}
}
```

Empty sections are acceptable if the subsystem is not yet explicitly configured.

Invented values are not acceptable.

---

# 17. Minimum setup JSON contents

The logger should capture, when available:

## Session

- telemetry filename;
- timestamp;
- physics tick rate;
- development phase;
- test identifier if one exists.

## Provenance

- Git commit SHA;
- Git branch;
- vehicle scene/resource path;
- track scene/resource path;
- relevant configuration/resource paths.

## Chassis

- mass;
- center of gravity;
- weight distribution;
- inertia-related parameters;
- wheelbase/track values if configurable.

## Tires

- radius;
- width;
- contact patch;
- grip multipliers;
- tire-related assistance values.

## Steering

- steering speed;
- countersteer speed;
- steering decay;
- steering exponent;
- maximum angle;
- slip assist;
- countersteer assist;
- any speed-sensitive steering parameter.

## Brakes

- front/rear bias;
- braking multipliers;
- lock-related parameters.

## Differential

- locking torque;
- current differential mode/configuration.

## Suspension

- spring values;
- damping;
- travel;
- anti-roll settings if present.

## Engine

- torque configuration;
- RPM limits;
- engine braking;
- throttle response parameters.

## Transmission

- gear ratios;
- final drive;
- automatic/manual state;
- shift parameters.

## Aerodynamics

- front aero;
- rear aero;
- drag;
- balance-related values.

## Assists

- automatic transmission;
- stability;
- steering aid;
- braking aid;
- grip aid;
- any hidden/default stabilization relevant to interpretation.

## Surfaces

For each surface:

- stiffness;
- friction;
- rolling resistance;
- lateral assist;
- longitudinal ratio;
- any other runtime coefficient affecting tire behavior.

---

# 18. Mid-session setup changes

Status: `[PLANNED RULE]`

A telemetry session should represent one physical setup.

If a relevant physical parameter changes during a test:

```text
close current CSV
close/finalize current setup snapshot
start a new telemetry session
write a new setup snapshot
```

Example:

```text
telemetry_..._001.csv
telemetry_..._001_setup.json

# steering parameter changed

telemetry_..._002.csv
telemetry_..._002_setup.json
```

This prevents a CSV from containing physically incompatible segments.

Aid toggles should also trigger a new session when the aid affects handling analysis.

---

# 19. Telemetry comparison philosophy

The purpose of setup snapshots is to enable comparisons such as:

```text
Run A
steering_speed = 3.7
countersteer_speed = 9.0
exponent = 1.70

vs

Run B
steering_speed = 3.4
countersteer_speed = 8.0
exponent = 1.55
```

and relate those differences to measured behavior:

```text
lateral G
front slip
rear slip
steering input
steering output
yaw response
vehicle speed
suspension compression
spin/recovery behavior
```

Never compare two telemetry CSV files without first checking the paired setup JSON.

---

# 20. La Chutana — current circuit role

Status: `[VALIDATED ENOUGH FOR PHASE C]`

La Chutana is the current handling-development circuit.

Public/reconstruction anchors used by the project include approximately:

```text
length             ≈ 2.420 km
main straight      ≈ 800 m
turn count         ≈ 7
location           = Lima / San Bartolo area, Peru
```

The implementation is a gameplay-oriented reconstruction, not survey-grade CAD.

---

# 21. La Chutana generated runtime architecture

Status: `[ACTIVE]`

Godot wrapper:

```text
game/scenes/tracks/test_field/la_chutana_generated.tscn
```

Canonical generated runtime asset:

```text
res://assets/generated/tracks/la_chutana/la_chutana.glb
```

The canonical GLB is generated locally and should be treated as a build output.

The old procedural Godot prototype remains historical:

```text
game/scenes/tracks/test_field/la_chutana_track.gd
game/scenes/tracks/test_field/la_chutana_track.tscn
```

Status of old prototype: `[DEPRECATED]`

Do not make it current authority again without an explicit architectural decision.

---

# 22. Track surface groups

Status: `[VALIDATED]`

Expected surface categories:

```text
Road
Curb
Grass
Wall
```

Project-owned group restoration/tagging:

```text
game/addons/formula90s/scripts/generated_track_surface_groups.gd
```

If driving behavior changes unexpectedly after track regeneration, inspect group
classification before changing vehicle grip.

---

# 23. La Chutana terrain/collision contract

Status: `[VALIDATED]`

The current generator uses:

- continuous terrain heightfield;
- upward-facing winding validation after coordinate conversion;
- collision underlay beneath road;
- local collision bridges where required;
- large invisible safety floor as catastrophic-fall failsafe;
- separate visual/collision treatment;
- visual terrain sink where needed to avoid road clipping.

Important coordinate convention:

```text
Godot/data:
(x, z, height)

Blender:
(x, -z, height)
```

The sign inversion affects triangle winding.

Do not reintroduce broad independently offset terrain ribbons around the track.

---

# 24. Current curb contract

Status: `[VALIDATED]`

Current La Chutana curb direction:

```text
width_m        ≈ 0.58
maximum rise   ≈ 0.022 m
```

Representative profile:

```text
(0.00, 0.000)
(0.12, 0.005)
(0.29, 0.022)
(0.46, 0.012)
(0.58, 0.002)
```

Curbs must remain low and gradual enough that a low Formula chassis can touch them
without treating them as ramps.

Do not soften vehicle suspension to hide malformed curb geometry.

---

# 25. Deterministic Blender track pipeline

Status: `[ACTIVE]`

Main directory:

```text
blender/track_pipeline/
```

Important files:

```text
configs/la_chutana.json
data/la_chutana_reference.json

pipeline_common.py
prepare_track.py
validate_track.py

terrain_grid.py

generate_procedural_textures.py
texture_forge.py
validate_texture_forge.py

procedural_catalog.py
procedural_materials_blender.py
procedural_assets_blender.py

generate_environment.py
validate_environment.py

build_track_blender.py
build_environment_blender.py
blender_output.py

requirements.txt
README.md
```

Runner:

```text
scripts/run_track_pipeline.ps1
```

Setup script:

```text
scripts/setup_track_pipeline.ps1
```

Agent skill:

```text
.agents/skills/track-reconstruction/SKILL.md
```

---

# 26. Base / Procedural generation gate

Status: `[ACTIVE]`

Base stage:

```powershell
.\scripts\run_track_pipeline.ps1 `
  -Mode Base `
  -Track la_chutana `
  -Seed 1995
```

Procedural stage:

```powershell
.\scripts\run_track_pipeline.ps1 `
  -Mode Procedural `
  -Track la_chutana `
  -TreesDensity medium `
  -BushesDensity medium `
  -GrassDensity medium `
  -BuildingsDensity medium `
  -Seed 1995
```

Allowed density labels:

```text
none
very_low
low
medium
high
```

A procedural run must begin from clean Base output.

It must not decorate the previous environment output.

---

# 27. Generated Blender/output contract

Base source:

```text
blender/generated/la_chutana/track_base.blend
```

Base GLB output:

```text
game/assets/generated/tracks/la_chutana/la_chutana_base.glb
```

Environment source:

```text
blender/generated/la_chutana/track_environment.blend
```

Environment GLB:

```text
game/assets/generated/tracks/la_chutana/la_chutana_environment.glb
```

Canonical published runtime:

```text
game/assets/generated/tracks/la_chutana/la_chutana.glb
```

Publishing must be atomic:

```text
export temporary .new.glb
→ verify export
→ replace canonical GLB
```

Never delete the last known-good canonical GLB before a replacement has succeeded.

---

# 28. Current La Chutana configuration highlights

Status: `[ACTIVE]`

Important current configuration concepts:

## Road

```text
width_m                = 12.0
thickness_m            = 0.16
edge_line_width_m      = 0.12
surface_elevation_m    = 0.025
```

## Terrain

```text
grid_cell_m                    = 6.0
texture_world_size_m           = 96.0
shoulder_falloff_m             = 18.0

collision_underlay_drop_m      = 0.12
collision_underlay_blend_m     = 1.0

visual_under_road_drop_m       = 0.22
visual_under_road_blend_m      = 1.4
visual_edge_blend_m            = 1.25
visual_sink_m                  = 0.003

roadside_visual_width_m        = 1.4
far_ground_z_m                 = -0.1
far_ground_safety_m            = 0.05
far_ground_margin_m            = 220.0

safety_floor_z_m               = -6.0
safety_floor_thickness_m       = 0.6
safety_floor_margin_m          = 80.0

roadside_collision_width_m     = 2.0
```

## Start/finish

```text
line_width_m       = 2.2
spawn_before_m     = 18.0
```

Agents must inspect `configs/la_chutana.json` before relying on this snapshot if the
configuration has changed since this file was updated.

---

# 29. Procedural environment taxonomy

Status: `[ACTIVE]`

Biome model:

```text
continent
+
longitudinal band
    west | center | east
+
altitude band
    low | medium | high
```

Current implemented continent family:

```text
South America
```

Current La Chutana biome:

```text
south_america / west / low
```

Artistic meaning:

- Peruvian coastal/semi-arid direction;
- dry-biased but not monochrome;
- green, ochre, brown and exposed soil can coexist;
- location variation is artistic rather than strict ecological simulation.

---

# 30. Procedural environment placement rules

Status: `[ACTIVE]`

Representative La Chutana placement zones:

```text
grass:
    min edge clearance ≈ 1.4 m
    max track distance ≈ 30 m

bushes:
    min edge clearance ≈ 3.0 m
    max track distance ≈ 44 m

trees:
    min edge clearance ≈ 8.0 m
    max track distance ≈ 100 m

fake buildings:
    min edge clearance ≈ 65 m
    max track distance ≈ 240 m
```

Placement must account for the asset footprint, not only centerline distance.

Vegetation distribution should use deterministic clustering rather than uniform scatter.

---

# 31. Vegetation geometry contract

Status: `[ACTIVE]`

Trees:

```text
3 crossed textured planes
6 visible directional faces
no gameplay collision
```

Bushes:

```text
2 crossed textured planes
4 visible directional faces
no gameplay collision
```

Grass:

```text
1 double-sided textured card
no gameplay collision
```

Structures:

```text
simple low-poly 3D shells
basic roof/top
medium/far scenery
```

Visual richness should come primarily from texture, silhouette, clustering and composition.

---

# 31.1 Vegetation v2 — La Chutana tree set + 2x visual scale

Status: `[VALIDATED]`

The La Chutana vegetation v2 tree set integrates the four uploaded
`new_tree*.png` sources (see `blender/vegetation_v2_upload_bundle/source_manifest.json`):

```text
tree_v2_a -> tree_v2_01   new_tree.png  1152x2048  tall warm broad-canopy
tree_v2_b -> tree_v2_02   new_tree2.png 1600x1600  wide flowering/willow-like
tree_v2_c -> tree_v2_03   new_tree3.png 1184x2096  tall cool drooping
tree_v2_d -> tree_v2_04   new_tree4.png 1184x2096  tall warm broad-canopy
```

All four trees: `planes: 3`, `collision: false`, transparent RGBA, bottom
anchored, 256x256 prepared cards.

`tree_visual_scale: 2.0` lives in
`blender/track_pipeline/layouts/la_chutana/layout_config.json` and is applied
in `semantic_layout_common.py` **only** when `category == "trees"`. It scales
the final semantic target height and footprint radius (and therefore barrier
clearance and tree-to-tree occupancy) in the compiler contract — the GLB
geometry is unchanged, so doubling geometry alone would be cancelled by
`scale = target_height_m / asset_height_m`.

Validated result:

```text
trees:      130  target height 12.22-22.9 m (was 6.0-11.5)  footprint 3.74-12.16 m
bushes:     110  unchanged
grass:      932  unchanged
tree-to-tree overlaps: 0
negative barrier margin: 0
deterministic compile:  yes (hash-compared twice)
```

Runtime published to `game/assets/generated/tracks/la_chutana/la_chutana.glb`
(source/runtime SHA-256 match verified).

Offline 3xBRZ upscale: **blocked at the license gate** (xBRZ reference by Zenju
is GPLv3; repository is MIT). No compatible MIT implementation or approved
clean-room route exists in this increment; see `THIRD_PARTY.md`. No 3xBRZ code
was written. A compatible implementation or an authorized non-derivative
derivation is a prerequisite for that feature.

---

# 32. Current art direction

Status: `[ACTIVE DIRECTION]`

Target visual family:

```text
late-1990s PS1 rally/racing
photo-derived / prerendered texture character
strong silhouettes
low geometry density
rich but controlled texture detail
fake/baked-looking shadow information
macro terrain variation
retro readability
```

Current stronger direction:

- more autumnal;
- richer texture;
- tree canopies with greater perceived mass;
- balanced palette leaning dry;
- photo-derived rather than illustrative;
- clear regional variation while keeping one coherent visual family.

La Chutana must not become a lush wet forest.

For La Chutana, interpret the autumnal/rich style through:

```text
dusty olive
straw
ochre
muted green
burnt umber
dry soil
weathered surfaces
```

---

# 33. Texture Forge

Status: `[ACTIVE]`

Main implementation:

```text
blender/track_pipeline/texture_forge.py
```

Validation:

```text
blender/track_pipeline/validate_texture_forge.py
```

Texture generation:

```text
blender/track_pipeline/generate_procedural_textures.py
```

Current style identifier:

```text
ps1_rally_clean
```

Current known forge version:

```text
1
```

Current known deterministic seed:

```text
1995
```

Texture Forge is the final deterministic art compiler.

Intended flow:

```text
generated / AI / source base
        ↓
Texture Forge
        ↓
alpha cleanup
silhouette cleanup
palette control
fake lighting
fake AO
lower/contact shadow
posterization
subtle ordered dithering
manifest + hashes
        ↓
runtime texture
```

Do not hand-edit generated runtime PNGs as a permanent source-of-truth workflow.

---

# 34. Current generated texture bank

Status: `[ACTIVE]`

Generated texture root:

```text
blender/generated/la_chutana/textures/
```

Active biome:

```text
blender/generated/la_chutana/textures/biomes/south_america/west/low/
```

Active manifest:

```text
blender/generated/la_chutana/textures/active_manifest.json
```

Current active biome contains:

```text
terrain
shoulder
bark

4 tree variants
4 bush variants
4 grass variants
4 building/facade variants
```

Shared textures include:

```text
asphalt
guardrail
start_finish
```

The broader bank contains the nine South America combinations:

```text
west   / low
west   / medium
west   / high

center / low
center / medium
center / high

east   / low
east   / medium
east   / high
```

---

# 35. Current texture refinement strategy

Status: `[ACTIVE / RECENT]`

The approved refinement strategy is:

```text
First refine only:
south_america / west / low

Include:
trees
bushes
grass
buildings
terrain
shoulder
bark
```

Three-pass supervision strategy:

```text
Round 1
conservative refinement
preserve strengths
increase photo-derived richness

Round 2
stronger artistic push
more PS1 rally character
more autumnal/prerendered presence

Round 3
directed correction
analyze weaknesses in R1/R2
correct without over-stylizing
```

Selection must happen **per category**, not per complete batch.

Example:

```text
trees      ← best round for trees
bushes     ← best round for bushes
grass      ← best round for grass
buildings  ← best round for buildings
terrain    ← best round for terrain
```

Once the active biome style is accepted, it can be propagated to the other biome combinations.

---

# 36. Visual scale notes

Status: `[ACTIVE]`

Trees:

```text
current scene scale accepted
do not globally resize
```

Perceived improvement should come from:

- fuller silhouette;
- internal texture contrast;
- fake light/shadow;
- richer canopy texture.

Bushes:

Historical art direction requested a narrower horizontal presentation.

If geometry-scale changes are revisited, inspect current repository values before acting;
do not assume an older percentage is still current without checking.

---

# 37. Fog / atmosphere

Status: `[UNKNOWN — INSPECT CURRENT RUNTIME]`

A previous visual goal was to reduce excessive scene haze/fog.

Do not assume the current fog implementation/value without inspecting:

- `WorldEnvironment`,
- `Environment` resources,
- track/test scene environment configuration,
- generated scene overrides.

Do not change handling parameters to compensate for visibility problems.

---

# 38. OpenTopography / future terrain direction

Status: `[PLANNED / ARCHITECTURAL OPTION]`

OpenTopography or equivalent DEM data may later be used for two separate problems.

## 38.1 Real circuit elevation

For circuits where elevation is part of circuit identity:

```text
DEM
→ sample centerline elevation
→ filter/smooth macro profile
→ drive road elevation spline
→ generate authoritative road/curb/collision from that spline
```

The DEM must not directly become the fine road collider.

Road-scale details remain project-generated.

## 38.2 Source-style 3D skybox

For distant scenery:

```text
DEM
→ aggressively simplified distant terrain
→ prerendered / Texture Forge art
→ cheap 3D skybox-style geometry
```

No gameplay collision.

This is a future option, not current La Chutana handling work.

---

# 39. Track debugging invariants

If the car behaves incorrectly on a generated track, diagnose in this order:

```text
1. reproduce
2. inspect telemetry
3. inspect wheel contact
4. inspect terrain/collision continuity
5. inspect surface groups
6. inspect vehicle setup
7. only then tune physics
```

Do not immediately change:

```text
mass
grip
suspension
steering
```

to hide a track-generation bug.

---

# 40. Known track failures that must not return

Status: `[FORBIDDEN REGRESSIONS]`

- infinite fall after leaving asphalt;
- wrong triangle winding after coordinate conversion;
- broad grass collision ribbons folding into invisible wedges;
- visual terrain clipping through road;
- grass cards intruding into asphalt due to footprint ignorance;
- visual guardrail mesh being used as detailed collision;
- procedural runs accumulating duplicate scenery;
- failed export deleting the last known-good GLB;
- wrong surface classification after Godot import;
- spawn sign errors due to coordinate conversion mismatch;
- curb shapes that launch the car.

If one of these symptoms appears, consult:

```text
game/docs/COMMON_ERRORS_AND_FIXES.md
```

before inventing a new fix.

---

# 41. Context garbage collection

Status: `[MANDATORY PROCESS]`

`PROJECT_STATE.md` must remain a current-state snapshot, not a chat transcript.

When information becomes reusable general knowledge:

```text
move it to:
game/docs/COMMON_ERRORS_AND_FIXES.md
```

When information becomes durable product/design direction:

```text
move it to:
game/docs/PROJECT_DIRECTION.md
```

When information becomes obsolete:

```text
remove it from PROJECT_STATE.md
or mark it [DEPRECATED] only if the historical warning is still useful
```

Do not accumulate abandoned hypotheses.

---

# 42. Context update triggers

The agent responsible for a change must update this file when any of the following occurs:

- development phase completed;
- validated physics setup changed;
- canonical vehicle scene changed;
- telemetry schema changed;
- track collision contract changed;
- active circuit changed;
- generated texture authority changed;
- active biome changed;
- pipeline architecture changed;
- important bug resolution changes a project invariant;
- frozen baseline intentionally reopened;
- a `[PLANNED]` value becomes `[VALIDATED]`.

Prefer updating `PROJECT_STATE.md` in the same work session/commit family.

---

# 43. How to update this file safely

Before editing:

1. inspect current branch;
2. inspect current HEAD;
3. inspect changed subsystem files;
4. compare against this snapshot;
5. update only facts that actually changed.

For each changed item:

```text
old status
→ new status
→ canonical path/value
→ validation evidence
```

Do not rewrite large unrelated sections merely for style.

---

# 44. Agent handoff rule

At the beginning of a new agent/chat session, the recommended bootstrap instruction is:

```text
Read PROJECT_STATE.md first.

Then read:
- game/docs/PROJECT_DIRECTION.md
- game/docs/COMMON_ERRORS_AND_FIXES.md
- relevant AGENTS.md files

Treat PROJECT_STATE.md as the current handoff snapshot.
Verify any value marked [UNKNOWN].
Do not reopen [FROZEN] or [VALIDATED] work without evidence.
```

This minimizes dependence on previous chat history.

---

# 45. Immediate next work

Status: `[ACTIVE PRIORITY]`

Before Phase C tuning:

## Task 1 — telemetry setup snapshot

Implement:

```text
<session>.csv
<session>_setup.json
```

Requirements:

- same basename;
- runtime snapshot;
- JSON;
- provenance;
- physical configuration;
- aids;
- surface configuration;
- new session when relevant setup changes.

## Task 2 — validate telemetry pairing

Prove that:

- every CSV receives a setup JSON;
- setup filename matches;
- values reflect runtime;
- missing optional families remain explicit rather than invented;
- changing steering setup starts a new run or produces a new paired snapshot.

## Task 3 — start Phase C

Only after telemetry provenance is trustworthy.

Then tune:

```text
steering
countersteer
unwind
steering exponent
steering assistance directly related to steering behavior
```

Do not simultaneously retune mechanical grip, suspension, powertrain or aero.

---

# 46. Definition of a valid Phase C test

A useful Phase C run should record:

```text
vehicle setup
track
Git commit
aid state
steering parameters
speed
steering input/output
yaw-related response if available
front/rear slip
lateral G
wheel contact/suspension state
```

Human test notes should describe:

- turn-in progression;
- steering buildup;
- unwind;
- initial oversteer response;
- countersteer timing;
- whether recovery feels understandable;
- whether a late correction still produces a spin;
- whether behavior changes materially with speed.

Do not judge Phase C from a single corner or one uncontrolled run.

---

# 47. Definition of done for Phase C

Phase C is not complete because a candidate setup merely feels better once.

Completion requires:

- paired telemetry/setup records;
- repeatable steering behavior;
- no obvious regressions to Phase B;
- progressive keyboard response;
- predictable unwind;
- recoverable but not omniscient countersteer;
- retained spin threshold;
- human validation on multiple corners/speeds;
- accepted final values recorded here as `[VALIDATED]`.

After Phase C closes:

1. replace candidate values with final validated values;
2. mark Phase C `[VALIDATED]`;
3. freeze those values for Phase D;
4. record any reusable debugging lesson in `COMMON_ERRORS_AND_FIXES.md`.

---

# 48. Forbidden interpretation shortcuts

Agents must not make these assumptions:

```text
"latest chat value" = validated value
"scene looks correct" = collision is correct
"generated texture exists" = source pipeline is correct
"same filename" = same physical setup
"aid OFF" = zero hidden stabilization
"current branch" = current file contents without inspection
"old telemetry" = usable without setup provenance
```

Always inspect the relevant authority.

---

# 49. Compact project invariants

The following principles should survive all future phases:

```text
1. Change one major physics family at a time.

2. Telemetry is paired with immutable setup provenance.

3. Runtime values beat remembered values.

4. Frozen GEVP baseline remains a diagnostic reference.

5. Vendor code is not the default place for Formula90s-specific fixes.

6. Track bugs are not solved by retuning vehicle physics.

7. Visual geometry and collision geometry are separate responsibilities.

8. Generated assets must be reproducible.

9. Texture Forge is the final deterministic texture compiler.

10. Procedural environment generation always starts from clean Base.

11. Generated runtime publication is atomic.

12. Current project state belongs in PROJECT_STATE.md,
    durable direction belongs in PROJECT_DIRECTION.md,
    reusable troubleshooting belongs in COMMON_ERRORS_AND_FIXES.md.

13. Candidate values must never be silently promoted to validated values.

14. A new chat/agent must be able to continue the project from repository context
    without requiring the previous conversation.
```

---

# 49.1 Runtime world/HUD presentation

Status: [VALIDATED]

Gameplay is composed through:

    GameBootstrap
    -> WorldHudCompositor
       -> WorldViewport (640x360 3D world)
       -> WorldPresenter (ViewportTexture in the root canvas)
       -> HudLayer (root CanvasLayer)

Canonical files:

    native/src/core/game_bootstrap.cpp
    native/include/formula90s/core/game_bootstrap.hpp
    game/scenes/runtime/world_hud_compositor.tscn
    game/scenes/runtime/world_hud_compositor.gd

The compositor moves only direct root Control nodes from the instantiated
world scene to HudLayer at runtime and retargets their vehicle/aids NodePaths.
Vehicle/track serialized node_paths remain inside the world scene unchanged.

This creates the required boundary for future world-only Super xBR processing:

    Super xBR shader -> WorldPresenter only
    HUD / minimap / menus -> never sampled by the world shader

Validation:

    smoke_test_world_hud_compositor.gd                    PASS
    smoke_test_bootstrap_world_hud_compositor.gd          PASS
    smoke_test_jordan_skybox_runtime.gd                   PASS
    smoke_test_la_chutana_skybox.gd                       PASS
    GPU visual composition capture at 1280x720            PASS

No final Super xBR shader, 3xBRZ texture generation, or player graphics toggle
is validated yet.

---

# 49.2 Arcade HUD and real-time La Chutana map

Status: [VALIDATED]

The normal gameplay HUD is a 16:9 late-1990s arcade presentation. It is still
drawn in `HudLayer`, never inside `WorldViewport`.

Canonical files:

    game/scenes/ui/debug_hud.tscn
    game/scenes/ui/la_chutana_hud_map.tres
    game/addons/formula90s/scripts/arcade_race_hud.gd
    game/addons/formula90s/scripts/arcade_speed_gauge.gd
    game/addons/formula90s/scripts/track_minimap_controller.gd
    game/addons/formula90s/scripts/track_map_data.gd
    game/addons/formula90s/scripts/driving_aids.gd

Presentation contract:

    left-middle: simple canonical La Chutana outline
                 + green start/finish marker
                 + oriented blue player marker derived from vehicle transform

    bottom-right: color-segment speed arc + runtime KPH + runtime gear

    top-right: transient white aid-state message, no panel/background

`TrackMapData` is presentation data only. It samples the canonical generated
La Chutana centerline; it must not be used as lap progress or race position
authority.

`DrivingAidsController` emits:

    signal aid_toggled(aid_label: String, enabled: bool)

`ArcadeRaceHud` consumes that signal and shows `AYUDA <label>
ACTIVADA/DESACTIVADA` for a short duration, then fades it away.

The prior cyan gameplay diagnostics panel is no longer shown in normal play.
`WheelDiagnostics` remains available but defaults to hidden in the Jordan
handling scene.

Future boundary:

    UI-002 / FUTURE_UI-002
    -> race-session authority for lap, position, countdown, and opponents.

Do not display invented LAP/POS/countdown/rival values before that authority
exists. The HUD scene documents this with semicolon-prefixed `.tscn` comments.

Validation:

    smoke_test_arcade_hud_scene.gd                      PASS
    smoke_test_arcade_hud_runtime.gd                    PASS
    smoke_test_world_hud_compositor.gd                  PASS
    smoke_test_bootstrap_world_hud_compositor.gd        PASS
    smoke_test_jordan_skybox_runtime.gd                 PASS
    smoke_test_la_chutana_skybox.gd                     PASS
    GPU capture arcade_hud_16x9.png at 1280x720         PASS
    GPU world-tint capture confirms HUD remains crisp   PASS

Default-route (GameBootstrap) evidence:

    The default bootstrap track (test_field.tscn) required a DrivingAids
    controller for the full HUD contract: without it the visual capture could
    not toggle the aid notification. Added the same DrivingAids node/script
    used by jordan_handling_test.tscn. Re-ran and captured:

        smoke_test_bootstrap_world_hud_compositor.gd   PASS
        (default route: start_game -> compositor -> minimap tracks the
         injected vehicle; projected map position changes after movement)

        visual_test_arcade_hud_capture.gd              PASS
        capture: user://arcade_hud_16x9.png (1280x720)
        pixel analysis: minimap track outline + green start marker + blue
        player marker present; speed gauge yellow/orange/red/purple segments
        and white KPH text present; no HUD filtering by WorldPresenter.

    The compact widget sizes match the acceptance contract at 1280x720
    (2x canvas scale): minimap 216x256, speed gauge 352x220 physical pixels.

    Test hardening: vehicle teleports in the default-route tests use
    PhysicsServer3D.body_set_state instead of assigning global_position,
    because the physics server can revert a direct transform set on a
    sleeping RigidBody3D (raced against the 60 Hz physics step). Both
    tests are deterministic (5/5 and 3/3 runs).

    smoke_test_jordan_skybox_runtime.gd: the camera-follow assertion waits
    two process frames (SourceSkyboxRig._process updates after the
    process_frame signal), fixing a one-frame race. Deterministic 5/5.

Known unrelated validation output:

    EngineAudioController requires EngineAudioConfig
    -> existing test_field default-scene configuration warning.

---

# 50. Maintenance footer

When updating this document, update this footer.

```text
Last reviewed:
Branch:
Commit:
Current active phase:
Next required gate:
Reviewed by:
```

Recommended current values at the time this snapshot is created:

```text
Last reviewed:
2026-08-10

Branch:
refactor/gevp-clean-baseline

Commit:
e772e370efe32bf1636f683189ded2d513076520

Current active phase:
pre-Phase-C instrumentation

Next required gate:
telemetry CSV + immutable _setup.json pairing

Next physics phase:
Phase C — steering / countersteer

Reviewed by:
Codex — world/HUD compositor validation
```

The exact commit SHA must be refreshed from Git before this file is treated as a
new canonical repository snapshot.

---

# End of current-state handoff
