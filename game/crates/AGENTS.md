# crates — Rust simulation workspace

Scope: the Cargo workspace behind every native library: physics, audio, simulation core, synthesis, skybox, and PSX presentation, plus the ABI checker.
Consumers: the SCons build emitting `game/addons/formula90s/bin/` libraries, and headless diagnostic binaries.
Rules: workspace discipline: shared dependencies resolve at the workspace root. DSP-facing layout changes run through `dsp-abi-check/`.
Subfolders: each child folder documents itself in its own AGENTS.md.
