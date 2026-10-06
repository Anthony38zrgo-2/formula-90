# Formula90s: Coupled Vehicle Dynamics and Rust-Owned Collision Backlog

## 1. Delivery mandate

### Full-backlog continuation status — 2026-10-05

Items 01–04 have foundation deliverables implemented and are awaiting human review. See `docs/coupled_vehicle_foundation_review.md`, `docs/coupled_vehicle_foundation_status.json`, `docs/coupled_vehicle_architecture_contract.json` and `docs/coupled_vehicle_validation_specification.json`.

The authorized implementation scope is all 50 items. Candidate progress and remaining acceptance work are recorded individually in `docs/coupled_vehicle_backlog_execution_status.json`; technical details and evidence limits are in `docs/coupled_vehicle_candidate_implementation.md`. This delivery remains incomplete and is not runtime activated or human approved.

The previous Sprint 0 review recorded 28 passing reference tests and two geometric failures. The continuation repairs the manifest linkage and preserves an equal-mass legacy fixture; the geometric suite now passes eight tests. The first complete candidate review instead exposes four pre-existing aerodynamic failures, reproduced independently from protected baseline sources. Preserve historical review records and these failures; do not infer full runtime validation from selected passing suites.

Implement the proposed physical mass inventory, coupled chassis and suspension dynamics, wheel rotation, and Rust-owned vehicle collision response. Evaluate Parry for geometric collision detection. Godot remains responsible for scene authoring, input collection, rendering, and presentation.

This document fully replaces the former instructions. Creating it is a planning task: it does not activate the reference solver, approve estimates as measurements, authorize publication, or establish runtime validation.

Follow the repository pipeline: plan sprint -> select backlog item -> implement -> review -> human approval -> done. Complete authorized work and present concrete review evidence before asking for sprint approval.

### Required outcomes

1. An auditable 600 kilogram vehicle-and-driver budget excluding wheel assemblies and fuel.
2. Provisional complete unfueled operating mass of 698 kilograms with the current wheel declarations.
3. Physical component masses, centers, inertia tensors, and trajectories contributing to coupled dynamics.
4. One authoritative Rust vehicle state and one owner for every force, integration operation, and contact response.
5. Collision impulses transmitted consistently through chassis, wheels, and suspension.
6. Shared physics kernels for standalone diagnostics and the Godot runtime.
7. Validated geometry, mounting forces, tire behavior, contacts, replay, and race performance.
8. A source-matched runtime build through scripts/run_f1_94.ps1 and a human-reviewed delivery.

### Scope

- Start with the Formula One 2030 profile and the existing Fuji integration. Preserve other profiles through explicit compatibility behavior.
- Include interactions between Rust-owned vehicles and define interoperability with remaining Godot-owned dynamic objects before enabling mixed scenes.
- Integrate existing tire friction, wheel rotation, brakes, drivetrain, fuel mass, aerodynamic forces, resets, and relevant gameplay consumers.
- Deliver physical collision events to existing telemetry, audio, and visual consumers. Reuse existing sound assets.
- Implement rigid collision shapes and physically coherent contacts. Structural fracture, deformable chassis, finite-element tires, soft barriers, networking rollback, and engine redesign require separate scope.
- Treat this as a fictional vehicle. Do not claim actual Formula One 2030 specifications.
- An audio engine mode named hybrid is not evidence of hybrid propulsion.
- Do not alter unrelated input mappings, sound banks, graphical assets, vehicle handling, or pit-stop behavior.

## 2. Provenance and protected work

Planning snapshot captured on 2026-10-05:

| Field | Recorded value |
| --- | --- |
| Workspace | D:\Formula90s |
| Branch | main-clean |
| Source HEAD | 001cfc9e6ba0f77a9a9fd0759e82a995228021a9 |
| Previous instrucciones.md SHA-256 | F558D04DFB3B85A10A85805D62FC52268F182735D60047C3359F4CA76674FBDD |
| Checkout condition | Dirty before this document replacement |
| Actions performed for planning | Read-only inventory and replacement of instrucciones.md |

Recapture branch, HEAD, status, relevant diffs, configuration hashes, and build provenance before each implementation sprint. This snapshot is not a claim about future checkout state.

### Pre-existing modified source and configuration

    game/crates/vehicle-physics-engine/src/lib.rs
    game/crates/vehicle-physics-engine/src/vehicle_config.rs
    game/crates/vehicle-physics-engine/tests/fuel_model_test.rs
    game/crates/vehicle-physics-engine/tests/suspension_f1_2030_geometric_test.rs
    game/data/vehicles/f1_2030/f1_2030_v10_geometric.json
    game/tests/visual_preview_f1_2030_suspension.gd
    tools/physics_diagnostics/preview_suspension_visuals.ps1

### Pre-existing generated modifications

    game/BUILD_SOURCE
    game/addons/formula90s/bin/formula90_core.dll
    game/addons/formula90s/bin/formula90_core.windows.template_debug.x86_64.dll
    game/addons/formula90s/bin/game_sim.dll
    game/addons/formula90s/bin/game_sim.windows.template_debug.x86_64.dll
    game/addons/formula90s/bin/libformula90s.windows.template_debug.x86_64.dll
    game/addons/formula90s/bin/psx_art_plugin.dll
    game/addons/formula90s/bin/psx_art_plugin.windows.template_debug.x86_64.dll
    game/addons/formula90s/bin/vehicle_audio_engine.dll
    game/addons/formula90s/bin/vehicle_audio_engine.windows.template_debug.x86_64.dll
    game/addons/formula90s/bin/vehicle_physics_engine.windows.template_debug.x86_64.dll

### Pre-existing untracked work

    docs/suspension_coupled_dynamics_backlog.json
    docs/suspension_coupled_dynamics_implementation.md
    game/crates/vehicle-physics-engine/examples/coupled_suspension_reference.rs
    game/crates/vehicle-physics-engine/examples/suspension_mass_audit.rs
    game/crates/vehicle-physics-engine/src/suspension_component_kinematics.rs
    game/crates/vehicle-physics-engine/src/suspension_coupled_solver.rs
    game/crates/vehicle-physics-engine/src/suspension_mass_audit.rs
    game/crates/vehicle-physics-engine/src/suspension_mass_properties.rs
    game/crates/vehicle-physics-engine/src/suspension_multibody.rs
    game/crates/vehicle-physics-engine/tests/suspension_mass_audit_test.rs
    game/crates/vehicle-physics-engine/tests/suspension_multibody_test.rs
    tools/blender/validate_rear_suspension_chassis_clearance.py
    game/assets/fonts/BarlowCondensed-Medium.ttf.import
    game/assets/models/drivers/driver.glb.import
    game/assets/models/pit_crew/racer/source/
    game/features/retro_hud/assets/digital_segment.svg.import
    game/features/retro_hud/assets/divider.svg.import
    game/features/retro_hud/assets/fuel_icon.svg.import
    game/features/retro_hud/assets/fuel_segment.svg.import
    game/features/retro_hud/assets/rpm_segment.svg.import

Inventory does not grant ownership. Preserve existing diff boundaries; review reusable physics changes before extending them. Never stage foreign, generated, or pre-existing content as new work without establishing ownership and authorization.

Do not switch, reset, move, or clean the dirty branch without inventory and an approved backup. Read applicable nested AGENTS.md before editing implementation paths. Use fully explanatory new identifiers and filenames without abbreviations; add no code comments. Existing interface names are compatibility references, not naming examples.

## 3. Current implementation baseline

### Confirmed integration surfaces

| Path | Current responsibility |
| --- | --- |
| game/crates/vehicle-physics-engine/src/vehicle_config.rs | Mass declarations and profile parsing |
| game/crates/vehicle-physics-engine/src/simulation.rs | External force solving and legacy standalone integration |
| game/crates/game-sim/src/c_abi.rs | Native external solver boundary |
| game/crates/game-sim/src/world.rs | Vehicle entities and world stepping |
| native/src/core/f90_core.cpp | Applies Rust forces and torques to a Godot body |
| native/src/vehicle/f1_94_rust_vehicle.cpp | Scene raycasts, host integration, telemetry, scalar mass updates |
| game/data/vehicles/f1_2030/f1_2030_v10_geometric.json | Current vehicle profile |
| game/project.godot | Currently declares 120 physics ticks per second |

### Existing reference capabilities

