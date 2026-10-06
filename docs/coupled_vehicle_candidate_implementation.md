# Coupled vehicle implementation: candidate engineering review

## Delivery state

The authorized scope is all 50 items in `instrucciones.md`. This scope remains unfinished. The current changes implement a substantial standalone Rust candidate; they do not deliver the Godot ownership migration or a release-ready collision system. `docs/coupled_vehicle_backlog_execution_status.json` records each item's remaining acceptance work.

Continue the repository pipeline: select the next dependency-ready item, implement it, review concrete evidence, obtain the required human review, and only then mark it done. Candidate source is not an approved sprint or a validated runtime.

## Source ownership and preservation

The continuation started on `main-clean`, HEAD `001cfc9e6ba0f77a9a9fd0759e82a995228021a9`, with an empty index and existing changes. The original inventory, hashes and source copies are in `scratch/coupled_vehicle_complete_implementation/pre_edit_provenance.json` and `protected_source_snapshot/`. No branch switch, reset, staging, commit or publication was performed. Existing installed DLLs and `game/BUILD_SOURCE` were not replaced.

All new Rust compilation started in task-owned scratch directories. A debug build consumed 3.3 GiB and exhausted drive D; Cargo subsequently cleaned only that task's debug profile. The reproducible candidate review uses one compilation job, no debug symbols and an unused build directory. It preserves failed tests and returns failure when a gate fails.

## Mass perimeter and estimates

The geometric profile contains a version 2 ledger of 80 physical components. Its base membership closes to 600 kilograms and includes the pilot, structures, engine, transmission, empty fuel system, fluids, brakes, hubs, uprights, links, dampers and driveshafts. Moving members are extracted from this budget; they are not added a second time.

The separately declared rims and tires total 98 kilograms: 21 kilograms per front assembly and 28 kilograms per rear assembly. Complete unfueled mass is therefore 698 kilograms; the initial 7.6 kilogram fuel load produces 705.6 kilograms before consumption. These wheel values are provisional profile declarations, not independently measured Formula One assemblies.

Individual allocations, component centers and simplified solid inertia tensors remain engineering estimates. The 82 kilogram driver allowance is informed by the historical FIA commission statement, not a measured pilot. The whole allocation represents a fictional vehicle, not an actual Formula One 2030 specification.

The ledger carries independent 20 percent mass bounds, 30 percent inertia uncertainty and 0.05 metre center uncertainty. Independent endpoint sums do not preserve the declared base budget. The candidate now generates bounded, budget-constrained mass variants: each component remains within its declared range, the base remains 600 kilograms, each rim-and-tire assembly retains its declared total, and each component tensor scales with its mass at fixed geometry. A regression rebuilds the coupled model and checks positive definiteness. This is a mass-distribution sensitivity tool; correlated center/tensor variants and the full dynamic sensitivity matrix remain open.

Evidence references:

- FIA Formula One Commission, 23 July 2024: https://www.fia.com/news/formula-1-commission-meeting-23072024-media-statement
- FIA initial 2026 regulation announcement, 6 June 2024: https://www.fia.com/news/new-era-competition-fia-showcases-future-focused-formula-1-regulations-2026-and-beyond
- User-declared base perimeter and existing wheel declarations are the controlling local inputs. Historical tire-only regulatory allowances must not be substituted for rim-plus-tire mass.

## Implemented candidate mechanics

`coupled_vehicle_dynamics.rs` extends the existing ten-coordinate reference to fourteen generalized velocities: body translation, body angular velocity, four suspension travels and four relative wheel rotations. The matrix includes actual component trajectories, rotated tensors, spin cross-coupling, inertial bias and gravity. Aligned wheel rotors are distinguished from the upright so configured camber and toe affect their inertia axes.

Internal drive torques are projected into relative spin coordinates; body reaction follows from the coupled matrix. Braking uses bounded simultaneous relative-spin impulses and cannot reverse a free wheel. Resolved brake work is deposited into the thermal system, including actual partial-event duration; wheel thermal energy matches the mechanical dissipation ledger in a regression. Coupled force, tire and thermal updates no longer impose a minimum elapsed duration. This does not yet establish complete engine, shaft and clutch rotational-energy accounting. End-of-interval brake splitting and simultaneous brake/travel/contact convergence remain open.

Physical kinetic energy includes prescribed rack motion and its cross terms. Fuel exchange uses the exact changing point-mass kinetic energy, gravitational potential and momentum at the tank position, without injecting a velocity impulse. Steering work is accounted for during force evolution and constraint impulses.

Suspension advancement uses semi-implicit force reevaluation, localized travel crossings and simultaneous generalized velocity projection. Quaternion normalization and unsupported-domain errors are explicit. This integrator and the provisional 960 hertz frequency have not passed the required full convergence and performance selection gates.

## Implemented candidate world and contacts

`coupled_vehicle_world.rs` provides a shared clock, stable entity identities, timestamped inputs applied at their physical times, pause, reset, atomic rejected steps and simultaneous vehicle contact solving. An unsuccessful trial rolls back fuel, thermal, drivetrain and physical state together. No elapsed host time is intentionally discarded.

