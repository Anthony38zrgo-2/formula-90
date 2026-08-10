---
name: context-garbage-collection
description: Compact active context to verified facts, current constraints and useful failed hypotheses before handoff or escalation.
---

# Context Garbage Collection

The context window is working memory, not historical storage.

Preserve:
- verified facts and evidence;
- current constraints;
- relevant files/symbols;
- unresolved unknowns;
- compact rejected hypotheses;
- validation and acceptance criteria.

Remove:
- superseded values presented as current;
- raw terminal/tool noise already summarized;
- duplicated explanations;
- irrelevant historical discussion.

The compact result should be written to `context_cache` through `agentdb cache-put`, not to scattered temporary instruction files.
