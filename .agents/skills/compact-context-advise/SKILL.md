---
name: compact-context-advise
description: Advise on compacting Formula-90 task context after a feature is implemented and validated. Use at feature completion, before a new feature, after a validated milestone, or when accumulated tool output is no longer needed. Always request explicit user authorization before context compaction; never compact, hand off, discard active evidence, or change files automatically.
---

# Compact context advise

Use this skill only after the feature has a meaningful completion state: accepted,
failed and rolled back, blocked with evidence, or intentionally paused.

## Required closeout check

Before advising, establish:

- the completed scope and current branch/worktree state;
- validation result: `PASS`, `FAIL`, `INCONCLUSIVE`, or `NOT RUN`;
- preserved artifacts: changed paths, commands, logs, baselines, and unresolved
  risks;
- whether a continuing task needs any of the detailed reasoning still in context.

Do not offer compaction as a substitute for validation or a way to hide an
inconclusive result.

## Authorization gate

At the end of the feature report, state the recommendation and ask exactly one
clear approval question. Use this compact form:

```text
Contexto: [recomendado/no recomendado] para compactacion.
Se conservaran: [verified facts, artifacts, constraints, remaining risk].
Autorizas compactar el contexto de esta feature? (si/no)
```

Do not compact when the user has not answered, gives an ambiguous answer, asks to
continue the same deep diagnostic path, or the result is `INCONCLUSIVE` and the
missing evidence is still required immediately.

If the user explicitly authorizes it in the current task, invoke
`context-garbage-collection` and retain only the verified facts, acceptance
criteria, current constraints, changed paths, validation evidence, unresolved
risks, and next decision. Otherwise, keep the context intact and continue.

Never delete files, alter Git state, create a handoff, delegate, or claim that
compaction occurred merely because it was recommended.
