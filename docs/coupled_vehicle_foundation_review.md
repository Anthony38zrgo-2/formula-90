# Coupled Vehicle Foundation: Sprint 0 Review

## Delivery and authority

Sprint 0 implements Items 01–04 of `instrucciones.md`. The deliverables are the provenance inventory, reference review and reproducible runner, architecture contract, diagnostic fixtures, and acceptance specification. These items establish the foundation for subsequent production implementation.

Delivery state: **implemented with baseline findings, awaiting human review**. The geometric regression gate fails. Do not mark the sprint approved, all tests passing, or the runtime validated. Record the failures before starting profile migration in Item 10. Later items retain their own implementation and acceptance gates.

The existing 30-item suspension backlog is historical. Its numbering differs from the authoritative 50-item backlog in `instrucciones.md`; do not merge their identifiers.

## Item 01: provenance and ownership

- Repository: `D:\Formula90s`.
- Branch: `main-clean`.
- Source HEAD: `001cfc9e6ba0f77a9a9fd0759e82a995228021a9`.
- Original inventory: `scratch/coupled_vehicle_foundation_20261005_233057/pre_edit_provenance.json`.
- Final execution provenance: `scratch/coupled_vehicle_foundation_20261005_final/execution_provenance.json`.
- Review report: `scratch/coupled_vehicle_foundation_20261005_final/foundation_validation_report.json`.
- Fresh build outputs: the report directory's `fresh_cargo_output`, with locked offline Cargo resolution.
- Git index, branch, HEAD, installed binaries, BUILD_SOURCE and protected pre-existing files must survive unchanged. The runner verifies their hashes.

The checkout remains in place. No branch switching, reset, cache cleaning, runtime binary promotion, staging, commit, or publication is required for this sprint. BUILD_SOURCE matching HEAD does not prove binary parity with dirty source; installed libraries are not executed.

| Existing surface | Classification | Action in this sprint |
| --- | --- | --- |
| Physics library/configuration, fuel tests and geometric tests | Pre-existing tracked source changes | Read and reproduce; preserve bytes |
| Mass properties, mass audit, component kinematics, multibody solver, coupled solver, examples and their tests | Pre-existing untracked reference source | Review as reusable reference; preserve bytes |
| Geometric vehicle profile and visual preview work | Pre-existing configuration and tooling | Audit actual profile; preserve bytes |
| Installed DLLs and BUILD_SOURCE | Pre-existing generated artifacts | Hash only; preserve; do not claim source parity |
| Font/model/HUD imports and pit crew source | Foreign pre-existing asset work | Inventory and preserve |
| Historical suspension backlog and implementation notes | Selected documentation | Resolve obsolete clarification flags; retain historical scope |
| Root backlog | Selected documentation | Add execution status; retain all future items |
| New contracts, fixtures, review, status and runner | Current sprint | Review explicit paths separately from existing work |

## Item 02: capability review

| Capability | Evidence | Reuse boundary |
| --- | --- | --- |
| Base/wheel/fuel accounting | Four mass audit and eight fuel tests; actual profile audit | Accounting is reproducible; physical component inventory remains unresolved |
| Full component inertia composition | Multibody tests: tensor validity, parallel-axis assembly, kinetic energy and matrix symmetry | Reuse kernel after complete component ledger validation |
| Component trajectories and derivatives | Actual hardpoint derivative tests, supported links, immutable configuration cache | Extend envelope checks and moving damper/driveshaft representation |
| Coupled gravity and inertial terms | Freefall, internal momentum balance and analytical fixtures | Reference ten-coordinate system, not complete rotating vehicle |
| Spring/damper/antiroll/stop projection | Work/energy tests and nonlinear captures | Event-localized position/impact solution remains pending |
| Planar unilateral normal contact | Separation/passivity tests and settling captures | No general collision detection, contact islands or integrated brush friction |
| Fuel exchange | Point mass/inertia update and exchange ledger tests | No exhaust momentum model; runtime connection pending |
| Host collision velocity correction | Generalized matrix correction tests | Cannot repair the host's initial impulse computed using a scalar mass |
| Linear heave frequency and conservative energy | Analytical test with velocity-Verlet stepping | Does not validate the runner's semi-implicit integrator |
| Exact geometry caching | Equality test with immutable configuration borrowing | No rounded state keys; cache cannot cross configuration revisions |
| Runtime coupling and world collisions | Bridge source trace only | Not implemented by this sprint |

