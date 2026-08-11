PRAGMA foreign_keys = ON;

-- =====================================================================
-- Telemetry tables (additive, do not alter existing schema)
-- =====================================================================

CREATE TABLE IF NOT EXISTS agent_runs (
    run_id TEXT PRIMARY KEY,

    task_id TEXT,
    backlog_id TEXT,

    agent_id TEXT NOT NULL,
    role TEXT,

    started_at INTEGER NOT NULL,
    finished_at INTEGER,

    outcome TEXT,

    commit_sha TEXT,

    model_calls INTEGER NOT NULL DEFAULT 0,
    retries INTEGER NOT NULL DEFAULT 0,

    knowledge_queries INTEGER NOT NULL DEFAULT 0,
    knowledge_hits INTEGER NOT NULL DEFAULT 0,

    problem_queries INTEGER NOT NULL DEFAULT 0,
    problem_hits INTEGER NOT NULL DEFAULT 0,

    cache_queries INTEGER NOT NULL DEFAULT 0,
    cache_hits INTEGER NOT NULL DEFAULT 0,

    input_tokens INTEGER NOT NULL DEFAULT 0,
    cached_input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,

    duration_ms INTEGER
);

CREATE INDEX IF NOT EXISTS idx_agent_runs_agent
ON agent_runs(agent_id, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_agent_runs_task
ON agent_runs(task_id);

CREATE INDEX IF NOT EXISTS idx_agent_runs_outcome
ON agent_runs(outcome, started_at DESC);

CREATE TABLE IF NOT EXISTS model_calls (
    call_id TEXT PRIMARY KEY,

    run_id TEXT NOT NULL,

    provider TEXT NOT NULL,

    requested_model TEXT,
    effective_model TEXT,

    requested_effort TEXT,
    effective_effort TEXT,

    model_tier TEXT,

    purpose TEXT,

    routing_rule TEXT,
    routing_reason TEXT,

    verification_status TEXT NOT NULL DEFAULT 'unverified',

    started_at INTEGER NOT NULL,
    duration_ms INTEGER,

    input_tokens INTEGER,
    cached_input_tokens INTEGER,
    output_tokens INTEGER,

    success INTEGER NOT NULL,

    FOREIGN KEY(run_id)
        REFERENCES agent_runs(run_id)
);

CREATE INDEX IF NOT EXISTS idx_model_calls_run
ON model_calls(run_id);

CREATE INDEX IF NOT EXISTS idx_model_calls_model
ON model_calls(effective_model);

CREATE INDEX IF NOT EXISTS idx_model_calls_requested
ON model_calls(requested_model);

CREATE TABLE IF NOT EXISTS telemetry_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    run_id TEXT,

    ts INTEGER NOT NULL DEFAULT (unixepoch()),

    event_type TEXT NOT NULL,

    source TEXT,
    key TEXT,

    value INTEGER,

    detail TEXT
);

CREATE INDEX IF NOT EXISTS idx_telemetry_run
ON telemetry_events(run_id, event_type);

CREATE INDEX IF NOT EXISTS idx_telemetry_type
ON telemetry_events(event_type, ts DESC);
