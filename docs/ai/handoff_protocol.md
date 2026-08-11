# Handoff Protocol

## Purpose

Transfer precise, verified execution context without raw conversational noise or obsolete reasoning.

## Context GC gate

Before a complex handoff, run `context-garbage-collection`:

1. retain verified facts, baseline, constraints, ownership, and unknowns;
2. compress rejected hypotheses with their evidence;
3. remove superseded information and raw tool noise;
4. identify contradictions instead of silently resolving them.

## Mandatory bundle

Use every header in `handoff_template.md`. A header may say `UNKNOWN` or `NOT APPLICABLE`, but must not disappear. Unknown ownership, missing acceptance criteria, or broken validation are stop conditions rather than implementation instructions.

Write handoffs in English unless the receiving workflow explicitly requires another language.
