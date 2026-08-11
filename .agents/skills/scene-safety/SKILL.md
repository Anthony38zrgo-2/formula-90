---
name: scene-safety
description: Apply fail-fast structural safety to Godot .tscn edits, including pre/post parsing, node_paths, resource references, ownership, and immediate rollback.
---

# Scene Safety

## Pre-patch

- Parse/load the current scene.
- Capture the relevant node structure, owners, exported assignments, and `node_paths`.
- Verify referenced resources exist.
- State the exact structural invariant and smallest required edit.

## Patch

Make one surgical change. Do not rewrite unrelated blocks or edit imported GLB internals.

## Post-patch

- Parse/load immediately.
- Compare captured structural invariants.
- Verify `node_paths`, owners, exported references, and resources.
- For wheel/chassis geometry, verify RayCast origin, expected surface reach, and chassis clearance.

If structural validation fails, rollback immediately. Do not launch a runtime test with an invalid scene. Only after structural PASS run the smallest affected runtime test.
