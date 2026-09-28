# formats — Interchange contracts

Scope: versioned file-format contracts shared by the Python tooling, the Rust runtime, and the Godot adapter.
Consumers: producers in `tools/` and consumers in `game/crates/` plus Godot scenes.
Rules: formats describe built artifacts, never how to synthesize them. Schema changes follow the evolution policy in the owning subfolder.
Subfolders: each child folder documents itself in its own AGENTS.md.
