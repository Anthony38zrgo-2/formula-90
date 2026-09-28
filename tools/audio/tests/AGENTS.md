# tests — Audio pipeline tests

Scope: pytest coverage for bank contracts, determinism, formats, playback metadata, scenario rendering, synthesis, and CLI boundaries.
Consumers: `scripts/test_fast.ps1` and the platform test gates.
Rules: determinism tests fail on any unexplained byte change; update goldens only with an approved audio change.
