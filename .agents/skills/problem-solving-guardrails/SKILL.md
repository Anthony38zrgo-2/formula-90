---
name: problem-solving-guardrails
description: Keep normal iteration fast and escalate only when evidence says guessing has stopped helping.
---
# Fast Iteration
1. Form a hypothesis.
2. Make the smallest reversible change.
3. Test it.
4. Keep it if improved; revert it if worse.
5. Record discoveries only when useful.

# Diagnostic Escalation
Escalate after three meaningful failed hypotheses, recurring failure, unclear ownership, contradictory results, or when the next step is a workaround/speculative refactor. Inspect evidence and ownership; use logs, telemetry, offline tools, and git history only when they reduce uncertainty; apply the smallest root-cause fix.
