# vehicle — Bridge vehicle contracts

Scope: C++ declarations for the Rust vehicle adapter and Formula physics surface.
Consumers: `native/src/vehicle/` and `game/crates/vehicle-physics-engine/`.
Rules: physics authority lives in Rust; headers adapt, never reimplement.

The physical-world ownership API freezes the Godot proxy, disables its collision response and accepts authoritative pose and wheel snapshots. Reset, refuel and tire services are requested through the world controller. Preserve the ownership guard on every legacy stepping and force path.