| Artifact | Available reference capability | Remaining limitation |
| --- | --- | --- |
| suspension_mass_audit.rs | Mass contract and load audit | No measured full inventory |
| suspension_mass_properties.rs | Provenance, tensor validation, component composition | Vehicle properties remain incomplete |
| suspension_component_kinematics.rs | Hardpoint-based poses and spatial derivatives | Full-domain validation and runtime wiring |
| suspension_multibody.rs | Ten velocity coordinates, coupled matrix, gravity, stop impulses | General world contacts and wheel spin |
| suspension_coupled_solver.rs | Standalone coupled suspension, fuel exchange, planar normal contact | Brush forces, rotating dynamics, general collision response, game activation |
| coupled_suspension_reference.rs | Actual-profile reference runner | Not an in-game implementation |
| suspension_multibody_test.rs | Mathematical reference checks | Not full-car calibration or release validation |

Earlier documentation reports 16 multibody tests, four mass-audit tests, eight fuel tests, and estimated settling captures. Reproduce them against the sprint snapshot.

The five-component scratch inventory uses placeholder body and wheel tensors and omits explicit brakes, uprights, and links. Keep it diagnostic-only. Its settling error and release execution time establish neither measured vehicle accuracy nor race performance.

The analytical 700 kilogram horizontal and 600 kilogram vertical effective body masses describe a synthetic test, not the actual Formula One 2030 vehicle.

The previous JSON backlog still identifies component inclusion and collision ownership as pending clarification. Reconcile it with the selected planning direction when implementing Item 10; do not silently treat outdated statuses as active requirements.

## 4. Mass contract and provisional allocations

### Definitions

- Vehicle budget excluding wheels and fuel: 600 kilograms including driver, engine, transmission, structures, systems, brakes, uprights, and suspension members.
- Wheel assembly: rim, inflated tire, and explicitly inventoried wheel-mounted accessories. Exclude upright, hub, and brake assembly from the provisional 21/28 kilogram declarations.
- Complete unfueled operating mass: every physical vehicle component and the driver, including non-fuel operating fluids, excluding fuel. This definition avoids ambiguity around the term dry mass.
- Fuel mass: separate time-varying reservoir, added exactly once.
- Suspended and unsuspended contributions: derived from component motion and constraints. Virtual tire-contact filter masses are not physical inventory.
- Budget groups are bookkeeping aggregates; production dynamics must decompose them into physical components.

| Budget group | Kilograms | Evidence classification |
| --- | ---: | --- |
| Driver, equipment, and driver ballast | 82 | Fictional allocation informed by FIA reference |
| V10 engine | 110 | Engineering estimate |
| Transmission and differential, excluding driveshafts | 50 | Engineering estimate |
| Monocoque, impact structures, driver protection, floor, aerodynamic bodywork | 180 | Engineering estimate |
| Cooling, non-fuel fluids, empty fuel system, electronics, hydraulics | 100 | Engineering estimate |
| Inboard suspension, steering, driveshafts, fixings, allocation reserve | 46 | Engineering estimate requiring decomposition |
| Outboard brakes, hubs, and uprights | 26 | Engineering estimate |
| Exterior suspension arms and rods | 6 | Engineering estimate |
| Base vehicle-and-driver budget | 600 | Human-declared total; proposed inclusion contract |
| Two front wheel assemblies | 42 | Current profile declaration, not measured |
| Two rear wheel assemblies | 56 | Current profile declaration, not measured |
| Complete unfueled operating mass | 698 | Arithmetic closure |
| Initial fuel | 7.6 | Current profile declaration |
| Initial operating mass | 705.6 | Arithmetic closure |
| Full configured fuel capacity | 110 | Configuration, not a regulated 2030 allowance |
| Operating mass at configured fuel capacity | 808 | Arithmetic closure |

The 26 kilogram outboard allocation starts with six kilograms per front corner and seven per rear corner. These are estimates, not manufacturer measurements.

Extract moving masses from the 600 kilogram aggregate before adding their explicit bodies. Extracting the provisional 26 kilogram outboard group and six kilogram exterior-link group leaves 568 kilograms in the remaining aggregate. This is not the final fixed chassis mass: inboard moving parts and driveshafts still require decomposition.

Do not invent ballast to conceal mismatched inventory. Replace allocation reserves with explicit equipment, documented assumptions, or actual modeled ballast before declaring closure.

The current front distribution of 0.45 is not automatically a complete-vehicle target. If it applies to the 600 kilogram base, front support is 270 kilograms; adding the front wheels gives 312 / 698, approximately 44.70 percent before fuel. Verify existing semantics, then derive the new distribution from component centers and axle positions. Changing a distribution target is calibration, not accounting validation.

## 5. Target architecture and physical contracts

### Ownership

| Responsibility | Owner |
| --- | --- |
| Authoritative vehicle position, orientation, and velocities | Rust simulation world |
| Component mass properties and trajectories | Rust physical model |
| Suspension, tire, brake, drivetrain, fuel, aerodynamic forces | Rust vehicle systems |
| Integration and physical collision response | Rust simulation world |
| Collision detection | Rust world with an evaluated library such as Parry |
| Physical asset preparation | Versioned tools and manifests |
| Input collection and scene lifecycle requests | Godot adapter |
| Meshes, camera, driver presentation, interface, sound playback | Godot presentation |
| Contact telemetry and collision events | Rust producer; adapter delivery |

Godot custom_integrator retains Godot collision response. It does not establish exclusive Rust ownership. Disable physical double integration and collision response for Rust-controlled vehicle representations.

Godot can represent multiple bodies and joints. The limitation motivating this decision is the current single Godot body with separate internal Rust suspension coordinates.

Parry provides geometric queries, not this vehicle's generalized impulse solver. Using a stock Rapier body with independently integrated internal wheels would preserve an ownership mismatch. Evaluate articulated representation compatibility before choosing any library response solver.

### Required mathematics

    generalized_mass_matrix = sum(
        linear_velocity_jacobian_transpose * component_mass * linear_velocity_jacobian
        + angular_velocity_jacobian_transpose * world_inertia_tensor * angular_velocity_jacobian
    )

    generalized_mass_matrix * generalized_acceleration
        = generalized_applied_force - generalized_inertial_bias

    generalized_velocity_change
        = solve(generalized_mass_matrix, contact_jacobian_transpose * contact_impulse)

    contact_inverse_effective_mass
        = contact_jacobian * solve(generalized_mass_matrix, contact_jacobian_transpose)

These specify equations, not instructions to explicitly invert dense matrices.

Include gravity, configuration-dependent acceleration terms, gyroscopic effects, actuator work, and fuel exchange. The current reference has six body velocities and four travel velocities. Production coupling also needs four wheel angular velocities and explicit engine/drivetrain states; do not discard mechanical states to preserve a fixed matrix size.

Require unilateral contact, calibrated restitution, dissipative friction, and valid travel limits. Elastic tire compression stores energy; damping and sliding dissipate it. Report stabilization work rather than hiding artificial energy.

### Time and representation

- Fixed physical steps with bounded subdivisions; rendering uses separate interpolation.
- Evaluate frequencies such as 480, 960, and 1920 steps per second through convergence and measured cost. These are candidates, not release requirements.
- Refresh contact geometry or conservatively validate it inside physical subdivisions. A rendered-frame raycast does not establish continuous collision safety.
- Double precision in the physical kernel; explicit conversions at Godot boundaries.
- Stable entity, shape, and contact identities; explicit coordinates, normal directions, time, and impulse units.
- Quaternion-aware render interpolation that is never fed back into physical state.
- Explicit behavior for initial overlap, subdivision exhaustion, unsupported shapes, and numerical failures. Never silently discard simulated time.

## 6. Backlog conventions

All numbered items are pending unless explicitly described as existing reference work awaiting review. Reference availability is not runtime completion.

Each item must produce a scoped diff, validation commands, machine-readable results, source and configuration hashes, limitations, and review outcome. Place generated evidence in a task-specific scratch directory. Reconcile older status records when their assertions become stale.

Select an item with satisfied dependencies. Internal implementation choices may be resolved within the authorized scope; clarify material requirement changes. Each sprint ends with technical review and a concrete human approval. Done requires acceptance evidence and the required approval.

Priority critical denotes a dependency or correctness gate. Priority high denotes required production integration or validation.

## Sprint 0: Establish continuation and acceptance contracts

### Item 01 — Capture provenance and change ownership

- Priority: critical. Dependencies: none.
- Surface: root and nested protocols, existing diffs, build provenance.
- Work: capture branch, HEAD, index, tracked diffs, untracked inventory, source/profile hashes, and installed binary provenance. Identify reusable physics changes and protected work. Choose a safe workspace before branch operations.
- Deliverable: provenance manifest and reviewed ownership map.
- Acceptance: committed source, local source changes, generated artifacts, imports, and foreign work are distinguished; no foreign changes are staged; all existing work survives.
- Validation: compare explicit path diffs and the recorded status inventory.
- Stop condition: ambiguous ownership affecting an edit or unsafe branch operations require clarification before that action.

