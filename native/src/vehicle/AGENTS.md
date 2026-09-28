# src/vehicle — Vehicle implementations

Scope: adapter from the bridge to the Rust vehicle physics engine.
Consumers: vehicle scenes under `game/scenes/vehicles/`.
Rules: never duplicate physics here; delegate to `game/crates/vehicle-physics-engine/`.
