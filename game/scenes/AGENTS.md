# scenes — Scene composition root

Scope: every Godot scene in the game, grouped by purpose: bootstrap, runtime sessions, showcases, tracks, interface, vehicles, and visual composition.
Consumers: the Godot project runtime and the `scripts/run_*` launchers.
Rules: scenes compose systems implemented in `game/scripts/`, `game/crates/`, and `native/`. A scene file wires; it never implements behavior.
Subfolders: each child folder documents itself in its own AGENTS.md.