### Item 02 — Review and reproduce the reference

- Priority: critical. Dependencies: 01.
- Surface: existing mass audit, properties, kinematics, multibody reference, runner, tests.
- Work: reproduce reported tests and captures using fresh isolated output. Audit coordinate signs, matrix construction, fuel updates, contact ownership, assumptions, and cache lifetime.
- Deliverable: reusable capability matrix, defects, missing integrations, and stale assertions.
- Acceptance: capabilities have reproducible evidence or recorded failure; scratch properties stay diagnostic-only; source tests are not represented as runtime approval.
- Validation: targeted existing tests and actual-profile audit; separate known manifest failures from newly introduced failures.

### Item 03 — Freeze mass and collision ownership contracts

- Priority: critical. Dependencies: 01, 02.
- Surface: configuration, world interfaces, native bridge, component schema.
- Work: encode the 600 kilogram inclusion contract, rim-and-tire definition, fluid policy, Rust state ownership, and Godot presentation role. Identify world objects requiring migration or interoperability.
- Deliverable: architecture decision and compatibility contract.
- Acceptance: each component, force, and integration operation has one owner; other profiles retain explicit legacy behavior; older clarification flags are resolved against this planning direction.
- Validation: trace one physical step, collision impulse, fuel update, and reset through all owners.
- Review: present concrete contract changes without re-asking decisions already established in the session.

### Item 04 — Define quantitative validation budgets

- Priority: critical. Dependencies: 02, 03.
- Surface: diagnostic scenarios and validation specification.
- Work: specify units, normalization, physical scales, reference solutions, seeds, supported speeds, and tolerances. Distinguish proposed development gates from calibrated or approved release targets.
- Deliverable: acceptance specification used by subsequent items.
- Acceptance: mass closure, equilibrium, derivatives, conservation, penetration, convergence, replay, and cost each have a measurable pass condition.
- Validation: preserve existing reference checks without silently changing their meaning.
- Gate: never loosen thresholds only to obtain passing results.

## Sprint 1: Close physical mass properties

### Item 05 — Extend the versioned physical inventory

- Priority: critical. Dependencies: 03, 04.
- Surface: suspension_mass_properties.rs, parser, future inventory data.
- Existing work: reference component schema and provenance enumeration.
- Work: define identity, attachment, mass, center, tensor, coordinate frame, wheel corner, source, uncertainty, budget membership, and rotating participation.
- Deliverable: validated schema, explicit migration, serialization round trip.
- Acceptance: duplicates, ambiguous attachments, non-finite properties, invalid tensors, and unsupported versions fail with actionable errors.
- Validation: invalid-input cases and representative complete-vehicle round trip.

### Item 06 — Produce the complete component ledger

- Priority: critical. Dependencies: 05.
- Surface: Formula One 2030 inventory and mass audit.
- Work: decompose driver, engine, fluids, fixed equipment, upright, hub, caliper, disc, pads, arms, rods, rocker, spring, moving damper parts, shafts, and steering assembly. Assign each mass once.
- Deliverable: component ledger with provenance and physical attachment.
- Acceptance: base closes at 600 kilograms within 0.000001 kilogram accounting tolerance; current wheels produce 698 kilograms; initial fuel produces 705.6 kilograms; virtual filter masses are excluded.
- Validation: independently sum membership groups and detect duplicate extraction.
- Limitation: accounting precision is not measurement precision.

### Item 07 — Establish estimation ranges and evidence

- Priority: high. Dependencies: 06.
- Surface: inventory provenance and source register.
- Work: evaluate the provisional engine, transmission, wheels, outboard assemblies, and structural allocations using primary sources and geometry. Record differing component perimeters and justified low/central/high estimates.
- Deliverable: assumption register linked to physical components.
- Acceptance: estimates state rationale and uncertainty; tire-only evidence is not treated as assembly mass; fictional values are not labeled manufacturer specifications.
- Validation: budget variants report complete mass, axle loads, and uncertainty effects.
- Gate: uncertain wheel masses must not be hidden by fitting spring stiffness.

### Item 08 — Derive centers and full inertia tensors

- Priority: critical. Dependencies: 05, 06, 07.
- Surface: geometry extraction, tensor composition, wheel mechanics.
- Work: derive properties from geometry, specifications, or explicit simplified solids; transform tensors and apply the parallel-axis theorem. Separate rotating rim/tire/hub/disc contributions from non-rotating uprights.
- Deliverable: component and composite center/inertia report.
- Acceptance: tensor validity and mirrored transformations pass; inertia changes correctly with configuration; rectangular-body and thin-ring placeholders are replaced or explicitly uncertainty-labeled.
- Validation: analytical compound bodies, asymmetric tensor rotation, and mirrored corners.

### Item 09 — Integrate fuel mass properties and exchange

- Priority: critical. Dependencies: 06, 08.
- Surface: fuel model, inventory composition, reference fuel exchange.
- Work: add fuel once at the tank position; distinguish point-reservoir approximation from finite tank inertia. Update at physical boundaries and report momentum/energy carried by consumed fuel under the chosen approximation.
- Deliverable: shared mass-property update for diagnostics and runtime.
- Acceptance: zero/initial/full fuel masses close; consumption introduces no unexplained velocity jump; refueling/reset invalidate caches; point fuel receives no fictitious intrinsic inertia.
- Validation: axle moments, constant-velocity consumption, stationary refueling, capacity bounds.

### Item 10 — Migrate loading and reconcile status records

- Priority: high. Dependencies: 06, 08, 09.
- Surface: vehicle_config.rs, geometric profile, vehicle manifests, previous backlog and implementation records.
- Work: load production inventory through the runtime parser; preserve complete-mass legacy profiles; trace and resolve the known profile/asset-manifest mismatch.
- Deliverable: canonical profile selection and consistent implementation statuses.
- Acceptance: game and diagnostics select the intended profile; serialization preserves semantics; invalid properties fail before clamps; unrelated profiles retain defaults.
- Validation: actual-profile loader, manifest linkage, serialization, compatibility, and reset.

## Sprint 2: Complete coupled suspension mechanics

### Item 11 — Validate hardpoints and full operating envelopes

- Priority: critical. Dependencies: 08, 10.
- Surface: suspension geometry, kinematics, rear clearance validator, preview fixtures.
- Work: sweep both axles through bump, rebound, steering, heave, roll, and combined travel. Check the rear damper mount, rocker, actuation rod, body shell, gearbox, shafts, and neighboring members with finite volumes.
- Deliverable: geometry domain, clearance report, and offending configurations.
- Acceptance: no unsupported singularity or branch switch inside the domain; constrained rod lengths hold; damper stroke remains valid; clearance accounts for member thickness, not just lines.
- Validation: mirrored sweeps, extreme combined poses, numerical endpoint checks, and orthographic rear views.

### Item 12 — Verify component poses and spatial derivatives

- Priority: critical. Dependencies: 08, 11.
- Surface: suspension_component_kinematics.rs and derivative diagnostics.
- Existing work: actual-hardpoint component poses and derivative support.
- Work: validate derivatives for chassis translation/rotation, corner travel, steering, and internal member motion. Derive linear/angular Jacobians and acceleration bias from the same pose convention.
- Deliverable: derivative report across the approved geometry domain.
- Acceptance: centered finite-difference errors converge over several perturbation sizes; signs match coordinates; steering actuator work is included; changed geometry invalidates caches.
- Validation: asymmetric hardpoints, corner mirroring, near-limit configurations, and mixed chassis-wheel motion.

### Item 13 — Assemble the production coupled mass matrix

- Priority: critical. Dependencies: 08, 09, 12.
- Surface: suspension_multibody.rs and generalized state.
- Existing work: ten-coordinate matrix and scaled positive-definite solve.
- Work: include every inventoried moving component, configuration-dependent inertial bias, and gravity. Define extension for wheel rotation and drivetrain states without duplicating their inertia.
- Deliverable: production matrix assembly and factorization interface.
- Acceptance: symmetry, physical energy equivalence, positive definiteness for supported free coordinates, and conditioning checks pass; unsupported singularities fail explicitly.
- Validation: analytical bodies, freefall, internal momentum balance, and configuration sweeps.
- Constraint: reuse a factorization only while its physical configuration and mass properties are unchanged.

### Item 14 — Integrate orientation and rotating inertia

