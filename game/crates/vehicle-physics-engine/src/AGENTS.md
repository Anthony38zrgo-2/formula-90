# src — Physics implementation

Scope: dynamics library sources and runner binaries.
Consumers: the bridge adapter.
Rules: determinism first: fixed step, seeded randomness only. The `bin/` headless-runner folder is ignore-blocked from carrying its own file and is covered here: runners emit telemetry the analyzers consume, with no Godot dependency.

## Coupled dynamics and collision ownership

- `vehicle_mass_inventory.rs` and the suspension mass modules define component budgets, attachment motion and inertia. Preserve budget closure and uncertainty validation.
- `coupled_vehicle_dynamics.rs` couples six chassis velocities, four suspension velocities and four wheel spin velocities. Preserve mass-matrix symmetry, inertial coupling, force application and energy accounting when changing constraints or forces.
- `physical_collision_world.rs` validates source and geometry digests and supplies collision geometry. `generalized_vehicle_contacts.rs` owns generalized contact response. Geometry-query success alone does not establish contact-solver acceptance.
- `coupled_vehicle_world.rs` owns stable entity identity, fixed stepping, queued input time, retained host fractions and service operations. Publish solved suspension corners from the authoritative state. Never advance a second vehicle simulation for presentation.
- `coupled_vehicle_world_interface.rs` exposes the versioned world interface and full source identifier. Preserve strict request validation and transport the original profile document where configuration identity must remain exact.
