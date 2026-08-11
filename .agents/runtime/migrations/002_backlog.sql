PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS backlog_items (
    id TEXT PRIMARY KEY,

    epic TEXT,
    title TEXT NOT NULL,
    description TEXT NOT NULL,

    item_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'proposed',

    priority INTEGER NOT NULL DEFAULT 50,
    sort_order INTEGER NOT NULL DEFAULT 0,

    rationale TEXT,
    technical_risk TEXT,
    user_value TEXT,

    dependencies_json TEXT NOT NULL DEFAULT '[]',
    affected_areas_json TEXT NOT NULL DEFAULT '[]',
    acceptance_criteria_json TEXT NOT NULL DEFAULT '[]',
    evidence_json TEXT NOT NULL DEFAULT '[]',

    source_agent TEXT,
    source_context TEXT,

    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_backlog_status_priority
ON backlog_items(status, priority DESC, sort_order ASC);

CREATE INDEX IF NOT EXISTS idx_backlog_epic
ON backlog_items(epic);
