use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::{collections::HashMap, error::Error, path::Path};

use crate::telemetry;
use crate::{query_instructions, query_knowledge, query_problems, tokenize};

pub const MAX_KNOWLEDGE_HITS: usize = 8;
pub const MAX_PROBLEM_HITS: usize = 5;
pub const MAX_INSTRUCTION_HITS: usize = 8;
pub const MAX_TERMS: usize = 10;

struct BacklogItem {
    id: String,
    epic: Option<String>,
    title: String,
    description: Option<String>,
    item_type: Option<String>,
    status: String,
    priority: i64,
    rationale: Option<String>,
    technical_risk: Option<String>,
    user_value: Option<String>,
    dependencies: Vec<String>,
    affected_areas: Vec<String>,
    acceptance_criteria: Vec<String>,
}

fn load_backlog_item(conn: &Connection, id: &str) -> Result<Option<BacklogItem>, Box<dyn Error>> {
    let item: Option<BacklogItem> = conn
        .query_row(
            "SELECT id,epic,title,description,item_type,status,priority,
                    rationale,technical_risk,user_value,
                    dependencies_json,affected_areas_json,acceptance_criteria_json
             FROM backlog_items WHERE id=?1",
            params![id],
            |r| {
                let deps: String = r.get(10)?;
                let areas: String = r.get(11)?;
                let criteria: String = r.get(12)?;
                Ok(BacklogItem {
                    id: r.get(0)?,
                    epic: r.get(1)?,
                    title: r.get(2)?,
                    description: r.get(3)?,
                    item_type: r.get(4)?,
                    status: r.get(5)?,
                    priority: r.get(6)?,
                    rationale: r.get(7)?,
                    technical_risk: r.get(8)?,
                    user_value: r.get(9)?,
                    dependencies: serde_json::from_str(&deps).unwrap_or_default(),
                    affected_areas: serde_json::from_str(&areas).unwrap_or_default(),
                    acceptance_criteria: serde_json::from_str(&criteria).unwrap_or_default(),
                })
            },
        )
        .optional()?;
    Ok(item)
}

/// Bounded search-term derivation from backlog fields. No repository scans,
/// no full-text search: tokenize the item text, keep meaningful unique terms,
/// cap the set.
fn derive_terms(item: &BacklogItem) -> String {
    let mut text = String::new();
    text.push_str(&item.title);
    text.push(' ');
    if let Some(d) = &item.description {
        text.push_str(d);
        text.push(' ');
    }
    if let Some(e) = &item.epic {
        text.push_str(e);
        text.push(' ');
    }
    if let Some(r) = &item.rationale {
        text.push_str(r);
        text.push(' ');
    }
    for a in &item.affected_areas {
        text.push_str(a);
        text.push(' ');
    }
    for d in &item.dependencies {
        text.push_str(d);
        text.push(' ');
    }
    for c in &item.acceptance_criteria {
        text.push_str(c);
        text.push(' ');
    }

    let mut seen: Vec<String> = Vec::new();
    for term in tokenize(&text) {
        if term.len() < 3 {
            continue;
        }
        if seen.contains(&term) {
            continue;
        }
        seen.push(term);
        if seen.len() >= MAX_TERMS {
            break;
        }
    }
    seen.join(" ")
}

/// Emit prefetch telemetry through the normal event path so agent_runs
/// counters (knowledge_queries/hits, problem_queries/hits) increment exactly
/// like manual queries do. No run id -> telemetry skipped, packet unaffected.
fn emit_prefetch_event(
    conn: &Connection,
    run_id: Option<&str>,
    source: &str,
    event_type: &str,
    key: &str,
) -> Result<(), Box<dyn Error>> {
    if let Some(rid) = run_id {
        telemetry::emit_event(
            conn,
            Some(rid),
            event_type,
            Some(source),
            Some(key),
            None,
            None,
        )?;
    }
    Ok(())
}

