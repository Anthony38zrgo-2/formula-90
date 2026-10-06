# vehicle-physics-engine — Runtime vehicle dynamics

Scope: the vehicle dynamics engine behind `vehicle_physics_engine.dll`.
Consumers: the bridge vehicle adapter and vehicle scenes.
Rules: physics authority lives here. Tunables arrive from `game/data/vehicles/`; constants require a data change, not a code fork.
Subfolders: each child folder documents itself in its own AGENTS.md.

## Coupled world candidate

The coupled vehicle world owns physical integration, suspension dynamics and collision response when explicitly selected. The legacy simulation remains the default. Parry supplies geometric collision queries; response and generalized vehicle dynamics belong to Rust. Changes must preserve a single integration owner and the source-verified physical package contract.

The F1 2030 mass inventory separates the 600 kg vehicle-and-driver budget from rims, tires and fuel. Treat this fictional vehicle's component allocation as an approximation with recorded uncertainty. Never add component masses twice.
