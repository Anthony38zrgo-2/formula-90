PRAGMA foreign_keys = ON;

-- Additive: routing provenance on model calls.
-- 'auto'          -> automatic deterministic routing
-- 'user_override' -> explicit user model selection (never silently replaced)
ALTER TABLE model_calls ADD COLUMN routing_source TEXT NOT NULL DEFAULT 'auto';
