# Guardrail asset drop zone

Place one or more repeatable low-poly guardrail modules here (`.blend`, `.glb` or `.gltf`).

The inspector uses the longest horizontal dimension as the default module length. If the source orientation needs correction, add a sidecar:

```json
{
  "id": "classic_guardrail",
  "category": "guardrails",
  "long_axis": "X",
  "rotation_correction_deg": 90.0
}
```

Visual modules are instanced in Blender. Collision is **not** taken from the detailed guardrail mesh: the environment builder creates simplified box collision geometry named with Godot's `-colonly` suffix.
