---
name: vehicle-physics
description: Procedimiento para ajustar o depurar dinámicas del chasis y comportamiento del vehículo.
---

# Skill: Vehicle Physics

## Purpose
Modificar el handling (manejo), suspensiones, frenos, centro de gravedad y neumáticos utilizando el motor GEVP subyacente, garantizando un estilo de conducción regido por las leyes de la física.

## Scope
Archivos `.tscn` de los vehículos (ej. `f1_2026_car.tscn`) y scripts de GEVP.

## Workflow
1. **Obligatorio:** Leer `docs/game-design/ai_physics_manual.md` para evitar asumir parámetros con intuición lingüística errónea.
2. Leer `docs/game-design/handling-philosophy.md`.
2. Identificar el parámetro físico a ajustar en lugar de añadir multiplicadores o "magia".
   - *Ej: Si el coche subvira, ajusta la distribución de peso, aerodinámica o rigidez del neumático, NO un multiplicador mágico de giro.*
3. **Verificación Python:** Si experimentas problemas ajustando los valores, ejecuta los scripts de verificación offline en Python (dentro de `tools/physics_diagnostics/`) para validar las fórmulas físicas.
4. Validar el cambio en `test_field.tscn`.

## Constraints & Forbidden Changes
- **No** aplicar grip instantáneo o ayudas invisibles.
- Todo cambio debe mantener la transferencia de peso comprensible.
- La pérdida de adherencia debe ser gradual.

## Common Pitfall: Chassis Collision vs RayCast Suspension (Two Separate Systems)

GEVP vehicles have **two independent collision systems** that can interact destructively:

| System | Node | What it does |
|---|---|---|
| **RayCast suspension** | `WheelFrontLeft` etc. (`RayCast3D`) | Shoots downward from Y-origin, detects surface, applies spring/damping forces |
| **RigidBody chassis** | `CollisionShape3D` (`BoxShape3D`) | Standard Godot rigid body collision — generates instantaneous impulse forces on contact |

**The pitfall:** If the `CollisionShape3D` extends below the RayCast origins or has zero ground clearance, the chassis will hit track geometry (curbs) BEFORE the RayCasts can detect them. This produces:
- `FL_Comp/FR_Comp = 150mm` (full travel, bottom-out) at curb contact
- `Long_G = -7 to -11G` (physically impossible via tire friction — it's a rigid body impact)
- `Front_Slip = 50-70+` (wheels unloaded after impact)
- Staggered pattern: left wheel bottoms → 100-200ms → right wheel bottoms with G-spike

**Diagnosis checklist when telemetry shows bottom-outs + extreme G-spikes:**
1. Calculate chassis bottom at rest: `CollisionShape3D.transform.Y - BoxShape3D.size.y/2 - static_sag`. If ≤ 0.02m, chassis is too low.
2. Compare RayCast origin Y to expected curb height. If `RayCast.Y < curb_height`, the curb is above the ray and invisible to the suspension.
3. Check that `RayCast.Y > chassis_bottom_Y` by a safety margin (5cm+).

**Fix protocol:**
1. Raise `CollisionShape3D` transform Y to give the chassis 5-10cm ground clearance at rest
2. Raise wheel `RayCast3D` transform Y proportionally so RayCast.Y > curb_height
3. Validate with telemetry: bottom-out events should disappear, G-spikes should be limited to tire grip limits (~2.6G on Road surface)

## Failure Escalation
If physics tuning fails repeatedly (e.g. attempting to fix understeer 3 times without success), stop adjusting coefficients and invoke `../problem-solving-guardrails/SKILL.md`.