- Priority: critical. Dependencies: 12, 13.
- Surface: coupled orientation, wheel mechanics, rotating properties.
- Work: define body angular-velocity frames, quaternion evolution, tensor rotations, gyroscopic terms, and spinning wheel angular momentum. Distinguish upright orientation from wheel spin.
- Deliverable: coupled rotational dynamics with explicit frame conversions.
- Acceptance: orientation normalization remains within budget; gyroscopic signs are correct; rigid rotation and spin energies can be audited separately; rotating inertia is counted once.
- Validation: torque-free asymmetric body, wheel steering while spinning, and angular momentum conservation.

### Item 15 — Project suspension forces from mechanical work

- Priority: critical. Dependencies: 12, 13.
- Surface: suspension_coupled_solver.rs, suspension_loads.rs, configuration.
- Existing work: spring, asymmetric damper, and conservative anti-roll reference.
- Work: project spring, damper, anti-roll, preload, and steering contributions through actual trajectories. Preserve motion-ratio direction and the preload term in tangent stiffness.
- Deliverable: generalized force, stored energy, and damping-work evaluation.
- Acceptance: conservative forces match potential gradients; dampers dissipate energy; symmetric heave and axle roll produce correct anti-roll behavior; invalid force definitions fail before runtime.
- Validation: isolated corner, heave, roll, asymmetric damping, and varying motion ratio.

### Item 16 — Resolve travel limits as mechanical events

- Priority: critical. Dependencies: 11, 13, 15.
- Surface: travel constraints and reference advancement.
- Existing work: simultaneous velocity projection; event and position solution remain pending.
- Work: distinguish progressive stop force from absolute travel bounds; localize crossings, apply coupled impulses, and correct positional error with accounted work.
- Deliverable: bounded suspension with auditable impact energy.
- Acceptance: simultaneous limits are solved together; no travel exceeds the approved domain; restitution is explicit; separating motion is not incorrectly clamped.
- Validation: one-corner compression, simultaneous axle stop, droop, repeated limit contact, and timestep refinement.

### Item 17 — Recover physical joint and mounting loads

- Priority: high. Dependencies: 13, 14, 15, 16.
- Surface: suspension_loads.rs and load telemetry.
- Work: recover rod axial forces, rocker moments, damper loads, joint reactions, and chassis mount loads using component free-body balance. Include inertial and contact contributions.
- Deliverable: front/rear mechanical load report.
- Acceptance: action/reaction balance closes; spring force is not mislabeled as total mount reaction; residuals and near-singular amplification are visible.
- Validation: static loading, braking, roll, one-wheel bump, and travel-stop impact.
- Limitation: this produces loads, not structural strength or fatigue certification.

## Sprint 3: Integrate all vehicle force systems

### Item 18 — Couple wheel spin, braking, and drivetrain reactions

- Priority: critical. Dependencies: 08, 13, 14.
- Surface: wheel_mechanics.rs, powertrain.rs, coupled state, brake model.
- Work: integrate four wheel angular velocities and torque exchange; project tire and brake torque with equal-and-opposite reactions; preserve engine, clutch, differential, gearing, and driveshaft states.
- Deliverable: wheel-spin and drivetrain coupling with a mechanical power ledger.
- Acceptance: no duplicate wheel inertia; friction impulse can influence wheel rotation; locked wheels, airborne spin, acceleration, braking, and gear changes preserve torque direction and balance.
- Validation: isolated spinning wheel, free drivetrain, braking without tire contact, driven traction, and reverse.
- Gate: do not use artificial airborne spin decay without accounting for dissipated energy.

### Item 19 — Connect the existing tire model to actual contact motion

- Priority: critical. Dependencies: 12, 15, 18.
- Surface: tire.rs, wheel mechanics, contact state, existing thermal/wear consumers.
- Work: compute hub and tread velocities from current coupled state and moving surface velocity. Feed actual load, camber, slip, pressure, and relaxation state into the brush model; return generalized forces and wheel torque.
- Deliverable: coherent normal/compliant and tangential tire interface.
- Acceptance: no tensile normal force; friction opposes slip under the selected convention; combined-slip saturation and low-speed behavior are stable; relaxation advances with physical time.
- Validation: rolling without slip, standstill, locked-wheel slide, combined slip, airborne transition, and pressure variation.
- Preserve: existing surface, tire temperature, wear, and brake heat behavior unless a physical inconsistency requires a scoped correction.

### Item 20 — Project aerodynamic and remaining external forces

- Priority: critical. Dependencies: 13, 14, 18, 19.
- Surface: aero.rs, simulation.rs, underfloor samples, existing force producers.
- Work: apply each force at its physical point with matching generalized work; migrate drag, downforce, gravity, rolling resistance, and other existing force channels. Refresh height-sensitive aerodynamic inputs at the physical cadence.
- Deliverable: complete force ownership and power report.
- Acceptance: force positions generate correct moments; gravity is applied once; old aggregate force integration does not run in the new path; assist contributions remain identifiable and configurable.
- Validation: straight-line drag, static downforce, pitched body, moving ground, and existing driving-aid compatibility.
- Gate: do not retune aerodynamic balance to conceal suspension or collision errors.

### Item 21 — Introduce an authoritative Rust simulation world

- Priority: critical. Dependencies: 03, 10, 13.
- Surface: game-sim world, vehicle state, configuration ownership.
- Work: own all vehicle states in one world; allocate stable entity identities; define input timestamps, state snapshots, shape attachment references, and lifecycle request queues.
- Deliverable: headless world interface and explicit state ownership.
- Acceptance: each vehicle advances exactly once per world step; physics is not driven independently from per-body Godot callbacks; entity ordering is stable; deleted entities cannot be addressed through stale handles.
- Validation: zero/one/multiple entities, creation/removal ordering, queued reset, and repeated identical input.
- Preserve: legacy vehicles through an explicit route, not accidental mixing of owners.

### Item 22 — Select and implement the production integrator

- Priority: critical. Dependencies: 13, 14, 15, 16, 18, 19, 20, 21.
- Surface: coupled advancement, step settings, scheduling.
- Work: compare the reference semi-implicit method with methods suitable for the observed stiffness; define fixed steps, subdivisions, constraint ordering, force reevaluation, and numerical failure handling.
- Deliverable: documented integration method and step selection report.
- Acceptance: convergence and passivity meet Item 04 budgets; physical time is never dropped silently; input samples are applied at declared times; repeated subdivisions do not repeat fuel consumption or accumulated events.
- Validation: frequency comparisons, stiff damping, travel-limit events, and unusual host frame durations.
- Gate: select frequencies from measured error and cost, not another simulator's advertised rate.

### Item 23 — Establish the complete pre-collision reference benches

- Priority: critical. Dependencies: 17, 18, 19, 20, 22.
- Surface: diagnostic runner, physical inventory, scenario fixtures.
- Work: add static equilibrium, freefall, heave, pitch, roll, one-wheel excitation, braking, cornering, and smooth-road tire cases using the production force path.
- Deliverable: baseline results and reproducible commands with source/configuration hashes.
- Acceptance: total support matches operating weight; mode shapes/frequencies are checked where analytical solutions exist; energy and momentum ledgers close; placeholder inventories are not used for calibration claims.
- Validation: independent analytical fixtures plus actual-profile cases.
- Review gate: collision development starts from a physically consistent reference, not a visually fitted baseline.

## Sprint 4: Implement collision detection and generalized response

### Item 24 — Evaluate and select the geometric detection library

- Priority: critical. Dependencies: 03, 04, 21, 23.
- Surface: isolated collision prototype, dependency manifest and lockfile.
- Work: evaluate Parry precision, convex/mesh support, contact generation, shape casts, rotating motion, performance, maintenance, and license. Compare any proposed alternative against the same fixtures.
- Deliverable: measured library decision and explicit supported-shape contract.
- Acceptance: required geometric cases work in double precision; limitations are documented; dependency versions are pinned; no stock response solver is assumed to support the vehicle matrix without proof.
- Validation: spheres, convex compounds, thin barriers, triangle edges, and rotating sweeps.
- Decision: retain the proposed Parry direction if the measured requirements are satisfied; clarify a material architecture change before adoption.

### Item 25 — Prepare and import the physical world geometry

- Priority: critical. Dependencies: 24.
- Surface: existing Fuji collision assets, game/tracks/fuji76_77/metadata/package.json, asset preparation tools, Rust importer.
- Work: define a versioned physical package containing geometry, transforms, materials, surfaces, stable shape identities, and source hashes. Read the existing collision asset rather than deriving contact data from rendered meshes at runtime.
- Deliverable: reproducible physical package and loader.
- Acceptance: world coordinates match Godot; winding, negative scales, normals, triangle adjacency, and surface tags survive import; stale packages fail provenance validation.
- Validation: selected known track points, curbs, walls, seams, transforms, and material identities.
- Constraint: use tools/common/output_policy.py for scratch promotion and respect parent instructions for binary leaves.

