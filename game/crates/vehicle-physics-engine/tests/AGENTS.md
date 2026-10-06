# tests — Physics Rust tests

Scope: Rust tests pinning vehicle dynamics behavior.
Consumers: `cargo test` gates.
Rules: golden telemetry pins behavior; model changes update goldens with review.

Coupled-world gates cover mass closure, coupled motion, contacts, timestamped inputs, fixed-step carry, entity lifecycle and services. Suspension gates cover component inertia, geometric convergence and chassis packaging. Run the suites relevant to the changed physical contract. Record existing baseline failures separately; passing candidate tests does not certify full-grid performance, rendered geometry or all backlog items.
