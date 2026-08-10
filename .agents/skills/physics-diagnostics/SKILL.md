---
name: physics-diagnostics
description: Evidence-first diagnostic procedure for Formula-90 vehicle behavior, GEVP integration, suspension, grip, collisions and curb impacts.
---

# Physics Diagnostics

1. State observed vs expected behavior.
2. Query `agentdb problem` using the dominant symptom.
3. Query `agentdb knowledge godot` for uncertain Godot/GEVP mechanics.
4. Identify ownership: track geometry, collider, RayCast/wheel, vehicle script/config, or presentation.
5. Gather telemetry/runtime configuration before tuning.
6. Change one causal variable group at a time.
7. Validate candidate vs baseline with `regression-validation`.

Do not infer physics semantics from variable names alone.
