---
name: telemetry
description: Procedimiento para exponer y utilizar telemetría en el análisis de físicas.
---

# Skill: Telemetry

## Purpose
Hacer observables los sistemas internos del vehículo para permitir un tuning objetivo y diagnóstico de fallos por parte de agentes o humanos, sin depender de descripciones subjetivas ("se siente raro").

## Related Documentation
- Leer `docs/architecture/telemetry.md`.

## Workflow
1. Asegurar que GEVP o el script del coche exponga variables críticas (velocidad, RPM, marcha, inputs, deslizamiento frontal y trasero, downforce, fuerzas G).
2. Si una métrica crítica falta durante el tuning de un problema, añádela a la telemetría del HUD o a la consola temporalmente antes de intentar adivinar los parámetros físicos.
3. **Lectura Asíncrona (V4 Flash):** Si hay problemas con un componente (suspensión, aero, etc.), el agente **V4 Flash** debe ser invocado para inspeccionar el CSV generado en `res://telemetry/` (o `game/telemetry/`).
   - **Eficiencia Obligatoria:** Un CSV puede tener decenas de miles de líneas. V4 Flash **JAMÁS** debe intentar leer el archivo entero usando herramientas de lectura de texto (`view_file`, `cat`).
   - **Método Correcto:** Flash debe utilizar el script Python dedicado para extraer un resumen automático de las anomalías:
     `python tools/physics_diagnostics/analyze_telemetry.py game/telemetry/<archivo>.csv`
   - El script escupirá un reporte de fuerzas G, compresiones y detectará si hubo impactos contra el chasis. Flash debe tomar este reporte y devolvérselo a V4 Pro.

## Failure Escalation
If telemetry integration fails repeatedly (e.g. values are incorrect or out of sync), stop coding and invoke `../problem-solving-guardrails/SKILL.md`.