### Reproduced measurements

The required foundation suites contain **28 passing tests, zero failures**. The separate geometric suite contains **five passes and two failures**. The report preserves process exit codes and failure names. Both failures occur in pre-existing source/configuration; no physics/profile file is changed by Sprint 0.

Actual-profile mass: 698 kg without fuel; 705.6 kg at initial fuel. Physical suspended mass is intentionally absent from the audit until the moving-component inventory closes.

| Diagnostic | Measurement | Frozen development gate |
| --- | --- | --- |
| 0.3 second settling | Approximately 0.021594 J residual; relative error 0.0000120821 | Relative residual <= 0.0001 |
| 2 second settling | Approximately 0.024497 J residual; relative error 0.0000137061 | Relative residual <= 0.0001 |
| 2 second support | 6919.5633 N against 6919.5722 N; relative error 0.00000129744 | Relative support error <= 0.001 |
| 0.01 second nonlinear refinement | Body position difference approximately 0.0000118077 m | Difference < 0.0001 m |
| Fine nonlinear energy | Relative residual approximately 0.00236590 | Relative residual < 0.015 |

The JSON report is authoritative for exact measurements and execution time. Five-component fixture masses/inertias are diagnostic estimates. Timings exclude the other race systems and cannot establish a production grid budget.

### Geometric regression findings

1. `godot_scene_points_at_geometric_by_default`: the F1 2030 model manifest still names `f1_2030_v10_physics.json`. The test expects `f1_2030_v10_geometric.json`. This previously known canonical-profile defect remains assigned to Item 10.
2. `ab_legacy_vs_geometric_static_matches_dynamic_differs_sanely`: newly reproduced baseline failure at the front-left static assertion. It computes each profile's road height separately, then compares both resulting forces to **the legacy profile's load**. The geometric profile now uses a different complete mass contract, so the claimed same-car fixture is stale. Item 10 must establish a controlled same-mass comparison or validate separate equilibria before comparing dynamics. Do not loosen the 5% load threshold to hide this failure. The geometric profile's own equilibrium test passes.

The acceptance specification continues to allow only the previously known manifest failure. Therefore the runner deliberately returns a failed geometric gate for the newly discovered baseline failure. This is evidence for review, not an all-green certificate.

### Defects and limits to carry forward

- `suspension_mass_properties.rs` accepts closure tolerance `1e-8 * total_mass`; at 698 kg this is 0.00000698 kg, looser than the new absolute accounting budget of 0.000001 kg. Reconcile the production inventory validator in Items 05–06. Exact foundation sums already meet the stricter accounting budget.
- At a travel limit, `suspension_coupled_solver.rs` clamps the position and projects velocities using the earlier evaluation's matrix. Configuration-dependent inertia and potential energy require a consistent event/position solve in Item 16. Smooth-motion energy evidence does not approve these impacts.
- The analytical heave fixture and nonlinear runner use different integrators. Maintain separate evidence and tolerance interpretation.
- Existing component kinds do not complete moving damper, driveshaft or wheel-spin dynamics. Extend the physical inventory and coordinate model before activation.
- The visual `SpinVisual` contains tire, rim, hub and brake disc. A merged visual mesh is not the rim/tire mass boundary. Partition component masses explicitly to avoid counting hubs and brakes twice.
- Current tangential contact force is an external reference input. Hub velocity, wheel spin, brush friction and drivetrain coupling remain later work.
- Numerical derivative scales must be validated across travel, steering and physical speed ranges; current tests are limited reference cases.
- Fuel update preserves velocity and records exchange under a point-mass approximation. Production fluid placement and slosh are not proven by this result.

## Item 03: selected architecture and traces

`coupled_vehicle_architecture_contract.json` freezes the implementation direction already proposed in the root backlog. It records current and target ownership separately; selection does not activate the target.

### Mass contract

The 600 kg aggregate includes the driver and all non-wheel, non-fuel vehicle systems. Moving brakes, hubs, uprights and suspension components must be **extracted from this aggregate**, not added on top of it. The configured front/rear rim-and-tire assemblies remain provisional 21/28 kg each. Thus:

`600 + 2 * 21 + 2 * 28 = 698 kg`