### Item 26 — Define collision shapes for the articulated vehicle

- Priority: critical. Dependencies: 11, 12, 24.
- Surface: collision shape configuration and physical attachments.
- Work: define compound convex chassis/body shapes and wheel collision volumes; attach them to the solved component poses. Establish self-collision filtering and shape classification for tire tread, tire sidewall, floor, wings, and chassis.
- Deliverable: versioned vehicle collision specification and visual debug overlay.
- Acceptance: shapes remain aligned throughout steering/travel; contact volumes do not protrude unexpectedly; harmless suspension connections do not self-collide; safety margins are explicit.
- Validation: envelope sweeps, side impacts on a wheel, floor clearance, and compound-body contacts.
- Limitation: collision geometry is an approximation, not a deformable structural model.

### Item 27 — Implement world queries and candidate filtering

- Priority: critical. Dependencies: 21, 25, 26.
- Surface: Rust collision world and query interface.
- Work: implement spatial acceleration, broad-phase candidate selection, contact queries, road raycasts/sweeps, shape filtering, and moving shape updates. Preserve surface velocity and material lookup.
- Deliverable: shared geometric query path for contact detection and vehicle road sampling.
- Acceptance: headless and Godot runtime use identical physical geometry; fast-moving bounds are conservative; unsupported moving concave shapes fail explicitly; queries do not depend on render frames.
- Validation: transformed tracks, multiple vehicles, empty regions, surface boundaries, and geometric query parity.
- Performance evidence: report candidate counts and query costs before optimization.

### Item 28 — Maintain stable persistent contact manifolds

- Priority: critical. Dependencies: 27.
- Surface: narrow-phase contact representation and caches.
- Work: retain supporting contacts with stable feature identity, local anchors, normals, separation, material, and age. Reduce redundant contacts while preserving floor and wall support.
- Deliverable: contact manifold system with explicit invalidation.
- Acceptance: stable resting contact; no stale manifold across reset, geometry change, or entity reuse; normal orientation and contact point frames are consistent; triangle seams do not cause unexplained impulse spikes.
- Validation: resting compound body, corner contacts, mesh seams, slowly rotating shapes, and repeated entering/leaving contact.
- Diagnostics: retain contact count, lifetime, and invalidation reason.

### Item 29 — Solve normal impulses with the generalized vehicle matrix

- Priority: critical. Dependencies: 13, 14, 22, 28.
- Surface: coupled contact Jacobians and impulse solver.
- Work: derive contact velocity from the actual impacted component; solve simultaneous unilateral normal constraints using generalized effective mass. Define restitution thresholds, position correction, solver iteration budgets, and warm-start scaling.
- Deliverable: generalized normal response for vehicles and static world shapes.
- Acceptance: impulses influence body and internal coordinates coherently; separating contacts receive no attractive impulse; simultaneous contacts converge; restitution and stabilization energy are accounted for.
- Validation: analytical mass impacts, off-center strikes, suspension-free versus constrained wheel motion, and multiple supports.
- Constraint: the reference host velocity correction is not a substitute for computing the correct initial impulse.

### Item 30 — Solve tangential impact friction and rotational effects

- Priority: critical. Dependencies: 18, 19, 29.
- Surface: tangential contact Jacobians, material model, coupled solver.
- Work: solve tangential impulses with normal/friction coupling; include wheel-spin leverage where appropriate; define friction cone or validated approximation, stick/slip transitions, and material pairing.
- Deliverable: dissipative impact friction with rotational response.
- Acceptance: impulses satisfy the friction bound, oppose relative slip, and account for energy changes; warm starts are remapped correctly when normals rotate; tire traction is not applied again to the same classified contact.
- Validation: oblique wall impact, sliding floor, spinning wheel against a barrier, and static support with tangential loading.
- Gate: distinguish impact friction coefficients from brush-model tire grip parameters.

### Item 31 — Add continuous collision detection and event localization

- Priority: critical. Dependencies: 22, 27, 29.
- Surface: conservative motion bounds, shape casts, event subdivision.
- Work: detect time of impact for moving/rotating shapes and articulated wheel travel; advance to events, solve contact, and consume remaining time. Define event limits and safe failure behavior.
- Deliverable: continuous detection with complete elapsed-time accounting.
- Acceptance: no tunneling through supported thin barriers at declared speeds; rotational and suspension motion are included or their conservative approximation is demonstrated; step exhaustion is reported, not silently truncated.
- Validation: straight and oblique high-speed impacts, rotation near a wall, wheel travel during impact, and repeated collisions.
- Evidence: record impact time, subdivisions, and geometric misses.

### Item 32 — Classify contacts and prevent duplicate response

- Priority: critical. Dependencies: 16, 19, 30, 31.
- Surface: tire-road, tire-barrier, chassis-ground, and travel-limit dispatcher.
- Work: route road tread contact to compliant tire mechanics, hard body/barrier contact to impulse response, and travel stops to internal constraints. Define transitions on curbs, edges, sidewalls, jumps, and floor strikes.
- Deliverable: contact ownership state machine and diagnostics.
- Acceptance: every physical interaction has one normal response owner; curb transitions are continuous; a tire-road spring and rigid normal solver never independently support the same contact; side impacts still reach wheel spin and suspension.
- Validation: curb climb, curb edge, jump landing, wall scrape, wheel-side contact, and bottoming under downforce.
- Gate: force caps must not be used to conceal duplicated contact forces.

### Item 33 — Solve vehicle-to-vehicle contact in one world step

- Priority: critical. Dependencies: 21, 29, 30, 31, 32.
- Surface: world contact islands and multi-vehicle response.
- Work: construct coupled contact equations across both vehicle states; apply equal-and-opposite impulses at the same physical time. Use stable pair ordering and solve connected groups of simultaneous contacts.
- Deliverable: multi-vehicle collision response.
- Acceptance: total linear/angular momentum closes for isolated impacts; events reach both vehicles consistently; neither car receives a one-frame delayed reaction; entity order does not change results beyond the declared determinism contract.
- Validation: frontal, rear, oblique, wheel-to-wheel, stationary-car, and three-car impacts.
- Performance evidence: report contact-island size and iteration cost.

### Item 34 — Define dynamic world-object interoperability

- Priority: high. Dependencies: 25, 33.
- Surface: cones, movable barriers/objects, legacy vehicles, Godot adapter.
- Work: inventory objects the new vehicle can strike. Choose shared Rust dynamics, a rigorously synchronized adapter, or an explicitly unsupported interaction for each class. Keep static Godot scene geometry as authoring data when Rust owns its physical representation.
- Deliverable: world-object ownership map and supported interaction implementation.
- Acceptance: no object receives impulses from two solvers; coupled objects participate in the same impulse exchange; unsupported interactions fail or are disabled visibly.
- Validation: vehicle/cone, vehicle/movable body, static wall, and legacy-profile scene combinations.
- Gate: do not advertise full mixed-world collision fidelity while exchanging stale host velocities.

### Item 35 — Stabilize persistent contacts and rest states

- Priority: high. Dependencies: 29, 30, 32, 33, 34.
- Surface: solver warm starts, resting contact, optional sleep/wake policy.
- Work: stabilize stacks and resting vehicles; handle low-speed tire mechanics and suspension motion; wake on input, force, nearby collision, terrain motion, or configuration changes.
- Deliverable: stable resting behavior with explicit sleep eligibility.
- Acceptance: no artificial suspension freeze while mechanically active; no accumulation of penetration or energy; persistent contact remains valid after waking.
- Validation: stationary car, long wall contact, inclined surface, moving platform, and repeated sleep/wake transitions.
- Constraint: sleep is optional for release if an awake implementation meets the measured runtime budget; correctness is mandatory.

## Sprint 5: Integrate the Rust world with Godot

### Item 36 — Define a versioned native world interface

- Priority: critical. Dependencies: 21, 22, 33, 34, 35.
- Surface: game/crates/game-sim/src/c_abi.rs, native headers, adapter.
- Work: expose world creation, geometry loading, entity registration, timestamped inputs, fixed stepping, snapshot retrieval, event retrieval, and lifecycle commands. Define ownership, buffer lengths, numeric layout, units, thread access, and error results.
- Deliverable: versioned native interface with shared declarations and boundary validation.
- Acceptance: binary interface mismatches are rejected; Rust panics never unwind across the native boundary; invalid handles and buffer sizes fail safely; snapshots contain the full required state rather than only force/torque.
- Validation: layout checks, invalid input, repeated creation/destruction, and one complete headless world step.
- Constraint: reuse existing abbreviated interfaces only where compatibility requires them; new symbols use complete explanatory names.

