# tools — In-engine utilities

Scope: GDScript utilities running inside Godot: track validation and collision diagnostics.
Consumers: developers validating generated tracks and collisions.
Rules: utilities report; they never modify scene or track data. The `diagnostics/` collision-probe folder is ignore-blocked from carrying its own file and is covered here: probes read the running scene, expectations stay in the caller.
