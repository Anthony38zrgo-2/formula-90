---
name: physics-diagnostics
description: Diagnose Formula-90 vehicle, GEVP, suspension, grip, collision, and curb-impact failures with evidence, ownership tracing, telemetry, and falsifiable physical hypotheses.
---

# Physics Diagnostics

1. State observed versus expected behavior.
2. Capture the baseline and a measurable failure signature.
3. Query `agentdb problem` using the dominant signature.
4. Query `agentdb knowledge godot` for uncertain Godot/GEVP mechanics.
5. Identify who owns, computes, mutates, consumes, and configures the state: track geometry, collider, RayCast/wheel, vehicle script/config, input, aero, or presentation.
6. Gather authoritative runtime configuration and relevant telemetry.
7. If mathematics can reject the hypothesis more cheaply, run the relevant script in `tools/physics_diagnostics/` before patching.
8. Apply one causal micro-patch only if the hypothesis survives.
9. Validate candidate versus baseline with `regression-validation`.

Do not infer semantics from variable names. If two implementation attempts preserve the same signature, or observed values contradict the physical model, enter Diagnostic Mode.
