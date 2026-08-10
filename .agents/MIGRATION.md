# Migration Plan

## Current repository -> POC

1. Replace the root `AGENTS.md` with the tiny bootstrap bridge included in this POC.
2. Keep all operational content under `.agents/`.
3. Replace the core referenced skills with the procedural versions in `.agents/skills/`.
4. Add `physics-diagnostics`, which is referenced by the current bootstrap but was absent at the inspected path.
5. Move project-specific incident facts out of generic skills and into `knowledge/common-problems.json`.
6. Deprecate `instrucciones.txt` handoffs; use `context_cache` JSON bundles.
7. Build `agentdb`, run `init`, `seed`, then `validate`.
8. Only after validation should this agent-system change be committed.

## Promotion rule

New knowledge must enter through reviewed JSON first. The LLM may propose an entry, but a human/project-authoritative process should validate it before it becomes canonical knowledge.

## Future step, not part of POC

When enough trajectories exist, add a cold Data Lake and retrospective pipeline outside the task hot path. Do not couple those systems to normal lookups.
