# Formula-90 Agent Guide

## Canonical principles

1. Fail hypotheses before implementations.
2. Prefer falsification over confirmation.
3. Never patch when a cheaper experiment can reject the hypothesis.
4. One experiment normally changes one causal dimension.
5. Same hypothesis plus same failure signature means stop implementation.
6. Every failure must reject a hypothesis, verify a fact, narrow the search,
   change the failure signature, or identify ownership.
7. Capture a baseline before meaningful behavioral changes.
8. Validate immediately after the smallest change.
9. Roll back failed candidates.
10. Remove falsified assumptions from active context.
11. Escalate on evidence stagnation, not frustration.
12. Understand enough to experiment; do not analyze the whole repository by default.

Fail Fast is not "code fast". Adapt Fast is not random strategy switching. Adaptation
must follow evidence learned from the previous experiment.

## Required workflow

### Preflight

Before a risky or behavioral change, determine the task type, affected subsystem,
risk, required skill, acceptance criterion, baseline, ownership, cheapest
falsification test, immediate validation command, and rollback strategy.

Trace who owns, computes, mutates, consumes, and serializes the relevant state.
Patch the owning subsystem, not merely the visible symptom.

### Hypothesis protocol

```text
HYPOTHESIS
EVIDENCE
FALSIFICATION TEST
EXPECTED SIGNAL
FAILURE SIGNATURE
```

States are `UNTESTED`, `SUPPORTED`, `FALSIFIED`, `INCONCLUSIVE`, and
`SUPERSEDED`. Parameter sweeps within one causal explanation remain one hypothesis.

### Attempt budget

- **Attempt 0 — no implementation:** observe, capture baseline, identify ownership,
  form a hypothesis, and run the cheapest falsification test.
- **Attempt 1 — micro-patch:** allowed only when the hypothesis survives Attempt 0.
  Change one causal variable or tightly coupled group and validate immediately.
- **Attempt 2 — second hypothesis/final local iteration:** allowed only with new
  evidence, a materially changed hypothesis, or a changed failure signature.

If the same failure signature survives Attempt 2, stop implementation and enter
Diagnostic Mode. A third blind implementation is forbidden.

### Validation and rollback

Validate the smallest affected surface immediately. A failed candidate must produce
information, then be rolled back unless independently useful and justified.

```text
BASELINE -> HYPOTHESIS -> EXPERIMENT -> MICRO-PATCH -> CANDIDATE
         -> DELTA -> PASS | FAIL | INCONCLUSIVE
```

A behavioral candidate without a relevant baseline is normally `INCONCLUSIVE`.
Structural or binary fixes may use direct pass/fail evidence.

## Stop conditions and Diagnostic Mode

Stop production implementation when the same signature survives two implementation
attempts, no measurable acceptance criterion exists, ownership is unknown, the next
change cannot be isolated, evidence contradicts the patch, a workaround would be
stacked, baseline noise hides the regression, validation tooling is broken, or
active context is contradictory.

Diagnostic Mode means **no production patches**. Inspect code, configuration,
history, known incidents and telemetry; run parsers, offline models, minimal
reproductions and isolated tests. Return to implementation only after evidence
supports a causal micro-patch.

## Domain routing

- handling/chassis/tires/suspension/brakes: `vehicle-physics`
- physics failure diagnosis: `physics-diagnostics`
- aerodynamics: `aerodynamics`
- engine/transmission: `powertrain`
- telemetry: `telemetry`
- `.tscn`: `scene-safety`
- known failures: `problem-lookup`
- uncertain APIs: `knowledge-query`
- baseline/candidate comparison: `regression-validation`
- unclear ownership/cross-system flow: `repo-analysis`
- repeated signature/workaround pressure: `problem-solving-guardrails`
- delegation: `context-handoff`
- falsified hypothesis/stale context: `context-garbage-collection`

## Safety and context

- Preserve unrelated dirty-worktree changes.
- Do not change vendor GEVP for Formula-90 tuning without evidence.
- Keep `.tscn` edits surgical and preserve `node_paths`, references, and owners.
- Validate RayCast origins, ground clearance, scene load, and referenced resources.
- Visual acceptance requires an image-capable reviewer or human confirmation.
- Never refactor a subsystem as a debugging tactic without evidence.

Before a complex handoff, run Context GC and use `context-handoff`. Preserve verified
facts, constraints, failure signature, hypothesis state, baseline, validation and
rollback conditions; discard raw tool noise and superseded reasoning.
