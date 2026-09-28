# src — Physics implementation

Scope: dynamics library sources and runner binaries.
Consumers: the bridge adapter.
Rules: determinism first: fixed step, seeded randomness only. The `bin/` headless-runner folder is ignore-blocked from carrying its own file and is covered here: runners emit telemetry the analyzers consume, with no Godot dependency.
