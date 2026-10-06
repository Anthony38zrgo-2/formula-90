# src/vehicle — Vehicle implementations

Scope: adapter from the bridge to the Rust vehicle physics engine.
Consumers: vehicle scenes under `game/scenes/vehicles/`.
Rules: never duplicate physics here; delegate to `game/crates/vehicle-physics-engine/`.

Physical-world proxies use zero collision layer/mask and gravity, a custom integrator and frozen state. Return before legacy force integration when physically controlled. Apply authoritative pose, wheel normals, loads, spin and surfaces from snapshots. Emit service requests instead of mutating a separate simulation; reject unsupported direct tuning while physical ownership is active.
