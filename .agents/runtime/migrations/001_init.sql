PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS sources (
    source_id TEXT PRIMARY KEY,
    channel TEXT NOT NULL,
    authority INTEGER NOT NULL,
    version_policy TEXT NOT NULL,
    source_type TEXT NOT NULL,
    source_ref TEXT NOT NULL
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS idx_sources_channel ON sources(channel, authority DESC);

CREATE TABLE IF NOT EXISTS instructions (
    id TEXT PRIMARY KEY,
    scope TEXT NOT NULL,
    trigger TEXT,
    priority INTEGER NOT NULL DEFAULT 100,
    body TEXT NOT NULL,
    source_ref TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS idx_instructions_scope
ON instructions(scope, enabled, priority DESC);

CREATE TABLE IF NOT EXISTS knowledge_entries (
    id TEXT PRIMARY KEY,
    channel TEXT NOT NULL,
    lookup_key TEXT NOT NULL,
    topic TEXT NOT NULL,
    content TEXT NOT NULL,
    symbols_json TEXT NOT NULL DEFAULT '[]',
    keywords_json TEXT NOT NULL DEFAULT '[]',
    source_id TEXT NOT NULL,
    source_ref TEXT,
    source_version TEXT,
    authority INTEGER NOT NULL DEFAULT 100,
    enabled INTEGER NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_knowledge_lookup
ON knowledge_entries(channel, lookup_key, source_version);
CREATE INDEX IF NOT EXISTS idx_knowledge_channel
ON knowledge_entries(channel, enabled, authority DESC);

-- Deliberately denormalized inverted index for the hot path.
CREATE TABLE IF NOT EXISTS knowledge_terms (
    term TEXT NOT NULL,
    entry_id TEXT NOT NULL,
    weight INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY(term, entry_id)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS common_problems (
    signature TEXT PRIMARY KEY,
    domain TEXT NOT NULL,
    symptom TEXT NOT NULL,
    cause TEXT NOT NULL,
    solution TEXT NOT NULL,
    prevention TEXT NOT NULL,
    search_terms_json TEXT NOT NULL DEFAULT '[]',
    confidence INTEGER NOT NULL DEFAULT 500,
    occurrences INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'active',
    source_ref TEXT,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS idx_common_problems_domain
ON common_problems(domain, status, confidence DESC);

CREATE TABLE IF NOT EXISTS problem_terms (
    term TEXT NOT NULL,
    signature TEXT NOT NULL,
    weight INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY(term, signature)
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS skill_registry (
    id TEXT PRIMARY KEY,
    manifest_path TEXT NOT NULL,
    triggers_json TEXT NOT NULL DEFAULT '[]',
    knowledge_channels_json TEXT NOT NULL DEFAULT '[]',
    risk TEXT NOT NULL DEFAULT 'low',
    enabled INTEGER NOT NULL DEFAULT 1
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS agent_registry (
    id TEXT PRIMARY KEY,
    role TEXT NOT NULL,
    model_tier TEXT NOT NULL,
    manifest_path TEXT NOT NULL,
    purpose TEXT NOT NULL,
    skills_json TEXT NOT NULL DEFAULT '[]',
    knowledge_channels_json TEXT NOT NULL DEFAULT '[]',
    enabled INTEGER NOT NULL DEFAULT 1
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS context_cache (
    cache_key TEXT PRIMARY KEY,
    scope TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    expires_at INTEGER,
    hits INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS idx_context_cache_scope
ON context_cache(scope, expires_at);
