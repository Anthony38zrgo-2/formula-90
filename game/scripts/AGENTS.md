# scripts — Godot behavior wiring

Scope: GDScript wiring scene behavior across audio, background, camera, HUD, input, session runtime, showcase, telemetry, track, and vehicle domains.
Consumers: scenes under `game/scenes/` and features under `game/features/`.
Rules: scripts wire and orchestrate; physics and DSP authority lives in `game/crates/`, tuning truth in `game/data/`. A new behavior domain adds one subfolder with its own AGENTS.md.
Subfolders: each child folder documents itself in its own AGENTS.md.
