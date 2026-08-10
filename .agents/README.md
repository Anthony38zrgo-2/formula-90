# Formula-90 Agent Runtime POC

This directory is the single control plane for agentic development in Formula-90.

## Design goals

- Keep the LLM working set small.
- Use SQLite for fast local persistence and exact indexed lookups.
- Keep human-reviewable knowledge in versioned JSON.
- Use Rust only for runtime database access.
- Keep ETL, embeddings, normalization, and analytics out of the hot path.
- Treat skills as procedures, not as a dumping ground for project facts.
- Treat common failures as indexed knowledge that can be queried before retrying.
- Keep model names outside role definitions. Roles request `fast` or `strong` tiers.

## Runtime model

```text
Agent
  |
  +--> agentdb instruction ...
  +--> agentdb knowledge ...
  +--> agentdb problem ...
  +--> agentdb agent ...
  +--> agentdb skill ...
  |
  v
Rust agentdb
  |
  +--> in-process result
  v
.agents/data/agents.db (SQLite, WAL)
```

Version-controlled JSON is the curated/canonical layer. SQLite is the compiled runtime representation.

## Bootstrap

From repository root:

```text
cd .agents/runtime
cargo build --release
cd ../..
.agents/runtime/target/release/agentdb init
.agents/runtime/target/release/agentdb seed
.agents/runtime/target/release/agentdb validate
```

The database path defaults to `.agents/data/agents.db`. Override with `AGENT_DB`.
The agent root defaults to `.agents`. Override with `AGENTS_ROOT`.

## Query examples

```text
agentdb instruction godot
agentdb knowledge godot RigidBody3D
agentdb knowledge cpp ownership
agentdb problem node_paths
agentdb agent developer-godot
agentdb skill scene-safety
```

All output is compact JSON so another model/tool can consume it with minimal token overhead.

## Knowledge scope for this POC

Only these channels are enabled:

- `godot`
- `blender`
- `cpp`
- `python`

The JSON files are intentionally small. They are indexes and curated notes, not copies of entire manuals.