### Item 37 — Replace the force-host loop with a single world step

- Priority: critical. Dependencies: 27, 36.
- Surface: native/src/core/f90_core.cpp, native/src/vehicle/f1_94_rust_vehicle.cpp, Godot runtime scheduling.
- Work: collect inputs once, step the Rust world once, and publish solved snapshots. Remove the new path's dependence on Godot collision-resolved body state, scalar host mass, host gravity, and host force integration.
- Deliverable: explicitly selected Rust-owned execution path.
- Acceptance: vehicle proxies receive no Godot physical impulses; gravity, fuel, integration, and friction occur once; Godot queries are not an unnoticed second contact source; multiple vehicles share a physical clock.
- Validation: compare one-car and multi-car state histories with the standalone world under identical inputs.
- Gate: custom_integrator alone is not an acceptable ownership switch.

### Item 38 — Implement lifecycle, reset, pause, and initialization

- Priority: critical. Dependencies: 10, 25, 36, 37.
- Surface: world lifecycle, race/session initialization, scene teardown.
- Work: initialize spawn poses, wheel travel, fuel, tire state, drivetrain state, contact caches, and time. Define reset, respawn, teleport, pause/resume, scene reload, and numerical-failure recovery.
- Deliverable: atomic lifecycle transitions.
- Acceptance: no stale impulses after teleport or reset; physical time does not advance during pause; initial overlaps receive the documented treatment; all native resources are released; reset returns a reproducible state.
- Validation: repeated session reload, reset during contact, pause on a slope, teleport across materials, and invalid configuration.
- Preserve: existing race/session semantics rather than inventing new respawn behavior.

### Item 39 — Render the solved mechanical state

- Priority: critical. Dependencies: 11, 12, 36, 37, 38.
- Surface: wheel visual controller, suspension geometry/link visuals, vehicle scenes, driver presentation.
- Work: interpolate authoritative body and component poses; display wheel steering/spin and actual damper/rocker motion. Replace independent visual travel estimation in the new path with solved geometry.
- Deliverable: snapshot-driven vehicle presentation and mechanical debug overlay.
- Acceptance: interpolation adds no physical feedback; wheels, brake assemblies, rods, and dampers remain attached; rear mount and stroke stay inside the validated envelopes; left/right poses match the physical solution.
- Validation: 30/60/120 frame-per-second rendering of identical physics, slow-motion full travel, rear orthographic views, and driver/wheel animation.
- Preserve: existing pit-stop visual scope; removing rim and tire must not inadvertently move static brake parts.

### Item 40 — Deliver telemetry and collision events to existing consumers

- Priority: high. Dependencies: 17, 32, 33, 36, 37.
- Surface: telemetry structures, collision audio producer, tire/surface effects, user interface.
- Work: expose operating mass, center, inertia summary, travel, damper stroke, contact owner, normal/tangential impulses, mounting loads, spin, energy ledger, and solver diagnostics. Replace host contact-monitor collision audio input with physical event data.
- Deliverable: versioned telemetry and physical-event adapters.
- Acceptance: units and signs are documented; event strength derives from solved impact quantities; subdivisions do not duplicate sound/effects; continuous scraping is distinct from discrete impacts; surface identities remain stable.
- Validation: wall strike, wheel strike, curb passage, long scrape, airborne state, and contact reset.
- Constraint: preserve existing assets and audio routing; this item does not redesign engine synthesis or sound banks.

### Item 41 — Preserve gameplay and configuration consumers

- Priority: high. Dependencies: 18, 19, 38, 39, 40.
- Surface: race state, checkpoints, camera, tire/brake systems, driving aids, pit stops, HUD consumers.
- Work: trace all consumers of host pose, velocities, raycasts, contact signals, and scalar mass; replace only the required source of truth. Preserve input, transmission, pit scheduling, fuel display, thermal updates, and control configuration semantics.
- Deliverable: consumer migration checklist and regression evidence.
- Acceptance: checkpoint and lap timing use physical state; cameras may interpolate presentation; gameplay actions do not restore host integration; thermal systems advance once; unsupported legacy dependencies are explicit.
- Validation: lap completion, reset, manual/automatic gearbox, pit scheduling, tire replacement, and vehicle profile selection.
- Gate: unrelated redesign must be proposed separately.

### Item 42 — Add explicit activation and compatibility gates

- Priority: critical. Dependencies: 10, 37, 38, 39, 40, 41.
- Surface: configuration loader, world initialization, runtime adapter.
- Work: define an explicit integration-owner selection and required physical-world package. Validate profile schema, native interface version, shapes, material coverage, and geometry hashes before spawning.
- Deliverable: controlled activation path and actionable initialization errors.
- Acceptance: unsupported configuration cannot silently fall back to a different physical model; legacy vehicles remain explicitly supported; release selection does not run both implementations; rollback requires a deliberate compatible route.
- Validation: missing inventory, stale geometry, mixed owner selection, unsupported dynamic objects, and old interface binaries.
- Gate: activation remains a reviewable candidate until the full validation and runtime gates pass.

## Sprint 6: Validate fidelity, robustness, and performance

### Item 43 — Execute the complete contact and impact matrix

- Priority: critical. Dependencies: 16, 23, 32, 33, 34, 35, 42.
- Surface: headless scenarios, generalized-contact tests, machine-readable reports.
- Work: implement the scenarios in Section 8 using analytical and independent numerical references where available. Run impact, rest, travel, tire-road transition, and multi-vehicle cases.
- Deliverable: numerical validation report including failures and baseline comparisons.
- Acceptance: every mandatory scenario passes its frozen criteria; artificial stabilization and restitution work are visible; known approximations remain labeled; no test merely duplicates the implementation algorithm.
- Validation: momentum, angular momentum, energy, penetration, contact classification, and timestep refinement.
- Gate: favorable driving impressions cannot replace numerical contact evidence.

### Item 44 — Validate replay and standalone/runtime parity

- Priority: critical. Dependencies: 36, 37, 38, 40, 42, 43.
- Surface: snapshot capture, deterministic inputs, replay diagnostics.
- Work: record world package, inventory, initial state, input timing, timestep sequence, build fingerprint, and event ordering. Compare standalone and Godot-driven runs and repeat runs on the same supported build.
- Deliverable: replay format and parity report.
- Acceptance: same-build repeat runs produce matching physical snapshots under the declared contract; render rate does not alter physics; event order and counts match; cross-platform guarantees are made only after dedicated proof.
- Validation: headless versus runtime, repeated reset, varied render rates, and entity insertion order.
- Limitation: using Rust or a deterministic dependency does not automatically make application inputs and mathematics deterministic.

### Item 45 — Quantify physical uncertainty and timestep sensitivity

- Priority: high. Dependencies: 07, 08, 43, 44.
- Surface: parameter sweep diagnostics and calibration report.
- Work: vary credible wheel, upright, link, body inertia, and fuel assumptions independently of timestep. Separate numerical convergence from uncertainty in fictional physical properties.
- Deliverable: sensitivity report showing which uncertain values dominate response.
- Acceptance: mass-property changes propagate to modes, loads, and impacts; published central results identify their assumptions; tire and suspension tuning do not conceal inventory errors.
- Validation: low/central/high inventory variants, multiple fuel loads, and coarse/medium/fine stepping.
- Decision: prioritize better component evidence where uncertainty dominates rather than adding solver complexity without benefit.

### Item 46 — Measure race cost and optimize without changing physics

- Priority: critical. Dependencies: 27, 28, 33, 35, 37, 43, 44.
- Surface: world stepping, matrix solves, geometry caches, contact islands, native transfer.
- Work: benchmark the designated target machine and the actual configured race grid. Record median, 95th and 99th percentile cost, spikes, contacts, iterations, subdivisions, allocations, and memory. Measure all race systems, not only the reference suspension.
- Deliverable: reproducible performance report and parity-preserving optimization diff.
- Acceptance: agreed race frame/physics budget is met; optimized results match the reference within frozen tolerance; no rounding of geometry or discarded substeps is introduced secretly.
- Validation: empty world, one car, full grid, racing traffic, and worst supported crash island.
- Gate: parallelism is introduced only when measured benefits exceed scheduling cost and replay remains valid; no automatic Rust-versus-C++ speed claim.

### Item 47 — Calibrate the complete physical vehicle

