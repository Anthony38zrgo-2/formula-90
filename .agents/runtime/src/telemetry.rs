use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::{
    error::Error,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub const ALLOWED_EVENTS: &[&str] = &[
    "knowledge_hit",
    "knowledge_miss",
    "knowledge_unavailable",
    "problem_hit",
    "problem_miss",
    "cache_hit",
    "cache_miss",
    "retry",
    "escalation",
    "routing_decision",
    "test_pass",
    "test_fail",
    "commit",
    "run_success",
    "run_failure",
    "infrastructure_bootstrap",
    "infrastructure_fallback",
];

pub const ALLOWED_RUN_KINDS: &[&str] = &[
    "productive",
    "probe",
    "control",
    "instrumentation",
    "bootstrap",
    "sync",
    "unknown",
];

pub const ALLOWED_PHASES: &[&str] = &[
    "bootstrap-poc",
    "reactive-router",
    "contract-first",
    "unclassified",
];

pub const ALLOWED_CAPACITY_SOURCES: &[&str] = &[
    "normal",
    "automatic_escalation",
    "manual_override",
    "planned_capacity",
    "unknown",
];

#[derive(serde::Deserialize)]
struct ExperimentConfig {
    #[allow(dead_code)]
    schema_version: u32,
    current_phase: String,
}

/// Project-level experiment phase default (`.agents/config/experiment.json`).
pub fn current_phase(agents_root: &Path) -> String {
    let path = agents_root.join("config/experiment.json");
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return "unclassified".into(),
    };
    match serde_json::from_str::<ExperimentConfig>(&data) {
        Ok(cfg) if ALLOWED_PHASES.contains(&cfg.current_phase.as_str()) => cfg.current_phase,
        _ => "unclassified".into(),
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

pub fn run_id_from_env() -> Option<String> {
    env_var_non_empty("AGENT_RUN_ID")
}

pub fn decode_payload_arg(raw: &str) -> Result<String, Box<dyn Error>> {
    if let Some(hex) = raw.strip_prefix("hex:") {
        let bytes = hex_decode(hex)?;
        return Ok(String::from_utf8(bytes)?);
    }
    Ok(raw.to_string())
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    let chars: Vec<char> = hex.chars().collect();
    if chars.len() % 2 != 0 {
        return Err("hex payload must have even length".into());
    }
    let mut i = 0;
    while i < chars.len() {
        let hi = chars[i]
            .to_digit(16)
            .ok_or_else(|| "invalid hex payload".to_string())?;
        let lo = chars[i + 1]
            .to_digit(16)
            .ok_or_else(|| "invalid hex payload".to_string())?;
        bytes.push((hi * 16 + lo) as u8);
        i += 2;
    }
    Ok(bytes)
}

pub fn env_var_non_empty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn commit_sha() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn emit_event_raw(
    conn: &Connection,
    run_id: Option<&str>,
    event_type: &str,
    source: Option<&str>,
    key: Option<&str>,
    value: Option<i64>,
    detail: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    if !ALLOWED_EVENTS.contains(&event_type) {
        return Err(format!("unknown event type: {event_type}").into());
    }
    conn.execute(
        "INSERT INTO telemetry_events(run_id, ts, event_type, source, key, value, detail)
         VALUES(?1, unixepoch(), ?2, ?3, ?4, ?5, ?6)",
        params![run_id, event_type, source, key, value, detail],
    )?;
    if let Some(rid) = run_id {
        match event_type {
            "knowledge_hit" => {
                conn.execute(
                    "UPDATE agent_runs
                     SET knowledge_queries=knowledge_queries+1, knowledge_hits=knowledge_hits+1
                     WHERE run_id=?1",
                    params![rid],
                )?;
            }
            "knowledge_miss" => {
                conn.execute(
                    "UPDATE agent_runs SET knowledge_queries=knowledge_queries+1 WHERE run_id=?1",
                    params![rid],
                )?;
            }
            "problem_hit" => {
                conn.execute(
                    "UPDATE agent_runs
                     SET problem_queries=problem_queries+1, problem_hits=problem_hits+1
                     WHERE run_id=?1",
                    params![rid],
                )?;
            }
            "problem_miss" => {
                conn.execute(
                    "UPDATE agent_runs SET problem_queries=problem_queries+1 WHERE run_id=?1",
                    params![rid],
                )?;
            }
            "cache_hit" => {
                conn.execute(
                    "UPDATE agent_runs
                     SET cache_queries=cache_queries+1, cache_hits=cache_hits+1
                     WHERE run_id=?1",
                    params![rid],
                )?;
            }
            "cache_miss" => {
                conn.execute(
                    "UPDATE agent_runs SET cache_queries=cache_queries+1 WHERE run_id=?1",
                    params![rid],
                )?;
            }
            "retry" => {
                conn.execute(
                    "UPDATE agent_runs SET retries=retries+1 WHERE run_id=?1",
                    params![rid],
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn emit_event(
    conn: &Connection,
    run_id: Option<&str>,
    event_type: &str,
    source: Option<&str>,
    key: Option<&str>,
    value: Option<i64>,
    detail: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    emit_event_raw(conn, run_id, event_type, source, key, value, detail)
}

pub fn cmd_run_start(
    conn: &Connection,
    args: &[String],
    agents_root: &Path,
) -> Result<Value, Box<dyn Error>> {
    let agent_id = args
        .get(2)
        .ok_or_else(|| "missing argument: agent-id".to_string())?;
    let mut explicit_run_id: Option<&str> = None;
    let mut run_kind: Option<&str> = None;
    let mut phase: Option<&str> = None;
    let mut ingestion_source: Option<&str> = None;
    let mut positional: Vec<&str> = Vec::new();

    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--run-id" => {
                i += 1;
                explicit_run_id = args.get(i).map(String::as_str);
                if explicit_run_id.is_none() {
                    return Err("missing value for --run-id".into());
                }
            }
            "--run-kind" => {
                i += 1;
                run_kind = args.get(i).map(String::as_str);
                if run_kind.is_none() {
                    return Err("missing value for --run-kind".into());
                }
                if !ALLOWED_RUN_KINDS.contains(&run_kind.unwrap()) {
                    return Err(format!(
                        "invalid run_kind: {} (allowed: {})",
                        run_kind.unwrap(),
                        ALLOWED_RUN_KINDS.join(", ")
                    )
                    .into());
                }
            }
            "--phase" => {
                i += 1;
                phase = args.get(i).map(String::as_str);
                if phase.is_none() {
                    return Err("missing value for --phase".into());
                }
                if !ALLOWED_PHASES.contains(&phase.unwrap()) {
                    return Err(format!(
                        "invalid experiment phase: {} (allowed: {})",
                        phase.unwrap(),
                        ALLOWED_PHASES.join(", ")
                    )
                    .into());
                }
            }
            "--ingestion-source" => {
                i += 1;
                ingestion_source = args.get(i).map(String::as_str);
                if ingestion_source.is_none() {
                    return Err("missing value for --ingestion-source".into());
                }
            }
            s if s.starts_with("--") => {
                return Err(format!("unknown run-start option: {s}").into());
            }
            s => positional.push(s),
        }
        i += 1;
    }
    let task_id: Option<&str> = positional.first().copied();
    let backlog_id: Option<&str> = positional.get(1).copied();

    let run_id = explicit_run_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("run_{}_{}", now_ms(), std::process::id()));

    let role: Option<String> = conn
        .query_row(
            "SELECT role FROM agent_registry WHERE enabled=1 AND id=?1",
            params![agent_id],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();

    let effective_phase = phase
        .map(|s| s.to_string())
        .unwrap_or_else(|| current_phase(agents_root));

    let started_at = now_ms();
    conn.execute(
        "INSERT INTO agent_runs(
            run_id,task_id,backlog_id,agent_id,role,started_at,commit_sha,
            run_kind,experiment_phase,ingestion_source
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            run_id,
            task_id,
            backlog_id,
            agent_id,
            role,
            started_at,
            commit_sha(),
            run_kind.unwrap_or("unknown"),
            effective_phase,
            ingestion_source
        ],
    )?;

    Ok(json!({
        "ok": true,
        "run_id": run_id,
        "run_kind": run_kind.unwrap_or("unknown"),
        "experiment_phase": effective_phase
    }))
}

pub fn cmd_run_end(conn: &Connection, args: &[String]) -> Result<Value, Box<dyn Error>> {
    let run_id = required_arg(args, 2, "run-id")?;
    let outcome = required_arg(args, 3, "outcome")?;
    if !matches!(outcome, "success" | "failure") {
        return Err(format!("outcome must be 'success' or 'failure', got: {outcome}").into());
    }
    let started_at: Option<i64> = conn
        .query_row(
            "SELECT started_at FROM agent_runs WHERE run_id=?1",
            params![run_id],
            |r| r.get(0),
        )
        .optional()?;
    let started_at = started_at.ok_or_else(|| format!("run not found: {run_id}"))?;

    let finished_at = now_ms();
    let duration_ms = finished_at - started_at;

    conn.execute(
        "UPDATE agent_runs SET
            model_calls=(SELECT COUNT(*) FROM model_calls WHERE run_id=?1),
            input_tokens=(SELECT COALESCE(SUM(input_tokens),0) FROM model_calls WHERE run_id=?1),
            cached_input_tokens=(SELECT COALESCE(SUM(cached_input_tokens),0) FROM model_calls WHERE run_id=?1),
            output_tokens=(SELECT COALESCE(SUM(output_tokens),0) FROM model_calls WHERE run_id=?1),
            finished_at=?2, duration_ms=?3, outcome=?4
         WHERE run_id=?1",
        params![run_id, finished_at, duration_ms, outcome],
    )?;

    emit_event_raw(
        conn,
        Some(run_id),
        if outcome == "success" {
            "run_success"
        } else {
            "run_failure"
        },
        Some("run-end"),
        None,
        None,
        None,
    )?;

    Ok(json!({"ok": true, "run_id": run_id, "outcome": outcome, "duration_ms": duration_ms}))
}

#[derive(serde::Deserialize)]
struct ModelCallStart {
    provider: String,
    requested_model: Option<String>,
    requested_effort: Option<String>,
    model_tier: Option<String>,
    purpose: Option<String>,
    routing_rule: Option<String>,
    routing_reason: Option<String>,
    routing_source: Option<String>,
    capacity_source: Option<String>,
}

/// WHY a model was allocated. `capacity_source` is persisted context; strong
/// usage itself stays derived from `model_tier` (never stored redundantly).
/// Mutually exclusive by construction (single enum value).
fn derive_capacity_source(meta: &ModelCallStart) -> String {
    if let Some(explicit) = &meta.capacity_source {
        if ALLOWED_CAPACITY_SOURCES.contains(&explicit.as_str()) {
            return explicit.clone();
        }
    }
    if meta.routing_source.as_deref() == Some("user_override") {
        return "manual_override".into();
    }
    match meta.routing_rule.as_deref() {
        Some("attempt_budget_exhausted")
        | Some("evidence_stagnation")
        | Some("diagnostic_mode") => "automatic_escalation".into(),
        Some("architecture_change")
        | Some("cross_subsystem_change")
        | Some("high_technical_risk")
        | Some("large_regression") => "automatic_escalation".into(),
        Some("role_product_owner") | Some("role_architect") | Some("role_retrospective") => {
            "planned_capacity".into()
        }
        _ => "normal".into(),
    }
}

pub fn cmd_model_call_start(conn: &Connection, args: &[String]) -> Result<Value, Box<dyn Error>> {
    let run_id = required_arg(args, 3, "run-id")?;
    let payload = required_arg(args, 4, "model-call-json")?;
    let payload = decode_payload_arg(payload)?;
    let m: ModelCallStart = serde_json::from_str(&payload)?;

    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM agent_runs WHERE run_id=?1",
            params![run_id],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !exists {
        return Err(format!("run not found: {run_id}").into());
    }

    let capacity_source = derive_capacity_source(&m);
    let routing_source = m
        .routing_source
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            if m.routing_rule.as_deref() == Some("user_override") {
                "user_override".into()
            } else {
                "auto".into()
            }
        });

    let call_id = format!("call_{}_{}", now_ms(), std::process::id());
    conn.execute(
        "INSERT INTO model_calls(
            call_id,run_id,provider,requested_model,requested_effort,model_tier,
            purpose,routing_rule,routing_reason,routing_source,capacity_source,
            started_at,success
         ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,0)",
        params![
            call_id,
            run_id,
            m.provider,
            m.requested_model,
            m.requested_effort,
            m.model_tier,
            m.purpose,
            m.routing_rule,
            m.routing_reason,
            routing_source,
            capacity_source,
            now_ms()
        ],
    )?;
    Ok(json!({
        "ok": true,
        "call_id": call_id,
        "run_id": run_id,
        "capacity_source": capacity_source
    }))
}

