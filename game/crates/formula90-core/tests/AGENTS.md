# tests — Core Rust tests

Scope: integration tests for the shared core.
Consumers: `cargo test` in CI and the `scripts/test_*` gates.
Rules: failures here block every downstream crate; fix core first.