- Priority: high. Dependencies: 11, 17, 19, 20, 43, 44, 45, 46.
- Surface: physical inventory and vehicle suspension/tire configuration.
- Work: calibrate static ride height, wheel loads, spring preload, damping, anti-roll response, camber/toe curves, and aerodynamic platform across fuel states. Use the reviewed inventory rather than the five-component placeholder.
- Deliverable: calibration report with changed values, physical rationale, and comparison captures.
- Acceptance: mass accounting remains unchanged unless explicitly reviewed; damper/rod forces remain plausible under the chosen assumptions; no clipping at supported operating conditions; mode and transient targets are documented.
- Validation: no/initial/full fuel, heave, pitch, roll, braking, acceleration, cornering, and curb response.
- Human review: provide a concrete driving and visual candidate, listing remaining uncertainties rather than asserting real Formula One fidelity.

## Sprint 7: Build, review, and deliver

### Item 48 — Produce a clean source-matched runtime build

- Priority: critical. Dependencies: 42, 43, 44, 46, 47.
- Surface: canonical launcher, Cargo/SCons build pipeline, game/BUILD_SOURCE.
- Work: verify the actual reviewed source state; use the repository-prescribed clean build procedure for Cargo, SCons, and game/.godot. Build all required native components from this source and enforce runtime parity.
- Deliverable: build provenance and runtime parity evidence.
- Acceptance: no cross-branch cache or foreign binary is reused; all required components share compatible source/interface identity; launcher rejects mismatched builds.
- Validation: scripts/run_f1_94.ps1 through its current supported options; intentionally mismatched build rejection; fresh import/build and startup.
- Constraint: HEAD-only parity cannot prove a dirty-source build matches tracked source. Define and record the development source fingerprint; produce the release build against reviewed committed source when publication is authorized.

### Item 49 — Perform integrated driving and visual review

- Priority: critical. Dependencies: 39, 40, 41, 47, 48.
- Surface: canonical runtime, Fuji race scenario, captures and telemetry.
- Work: inspect static stance, rear inboard damper packaging, full travel, steering, braking, curb response, jumps, wall impacts, wheel impacts, and traffic. Review camera, audio events, thermal state, pit behavior, and race continuity.
- Deliverable: synchronized video/still captures and telemetry from the source-matched build.
- Acceptance: rear suspension presentation agrees with physical hardpoints; no visual/body collision mismatch or apparent floating damper; no duplicated sounds, jitter, lost race state, or regression in protected behavior.
- Validation: documented route and reproducible scenarios on the designated machine.
- Human approval: present this evidence and the remaining limitations. A passing headless smoke does not satisfy the driving review.

### Item 50 — Audit delivery and prepare atomic publication

- Priority: critical. Dependencies: 01 through 49.
- Surface: implementation diffs, status records, calibration, provenance, delivery notes.
- Work: reconcile every item's status; review dependency closure, tests, estimates, known defects, and build evidence. Prepare explicit source scopes separated by physical inventory, solver/contact work, bridge/presentation, and validation.
- Deliverable: review package, unresolved follow-on list, and atomic commit plan.
- Acceptance: no item is marked done solely because code exists; generated libraries, BUILD_SOURCE outputs, foreign changes, and unrelated imports stay outside source commits; intentional goldens change with their reviewed implementation.
- Validation: inspect explicit diffs and, only when committing is authorized, inspect git diff --cached before each atomic commit.
- Publication: commit, push, asset promotion, or deployment requires the authorization applicable to that action. This document does not itself request publication.
- Completion: human approval and all mandatory release gates are recorded; otherwise report the exact outstanding gate.

## 7. Sprint dependency and review map

| Sprint | Items | Required entry | Reviewable exit |
| --- | --- | --- | --- |
| 0 | 01–04 | Current checkout inventory | Ownership, architecture contract, validation budgets |
| 1 | 05–10 | Sprint 0 reviewed | Closed component ledger, tensors, canonical profile |
| 2 | 11–17 | Inventory available | Geometry envelopes, derivatives, coupled matrix, mounting loads |
| 3 | 18–23 | Coupled mechanics | Full force path, wheel rotation, authoritative world, reference benches |
| 4 | 24–35 | Valid pre-collision reference | Physical world, manifolds, impulses, friction, continuous detection, multi-car response |
| 5 | 36–42 | Supported collision world | Native interface, single owner, lifecycle, presentation, compatibility gates |
| 6 | 43–47 | Reviewable runtime candidate | Contact evidence, replay parity, uncertainty, race cost, calibration |
| 7 | 48–50 | Numerical and performance gates | Clean build, integrated driving review, delivery approval |

Dependencies are item-level correctness requirements, not permission to skip sprint review. Independent geometry preparation can be scheduled while other items progress, but production contact activation must respect the full dependency chain. Delegating to other agents requires the authorization specified by the active agent protocol.

## 8. Mandatory scenario matrix

Each scenario definition records source/build fingerprint, world package hash, physical inventory hash, start state, inputs, external work, material parameters, step sequence, and applicable tolerances.

| Scenario | Required observations | Acceptance basis |
| --- | --- | --- |
| Component accounting | Base, wheels, fuel, extracted moving components | Exact budget membership; no duplicate mass |
| Static equilibrium | Corner normal forces, body pose, travel, damper stroke | Support equals operating weight; pose within calibrated envelope |
| Whole-system freefall | Body/wheel velocities, relative travel, momentum | Correct gravity; no artificial suspension force from double gravity |
| Conservative heave | Frequency, mode shape, kinetic/potential energy | Analytical solution and timestep convergence |
| Damped heave | Energy decay, damper work, settling | Passive damping; no unexplained energy gain |
| Pitch and roll | Axle travel, anti-roll torque, component loads | Correct force/moment and mechanical-work balance |
| Steering while suspended | Component velocities, spin angular momentum, actuator work | Correct derivatives and accounted actuator work |
| One-wheel bump | Hub motion, body coupling, tire load, mounting loads | Coherent response without independent filter artifacts |
| Pure rolling and locked-wheel slide | Slip, wheel torque, tire power | Consistent contact motion and dissipative slip |
| Single wall impact | Impact time, impulse, rebound, energy | Analytical generalized effective mass; explicit restitution |
| Off-center body impact | Linear/angular velocities, contact work | Correct lever arm and momentum change |
| Wheel-to-barrier impact | Wheel travel/spin, body motion, reactions | Impulse reaches all mechanically coupled states |
| Floor bottoming | Multiple contact impulses, platform motion, energy | Stable support; no tire/floor duplicate response |
| Simultaneous travel stops | Coupled limit impulses and residuals | Joint solve; no order-dependent velocity resets |
| High-speed thin barrier | Swept motion, first contact time, subdivision count | No missed impact within declared speed/shape envelope |
| Rotating near-wall impact | Angular sweep, contact points | Rotation included in collision bounds/localization |
| Curb climb and edge | Contact classification, force continuity | One normal owner; no seam-driven load spikes |
| Jump and landing | Tire compression, damping, wheel travel, impact work | Correct compliant/rigid transition and bounded penetration |
| Long resting contact | Contact drift, load, energy, wake behavior | Stable support over long durations |
| Two-car impact | Total momentum, paired events, contact timing | Equal/opposite simultaneous response |
| Three-car contact island | Constraint residuals, ordering, runtime | Stable simultaneous solution within budget |
| Moving world object | Paired impulse and object state | Supported ownership contract; no split-time exchange |
| Fuel consumption/refueling | Mass, center, tensor, exchange ledger | One update; no unexplained state discontinuity |
| Reset during impact | Caches, events, physical state | No stale warm start or duplicate historical event |
| Render-frequency change | Authoritative snapshots and input times | Same physical history for the same physical inputs |
| Headless/runtime parity | State sequence, event sequence, energy | Same kernel, geometry, timing, and configuration |
| Full-grid traffic and crash | Percentile costs, memory, solver work | Agreed race budget on identified target hardware |
| Rear mechanical clearance | Finite-volume clearance, damper stroke | Supported poses remain within verified packaging bounds |

### Provisional development thresholds

Freeze scenario-specific budgets in Item 04 before tuning. The following starting values are proposals, not validated vehicle accuracy claims:

