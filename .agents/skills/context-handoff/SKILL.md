---
name: context-handoff
description: Produce a compact machine-readable Context Bundle for delegation without propagating raw history.
---

# Context Handoff

Create a JSON Context Bundle containing:

- objective
- observed_problem
- expected_behavior
- evidence
- relevant_files
- relevant_symbols
- dependencies
- constraints
- do_not_modify
- rejected_approaches
- validation_commands
- acceptance_criteria
- attempt_budget
- rollback_conditions

Store it with:

`agentdb cache-put <scope> <key> '<json>' <ttl-seconds>`

The receiving agent loads only that bundle plus explicitly routed skills/knowledge.