pub fn cmd_context_packet(
    conn: &Connection,
    args: &[String],
    agents_root: &Path,
    _db_path: &Path,
) -> Result<Value, Box<dyn Error>> {
    let backlog_id = args
        .get(2)
        .ok_or_else(|| "missing argument: backlog-id".to_string())?;
    let phase = args
        .get(3)
        .ok_or_else(|| "missing argument: phase (planning|execution)".to_string())?;
    if !matches!(phase.as_str(), "planning" | "execution") {
        return Err(format!("invalid phase: {phase} (allowed: planning|execution)").into());
    }

    let item = load_backlog_item(conn, backlog_id)?
        .ok_or_else(|| format!("backlog item not found: {backlog_id}"))?;
    let terms = derive_terms(&item);

    let run_id = telemetry::run_id_from_env();
    let source = if phase == "planning" {
        "planning_prefetch"
    } else {
        "execution_prefetch"
    };

    // ---- Knowledge: one query per enabled channel, reusing the normal
    // query_knowledge implementation (same ranking, same semantics).
    let mut merged: HashMap<String, (i64, Value)> = HashMap::new();
    let mut knowledge_queries: i64 = 0;
    let mut knowledge_hits_count: i64 = 0;

    let channels: Vec<String> = {
        let mut stmt = conn.prepare("SELECT DISTINCT channel FROM sources ORDER BY channel")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for channel in channels {
        let result = query_knowledge(conn, &channel, &terms, MAX_KNOWLEDGE_HITS)?;
        knowledge_queries += 1;
        let results = result
            .get("results")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let hit = !results.is_empty();
        if hit {
            knowledge_hits_count += 1;
        }
        emit_prefetch_event(
            conn,
            run_id.as_deref(),
            source,
            if hit {
                "knowledge_hit"
            } else {
                "knowledge_miss"
            },
            &channel,
        )?;
        for r in results {
            let id = r
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let score = r.get("score").and_then(|v| v.as_i64()).unwrap_or(0);
            merged
                .entry(id.clone())
                .and_modify(|(s, v)| {
                    *s += score;
                    v["score"] = json!(*s);
                })
                .or_insert((score, r));
        }
    }
    let mut merged_values: Vec<(i64, Value)> = merged.into_values().collect();
    merged_values.sort_by(|a, b| b.0.cmp(&a.0));
    let knowledge_hits: Vec<Value> = merged_values
        .into_iter()
        .take(MAX_KNOWLEDGE_HITS)
        .map(|(_, v)| v)
        .collect();

    // ---- Common Problems: one bounded query through the normal path.
    let mut problem_queries: i64 = 0;
    let mut problem_hits_count: i64 = 0;
    let common_problems: Vec<Value> = {
        let result = query_problems(conn, &terms, MAX_PROBLEM_HITS)?;
        problem_queries += 1;
        let results = result
            .get("results")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let hit = !results.is_empty();
        if hit {
            problem_hits_count += 1;
        }
        emit_prefetch_event(
            conn,
            run_id.as_deref(),
            source,
            if hit { "problem_hit" } else { "problem_miss" },
            "context-packet",
        )?;
        results
    };

    // ---- Instructions/constraints: bounded merge across scopes through the
    // normal query_instructions implementation.
    let instructions: Vec<Value> = {
        let mut by_id: HashMap<String, Value> = HashMap::new();
        let scopes: Vec<String> = {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT scope FROM instructions WHERE enabled=1 ORDER BY scope",
            )?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        for scope in scopes {
            let result = query_instructions(conn, &scope, None)?;
            let results = result
                .get("results")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for r in results {
                let id = r
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                by_id.entry(id).or_insert(r);
            }
        }
        let mut values: Vec<Value> = by_id.into_values().collect();
        values.sort_by(|a, b| {
            let pa = a.get("priority").and_then(|v| v.as_i64()).unwrap_or(0);
            let pb = b.get("priority").and_then(|v| v.as_i64()).unwrap_or(0);
            pb.cmp(&pa)
        });
        values.into_iter().take(MAX_INSTRUCTION_HITS).collect()
    };

    let experiment_phase = telemetry::current_phase(agents_root);

    let backlog_json = if phase == "planning" {
        json!({
            "id": item.id,
            "title": item.title,
            "description": item.description,
            "epic": item.epic,
            "item_type": item.item_type,
            "status": item.status,
            "priority": item.priority,
            "risk": item.technical_risk,
            "rationale": item.rationale,
            "user_value": item.user_value,
            "dependencies": item.dependencies,
            "affected_areas": item.affected_areas,
            "acceptance_criteria": item.acceptance_criteria
        })
    } else {
        json!({
            "id": item.id,
            "objective": item.title,
            "description": item.description,
            "status": item.status,
            "acceptance_criteria": item.acceptance_criteria,
            "affected_areas": item.affected_areas,
            "technical_constraints": item.technical_risk
        })
    };

    Ok(json!({
        "ok": true,
        "backlog_id": backlog_id,
        "phase": phase,
        "experiment_phase": experiment_phase,
        "commit": telemetry::commit_sha(),
        "backlog": backlog_json,
        "knowledge": knowledge_hits,
        "common_problems": common_problems,
        "instructions": instructions,
        "meta": {
            "terms": terms,
            "knowledge_queries": knowledge_queries,
            "knowledge_hits": knowledge_hits_count,
            "problem_queries": problem_queries,
            "problem_hits": problem_hits_count
        }
    }))
}
