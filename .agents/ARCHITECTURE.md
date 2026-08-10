# POC Architecture

## Scope

The POC solves one narrow problem: give Formula-90 agents a fast, local, deterministic way to retrieve project instructions, curated manual knowledge, known problems, skill metadata, role metadata and short-lived context without loading large files into the LLM context.

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
