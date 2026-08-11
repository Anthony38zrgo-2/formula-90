---
name: regression-validation
description: Compare a candidate against a relevant baseline and classify behavioral or structural changes as PASS, FAIL, or INCONCLUSIVE with proportionate validation.
---

# Regression Validation

For significant behavior changes, use:

```text
BASELINE -> HYPOTHESIS -> EXPERIMENT -> MICRO-PATCH
         -> CANDIDATE -> DELTA -> PASS | FAIL | INCONCLUSIVE
```

Capture the relevant baseline before the patch. A behavioral candidate without a baseline is normally `INCONCLUSIVE`; binary structural fixes may use direct pass/fail evidence.

Validate in increasing scope:

1. smallest deterministic/static check;
2. affected unit, parser, scene, or offline model;
3. isolated runtime scenario;
4. telemetry comparison when it answers the hypothesis;
5. broader regression only for accepted candidates, milestones, or high risk.

Report the baseline, candidate, measured delta, noise/uncertainty, and classification. Roll back a failed candidate. Do not claim improvement when the delta cannot be distinguished from baseline noise.
