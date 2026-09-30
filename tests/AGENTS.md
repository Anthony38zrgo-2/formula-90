# tests — Repository-level test homes

Scope: the manual smoke gate and the shared fixture home, outside the per-runtime suites.
Consumers: the human approval step of the sprint pipeline and tooling tests needing canned inputs.
Rules: automated suites live beside their runtime in `game/tests/`, `native/tests/`, and `tools/*/tests/`; this folder holds the manual checklist and repo-level fixtures only.
Subfolders: each child folder documents itself in its own AGENTS.md.
