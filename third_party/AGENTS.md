# third_party — Vendored external code

Scope: external dependencies consumed as source, plus the third-party record.
Consumers: the native and validation builds.
Rules: never patch in place. Upgrades are explicit vendor drops recorded in `THIRD_PARTY.md`. The `godot-cpp` submodule carries no AGENTS.md and is covered here.
Subfolders: each owned child folder documents itself in its own AGENTS.md.
