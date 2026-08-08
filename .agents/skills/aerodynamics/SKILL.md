---
name: aerodynamics
description: Procedimiento para balancear y configurar carga aerodinámica y drag.
---

# Skill: Aerodynamics

## Purpose
Establecer la carga aerodinámica (Downforce) y resistencia al avance (Drag) dependientes de la velocidad, logrando un balance realista Front/Rear.

## Workflow
1. Modificar parámetros en GEVP o el script aerodinámico (`coefficient_of_drag`, áreas frontales).
2. Asegurar que los efectos aumenten geométricamente con la velocidad ($V^2$).
3. **Simplicidad:** No implementar CFD ni fórmulas fluidodinámicas innecesariamente complejas. El comportamiento debe ser determinista, fácilmente ajustable vía Inspector y observable mediante Telemetría.
4. **Verificación Python:** Utiliza los scripts de análisis aerodinámico offline en Python (`tools/physics_diagnostics/`) para precalcular las fuerzas de drag (resistencia) y transferencia de peso a distintas velocidades antes de alterar el código en GDScript.

## Failure Escalation
If aerodynamic tuning fails repeatedly (e.g. lift-off oversteer is unpredictable after multiple tweaks), stop adjusting coefficients and invoke `../problem-solving-guardrails/SKILL.md`.
