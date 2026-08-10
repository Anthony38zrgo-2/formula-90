---
name: regression-validation
description: Compare a candidate change against a baseline and reject fixes that silently regress a secondary scenario.
---

# Regression Validation

For significant behavior changes:
1. Choose the primary scenario.
2. Choose at least one likely secondary regression scenario.
3. Capture baseline evidence.
4. Apply the candidate.
5. Capture candidate evidence.
6. Compare deltas.
7. Return PASS, FAIL, or INCONCLUSIVE with evidence.

A primary improvement with an unacceptable secondary regression is FAIL.