#[derive(serde::Deserialize)]
struct ModelCallEnd {
    success: bool,
    duration_ms: Option<i64>,
    effective_model: Option<String>,
    effective_effort: Option<String>,
    verification_status: Option<String>,
    input_tokens: Option<i64>,
    cached_input_tokens: Option<i64>,
    output_tokens: Option<i64>,
}

pub fn cmd_model_call_end(conn: &Connection, args: &[String]) -> Result<Value, Box<dyn Error>> {
    let call_id = required_arg(args, 3, "call-id")?;
    let payload = required_arg(args, 4, "model-call-end-json")?;
    let payload = decode_payload_arg(payload)?;
    let m: ModelCallEnd = serde_json::from_str(&payload)?;

    let updated = conn.execute(
        "UPDATE model_calls SET
            effective_model=?1, effective_effort=?2,
            verification_status=?3, duration_ms=?4,
            input_tokens=?5, cached_input_tokens=?6, output_tokens=?7,
            success=?8
         WHERE call_id=?9",
        params![
            m.effective_model,
            m.effective_effort,
            m.verification_status.unwrap_or_else(|| "unverified".into()),
            m.duration_ms,
            m.input_tokens,
            m.cached_input_tokens,
            m.output_tokens,
            if m.success { 1 } else { 0 },
            call_id
        ],
    )?;
    if updated == 0 {
        return Err(format!("model call not found: {call_id}").into());
    }
    Ok(json!({"ok": true, "call_id": call_id, "success": m.success}))
}

