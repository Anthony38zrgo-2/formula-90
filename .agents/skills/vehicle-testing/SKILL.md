---
name: vehicle-testing
description: Procedimiento para validar cambios en el comportamiento del vehículo mediante escenarios reproducibles.
---

# Skill: Vehicle Testing

## Purpose
Validar cualquier cambio en físicas, aerodinámica o powertrain de manera repetible, garantizando que no se introducen regresiones en el manejo básico.

## Workflow
Para probar un vehículo, utiliza `test_field.tscn` y ejecuta de ser posible los siguientes escenarios de control:
1. **Aceleración en línea recta:** Validar marchas, RPM y ausencia de wheelspin incontrolable.
2. **Frenada a alta velocidad:** Validar distancias, bloqueo de ruedas y el impacto del brake bias.
3. **Curva de radio constante:** Validar el balance understeer/oversteer a mid-corner.
4. **Slalom / Cambios de dirección rápidos:** Validar la transferencia de peso.
5. **Lift-off mid-corner:** Validar que levantar el pie del acelerador súbitamente en curva genera el esperado oversteer por retención del V10.

No inventes un nuevo framework de test si el circuito de pruebas (`test_field`) ya permite realizar estas validaciones manuales o semiautomáticas.

## Failure Escalation
If a test continues to fail unexpectedly and causes cannot be identified within 2 attempts, stop testing and invoke `../problem-solving-guardrails/SKILL.md`.
