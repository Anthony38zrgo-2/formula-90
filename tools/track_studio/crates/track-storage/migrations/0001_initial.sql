PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS project (
    track_id TEXT PRIMARY KEY,
    schema_version INTEGER NOT NULL,
    project_version TEXT NOT NULL,
    compiler_version TEXT NOT NULL,
    document_json TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS storage_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO storage_metadata(key, value)
VALUES ('schema_version', '1');
