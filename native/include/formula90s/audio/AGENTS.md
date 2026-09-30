# audio — Bridge audio contracts

Scope: C++ declarations for engine DSP and audio plumbing exposed to the bridge.
Consumers: `native/src/` and the vehicle audio Rust crates.
Rules: sample-format invariants are owned by `formats/audio_bank/AGENTS.md`; this folder declares plumbing, never format policy.
