# Suspension coupled dynamics implementation

## Source provenance

Implementation started on 2026-10-05 in `main-clean`, source HEAD
`001cfc9e6ba0f77a9a9fd0759e82a995228021a9`.
The checkout already contained generated libraries, BUILD_SOURCE changes,
rear suspension geometry and regression changes, preview changes and untracked
imports. These changes were inventoried before editing and preserved.
No branch switch, staging, commit or runtime binary promotion was performed.

## Selected backlog item

Sprint 1, item 01: audit mass, gravity and integration ownership.

The executable `suspension_mass_audit` loads an actual vehicle profile through
the runtime configuration parser. It reports corner masses and static normal
forces using the same functions as the runtime, and rejects invalid masses
before wheel tuning clamps can conceal them.

Current F1 2030 profile:

| Quantity | Value |
| --- | ---: |
| Configured body and driver mass, excluding wheels and fuel | 600 kg |
| Complete dry mass with configured wheels | 698 kg |
| Current fuel | 7.6 kg |
| Total rigid body mass | 705.6 kg |
| Configured rotating assemblies, four wheels | 98 kg |
| Virtual contact filter masses, four corners | 132.3 kg |
| Physical suspended mass | Unresolved |

The human clarified that 600 kg includes body and driver, excluding wheels and
fuel. The active profile now declares `vehicle_mass_excludes_wheel_assemblies`.
The runtime adds the four configured wheel masses once, and weights the axle
distribution by those masses and fuel location. Other profiles default to the
previous complete dry mass contract. Serialization preserves the new contract.
The 132.3 kg filter estimate must not be subtracted from dry mass as a physical
inventory. Brake, upright and link inclusion still requires a component inventory;
698 kg is the closed budget for the currently declared components.

Spring free lengths were adjusted from 0.29504772 to 0.3005 m at the front and
from 0.272741884 to 0.27668 m at the rear. The additional preload compensates for
the added wheel weight using spring rate and rest motion ratio. The existing
static stance test passes without relaxing its 5 percent force and 10 mm position
tolerances. This is a static calibration; dynamic calibration remains pending.

## Integration ownership audit

`VehicleConfig::total_vehicle_mass` adds current fuel once to complete dry mass.
`mass_over_wheel` partitions that total using fuel-adjusted front distribution.
`WheelMechanicalTuning::for_wheel` estimates filter mass from rotating wheel
mass with a factor of 1.35.

`suspension.rs` integrates relative wheel travel with tire and suspension forces
and deliberately omits filter gravity. `simulation.rs::integrate_standalone`
integrates the total rigid body and supplies its gravity. In the external path,
`F90Core::drive_integrate` obtains Godot body state and applies Rust force and
torque; Godot supplies body integration and gravity.

A physical coupled mass matrix cannot be enabled by simply changing the Godot
scalar mass. The coupled inertia blocks, collision impulse response and gravity
ownership must be handled by the selected architecture in sprint 4.

## Validation

Four mass audit tests cover weight balance, invalid inputs, body mass plus wheels,
axle moments, serialization and backward compatibility. The eight fuel model
tests cover consumption, mass and reset behavior. The static stance test passes.
The executable was run against the active geometric profile. This verifies the
current mass contract, not coupled dynamics or in-game behavior.

## Remaining sprint 1 gates

Item 02: inventory measured, documented or explicitly estimated component mass,
center of mass and inertia, including brakes, uprights and links.

Item 03: body and driver definition resolved; declared body, wheels and fuel
budget implemented. Close the detailed physical budget after component inventory.

Item 04: capture controlled static, freefall, heave and one-wheel baseline cases.

Item 05: agree error budgets for mass closure, energy drift, timestep convergence,
kinematic derivatives and runtime cost before calibrating the new solver.

The audit returns no complete physical suspended mass until the brake, upright
and link inventory is resolved. The mass contract change is implemented in Rust;
Godot runtime binaries have not been rebuilt or validated. The pre-existing asset
manifest profile mismatch still fails its dedicated geometric profile test.

