---
name: aerodynamics
description: Configure and diagnose Formula-90 drag, downforce, and aerodynamic balance with offline falsification, runtime telemetry, and isolated causal changes.
---

# Aerodynamics

Capture the high-speed baseline and define the expected signal before modifying coefficients. Identify whether drag, lift/downforce, balance, tire grip, or weight transfer owns the symptom.

When the hypothesis can be checked mathematically, run the relevant offline model in `tools/physics_diagnostics/` before changing production values. Verify velocity-squared behavior and units.

Change one causal dimension: drag, total load, or balance. Treat coupled front/rear changes as one experiment only when maintaining a defined total or balance constraint. Validate in an isolated speed range, then compare telemetry to baseline.

Do not introduce CFD or opaque compensation layers. Enter Diagnostic Mode immediately when two implementation attempts preserve the same signature, values contradict the model, ownership is unknown, or the next change would stack a workaround.