pub fn cmd_event(conn: &Connection, args: &[String]) -> Result<Value, Box<dyn Error>> {
    let run_id: Option<&str> = match args.get(2).map(String::as_str) {
        Some("-") | Some("") => None,
        Some(v) => Some(v),
        None => return Err("missing argument: run-id (use '-' for anonymous)".into()),
    };
    let event_type = required_arg(args, 3, "event-type")?;

    let mut source: Option<&str> = None;
    let mut positionals: Vec<&str> = Vec::new();
    let mut i = 4;
    while i < args.len() {
        match args[i].as_str() {
            "--source" => {
                i += 1;
                source = Some(required_arg(args, i, "--source value")?);
            }
            s if s.starts_with("--") => {
                return Err(format!("unknown event option: {s}").into());
            }
            s => positionals.push(s),
        }
        i += 1;
    }
    if positionals.len() > 3 {
        return Err("too many event arguments".into());
    }

    let key = positionals.first().copied();
    let value = positionals
        .get(1)
        .map(|v| v.parse::<i64>())
        .transpose()
        .map_err(|_| "event value must be an integer".to_string())?;
    let detail = positionals.get(2).copied();

    emit_event_raw(
        conn,
        run_id,
        event_type,
        Some(source.unwrap_or("cli")),
        key,
        value,
        detail,
    )?;
    Ok(json!({"ok": true, "event_type": event_type, "run_id": run_id}))
}

