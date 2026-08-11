---
name: telemetry
description: Expose, capture, validate, and summarize Formula-90 vehicle telemetry when measurements are required to falsify physics or runtime hypotheses.
---

# Telemetry

Read `docs/architecture/telemetry.md`.

1. Define the question telemetry must answer and the expected signal.
2. Verify runtime configuration/setup provenance before comparing sessions.
3. Capture only metrics needed for the hypothesis: speed, RPM, gear, inputs, slip, G forces, suspension, aero, contacts, or other owning-system state.
4. If a critical metric is missing, add the smallest observation point before guessing at physics.
5. Summarize large CSV files with `python tools/physics_diagnostics/analyze_telemetry.py game/telemetry/<file>.csv`. Do not load an entire large CSV into model context.
6. Compare candidate versus baseline and report uncertainty.

Telemetry is evidence, not automatic proof of cause. If values are missing, out of sync, contradict configuration, or preserve the same signature across two implementations, enter Diagnostic Mode and repair observability before further tuning.
