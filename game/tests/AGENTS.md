# tests — Godot-side test suites

Scope: GDScript smoke, visual-capture, audit, and verification suites run in Godot, plus the GdUnit framework suites.
Consumers: the `scripts/test_gdunit.*` gates and human visual review.
Rules: tests observe the running game; shared engine assertions live in `game/tests/gdunit/`. Capture tests produce evidence, not golden screenshots committed blindly.
Subfolders: each child folder documents itself in its own AGENTS.md.
