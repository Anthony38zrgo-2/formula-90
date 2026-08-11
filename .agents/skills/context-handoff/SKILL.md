---
name: context-handoff
description: Create evidence-focused Context Bundles for delegation, task resumption, planner-to-executor transitions, or complex debugging handoffs without passing raw or obsolete context.
---

# Context Handoff

Run `context-garbage-collection` first. Write the handoff in English unless the receiver explicitly requires another language. Include only task-relevant evidence.

Use this mandatory structure:

```text
# OBJECTIVE
# OBSERVED PROBLEM
# EXPECTED BEHAVIOR
# FAILURE SIGNATURE
# CURRENT EVIDENCE
# VERIFIED FACTS
# CURRENT HYPOTHESIS
# HYPOTHESIS STATUS
# CHEAPEST FALSIFICATION TEST
# EXPECTED SIGNAL
# RELEVANT FILES
# RELEVANT SYMBOLS
# OWNERSHIP
# DEPENDENCIES
# KNOWN CONSTRAINTS
# DO NOT MODIFY
# REJECTED HYPOTHESES
# VALIDATION COMMANDS
# BASELINE
# ACCEPTANCE CRITERIA
# ATTEMPT / HYPOTHESIS BUDGET
# ROLLBACK CONDITIONS
# ESCALATION CONDITIONS
```

Separate observations from interpretations. Use hypothesis states `UNTESTED`, `SUPPORTED`, `FALSIFIED`, `INCONCLUSIVE`, or `SUPERSEDED`. Do not include raw terminal transcripts, obsolete plans, duplicated history, or a rejected hypothesis without its evidence.

If ownership, acceptance criteria, or the cheapest falsification test is unknown, state that explicitly; do not hide missing prerequisites inside implementation steps.
