# features — Self-contained feature slices

Scope: vertical feature slices bundling their own scenes, scripts, configuration, and assets: the arcade HUD and the retro HUD.
Consumers: `game/scenes/` sessions and `game/tests/` coverage.
Rules: slice discipline: a feature works without touching sibling features; shared engine behavior stays in `game/scripts/` and `game/crates/`.
Subfolders: each child folder documents itself in its own AGENTS.md.
