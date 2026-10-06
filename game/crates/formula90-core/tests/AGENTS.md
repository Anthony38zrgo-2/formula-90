# tests — Core Rust tests

Scope: integration tests for the shared core.
Consumers: `cargo test` in CI and the `scripts/test_*` gates.
Rules: failures here block every downstream crate; fix core first.

`physical_world_presentation.rs` verifies ownership, clock isolation, shadow-service rejection, incompatible snapshot rejection and finite nonzero sampler output. Keep the pit-service facade regression gate when changing service publication. Signal-generation tests do not replace runtime listening approval.
