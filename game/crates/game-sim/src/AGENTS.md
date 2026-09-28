# src — Simulation implementation

Scope: simulation library sources and runnable binaries.
Consumers: the bridge adapter and headless runners.
Rules: scene and HUD concerns never enter this tree. The `bin/` headless-runner folder is ignore-blocked from carrying its own file and is covered here: runners emit machine-readable results for diagnostics and probes.