- Accounting closure: absolute error at most 0.000001 kilogram for declared component sums.
- Synthetic conservative analytical energy/frequency fixtures: relative error below 0.01 percent, retaining or strengthening existing meaningful reference checks.
- Analytical isolated collision momentum: relative error at most 0.000001, normalized against initial or impulse-scale momentum; specify an absolute floor near zero.
- Dissipative isolated contacts: final kinetic energy must not exceed initial energy plus declared external work and restitution allowance beyond the scenario's numeric budget.
- Rest equilibrium after settling: total support within 0.1 percent of weight, with pose and travel tolerances defined from geometry and calibration.
- Physical position convergence: at most 0.1 millimetre for selected smooth diagnostic windows; define separate discontinuous-impact comparisons using event time and impulse.
- Contact penetration: initial candidate limit of one millimetre for rigid chassis/world support; compliant tire compression is a physical state and must not be compared with that limit.
- Derivatives: demonstrate convergence across multiple perturbation sizes with dimension-aware absolute and relative budgets; do not prescribe one unscaled tolerance for metres and radians.
- Replay: exact repeated state equality only for the supported same-build contract; cross-platform or parallel equality requires additional evidence.
- Performance: measure the actual configured grid on identified hardware and agree a percentile-based budget before release. No universal per-car threshold can be asserted from the suspension-only reference timing.

If a threshold is unattainable, diagnose model, geometry, conditioning, and integration before proposing a reviewed adjustment. Keep the earlier 10 millisecond reference tolerance separate from a complete-race release claim.

## 9. Evidence and artifact contracts

Each implementation item stores:

1. Item number, title, status, dependency state, selected sprint, reviewer, and approval state.
2. Source branch, HEAD, dirty-source fingerprint where applicable, explicit change ownership, and toolchain versions.
3. Vehicle configuration, component inventory, world geometry, scenario, and dependency lockfile hashes.
4. Reproducible command and environment, numeric inputs, units, outputs, and exact pass conditions.
5. Expected-versus-observed results, errors, energy/momentum ledgers, and failure classification.
6. Runtime/build identity for any in-game claim.
7. Remaining assumptions, estimated data, unsupported shapes/interactions, and follow-on requirements.

Use machine-readable JSON results with complete explanatory field names. Proposed artifact filenames should use full words, such as physical_mass_inventory.json, collision_world_manifest.json, coupled_vehicle_validation_report.json, and vehicle_dynamics_provenance.json. These are proposed interfaces, not files already implemented.

Track accounting, physical estimation, numerical verification, calibration, visual review, and human approval separately. An accounting pass never upgrades estimated mass to measured mass. A synthetic analytical test never proves real tire fidelity.

### Status vocabulary

- pending: implementation has not met its prerequisites.
- reference_available: reusable reference code exists but production acceptance remains open.
- implementing: scoped changes are in progress.
- implemented_awaiting_review: acceptance evidence exists but review is incomplete.
- validated_awaiting_human_approval: required technical gates pass and concrete evidence is ready.
- done: technical review and required human approval are complete.
- blocked: record the specific unresolved external dependency or requirement; do not use difficulty as the reason.
- deferred_follow_on: explicitly outside this delivery; never substitute this for an incomplete required item.

## 10. Research evidence and interpretation limits

The following sources informed the preceding decision. Preserve document versions and component perimeters when importing numeric data. Their publication and access context belongs to the research baseline; recheck evolving dependency documentation before implementation.

### Mass references

- [FIA 2006 technical regulations](https://argent.fia.com/web/fia-public.nsf/035E7BF2DE8E684CC12573290033747B/$FILE/07F1_TECHNICAL_REGULATIONS.pdf?Openelement=): historical complete-vehicle minimum of 600 kilograms, 605 in qualifying, including the driver. This does not define the fictional 600 kilogram wheel-excluded budget.
- [FIA 2026 technical regulations, Issue 20, 05 August 2026](https://www.fia.com/system/files/documents/fia_2026_f1_regulations_-_section_c_technical_-_iss_20_-_2026-08-05.pdf): minimum mass uses 724/726 kilograms plus nominal tire mass; car mass excludes fuel and includes driver/ballast. Driver reference plus ballast is at least 82 kilograms. Tire-only nominal mass is not complete wheel-assembly mass.
- [Mercedes W17 technical specifications](https://www.mercedesamgf1.com/f1-w17-2026-technical-specifications): published overall mass 772 kilograms and hybrid power-unit minimum 185 kilograms. Its component perimeter differs from an atmospheric V10.
- [Formula 2 car and engine](https://www.fiaformula2.com/en/latest/article/the-car-and-engine-f2.14LCsEEMG9yyx5DkhcN1J8): published 795 kilograms with driver; the page does not establish the fictional vehicle's fuel-free component ledger.
- [Honda RA005E](https://global.honda/en/POWEREDbyHONDA/2005_ra005e/): historical V10 engine mass 88.6 kilograms. This supports an order-of-magnitude comparison, not an exact 3.5 litre engine allocation.

The recommended approximately 700 kilogram complete unfueled operating mass is an engineering design target. The exact 698 kilogram figure comes from current declarations, not independent physical measurement. The 600 kilogram budget table must remain estimated until Item 07 produces stronger evidence.

### Simulation and collision references

- [iRacing: The Road To New Damage](https://www.iracing.com/road-new-damage/): a historical developer description of proprietary multibody dynamics, persistent contact sets, collision geometry, and removable mass/inertia. It illustrates integration requirements and does not establish that iRacing uses Rust.
- [BeamNG JBeam introduction](https://documentation.beamng.com/modding/vehicle/intro_jbeam/): node-and-beam vehicle modeling.
- [BeamNG.tech tight coupling](https://documentation.beamng.com/beamng_tech/tools/cosimulationeditor/tight_coupling/): physics separated from graphics, with a standard 0.5 millisecond physical step. This is not a requirement to run Formula90s at 2000 hertz.
- [PhysX 5.4.1 Vehicles](https://nvidia-omniverse.github.io/PhysX/physx/5.4.1/docs/Vehicles.html): modular force computation and scene-owned rigid contacts demonstrate that partitioned architectures can be valid when their representation and synchronization are defined.
- [Project Chrono vehicle cosimulation](https://api.projectchrono.org/9.0.0/group__vehicle__cosim.html): explicit exchange of body state and forces among vehicle, tires, and terrain. Separation requires consistent synchronization.
- [Godot RigidBody3D custom integrator](https://docs.godotengine.org/en/stable/classes/class_rigidbody3d.html#class-rigidbody3d-property-custom-integrator): custom force integration retains collision response.
- [Parry geometric queries](https://parry.rs/docs/user_guide/geometric_queries/): contact queries and geometric sweeps; generalized impulse response remains a separate responsibility.
- [Rapier determinism](https://rapier.rs/docs/user_guides/templates/determinism/): deterministic behavior has initialization, timestep, platform, and math conditions; Rust alone provides no guarantee.

Rust ownership is selected to make the vehicle's physical state, mass matrix, force evaluation, and impact response consistent and testable. No automatic accuracy, frame-rate, cross-platform replay, or library compatibility guarantee is implied.

## 11. Delivery completion checklist

- [ ] Current source, protected changes, and ownership are inventoried.
- [ ] Component ledger closes; every estimate retains its provenance and uncertainty.
- [ ] Centers and tensors describe physical components and avoid rotating-inertia duplication.
- [ ] Actual rear suspension packaging is validated over the complete supported domain.
- [ ] Chassis, travel, wheel rotation, and drivetrain exchanges are coupled consistently.
- [ ] Tire, suspension, aerodynamic, gravity, fuel, and contact contributions have one owner each.
- [ ] Physical world geometry and materials have reproducible source hashes.
- [ ] Generalized normal/friction impulses, travel events, and continuous detection pass their scenarios.
- [ ] Vehicle-to-vehicle contacts and supported dynamic objects share a coherent impulse exchange.
- [ ] Godot vehicle proxies do not receive duplicate integration or contact response.
- [ ] Resets, pause, lifecycle, presentation, telemetry, audio events, and gameplay consumers pass review.
- [ ] Headless/runtime parity and the declared replay contract are demonstrated.
- [ ] Numerical convergence and physical uncertainty are reported separately.
- [ ] Complete-race percentile costs pass the agreed hardware/grid budget.
- [ ] Calibration uses the reviewed component inventory and respects geometry.
- [ ] Clean build and launcher parity checks match the reviewed source.
- [ ] Integrated driving and rear suspension visual evidence are reviewed.
- [ ] Every required item has its technical evidence and human approval.
- [ ] Publication scope excludes generated, unrelated, and foreign changes.
- [ ] Remaining optional follow-on features are listed without being called implemented.

## 12. First executable continuation

Begin with Item 01, then reproduce Item 02 and finalize Items 03–04. Implement the complete mass inventory before changing collision ownership in the live vehicle. Progress through the dependency map, keeping the Rust world headlessly executable until contact correctness is demonstrated.

Do not start with a visual-only rear suspension adjustment, a scalar host-mass change, or enabling custom_integrator. None establishes the proposed coupled architecture. The first review package must make the physical budget, reusable reference capabilities, supported collision world, and acceptance measurements concrete.