Parry `parry3d-f64 = 0.23.0` is pinned as the geometric candidate. Rust owns response; Parry supplies mesh queries, shape contacts and continuous shape casts. This is a prototype selection, not a completed library evaluation. Review its license, maintenance and measured geometry coverage before Item 24 approval.

The Fuji extraction retains 74 source meshes, 13,696 triangles, transformations, surface codes including metal and collision roles. Source SHA-256 and a separate digest of transformed vertices, triangles and metadata reject stale or altered packages. The physical package remains in scratch pending promotion and full validation.

Configured body boxes and solved wheel cylinders generate contacts. Normal constraints use generalized effective mass. Tangential impulses converge against a projected friction disk with coupled effective masses. Restitution thresholds, iteration budgets, passivity checks and prescribed-actuator work are explicit. Rejected solves do not commit partial impulses or velocities.

Swept bounding volumes reduce candidate pairs. Translation and rotation casts localize candidate impacts, with rollback and re-advancement to the event. Vertical curb faces remain detectable for wheels; tread support is provisionally distinguished from rigid impacts. Articulated-path error bounds, material pairing, persistent feature manifolds, warm starting, mesh-seam stability and dynamic object interoperability are not complete.

## Native boundary

`coupled_vehicle_world_interface.rs` exposes version 1 owned JSON request/response buffers. It validates lengths, UTF-8, versions, handles and owning-thread access, and catches Rust panics inside the request boundary. Creation, geometry loading, registration, input, advancement, snapshots, pause, reset, refuel and tire replacement are available.

The registered C++ `PhysicalVehicleWorldInterface` loads the matching symbols, frees Rust-owned response buffers and destroys its owned worlds before unloading the library. Its translation unit compiles independently. `game/tests/test_physical_vehicle_world_interface.gd` passes an isolated parser check, but its native registration/lifecycle execution still requires the source-matched rebuilt extension. Do not present the parser or single translation-unit check as runtime validation.

## Verified evidence and limits

The first complete candidate review recorded 341 passing tests and four failing aerodynamic tests. The same four failures reproduce in a scratch copy restored from HEAD plus the protected pre-edit sources. They were not introduced by this candidate. The latest complete library, integration-test and example review records 351 passing tests and the same four failures; the coupled world suite includes 25 passing tests and the geometric suite includes eight passing tests. Commands, logs, source hashes and diagnostic outputs are in `scratch/coupled_vehicle_complete_implementation/final_continuation_review/review.json`. The reviewed sources stayed unchanged during that run. Its task-owned same-branch Cargo output is not a clean production runtime build.

Both base and tribute-livery manifests now select the geometric profile and its digest. The geometric suite includes the preserved same-mass legacy comparison.

Blender's existing rear damper validator reports 0.0484 metres minimum upper-skin clearance after subtracting a 0.04 metre damper radius. This is an upper-skin test, not a complete finite-volume collision sweep against the gearbox, shafts, lower shell and every neighboring suspension member.

The optimized one-car Fuji sample took about 0.5865 seconds for 0.2 seconds of requested physical time. A 32-car sample from the authored Fuji grid took about 1.8755 seconds for 0.02 seconds of requested time. These are short idle samples, not percentile race benchmarks. They fail the real-time feasibility target and cannot justify activation. Fuji has 50 authored grid spots; the 32-car sample is not proof of complete-grid performance.

The isolated `coupled_vehicle_kernel_profile` example initially measured approximately 1.126 milliseconds for the 80 component trajectories and 2.141 milliseconds for a complete coupled evaluation. The latest 200-repetition sample measured 0.271 milliseconds and 0.455 milliseconds respectively, with 0.220 milliseconds for four uncached wheel trajectories and 0.000902 milliseconds for one road ray. Because uncached stages also varied substantially, these samples cannot attribute the whole difference to the cache. They exclude shared-world contacts, native serialization and rendering and do not qualify the race-performance gate.

The reference model now reuses exact geometric solutions across evaluations through a bounded shared cache. It checks the complete hardpoint configuration, keys solutions by exact wheel/travel/rack values and does not interpolate or approximate trajectories. Rejected trials may populate this derived cache without changing physical state. Cached and uncached corner solutions match exactly in the regression; changed hardpoints are rejected. The latest 32-car, 20 millisecond requested Fuji sample took 1.3179 seconds, still approximately 66 times the requested physical interval and unsuitable for runtime activation.

## Next dependency work

1. Profile component trajectory differentiation and matrix evaluation; replace repeated runtime geometry solves only with a mathematically equivalent, quantitatively validated approach. Preserve reference comparison and configuration invalidation.
2. Close complete geometry/derivative envelopes, mechanical joint loads and powertrain/tire work accounting.
3. Implement persistent contact manifolds, material pairing, articulated continuous-detection bounds and supported dynamic object ownership.
4. Finish smooth/impact timestep convergence, full contact benches and the configured race-cost gate before selecting the production integrator and frequency.
5. Implement the single Godot world scheduler, physically inert proxies, authoritative visual snapshots, existing audio/telemetry/gameplay consumers and explicit owner selection.
6. Produce a clean source-matched build through the canonical launcher, complete standalone/runtime parity and integrated driving/visual review, then prepare the reviewed atomic publication scope.

Neither these notes nor candidate configuration authorize bypassing failed gates or marking all 50 items done.