## Reproduction

```powershell
cargo test --manifest-path game/crates/vehicle-physics-engine/Cargo.toml --test suspension_mass_audit_test
cargo run --manifest-path game/crates/vehicle-physics-engine/Cargo.toml --example suspension_mass_audit -- game/data/vehicles/f1_2030/f1_2030_v10_geometric.json
```

## Coupled reference implementation, 2026-10-05

The following source modules implement an independent Rust reference model.
They are exported by the physics crate but are not called by the existing
`VehicleSimulator`, Godot bridge, tire friction solver or visual controller.
No coupled model has been activated in the production vehicle profile.

| Module | Responsibility |
| --- | --- |
| suspension_mass_properties.rs | Versioned component inventory, independent provenance for mass, center and inertia, physical tensor validation and parallel axis composition |
| suspension_component_kinematics.rs | Body, upright, wishbone, track rod, actuation rod and rocker trajectories; spatial velocity Jacobians and acceleration biases; prescribed rack velocity and acceleration |
| suspension_multibody.rs | Joint ten-coordinate mass matrix, gravity, Coriolis and gyroscopic terms, scaled positive definite solve, body collision velocity correction and simultaneous travel stop impulses |
| suspension_coupled_solver.rs | Nonlinear spring motion ratio, asymmetric digressive dampers, conservative antiroll coupling, travel/stroke limits, unilateral planar tire contacts, coupled stepping, fuel exchange and energy/momentum diagnostics |
| examples/coupled_suspension_reference.rs | Profile plus explicit mass definition and scenario loader; headless JSON telemetry and execution timing |

Velocity ordering is world body translation, world body angular velocity, then
four relative wheel travel rates in FrontLeft, FrontRight, RearLeft, RearRight
order. Body orientation uses a normalized quaternion. All ten accelerations are
solved together. The matrix is assembled as a sum of translational and rotational
component kinetic energies. No virtual mass multiplier enters that matrix.

Prescribed steering velocities are included in physical component velocities.
Their inertial and elastic actuator work is reported separately. The homogeneous
matrix energy equals summed component kinetic energy when prescribed velocities
are zero; the diagnostics use full component energy when steering is moving.

Fuel is represented as mass at the tank location. Its translational Jacobian adds
the correct parallel axis contribution without fabricating local tank inertia.
The fuel exchange operation retains velocities and reports the energy and
momentum brought in or removed with mass moving at the tank velocity. Slosh and
fuel exhaust momentum are outside this reference model.

Contacts use current hub velocity relative to the moving surface. Normal force
is zero when separated and cannot pull the tire toward the surface. Tangential
forces are explicit inputs; the existing tire brush model and wheel spin dynamics
are not integrated into this reference runner. Upright rotational inertia is
included; rotating wheel spin and its drivetrain coupling remain separate work.

The integrator is deterministic semi-implicit Euler with an explicit maximum
substep. It does not silently truncate substeps. Simultaneous travel stop velocity
constraints use the full matrix rather than resetting only the wheel velocity.
Impulsive kinetic energy loss is reported. Position projection at hard limits
still introduces integration error; a full event-based impact/position solve
has not been implemented. Energy claims below concern the tested smooth motion.

Exact geometric solutions are shared inside each evaluation, without numerical
rounding or tables. Cache lifetime borrows one immutable vehicle configuration,
preventing reuse against changed geometry. Constant antiroll rest stiffness and
travel envelopes are prepared once. The rest tangent includes the preload times
motion-ratio derivative term.

## Reference validation and interpretation

Sixteen multibody tests pass. They cover matrix symmetry and kinetic energy,
freefall, internal force momentum balance, parallel axis composition, physical
tensor rejection, inventory closure, actual-hardpoint derivative sensitivity,
all supported link trajectories, unilateral contact, fuel mass exchange, body
collision correction, simultaneous stop impulses, an analytic symmetric heave
frequency, timestep refinement, damper work, steering actuator work and exact
cache parity. The existing four mass audit and eight fuel tests also pass.

