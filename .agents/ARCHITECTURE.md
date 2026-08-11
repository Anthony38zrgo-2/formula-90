# POC Architecture

## Scope

The POC solves one narrow problem: give Formula-90 agents a fast, local, deterministic way to retrieve project instructions, curated manual knowledge, known problems, skill metadata, role metadata and short-lived context without loading large files into the LLM context.

## Agent Runtime Bootstrap

The `.agents` runtime must bootstrap itself from any working directory inside
the repository, including Git worktrees and fresh checkouts.

- **Repository root discovery**: `git rev-parse --show-toplevel` (executed from
  `$PSScriptRoot`, so CWD never matters), with a bounded parent-directory
  `.git` search (file or directory, so worktrees work) as fallback. All paths
  are then normalized absolute: `RepoRoot`, `AgentsRoot=<repo>\.agents`,
  `AgentDbPath=<repo>\.agents\data\agents.db`, `RuntimeRoot=<repo>\.agents\runtime`.
- **Project-local binary**: `agentdb` is built into
  `<repo>\.agents\runtime\target\release\agentdb.exe`. `runtime/target/` is
  Git-ignored and never committed.
- **Automatic local build**: when the binary is absent and `Cargo.toml` exists,
  `cargo build --release` runs from `.agents/runtime`. No administrator
  privileges, no global install, no system PATH modification.
- **Resolution order** (centralized in `_common.ps1` -> `Resolve-AgentDb`):
  `AGENTDB_EXE` override -> project release -> project debug (temporary
  fallback) -> PATH -> local build. `source` is one of `explicit_env`,
  `project_release`, `project_debug`, `system_path`, `built_release`.
- **Binary validation**: finding a file is not enough. The resolved binary is
  executed (`agentdb stats`, with an idempotent `init` retry on fresh
  databases) under absolute `AGENTS_ROOT`/`AGENT_DB` before it is reported
  available.
- **Absolute database path**: every PowerShell wrapper executes agentdb with
  `AGENTS_ROOT=<repo>\.agents` and `AGENT_DB=<repo>\.agents\data\agents.db`
  (set at `_common.ps1` dot-source time and again in `Invoke-AgentDb`). The
  database is never resolved against the shell CWD.
- **Fresh database**: a missing `agents.db` is initialized reproducibly with
  `init` + `seed` + `validate` (only appropriate for a fresh workspace). An
  existing database is never deleted or reseeded because resolution failed;
  binary availability and database state are separate concerns.
- **Bootstrap result**: `Resolve-Bootstrap` returns
  `{ ok, repo_root, agents_root, agentdb: { available, path, source, built_now },
  database: { path, available } }`, consumed by preflight, `00-env.ps1` and
  `invoke-agent.ps1`.
- **Fallback policy**: `knowledge_hit` (query ok, result exists),
  `knowledge_miss` (query ok, no result) and `knowledge_unavailable` (agentdb
  itself down) are distinct states. Significant engineering work fails closed
  when bootstrap fails; trivial/read-only work may fall back but must report
  `AGENT INFRASTRUCTURE UNAVAILABLE` honestly.
- **Telemetry**: event types `infrastructure_bootstrap`,
  `infrastructure_fallback` and `knowledge_unavailable` are accepted by the
  `telemetry_events` table. `agentdb event ... --source <src>` overrides the
  default `cli` source. `knowledge_unavailable` is never counted as
  `knowledge_miss`.

## Experimental Telemetry Semantics

### `run_kind` (agent_runs)

Classifies every run so synthetic validation never distorts primary metrics.
Constrained set: `productive`, `probe`, `control`, `instrumentation`,
`bootstrap`, `sync`, `unknown`.

- `productive` — real project engineering work (feature, bug fix, refactor,
  documentation-as-work, architecture, review, polish, validation of a real
  backlog item).
- `probe` — manual/model-routing capability probes (PROBE-*).
- `control` — deterministic synthetic routing cases (13-routing-control.ps1).
- `instrumentation` — synthetic proof of knowledge/problem/cache counters.
- `bootstrap` — tests of root discovery / agentdb resolution / auto-build.
- `sync` — imported from an external/unmanaged environment.

