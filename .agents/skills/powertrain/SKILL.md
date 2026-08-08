---
name: powertrain
description: Procedimiento para configurar y afinar los motores de combustión y transmisiones.
---

# Skill: Powertrain

## Purpose
Simular y balancear motores V8, V10 o V12 atmosféricos, asegurando una curva de potencia, inercias y revoluciones correctas.

## Related Documentation
- Leer `docs/game-design/powertrain-philosophy.md`.

## Workflow & Constraints
1. **Atmosférico Puro:** Queda estrictamente prohibido introducir ERS, MGU-K, despliegue eléctrico o baterías. 
2. **Implementación Única:** Evitar crear múltiples archivos rígidos (`V10Engine.cpp`, `V12Engine.cpp`). Modificar o usar los recursos (`.tres`) o parámetros GEVP base que permitan que un motor difiera de otro simplemente cambiando los datos (Torque, Max RPM, Inercia, Freno motor).
3. **Engine Braking:** Asegurar que el freno motor sea sustancial.
4. **Verificación Python:** Si el tuning del motor genera comportamientos extraños, utiliza los scripts de verificación offline en Python (en `tools/physics_diagnostics/`) para graficar la curva de torque y las relaciones de caja teóricas.

## Failure Escalation
If tuning fails repeatedly (e.g. RPM oscillations or torque curve issues), stop adjusting parameters and invoke `../problem-solving-guardrails/SKILL.md`.
