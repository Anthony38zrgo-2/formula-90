PRAGMA foreign_keys = ON;

-- Additive: experimental telemetry semantics (migration 005).
-- run_kind classifies the nature of a run so synthetic validation never
-- distorts primary evaluation metrics.
-- experiment_phase records the experiment boundary (bootstrap-poc /
-- reactive-router / contract-first / unclassified).
-- ingestion_source records the mechanism that created a run when it is not
-- the managed invoke-agent path (e.g. desktop_sync).
ALTER TABLE agent_runs ADD COLUMN run_kind TEXT NOT NULL DEFAULT 'unknown';
ALTER TABLE agent_runs ADD COLUMN experiment_phase TEXT NOT NULL DEFAULT 'unclassified';
ALTER TABLE agent_runs ADD COLUMN ingestion_source TEXT;

-- capacity_source explains WHY a model with high capacity was allocated:
-- normal / automatic_escalation / manual_override / planned_capacity / unknown.
ALTER TABLE model_calls ADD COLUMN capacity_source TEXT NOT NULL DEFAULT 'unknown';

-- --------------------------------------------------------------------------
-- HISTORICAL BACKFILL (deterministic rules only; documented in ARCHITECTURE.md)
-- --------------------------------------------------------------------------
-- Backfill rule R1: task_id prefixes identify deterministic run kinds.
UPDATE agent_runs SET run_kind = 'probe',
       experiment_phase = 'reactive-router'
 WHERE task_id LIKE 'PROBE-%';
UPDATE agent_runs SET run_kind = 'control',
       experiment_phase = 'reactive-router'
 WHERE task_id LIKE 'CONTROL-TESTS%';
UPDATE agent_runs SET run_kind = 'instrumentation',
       experiment_phase = 'reactive-router'
 WHERE task_id LIKE 'INSTRUMENTATION-TEST%';
UPDATE agent_runs SET run_kind = 'bootstrap',
       experiment_phase = 'reactive-router'
 WHERE task_id LIKE 'BOOTSTRAP-TEST%';

-- Backfill rule R2: the synced Codex Desktop session was real feature work
-- ingested via the sync mechanism. Work semantics = productive; mechanism
-- recorded separately in ingestion_source.
UPDATE agent_runs SET run_kind = 'productive',
       experiment_phase = 'reactive-router',
       ingestion_source = 'desktop_sync'
 WHERE task_id LIKE 'SKYBOX-FEATURE%';

-- Backfill rule R3: model_calls.capacity_source derivation from persisted
-- routing provenance (routing_source / routing_rule). No guesses from model
-- name or tier alone.
UPDATE model_calls SET capacity_source = 'manual_override'
 WHERE routing_source = 'user_override';
UPDATE model_calls SET capacity_source = 'automatic_escalation'
 WHERE routing_rule = 'attempt_budget_exhausted';
-- role-driven strong allocation is policy-planned capacity, not failure
-- escalation (product-owner / architect / retrospective hard rules).
UPDATE model_calls SET capacity_source = 'planned_capacity'
 WHERE routing_rule IN ('role_product_owner', 'role_architect', 'role_retrospective');
UPDATE model_calls SET capacity_source = 'normal'
 WHERE capacity_source = 'unknown';