A synced Desktop feature is REAL work: `run_kind = productive` with
`ingestion_source = desktop_sync`. Ingestion mechanism and work semantics are
separate concepts (migration 005).

### `experiment_phase` (agent_runs)

Experiment boundary. Initial values: `bootstrap-poc`, `reactive-router`,
`contract-first`, `unclassified`. The project default lives in
`.agents/config/experiment.json` (`current_phase`); `run-start` reads it when
`--phase` is omitted. Historical runs are classified `reactive-router` by
deterministic backfill rules only (never guessed from timestamps).

### `capacity_source` (model_calls)

WHY a model was allocated. Mutually exclusive values: `normal`,
`automatic_escalation`, `manual_override`, `planned_capacity`, `unknown`.
Strong usage itself is derived from `model_tier` (never stored redundantly);
`capacity_source` is persisted context. Derivation order at `model-call start`:
explicit metadata value -> `routing_source=user_override` -> routing-rule map
(`attempt_budget_exhausted`/`architecture_change`/`cross_subsystem_change`/
`high_technical_risk`/`large_regression` -> automatic_escalation;
`role_product_owner`/`role_architect`/`role_retrospective` -> planned_capacity)
-> `normal`. PO/Senior Architect strong allocation is planned capacity, not
failure escalation.

### Metrics

`agentdb metrics [all|productive] [phase <phase>]`. Reports separately
`strong_usage_rate`, `automatic_escalation_rate`, `manual_override_rate`,
`planned_capacity_rate`, `productive_knowledge_usage_rate` (productive runs
with >=1 knowledge query / productive runs) and a `knowledge_consumption`
breakdown (productive vs synthetic vs planning/execution prefetch). Never
report "strong escalation" when what is measured is strong usage.

## Automatic Context Prefetch

`agentdb context-packet <backlog-id> <planning|execution>` prepares bounded
deterministic context for the future planner/executor. It never invokes an
LLM and does not implement Execution Contracts.

```text
backlog item
      |
      v
derive bounded search terms (max 10, from title/description/epic/rationale/
areas/dependencies/acceptance criteria)
      |
      v
context-packet (reuses the SAME internal query functions as
               `agentdb knowledge` / `agentdb problem` / `agentdb instruction`)
      |
      +-> knowledge: 1 query per enabled channel, top 8 merged hits
      +-> common problems: top 5 hits
      +-> instructions: global + scope instructions, top 8
      |
      v
bounded packet JSON (backlog summary, knowledge, problems, instructions,
commit, experiment_phase, meta with real query counters)
      |
      v
future planner / executor
```

Telemetry: under `AGENT_RUN_ID` the packet's internal queries emit normal
`knowledge_hit|miss` / `problem_hit|miss` events with
`source = planning_prefetch` or `execution_prefetch`, incrementing the
standard `agent_runs` counters — automatic consumption is measurable
separately from agent-initiated lookups. Without a run id the packet still
works and telemetry is skipped.

## Storage policy

### SQLite: runtime persistence

One local database: `.agents/data/agents.db`.

It is intentionally not committed to Git. It is rebuilt/updated from reviewed JSON and then persists locally across agent tasks. This avoids binary merge conflicts while retaining fast runtime state.

### JSON: human-governed canonical inputs

Version-controlled JSON stores:
- role manifests;
- skill registry metadata;
- curated knowledge entries;
- common problems;
- global/project instructions.

Humans can review these diffs before promotion. The runtime never needs to parse every JSON file during normal task execution; only `seed` does that.

## Tables

### `instructions`
Hot lookup of global/domain instructions. Deliberately denormalized.

Primary access:
`scope + trigger -> ordered instruction packet`

### `knowledge_entries`
Compact curated knowledge snippets. One row is already suitable for injection into an LLM context.

Primary access:
`channel + exact lookup_key -> entry`

### `knowledge_terms`
Small inverted index. This is not domain normalization; it is a read-optimization table generated during seed.

Primary access:
`term -> entry ids`

### `common_problems`
Known failure signatures with symptom, cause, solution, prevention and confidence.

