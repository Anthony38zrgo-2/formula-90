# Formula-90 Agent Guide

## Default workflow

1. Identify expected observable behavior and its owner.
2. Make the smallest reversible change that tests the hypothesis.
3. Test what changed.
4. Keep the improvement or revert it and continue.

Validation is proportional to risk: low-risk changes need an affected-behavior test; local multi-file or scene changes need targeted validation; broad architecture, migration, replacement, or repeated unexplained failures need diagnosis and relevant regression checks.

Tune observable vehicle behavior, not parameter purity. Prefer progressive turn-in, recoverable oversteer, understandable weight transfer, stable braking, predictable curbs, and high-speed stability when aero is active.

## Diagnostic escalation

Escalate only after three meaningful failed hypotheses, a recurring bug, unclear ownership, contradictory behavior, an unexpected secondary regression, or a proposed workaround/speculative refactor. Then inspect evidence, logs, telemetry, offline tools, and history only when useful; apply the smallest root-cause fix.

Never refactor a repository or subsystem as a debugging tactic unless evidence identifies the architecture as the cause.

## Safety

- Preserve unrelated work in a dirty worktree.
- Do not change vendor GEVP for Formula90s tuning without evidence.
- Edit `.tscn` files surgically: preserve `node_paths`, references, and unrelated nodes; load the affected scene afterward.
- For wheel/chassis geometry, check RayCast origin, ground clearance, and scene load.
- Visual acceptance requires an image-capable reviewer or explicit human confirmation.

## Load skills only when relevant

- handling/tuning: `vehicle-physics`
- aero: `aerodynamics`
- engine/transmission: `powertrain`
- `.tscn` structure: `scene-safety`
- repeated failure/unclear root cause: `problem-solving-guardrails`
- cross-system investigation/migration: `repo-analysis`

Telemetry and regression validation are optional during normal iteration; use them for uncertainty, milestone freeze, or high-risk change.

## Handoffs

Complete clear tasks directly. Delegate only when useful. Default handoffs use only:

```text
# OBJECTIVE
# RELEVANT FILES
# CURRENT EVIDENCE
# CHANGE / CONSTRAINTS
# DONE WHEN
```

For large handoffs or repeated failures, pass only current facts, constraints, relevant files, and useful rejected hypotheses.
