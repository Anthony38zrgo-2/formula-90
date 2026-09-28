# audio — Bank authoring pipeline

Scope: the canonical V10 bank builder, bank contract and manifest handling, DSP helpers, scenario rendering, and sound promotion utilities.
Consumers: `game/sounds/banks/` manifests and the runtime sampler crates.
Rules: the builder is deterministic. Any intentional audio change updates raw sources, tool revision, derived hashes, and the suite in `tests/` in the same change. Format truth is `formats/audio_bank/`; this folder implements it.
Subfolders: each child folder documents itself in its own AGENTS.md.
