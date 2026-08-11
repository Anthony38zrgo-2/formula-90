---
name: repo-analysis
description: Perform bounded repository analysis for unclear ownership, cross-system failures, migrations, architecture changes, or regressions without defaulting to exhaustive exploration.
---

# Bounded Repository Analysis

Inspect only enough to identify execution flow, ownership, constraints, unknowns, and the cheapest useful experiment.

1. Read root `AGENTS.md`.
2. Identify the affected subsystem and observable failure.
3. Inspect authoritative configuration and relevant recent history.
4. Check known incidents and indexed problems.
5. Trace who owns, computes, mutates, consumes, and serializes the state.
6. List remaining unknowns.
7. Define the failure signature.
8. Design the cheapest falsification experiment.
9. Expand analysis only when evidence requires it.

Do not require understanding the entire repository for a focused task. Do not create a handoff artifact unless work is actually delegated or resumed; then use `context-handoff`.