Primary access:
`signature -> problem`

### `problem_terms`
Inverted index for symptom/error vocabulary.

### `skill_registry`
Maps discoverable skill ids to `SKILL.md` paths and trigger metadata.

### `agent_registry`
Maps role ids to model tier, skills and knowledge channels.

### `context_cache`
Short-lived JSON Context Bundles between agents. It replaces scattered temporary instruction files.

### `sources`
Tracks the curated source/channel represented by each manual JSON.

## Hot path

```text
Task
  -> agentdb agent <role>
  -> agentdb instruction <scope> [trigger]
  -> optional agentdb problem <term>
  -> optional agentdb knowledge <channel> <term>
  -> LLM works
```

No ETL, embeddings, bulk-document parsing, retrospectives or manual downloads occur here.

## Write path

```text
Human-reviewed JSON change
  -> agentdb seed
  -> single SQLite transaction
  -> runtime queries immediately use the new rows
```

Context-cache writes are the only routine task-time writes in this POC.

## Why the inverted-index tables exist

A CSV-like scan is avoided. `knowledge_terms` and `problem_terms` trade a little duplicated disk data for exact indexed lookup and small result packets. This matches the goal of reducing agent latency and token use rather than optimizing relational purity.

## Deliberate non-goals for POC v0.1

- vector database;
- embeddings;
- Data Lake ETL;
- DuckDB/Parquet;
- network service;
- distributed locking;
- automatic web crawling;
- automated promotion of LLM-generated knowledge;
- committing SQLite binary state to Git.

These should only be introduced after measured need.

## Model routing and telemetry (increment 2)

### Routing policy

Deterministic routing lives in `config/model-routing.json` and is executed by
`agentdb route '<task-metadata-json>'`. The LLM never picks the model; software
decides, and the decision is persisted as a `routing_decision` event plus the
`routing_rule`/`routing_reason`/`routing_source` columns of `model_calls`.

Order of evaluation:
1. user override (`user_model_override` in task metadata);
2. hard Sol rules (product-owner/architect/retrospective roles, architecture
   change, cross-subsystem >= 3, attempt budget exhausted, high risk, ...);
3. bounded read-only/lookup/test -> Luna low;
4. routine implementation -> DeepSeek Flash, fallback Luna medium;
5. bounded debugging after first failure -> Luna high;
6. second unresolved failure -> Sol medium.

Never escalate merely for large context, many files, or pre-tooling uncertainty.

### Execution flows

Two supported paths:

- **Managed path**: `scripts/invoke-agent.ps1` routes, invokes the provider
  (`codex exec -p <profile>` for openai_codex, `opencode run` for deepseek),
  captures measured token usage and records `model_calls` telemetry. A run is
  always closed (`run-end`) in a `finally` block, including early failures.
- **Codex Desktop path**: desktop sessions bypass invoke-agent.ps1 and are not
  recorded automatically. Use `scripts/14-sync-codex-session.ps1` (read-only
  JSONL sync) to register them afterwards. It records the effective model from
  `thread_settings_applied` runtime evidence with `verification_status=verified`
  and `routing_source=user_override` for manual selections.

### Telemetry rules

- `requested_model`/`requested_effort` and `effective_model`/`effective_effort`
  are stored separately; `verification_status` is only `verified` when runtime
  evidence exists. The installed Codex CLI does not expose the effective model,
  so Codex calls are reported `unverified` by design.
- Token counts are only persisted when the provider exposes measured usage
  (`codex exec` `turn.completed`, `opencode` `step_finish`/`export`). Never
  estimated.
- Cost is derived later from a pricing snapshot, never stored as primary data.
- `AGENT_RUN_ID` associates child tool calls (`knowledge`, `problem`,
  `cache-get`) with the active run automatically.

### Verification state

`scripts/11-routing-status.ps1` shows policy, provider availability, Codex
version, profiles and per-profile requested/effective verification state.
`scripts/12-telemetry-summary.ps1` renders the dashboard; `scripts/13-routing-control.ps1`
executes the deterministic control cases (including no-escalation regressions).