The analytic frequency and conservative energy errors are required below 0.01
percent in the synthetic linear case. The nonlinear ten-millisecond reference
case requires energy balance error below 1.5 percent and coarse/fine body position
difference below 0.1 mm. These are implemented development gates, not approved
full-car accuracy targets or calibration evidence.

An explicit estimated five-component definition and settling scenario are in
`scratch/coupled_suspension_review_20261005/`. They use the declared body/wheel
masses, an estimated rectangular body inertia and thin-ring wheel tensors.
They omit separately inventoried brakes, uprights and links, and are labeled
diagnostic-only. They must not be promoted as measured physical properties.
The measured reference result for 0.3 seconds of settling has approximately
0.0216 J residual in energy plus accumulated dissipation, against about
1787.3 J initial total energy. This demonstrates the reference calculation,
not a validated vehicle tune or real-time driving performance.

The extended two-second estimated settling case reaches 6919.5633 N total
normal force versus 6919.5722 N weight, a relative equilibrium error of about
0.00013 percent. Its accumulated energy residual is 0.0245 J. The final release
capture takes about 1.25 seconds for two simulated seconds on this machine;
the shorter 0.3-second capture takes about 0.188 seconds after exact caching.
These timings exclude other vehicle systems and are not a full race budget.

The release runner uses a fresh isolated Cargo output directory under scratch;
its execution time is in the JSON report. BUILD_SOURCE and installed runtime
libraries are untouched. No Godot run, collision parity, human driving review,
commit or publication has occurred for this reference implementation.

```powershell
cargo test --manifest-path game/crates/vehicle-physics-engine/Cargo.toml --test suspension_multibody_test --test suspension_mass_audit_test --test fuel_model_test
cargo run --release --target-dir scratch/coupled_suspension_review_20261005/reference_release_build --manifest-path game/crates/vehicle-physics-engine/Cargo.toml --example coupled_suspension_reference -- game/data/vehicles/f1_2030/f1_2030_v10_geometric.json scratch/coupled_suspension_review_20261005/estimated_reference_definition.json scratch/coupled_suspension_review_20261005/settling_scenario.json
```

## Selected integration boundary; runtime implementation pending

The effective collision mass of the body is the Schur complement of the four
wheel coordinates. The analytic test gives 700 kg in horizontal directions but
600 kg vertically when the wheels are free to move. One scalar body mass cannot
represent this response. The current native bridge applies forces and torque to
a Godot rigid body; it does not supply a generalized collision inverse mass.

`apply_host_collision_velocity_change` consistently updates relative wheel
velocities after a given body velocity change. It does not correct a host's
initial collision impulse or make Godot collision response exact. Selecting an
iterative Godot partition requires an explicitly accepted impact approximation;
owning collision response in Rust affects the collision integration architecture.
The authoritative backlog now selects Rust-owned integration and collision
response. Sprint 0 records that direction in
`docs/coupled_vehicle_architecture_contract.json`. Runtime activation remains
pending. The historical item numbers in this note do not match the new backlog.

The selected contract includes engine, transmission, brakes, uprights and
suspension links inside the 600 kg vehicle-and-driver budget. The provisional
21/28 kg assemblies mean rim plus tire. Moving components are extracted from
the aggregate without increasing the complete 698 kg unfueled mass. Full
component calibration and coupled runtime activation remain later work.

## Sprint 0 reproduction findings

See `docs/coupled_vehicle_foundation_review.md` for the reproducible 28 passing
foundation tests and the separate geometric suite's five passes/two failures.
The manifest linkage and stale same-mass comparison remain unresolved baseline
findings. The analytical heave fixture uses velocity Verlet; its conservative
energy result must not be attributed to the nonlinear semi-implicit runner.