fn required_arg<'a>(
    args: &'a [String],
    index: usize,
    name: &str,
) -> Result<&'a str, Box<dyn Error>> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("missing argument: {name}").into())
}

fn ratio(hits: i64, queries: i64) -> Option<f64> {
    if queries > 0 {
        Some(hits as f64 / queries as f64)
    } else {
        None
    }
}

/// Metrics scope: `agentdb metrics [all|productive] [phase <phase>]`.
/// Default scope is `all`. Filters apply to runs; model-call metrics are
/// computed over calls belonging to the filtered run set.
struct MetricsFilter {
    productive: bool,
    phase: Option<String>,
}

impl MetricsFilter {
    fn parse(args: &[String]) -> Result<MetricsFilter, Box<dyn Error>> {
        let mut productive = false;
        let mut phase: Option<String> = None;
        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "all" => {}
                "productive" => productive = true,
                "phase" => {
                    i += 1;
                    let p = required_arg(args, i, "phase value")?;
                    if !ALLOWED_PHASES.contains(&p) {
                        return Err(format!(
                            "invalid phase: {p} (allowed: {})",
                            ALLOWED_PHASES.join(", ")
                        )
                        .into());
                    }
                    phase = Some(p.to_string());
                }
                other => {
                    return Err(format!(
                        "unknown metrics argument: {other} (expected all|productive|phase <phase>)"
                    )
                    .into());
                }
            }
            i += 1;
        }
        Ok(MetricsFilter { productive, phase })
    }

    /// Always returns a valid WHERE clause ("WHERE 1=1" when unfiltered) so
    /// callers can safely append extra conditions.
    fn run_where(&self) -> String {
        let mut clauses: Vec<String> = Vec::new();
        if self.productive {
            clauses.push("run_kind='productive'".to_string());
        }
        // phase values come from ALLOWED_PHASES (validated whitelist), so
        // inlining is injection-safe.
        if let Some(p) = &self.phase {
            clauses.push(format!("experiment_phase='{p}'"));
        }
        if clauses.is_empty() {
            "WHERE 1=1".to_string()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        }
    }
}

