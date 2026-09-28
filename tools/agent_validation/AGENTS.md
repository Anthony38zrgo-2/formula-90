# agent_validation — Governance validator

Scope: the deterministic structural validator for agent governance plus its own unit tests.
Consumers: CI and the `scripts/test_*` gates guarding `.agents/` consistency.
Rules: validator failures block agent-system changes; keep checks structural, never stylistic.
