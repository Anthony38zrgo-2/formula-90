# vehicle — Bridge vehicle contracts

Scope: C++ declarations for the Rust vehicle adapter and Formula physics surface.
Consumers: `native/src/vehicle/` and `game/crates/vehicle-physics-engine/`.
Rules: physics authority lives in Rust; headers adapt, never reimplement.
