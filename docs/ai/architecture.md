# Agent Collaboration Architecture

## Purpose

Keep planning, execution, validation, and context transfer causally clear without
binding governance to a specific model name. Model routing may change; the protocol
in `.agents/AGENTS.md` remains authoritative.

## Roles

### Planner / diagnostic owner

- classify the task and complete preflight;
- establish expected behavior, baseline, ownership, and unknowns;
- define a falsifiable hypothesis, expected signal, and failure signature;
- choose the cheapest experiment that can reduce uncertainty;
- create a Context Bundle only when delegation is useful.

### Executor

- consume the evidence-focused Context Bundle;
- run Attempt 0 checks before production implementation;
- apply one causal micro-patch only when the hypothesis survives;
- validate the smallest affected surface immediately;
- report evidence and rollback failed candidates.

### Validator

- verify structural/build integrity;
- compare baseline and candidate under equivalent conditions;
- classify the result as `PASS`, `FAIL`, or `INCONCLUSIVE`;
- reject unsupported success claims.

The same agent may perform more than one role on a bounded task, but must not merge
observations and interpretations or bypass the attempt budget.

## Collaboration workflow

```text
TASK -> PREFLIGHT -> BASELINE/OWNERSHIP -> HYPOTHESIS
     -> CHEAPEST FALSIFICATION TEST
     -> survives? no: RECORD + CONTEXT GC + NEW HYPOTHESIS
                  yes: MICRO-PATCH + FAST VALIDATION
     -> pass: REGRESSION -> DONE
     -> fail: RECORD + ROLLBACK + CONTEXT GC
     -> same signature/evidence stagnation: DIAGNOSTIC MODE
```

Diagnostic Mode permits inspection, telemetry, parsers, offline models, minimal
reproductions, and isolated tests, but no production patches.
