---
name: scene-safety
description: Safe, lightweight procedure for Godot .tscn edits.
---
# Scene Safety
- Make surgical edits; do not rewrite unrelated scene blocks.
- Preserve `node_paths`, serialized references, and exported assignments.
- Never edit imported GLB scene internals directly.
- Parse/load the modified scene and test the affected behavior.
- When changing vehicle wheel/chassis geometry, confirm RayCast origin is above expected surfaces, chassis clearance is adequate, and the scene loads.
