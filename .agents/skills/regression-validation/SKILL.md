---
name: regression-validation
description: Protocolo para comparar el comportamiento propuesto (Candidate) contra el estado anterior (Baseline).
---

# Skill: Regression Validation

## Purpose
Every significant handling improvement should be compared against a baseline. A fix for one condition must not silently destroy another.

## Concept
1. **Baseline:** The measured state of the system *before* the change.
2. **Candidate:** The measured state of the system *after* the change.
3. **Delta:** The difference between them.
4. **Result:** PASS / FAIL / INCONCLUSIVE.

## Workflow
1. Identify the primary scenario (e.g. `curb_behavior`).
2. Identify a secondary scenario that could be affected (e.g. `suspension_behavior` under braking).
3. Record Baseline telemetry for both.
4. Apply the Candidate change.
5. Record Candidate telemetry for both.
6. If the primary scenario improves but the secondary regresses beyond the accepted threshold, the result is FAIL.

## Output Format
```text
TEST: [Scenario Name]

Baseline:
- Metric A: X
- Metric B: Y

Candidate:
- Metric A: Z (Improved)
- Metric B: W (Regressed)

Result: [PASS/FAIL/INCONCLUSIVE]
```
