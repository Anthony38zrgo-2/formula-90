---
name: context-garbage-collection
description: Compact active debugging context after a falsified hypothesis, failed candidate, major handoff, attempt-budget exhaustion, contradiction, or excessive context growth.
---

# Context Garbage Collection

Run Context GC after every meaningful hypothesis falsification and before complex
user-approved handoffs. Preserve only information that changes the next decision.

Context GC never authorizes delegation or escalation. Do not promote, escalate,
hand off, delegate, or transfer work to another agent, subagent, model, or
higher-capability model unless the user explicitly requested that exact action
in the current task.

For each rejected hypothesis, emit:

```text
REJECTED HYPOTHESIS:
EVIDENCE:
NEW VERIFIED FACT:
STATUS: Do not revisit without new evidence.
```

Then retain:

- objective and expected behavior;
- current failure signature and baseline;
- verified facts and active constraints;
- current unknowns and ownership state;
- supported/current hypothesis and its status;
- cheapest next falsification test and expected signal;
- validation and rollback conditions.

Remove raw tool output already summarized, superseded hypotheses, repeated narrative, stale file lists, abandoned plans, and assumptions contradicted by evidence. Do not erase failed experiments; compress the information they produced.

If active facts contradict each other, mark the contradiction and enter Diagnostic Mode instead of choosing whichever fact supports the next patch.
