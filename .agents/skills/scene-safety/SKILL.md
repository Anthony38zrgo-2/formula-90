---
name: scene-safety
description: Reglas y procedimientos para modificar archivos .tscn de Godot 4 de forma segura sin corromper referencias.
---

# Skill: Scene Safety

## Purpose
`.tscn` files are high-risk assets. Modifying them manually via text replacements can corrupt the scene if structural rules are ignored. This skill defines the mandatory protocol for modifying scenes safely.

## 1. The `node_paths` Rule
In Godot 4, if a script uses `@export var node: Node`, the engine stores the assigned reference inside the scene file in an array called `node_paths`.
Example:
```text
[node name="Car" type="VehicleBody3D"]
node_paths=PackedStringArray("camera_node", "wheel_node")
```
**RULE:** NEVER delete or overwrite the `node_paths` line when modifying a node's parameters. If you overwrite the node definition and forget `node_paths`, the game will silently crash because references become null.

## 2. Minimal Modification Principle
Agents must NOT rewrite entire scene files or large blocks for small local modifications.
- Use surgical text replacements (`replace_file_content` targeting exact lines).
- Avoid multiline replacements that encompass unaffected properties.

## 3. Structural Validation
After modifying a `.tscn`:
1. Use `tscn_parser.py` or equivalent tools to ensure the file can still be parsed.
2. If `node_paths` were accidentally removed, Revert the change immediately.
3. Unexpected structural deletion must fail validation.

## 4. Common Pitfall: RayCast3D Vertical Blindness
When modifying vehicle `.tscn` files with GEVP wheel RayCasts, this error pattern is frequent:

**Symptoms:**
- Chassis bottom-out (FL_Comp/FR_Comp = 150mm = full travel) at moderate speeds
- Extreme G-spikes (7-11G lateral/longitudinal) not explainable by tire grip
- Staggered bottom-out: one wheel bottoms, then the other 100-200ms later
- Wheels fully extended (0mm compression) while G-spikes occur (airborne chassis impact)

**Root Cause:**
The GEVP `Wheel` nodes are `RayCast3D` that shoot **downward** from their transform origin (`wheel.gd:91`):
```
set_target_position(Vector3.DOWN * (spring_length + tire_radius))
```
If the RayCast origin Y is below the top of a curb, the curb is **above the ray** and cannot be detected. The chassis `BoxShape3D` (a separate RigidBody3D collision shape) then collides directly with the curb geometry → hard impact → massive G-spike without suspension damping.

**The Fix Always Requires TWO Changes:**
1. **Check chassis ground clearance:** `CollisionShape3D` transform Y must position the box bottom high enough that only large obstacles hit it directly. With `BoxShape3D` half-height `h`, bottom = `transform.Y - h`. With static suspension sag, this can reach 0.0m → any curb hits.
2. **Check RayCast origin height:** The wheel transform Y must exceed the expected curb height so the downward ray intersects the curb surface.

**Rule of thumb:** `RayCast.Y > curb_height > chassis_bottom_Y`

**Diagnostic evidence before/after:**
- Before: `FL_Comp=150mm` (bottom-out) + `Long_G=-11G` in same frame → chassis impact
- After: `FL_Comp=50-100mm` (normal compression) at curb entry → RayCast detects curb, suspension absorbs
