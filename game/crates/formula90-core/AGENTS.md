# formula90-core — Shared simulation core

Scope: shared types, bootstrap, and modules used by every simulation crate.
Consumers: `game-sim`, the vehicle engines, and the Godot bridge.
Rules: core stays dependency-light; domain logic lives in the sibling crates, never here.
Subfolders: each child folder documents itself in its own AGENTS.md.
