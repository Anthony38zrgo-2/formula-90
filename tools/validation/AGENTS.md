# validation — C++ content validators

Scope: native validators for audio WAVs and game assets, with shared analyzer headers, entry-point sources, and C++ tests.
Consumers: the native build and content gates over `game/sounds/` and `game/assets/`.
Rules: validators reject; they never repair. Repair logic belongs in the authoring pipeline.
Subfolders: each child folder documents itself in its own AGENTS.md.
