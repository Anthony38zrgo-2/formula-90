---
name: powertrain
description: Configure and diagnose Formula-90 combustion engines, torque curves, RPM behavior, clutch, gearing, and transmission with offline models and measurable runtime validation.
---

# Powertrain

Read `docs/game-design/powertrain-philosophy.md`. Preserve naturally aspirated V8/V10/V12 scope; do not introduce ERS, MGU-K, batteries, or duplicated engine implementations when data resources suffice.

Capture RPM, gear, throttle, speed, and acceleration baseline. Define the failure signature and identify whether the engine curve, clutch, ratio, shift logic, drag, or tire slip owns it.

Use `tools/physics_diagnostics/analyze_powertrain.py` before production changes when theoretical per-gear speed, shift point, torque, or RPM behavior can reject the hypothesis. Change one causal group and validate immediately.

Enter Diagnostic Mode when two attempts preserve the signature, the next patch still relies on a falsified engine hypothesis, or runtime behavior contradicts the offline model.
