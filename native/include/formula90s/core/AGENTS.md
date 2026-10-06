# core — Bridge core contracts

Scope: C++ declarations for bootstrap, shared types, and the C entry surface.
Consumers: every bridge module and the Godot module registration.
Rules: breaking the C surface requires a coordinated rebuild of all `game/addons/formula90s/bin/` libraries.

The physical-world interface owns world handles and requires the full Rust source identifier to match the native source identifier and `BUILD_SOURCE`. The core snapshot function is an additive ABI 17 capability. `physical_world_document.hpp` normalizes exact integral JSON values and preserves document precision; do not relax Rust request validation to compensate for Godot numeric serialization.
