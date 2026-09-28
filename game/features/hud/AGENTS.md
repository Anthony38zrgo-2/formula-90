# hud — Arcade HUD feature

Scope: the arcade HUD feature configuration consumed by HUD scenes and scripts.
Consumers: `game/scripts/hud/` panels and the world HUD compositor.
Rules: feature defaults live in `config/`; panels read them, never hardcode them.
Subfolders: each child folder documents itself in its own AGENTS.md.
