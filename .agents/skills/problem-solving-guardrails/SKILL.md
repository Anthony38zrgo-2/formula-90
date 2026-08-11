---
name: problem-solving-guardrails
description: Enforce hypothesis-based debugging, the canonical implementation-attempt budget, stop conditions, rollback, and Diagnostic Mode when evidence stops improving.
---

# Problem-Solving Guardrails

## Attempt budget

- Attempt 0: no production patch. Establish baseline, ownership, hypothesis, falsification test, expected signal, and failure signature.
- Attempt 1: one reversible causal micro-patch after the hypothesis survives Attempt 0.
- Attempt 2: final local implementation only with new evidence, a materially different hypothesis, or a changed failure signature.

Same hypothesis plus same failure signature forbids another patch. After two implementation attempts with an equivalent signature, enter Diagnostic Mode.

## Failed experiments

A failure must reject a hypothesis, verify a fact, narrow the search, change the failure signature, or identify ownership. Otherwise redesign the experiment before continuing.

On failure: record evidence, rollback the candidate, run Context GC, then derive the next hypothesis from what changed in the problem model.

## Diagnostic Mode

Make no production patches. Inspect ownership, configuration, logs, telemetry, history, parsers, offline models, and minimal reproductions. Return to implementation only when evidence supports an isolated causal patch.

Escalate immediately when ownership is unknown, evidence contradicts the assumed model, validation is broken, a workaround would stack on another workaround, active context is contradictory, or a local fix creates a symmetric regression.
