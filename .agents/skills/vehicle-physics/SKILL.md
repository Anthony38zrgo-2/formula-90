---
name: vehicle-physics
description: Tune or diagnose Formula-90 chassis, steering, suspension, braking, tire, weight-transfer, and handling behavior through baselines, physical hypotheses, offline falsification, telemetry, and reversible micro-patches.
---

# Vehicle Physics

Read `docs/game-design/ai_physics_manual.md` and `docs/game-design/handling-philosophy.md`.

Use this sequence:

```text
observed behavior -> baseline telemetry -> configuration authority -> ownership
-> physical hypothesis -> cheapest offline/static falsification
-> micro-patch -> isolated test field -> telemetry delta -> regression
```

Prefer physical parameters over opaque multipliers. Maintain understandable weight transfer, gradual grip loss, recoverable oversteer, stable braking, and predictable curbs.

One experiment normally changes one causal dimension. Tightly coupled geometry changes are allowed only when the physical relationship proves they form one fix.

## Chassis collision versus RayCast suspension

Treat the rigid chassis collider and RayCast suspension as independent systems. A curb signature with full suspension travel, an extreme rigid-body G spike, and wheel unload may mean the chassis collides before the RayCast responds.

Before changing springs or damping:

1. calculate chassis bottom at rest;
2. compare RayCast origin/target with curb height;
3. verify RayCast origin remains above chassis bottom with margin;
4. inspect whether the G spike precedes suspension compression.

If geometry owns the failure, modify only the proven collider/RayCast relationship and run `scene-safety`.

Enter Diagnostic Mode when two implementation attempts preserve the same signature, a falsified parameter family is proposed again without new evidence, ownership remains unknown, or telemetry contradicts the assumed physical model.
