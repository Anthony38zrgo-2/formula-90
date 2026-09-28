# sim — Bridge simulation contracts

Scope: C++ declarations facing the headless race simulation crate.
Consumers: `game/crates/game-sim/` across the FFI boundary.
Rules: struct layout must match the Rust side exactly; `game/crates/dsp-abi-check/` exists to catch drift.
