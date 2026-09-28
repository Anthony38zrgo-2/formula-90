# fixtures — Shared test fixtures

Scope: placeholder and future shared fixtures for repository-level tests.
Consumers: suites under `tests/` and any tooling test needing canned inputs.
Rules: fixtures are immutable inputs; generating them is a `tools/` job, never a test job.
