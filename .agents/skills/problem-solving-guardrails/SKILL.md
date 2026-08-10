---
name: problem-solving-guardrails
description: Detect brute-force debugging loops and switch execution from guessing to evidence-driven diagnosis.
---

# Problem Solving Guardrails

- Attempt 1: direct, bounded implementation is allowed.
- If Attempt 1 fails: query `agentdb problem` and gather new evidence.
- If an equivalent Attempt 2 fails: stop implementation and escalate to diagnostic mode.
- Strong-model escalation is reserved for unresolved root cause, architectural uncertainty, or cross-system risk.

Diagnostic mode:
1. observed vs expected;
2. common-problem lookup;
3. ownership and dependency check;
4. one falsifiable hypothesis;
5. one experiment that reduces uncertainty;
6. minimal fix only after root cause evidence.

Project-specific failure facts must live in the common-problem index, not inside this generic procedure.