`698 + 7.6 = 705.6 kg` at initial fuel; `698 + 110 = 808 kg` at tank capacity.

The allocation table is an engineering estimate, not measured fictional-vehicle data. Non-fuel operating fluids remain inside the base budget. The contact filter's virtual mass is not a physical unsprung mass allocation. Legacy profiles without the exclusion declaration retain their established mass interpretation.

### Ownership

Rust owns the target authoritative body/wheel state, physical properties, force evaluation, integration, collision impulses and event production. Godot owns authoring, input/lifecycle requests, interpolation and presentation. Evaluate Parry for detection in Item 24; a geometry library does not supply the required generalized response automatically. Godot `custom_integrator` alone does not disable its collision response.

Static track geometry requires a versioned Rust package. Rust vehicles share the world clock and contact-island solve. Movable scene objects and legacy Godot vehicles require validated interoperability before mixed physical interactions are enabled.

| Operation | Current source trace | Target invariant |
| --- | --- | --- |
| Physical step | `native/src/core/f90_core.cpp` reads host body state and rays; `formula90-core/src/lib.rs` calls `solve_external_with_aero`; native bridge applies central force and torque | One Rust world step assembles mass, forces, contacts and integration; Godot consumes snapshots |
| Collision | Godot resolves body contacts; native contact monitor produces presentation events; reference correction accepts a supplied body velocity change | Generalized contact solve computes impulses and all body/wheel changes once; events derive from that solve |
| Fuel | Core entity forwards fuel service to `VehicleSim`; configuration supplies current mass; reference solver separately updates tank point mass | Consume fuel once per physical interval and update mass/inertia once; explicit exchange ledger |
| Reset | `Formula90Core::reset` refills configured fuel, rebuilds simulation state and resets modules; current reference is separate | Atomic reset of pose, clock, travel, spin, tire/drivetrain, fuel and contact history; invalidate caches |

The game-sim external C interface provides an analogous external solve route. It must be audited alongside the active core/native adapter during later migration; changing only one entry point cannot establish runtime ownership.

## Item 04: acceptance specification

`coupled_vehicle_validation_specification.json` is the machine-readable development specification. It records units, normalization floors, supported fixture windows, deterministic input policy, reference capability limits and future measurements. The runner consumes the foundation thresholds without altering production configuration.

- Mass accounting: absolute error <= 0.000001 kg; precision is accounting precision, not component measurement accuracy.
- Weight accounting: relative error <= 1e-10 with a 1 N floor.
- Synthetic heave frequency/energy: relative error <= 0.0001 in its analytical fixture.
- Estimated settling: energy residual <= 0.0001; support error <= 0.001.
- Short smooth convergence: position difference < 0.0001 m; fine energy residual < 0.015.
- Generalized impact momentum: planned relative error <= 0.000001 with a scenario-specific dimensional floor and declared external exchanges.
- Rigid penetration: provisional <= 0.001 m, excluding tire compression.
- Derivatives: separate translational/angular finite-difference convergence, with domain-dependent scales frozen before Item 13 acceptance.
- Replay: identical same-build authoritative state/event order for identical inputs and step sequence; no cross-platform exactness claim.
- Complete race cost: identified hardware, full grid and selected frequency; 99th percentile cost must fit the physical interval without discarded time. Absolute production frequency and nonphysics budget require Item 46 measurements.

Contact passivity and impact convergence require frozen material/event scales before collision activation. These future conditions specify what must be measured; they do not claim calibrated release thresholds or successful collision validation today.

## Reproduction and next gate

Use a new scratch directory every execution:

```powershell
python tools/physics_diagnostics/run_coupled_vehicle_foundation_review.py --output-directory scratch/coupled_vehicle_foundation_new_run
```

The runner refuses an existing Cargo output/report, records source/profile/fixture hashes and commands, writes only validated scratch destinations, and never patches vehicle data. It requires the local Cargo lockfile dependencies and the recorded toolchain. The versionable diagnostic fixture candidates live in `docs/`; no historical scratch inputs are required.

Review the baseline findings and contract before declaring Sprint 0 done. Sprint 1 proceeds with Items 05–10: complete physical inventory, centers/tensors, mass closure and canonical-profile validation. No full vehicle physics or collision runtime approval follows from the foundation tests.