fn count_with_filter(conn: &Connection, f: &MetricsFilter) -> Result<i64, rusqlite::Error> {
    let where_clause = f.run_where();
    let sql = format!("SELECT COUNT(*) FROM agent_runs {where_clause}");
    conn.query_row(&sql, [], |r| r.get(0))
}

fn sum_with_filter(
    conn: &Connection,
    column: &str,
    f: &MetricsFilter,
) -> Result<i64, rusqlite::Error> {
    let where_clause = f.run_where();
    let sql = format!("SELECT COALESCE(SUM({column}),0) FROM agent_runs {where_clause}");
    conn.query_row(&sql, [], |r| r.get(0))
}

/// WHERE fragment restricting model_calls to runs matching the filter.
fn calls_in_runs_sql(f: &MetricsFilter) -> String {
    let run_where = f.run_where();
    format!("SELECT run_id FROM agent_runs {run_where}")
}

fn count_calls_in_runs(
    conn: &Connection,
    count_sql: &str,
    f: &MetricsFilter,
) -> Result<i64, rusqlite::Error> {
    let sub = calls_in_runs_sql(f);
    let sql = format!("SELECT COUNT(*) FROM model_calls WHERE run_id IN ({sub}) AND {count_sql}");
    conn.query_row(&sql, [], |r| r.get(0))
}

