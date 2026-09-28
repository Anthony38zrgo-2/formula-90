# common — Shared pipeline infrastructure

Scope: domain-neutral infrastructure: the fail-closed output policy, the architecture inventory, and content-flow helpers.
Consumers: every `tools/` entry point that writes files.
Rules: neutral ground by design. Never add audio, vehicle, terrain, or track rules here; adopt legacy entry points per domain instead.
Subfolders: each child folder documents itself in its own AGENTS.md.
