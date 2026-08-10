---
name: scene-safety
description: Safe procedure for modifying Godot .tscn files without losing serialized references or introducing structural corruption.
---

# Scene Safety

1. Query `agentdb problem node_paths` before broad scene work.
2. Prefer surgical changes to specific properties/nodes.
3. Preserve serialized references and exported NodePath metadata.
4. Do not rewrite a whole scene/node block for a small change.
5. Run structural parsing and a Godot scene-load/smoke validation after modification.
6. If vehicle collision/curb symptoms appear, query the problem index rather than embedding one-off physics incidents in this skill.

This skill is procedural. Specific incidents belong in `common_problems`.
