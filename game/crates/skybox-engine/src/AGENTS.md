# src — Skybox implementation

Scope: sky engine library and exercise binaries.
Consumers: background rigs and diagnostic scenes.
Rules: deterministic output for a given preset and time; no wall-clock dependence. The `bin/` exercise-binary folder is ignore-blocked from carrying its own file and is covered here: dump parameters and pixels, keep scene composition in Godot.
