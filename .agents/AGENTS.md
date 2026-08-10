# Formula-90 Agent Operating Contract

This file is the authoritative bootstrap for every AI agent working in this repository.

## 1. Fast-path rules

1. Do not load complete manuals into context.
2. Query `.agents/data/agents.db` through the Rust `agentdb` binary.
3. Query exact/project knowledge before relying on model memory when:
   - an API/symbol is uncertain;
   - a framework/version detail matters;
   - a known failure may match;
   - the same attempt has failed once already.
4. Do not run ETL, embeddings, retrospectives, or bulk indexing in the task hot path.
5. A failed attempt must reduce uncertainty. Two equivalent failures trigger diagnostic mode.

## 2. Authority order

Use this order when information conflicts:

1. Explicit user/product decision.
2. Current repository state and verified runtime evidence.
3. Curated project instructions in SQLite/JSON.
4. Official manual knowledge indexed under the matching channel/version.
5. Validated common-problem entries.
6. Model parametric memory.

Never override a higher-authority source with a plausible recollection.

## 3. Role routing

Read the role manifest with:

`agentdb agent <agent-id>`

Model tiers:
- `fast`: default execution, tests, routine fixes, lookups, review.
- `strong`: product prioritization, architecture escalation, retrospective/root-cause synthesis.

The Product Owner orders product work. The Scrum Master protects execution flow and attempt budgets; it does not replace product priority decisions.

## 4. Skills

Skills are procedural memory. Discover them through `agentdb skill <skill-id>`.

Facts, incidents, version-specific API notes, and project traps belong in knowledge/common-problem JSON and SQLite, not embedded permanently inside procedural skills.

## 5. Context handoff

Context handoffs use the `context_cache` table instead of ad-hoc `instrucciones.txt` files.
Only verified facts, current constraints, relevant symbols/files, rejected hypotheses, validation commands, and acceptance criteria should survive a handoff.

## 6. Formula-90 project constraints

- Behavior-first vehicle work.
- Do not guess physics from variable names.
- Validate actual runtime configuration and test evidence.
- Do not claim success without evidence.
- Godot scene edits require structural validation.
- Repository-relative paths only in handoffs.