pub fn cmd_metrics(conn: &Connection, args: &[String]) -> Result<Value, Box<dyn Error>> {
    let f = MetricsFilter::parse(args)?;

    let total_runs = count_with_filter(conn, &f)?;
    let success_runs = {
        let w = f.run_where();
        let sql = format!("SELECT COUNT(*) FROM agent_runs {w} AND outcome='success'");
        conn.query_row(&sql, [], |r| r.get(0))?
    };
    let first_attempt_success = {
        let w = f.run_where();
        let sql =
            format!("SELECT COUNT(*) FROM agent_runs {w} AND outcome='success' AND retries=0");
        conn.query_row(&sql, [], |r| r.get(0))?
    };
    let retries_total = sum_with_filter(conn, "retries", &f)?;
    let avg_retries_per_success: Option<f64> = {
        let w = f.run_where();
        let sql = format!("SELECT AVG(retries) FROM agent_runs {w} AND outcome='success'");
        conn.query_row(&sql, [], |r| r.get(0)).optional()?
    };

    let knowledge_queries = sum_with_filter(conn, "knowledge_queries", &f)?;
    let knowledge_hits = sum_with_filter(conn, "knowledge_hits", &f)?;
    let problem_queries = sum_with_filter(conn, "problem_queries", &f)?;
    let problem_hits = sum_with_filter(conn, "problem_hits", &f)?;
    let cache_queries = sum_with_filter(conn, "cache_queries", &f)?;
    let cache_hits = sum_with_filter(conn, "cache_hits", &f)?;

    let total_calls = count_calls_in_runs(conn, "1=1", &f)?;
    let strong_calls = count_calls_in_runs(conn, "model_tier='strong'", &f)?;

    // capacity counts with explicit parameters (count_calls_in_runs only
    // binds run-filter params; capacity_source needs its own binding)
    let mut capacity_map: Vec<(String, i64)> = Vec::new();
    {
        let run_where = f.run_where();
        let sub = if run_where.is_empty() {
            "SELECT run_id FROM agent_runs".to_string()
        } else {
            format!("SELECT run_id FROM agent_runs {run_where}")
        };
        let stmt = format!(
            "SELECT capacity_source, COUNT(*) FROM model_calls
             WHERE run_id IN ({sub}) GROUP BY capacity_source"
        );
        let mut s = conn.prepare(&stmt)?;
        let rows = s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            let (src, n) = row?;
            capacity_map.push((src, n));
        }
    }
    let cap = |name: &str| -> i64 {
        capacity_map
            .iter()
            .find(|(s, _)| s == name)
            .map(|(_, n)| *n)
            .unwrap_or(0)
    };
    let automatic_escalation_calls = cap("automatic_escalation");
    let manual_override_calls = cap("manual_override");
    let planned_capacity_calls = cap("planned_capacity");
    let normal_calls = cap("normal");

    let tokens_per_success: Option<(i64, i64)> = {
        let w = f.run_where();
        let sql = format!(
            "SELECT COALESCE(SUM(input_tokens+output_tokens),0), COUNT(*)
             FROM agent_runs {w} AND outcome='success'"
        );
        conn.query_row(&sql, [], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?
    };
    let duration_per_success: Option<f64> = {
        let w = f.run_where();
        let sql = format!("SELECT AVG(duration_ms) FROM agent_runs {w} AND outcome='success'");
        conn.query_row(&sql, [], |r| r.get(0)).optional()?
    };

    // Productive usage: productive runs (inside the filter) that actually
    // consulted Knowledge / Common Problems. This is the metric that matters
    // more than raw hit rate: our gap is non-consumption, not retrieval.
    let productive_runs = {
        let w = f.run_where();
        let sql = format!("SELECT COUNT(*) FROM agent_runs {w} AND run_kind='productive'");
        conn.query_row(&sql, [], |r| r.get(0))?
    };
    let productive_knowledge_users = {
        let w = f.run_where();
        let sql = format!(
            "SELECT COUNT(*) FROM agent_runs {w} AND run_kind='productive' AND knowledge_queries>=1"
        );
        conn.query_row(&sql, [], |r| r.get(0))?
    };
    let productive_problem_users = {
        let w = f.run_where();
        let sql = format!(
            "SELECT COUNT(*) FROM agent_runs {w} AND run_kind='productive' AND problem_queries>=1"
        );
        conn.query_row(&sql, [], |r| r.get(0))?
    };

    // Knowledge consumption by origin (manual vs automatic prefetch).
    let event_query_hits = |source: &str| -> Result<(i64, i64), rusqlite::Error> {
        conn.query_row(
            "SELECT
                (SELECT COUNT(*) FROM telemetry_events WHERE source=?1 AND event_type IN ('knowledge_hit','knowledge_miss')),
                (SELECT COUNT(*) FROM telemetry_events WHERE source=?1 AND event_type='knowledge_hit')",
            params![source],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
    };
    let (planning_prefetch_queries, planning_prefetch_hits) =
        event_query_hits("planning_prefetch")?;
    let (execution_prefetch_queries, execution_prefetch_hits) =
        event_query_hits("execution_prefetch")?;

    // run_kind / phase breakdowns (global, unfiltered — structural facts).
    let run_kind_counts = {
        let mut map = std::collections::HashMap::new();
        let mut stmt =
            conn.prepare("SELECT run_kind, COUNT(*) FROM agent_runs GROUP BY run_kind")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            let (k, n) = row?;
            map.insert(k, n);
        }
        map
    };
    let phase_counts = {
        let mut map = std::collections::HashMap::new();
        let mut stmt = conn.prepare(
            "SELECT experiment_phase, COUNT(*) FROM agent_runs GROUP BY experiment_phase",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            let (k, n) = row?;
            map.insert(k, n);
        }
        map
    };

    let mut distribution = Vec::new();
    {
        let run_where = f.run_where();
        let sub = if run_where.is_empty() {
            "SELECT run_id FROM agent_runs".to_string()
        } else {
            format!("SELECT run_id FROM agent_runs {run_where}")
        };
        let mut stmt = conn.prepare(&format!(
            "SELECT COALESCE(effective_model, requested_model) AS model, COUNT(*)
             FROM model_calls WHERE run_id IN ({sub}) GROUP BY model ORDER BY 2 DESC"
        ))?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, Option<String>>(0)?, r.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (model, count) = row?;
            distribution.push(json!({"model": model, "calls": count}));
        }
    }

    let mut tier_distribution = Vec::new();
    {
        let run_where = f.run_where();
        let sub = if run_where.is_empty() {
            "SELECT run_id FROM agent_runs".to_string()
        } else {
            format!("SELECT run_id FROM agent_runs {run_where}")
        };
        let mut stmt = conn.prepare(&format!(
            "SELECT model_tier, COUNT(*) FROM model_calls WHERE run_id IN ({sub})
             GROUP BY model_tier ORDER BY 2 DESC"
        ))?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, Option<String>>(0)?, r.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (tier, count) = row?;
            tier_distribution.push(json!({"tier": tier, "calls": count}));
        }
    }

    let mut effort_distribution = Vec::new();
    {
        let run_where = f.run_where();
        let sub = if run_where.is_empty() {
            "SELECT run_id FROM agent_runs".to_string()
        } else {
            format!("SELECT run_id FROM agent_runs {run_where}")
        };
        let mut stmt = conn.prepare(&format!(
            "SELECT requested_model, requested_effort, COUNT(*)
             FROM model_calls WHERE run_id IN ({sub}) GROUP BY 1,2 ORDER BY 3 DESC"
        ))?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        for row in rows {
            let (model, effort, count) = row?;
            effort_distribution.push(json!({"model": model, "effort": effort, "calls": count}));
        }
    }

    let mut tokens_per_model = Vec::new();
    {
        let run_where = f.run_where();
        let sub = if run_where.is_empty() {
            "SELECT run_id FROM agent_runs".to_string()
        } else {
            format!("SELECT run_id FROM agent_runs {run_where}")
        };
        let mut stmt = conn.prepare(&format!(
            "SELECT COALESCE(effective_model, requested_model),
                    SUM(input_tokens), SUM(cached_input_tokens), SUM(output_tokens), COUNT(*)
             FROM model_calls WHERE run_id IN ({sub}) GROUP BY 1 ORDER BY 5 DESC"
        ))?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?,
                r.get::<_, Option<i64>>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, Option<i64>>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })?;
        for row in rows {
            let (model, input, cached, output, count) = row?;
            tokens_per_model.push(json!({
                "model": model,
                "input_tokens": input,
                "cached_input_tokens": cached,
                "output_tokens": output,
                "calls": count
            }));
        }
    }

    Ok(json!({
        "ok": true,
        "scope": if f.productive { "productive" } else { "all" },
        "experiment_phase": f.phase,
        "successful_run_rate": ratio(success_runs, total_runs),
        "first_attempt_success_rate": ratio(first_attempt_success, success_runs),
        "average_retries_per_success": avg_retries_per_success,
        "knowledge_hit_rate": ratio(knowledge_hits, knowledge_queries),
        "problem_hit_rate": ratio(problem_hits, problem_queries),
        "cache_hit_rate": ratio(cache_hits, cache_queries),
        "strong_usage_rate": ratio(strong_calls, total_calls),
        "automatic_escalation_rate": ratio(automatic_escalation_calls, total_calls),
        "manual_override_rate": ratio(manual_override_calls, total_calls),
        "planned_capacity_rate": ratio(planned_capacity_calls, total_calls),
        "productive_knowledge_usage_rate": ratio(productive_knowledge_users, productive_runs),
        "productive_problem_usage_rate": ratio(productive_problem_users, productive_runs),
        "input_tokens_per_success": tokens_per_success.map(|(t, n)| t as f64 / n as f64),
        "output_tokens_per_success": None::<f64>,
        "duration_per_success_ms": duration_per_success,
        "counts": {
            "runs": total_runs,
            "productive_runs": productive_runs,
            "successful_runs": success_runs,
            "first_attempt_success": first_attempt_success,
            "retries": retries_total,
            "knowledge_queries": knowledge_queries,
            "knowledge_hits": knowledge_hits,
            "problem_queries": problem_queries,
            "problem_hits": problem_hits,
            "cache_queries": cache_queries,
            "cache_hits": cache_hits,
            "model_calls": total_calls,
            "strong_model_calls": strong_calls,
            "automatic_escalation_calls": automatic_escalation_calls,
            "manual_override_calls": manual_override_calls,
            "planned_capacity_calls": planned_capacity_calls,
            "normal_calls": normal_calls
        },
        "capacity": {
            "strong_usage": strong_calls,
            "automatic_escalation": automatic_escalation_calls,
            "manual_override": manual_override_calls,
            "planned_capacity": planned_capacity_calls,
            "normal": normal_calls
        },
        "run_kind_counts": run_kind_counts,
        "phase_counts": phase_counts,
        "knowledge_consumption": {
            "productive_queries": productive_knowledge_queries_all(conn)?,
            "productive_hits": productive_knowledge_hits_all(conn)?,
            "synthetic_queries": synthetic_knowledge_queries_all(conn)?,
            "synthetic_hits": synthetic_knowledge_hits_all(conn)?,
            "planning_prefetch_queries": planning_prefetch_queries,
            "planning_prefetch_hits": planning_prefetch_hits,
            "execution_prefetch_queries": execution_prefetch_queries,
            "execution_prefetch_hits": execution_prefetch_hits
        },
        "model_call_distribution": distribution,
        "tier_distribution": tier_distribution,
        "effort_distribution": effort_distribution,
        "tokens_per_model": tokens_per_model
    }))
}

fn productive_knowledge_queries_all(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT COALESCE(SUM(knowledge_queries),0) FROM agent_runs WHERE run_kind='productive'",
        [],
        |r| r.get(0),
    )
}

fn productive_knowledge_hits_all(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT COALESCE(SUM(knowledge_hits),0) FROM agent_runs WHERE run_kind='productive'",
        [],
        |r| r.get(0),
    )
}

fn synthetic_knowledge_queries_all(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT COALESCE(SUM(knowledge_queries),0) FROM agent_runs
         WHERE run_kind IN ('probe','control','instrumentation','bootstrap','unknown')",
        [],
        |r| r.get(0),
    )
}

fn synthetic_knowledge_hits_all(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT COALESCE(SUM(knowledge_hits),0) FROM agent_runs
         WHERE run_kind IN ('probe','control','instrumentation','bootstrap','unknown')",
        [],
        |r| r.get(0),
    )
}
