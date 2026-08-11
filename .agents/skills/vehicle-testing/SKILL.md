---
name: vehicle-testing
description: Validate Formula-90 vehicle behavior through reproducible isolated scenarios, baseline/candidate comparison, telemetry, and explicit PASS, FAIL, or INCONCLUSIVE results.
---

# Vehicle Testing

Use the smallest scenario that isolates the hypothesis. Prefer the existing test field over a new framework.

Relevant scenarios include straight acceleration, high-speed braking, constant-radius corner, slalom, lift-off mid-corner, fast corner, and curb impact. Do not run all scenarios when one targeted test answers the current question.

For behavioral changes:

1. capture the same scenario as baseline;
2. state acceptance criteria and noise tolerance;
3. apply the candidate;
4. repeat the scenario under equivalent setup;
5. report delta as PASS, FAIL, or INCONCLUSIVE;
6. rollback FAIL candidates.

If causes remain unknown or the same signature survives two implementation attempts, stop driving/testing loops and enter Diagnostic Mode.
