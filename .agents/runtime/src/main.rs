use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

mod router;
mod telemetry;

mod context_packet;

const SCHEMA: &str = include_str!("../migrations/001_init.sql");
const SCHEMA_V2: &str = include_str!("../migrations/002_backlog.sql");
const SCHEMA_V3: &str = include_str!("../migrations/003_telemetry.sql");
const SCHEMA_V4: &str = include_str!("../migrations/004_routing_source.sql");
const SCHEMA_V5: &str = include_str!("../migrations/005_experiment_semantics.sql");

#[derive(Deserialize)]
struct InstructionsFile {
    schema_version: u32,
    instructions: Vec<InstructionSeed>,
}

#[derive(Deserialize)]
struct InstructionSeed {
    id: String,
    scope: String,
    trigger: Option<String>,
    priority: i64,
    body: String,
    source_ref: Option<String>,
}

#[derive(Deserialize)]
struct KnowledgeFile {
    source_id: String,
    channel: String,
    authority: i64,
    version_policy: String,
    source_type: String,
    source_ref: String,
    entries: Vec<KnowledgeSeed>,
}

#[derive(Deserialize)]
struct KnowledgeSeed {
    id: String,
    lookup_key: String,
    topic: String,
    #[serde(default)]
    symbols: Vec<String>,
    #[serde(default)]
    keywords: Vec<String>,
    content: String,
    source_ref: Option<String>,
    source_version: Option<String>,
}

#[derive(Deserialize)]
struct ProblemsFile {
    schema_version: u32,
    problems: Vec<ProblemSeed>,
}

#[derive(Deserialize)]
struct ProblemSeed {
    signature: String,
    domain: String,
    symptom: String,
    cause: String,
    solution: String,
    prevention: String,
    #[serde(default)]
    search_terms: Vec<String>,
    confidence: i64,
    occurrences: i64,
    status: String,
    source_ref: Option<String>,
}

#[derive(Deserialize)]
struct SkillsFile {
    schema_version: u32,
    skills: Vec<SkillSeed>,
}

#[derive(Deserialize)]
struct SkillSeed {
    id: String,
    manifest_path: String,
    #[serde(default)]
    triggers: Vec<String>,
    #[serde(default)]
    knowledge_channels: Vec<String>,
    risk: String,
    enabled: bool,
}

#[derive(Deserialize)]
struct AgentSeed {
    id: String,
    role: String,
    model_tier: String,
    purpose: String,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    knowledge_channels: Vec<String>,
}

#[derive(Deserialize)]
struct BacklogFile {
    schema_version: u32,
    items: Vec<BacklogSeed>,
}

#[derive(Deserialize)]
struct BacklogSeed {
    id: String,
    epic: Option<String>,
    title: String,
    description: String,
    item_type: String,
    #[serde(default = "default_status")]
    status: String,
    #[serde(default = "default_priority")]
    priority: i64,
    #[serde(default)]
    sort_order: i64,
    rationale: Option<String>,
    technical_risk: Option<String>,
    user_value: Option<String>,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    affected_areas: Vec<String>,
    #[serde(default)]
    acceptance_criteria: Vec<String>,
    #[serde(default)]
    evidence: Vec<String>,
    #[serde(default)]
    source_agent: String,
    #[serde(default)]
    source_context: String,
}

fn default_status() -> String {
    "proposed".into()
}

fn default_priority() -> i64 {
    50
}

#[derive(Serialize, Clone)]
pub(crate) struct KnowledgeHit {
    id: String,
    channel: String,
    lookup_key: String,
    topic: String,
    content: String,
    source_ref: Option<String>,
    source_version: Option<String>,
    authority: i64,
    score: i64,
}

#[derive(Serialize, Clone)]
struct BacklogHit {
    id: String,
    epic: Option<String>,
    title: String,
    item_type: String,
    status: String,
    priority: i64,
    sort_order: i64,
    rationale: Option<String>,
    technical_risk: Option<String>,
    user_value: Option<String>,
    dependencies: Vec<String>,
    affected_areas: Vec<String>,
    acceptance_criteria: Vec<String>,
    evidence: Vec<String>,
}

#[derive(Serialize, Clone)]
pub(crate) struct ProblemHit {
    signature: String,
    domain: String,
    symptom: String,
    cause: String,
    solution: String,
    prevention: String,
    confidence: i64,
    occurrences: i64,
    source_ref: Option<String>,
    score: i64,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", json!({"ok": false, "error": err.to_string()}));
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("help");
    let agents_root = PathBuf::from(env::var("AGENTS_ROOT").unwrap_or_else(|_| ".agents".into()));
    let db_path =
        PathBuf::from(env::var("AGENT_DB").unwrap_or_else(|_| ".agents/data/agents.db".into()));

    match command {
        "init" => {
            let conn = open_db(&db_path)?;
            init_schema(&conn)?;
            print_json(json!({"ok": true, "db": db_path}));
        }
        "seed" => {
            let mut conn = open_db(&db_path)?;
            init_schema(&conn)?;
            seed_all(&mut conn, &agents_root)?;
            print_json(json!({"ok": true, "seeded": true, "db": db_path}));
        }
        "instruction" => {
            let scope = required_arg(&args, 2, "scope")?;
            let trigger = args.get(3).map(String::as_str);
            let conn = open_db(&db_path)?;
            print_json(query_instructions(&conn, scope, trigger)?);
        }
        "knowledge" => {
            let channel = required_arg(&args, 2, "channel")?;
            let query = required_arg(&args, 3, "query")?;
            let conn = open_db(&db_path)?;
            let result = query_knowledge(&conn, channel, query, 5)?;
            let run_id = telemetry::run_id_from_env();
            if let Some(rid) = run_id.as_deref() {
                let hit = result
                    .get("results")
                    .and_then(|v| v.as_array())
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
                telemetry::emit_event(
                    &conn,
                    Some(rid),
                    if hit {
                        "knowledge_hit"
                    } else {
                        "knowledge_miss"
                    },
                    Some("knowledge"),
                    Some(query),
                    None,
                    None,
                )?;
            }
            print_json(result);
        }
        "problem" => {
            let query = required_arg(&args, 2, "query")?;
            let conn = open_db(&db_path)?;
            let result = query_problems(&conn, query, 5)?;
            let run_id = telemetry::run_id_from_env();
            if let Some(rid) = run_id.as_deref() {
                let hit = result
                    .get("results")
                    .and_then(|v| v.as_array())
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
                telemetry::emit_event(
                    &conn,
                    Some(rid),
                    if hit { "problem_hit" } else { "problem_miss" },
                    Some("problem"),
                    Some(query),
                    None,
                    None,
                )?;
            }
            print_json(result);
        }
        "backlog" => {
            let conn = open_db(&db_path)?;
            print_json(query_backlog(&conn, &args[2..])?);
        }
        "agent" => {
            let id = required_arg(&args, 2, "agent-id")?;
            let conn = open_db(&db_path)?;
            print_json(query_agent(&conn, id)?);
        }
        "skill" => {
            let id = required_arg(&args, 2, "skill-id")?;
            let conn = open_db(&db_path)?;
            print_json(query_skill(&conn, id)?);
        }
        "cache-get" => {
            let scope = required_arg(&args, 2, "scope")?;
            let key = required_arg(&args, 3, "key")?;
            let conn = open_db(&db_path)?;
            let result = cache_get(&conn, scope, key)?;
            let run_id = telemetry::run_id_from_env();
            if let Some(rid) = run_id.as_deref() {
                let hit = result
                    .get("result")
                    .and_then(|v| v.as_object())
                    .map(|o| !o.is_empty())
                    .unwrap_or(false);
                telemetry::emit_event(
                    &conn,
                    Some(rid),
                    if hit { "cache_hit" } else { "cache_miss" },
                    Some("cache"),
                    Some(key),
                    None,
                    None,
                )?;
            }
            print_json(result);
        }
        "cache-put" => {
            let scope = required_arg(&args, 2, "scope")?;
            let key = required_arg(&args, 3, "key")?;
            let payload = required_arg(&args, 4, "json-payload")?;
            let payload = telemetry::decode_payload_arg(payload)?;
            let ttl = args
                .get(5)
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(3600);
            let conn = open_db(&db_path)?;
            cache_put(&conn, scope, key, &payload, ttl)?;
            print_json(json!({"ok": true, "scope": scope, "key": key}));
        }
        "validate" => {
            let conn = open_db(&db_path)?;
            print_json(validate_registry(&conn, Path::new("."))?);
        }
        "stats" => {
            let conn = open_db(&db_path)?;
            print_json(stats(&conn)?);
        }
        "run-start" => {
            let conn = open_db(&db_path)?;
            ensure_schema(&conn)?;
            print_json(telemetry::cmd_run_start(&conn, &args, &agents_root)?);
        }
        "run-end" => {
            let conn = open_db(&db_path)?;
            print_json(telemetry::cmd_run_end(&conn, &args)?);
        }
        "model-call" => match args.get(2).map(String::as_str) {
            Some("start") => {
                let conn = open_db(&db_path)?;
                ensure_schema(&conn)?;
                print_json(telemetry::cmd_model_call_start(&conn, &args)?);
            }
            Some("end") => {
                let conn = open_db(&db_path)?;
                print_json(telemetry::cmd_model_call_end(&conn, &args)?);
            }
            _ => {
                return Err("usage: agentdb model-call start|end ...".into());
            }
        },
        "event" => {
            let conn = open_db(&db_path)?;
            print_json(telemetry::cmd_event(&conn, &args)?);
        }
        "route" => {
            let conn = open_db(&db_path)?;
            print_json(router::cmd_route(&conn, &args, &agents_root)?);
        }
        "metrics" => {
            let conn = open_db(&db_path)?;
            ensure_schema(&conn)?;
            print_json(telemetry::cmd_metrics(&conn, &args)?);
        }
        "context-packet" => {
            let conn = open_db(&db_path)?;
            ensure_schema(&conn)?;
            print_json(context_packet::cmd_context_packet(
                &conn,
                &args,
                &agents_root,
                &db_path,
            )?);
        }
        _ => print_json(json!({
            "ok": true,
            "usage": [
                "agentdb init",
                "agentdb seed",
                "agentdb instruction <scope> [trigger]",
                "agentdb knowledge <channel> <term>",
                "agentdb problem <signature-or-term>",
                "agentdb backlog [--status <status>] [--epic <epic>]",
                "agentdb agent <agent-id>",
                "agentdb skill <skill-id>",
                "agentdb cache-get <scope> <key>",
                "agentdb cache-put <scope> <key> <json> [ttl-seconds]",
                "agentdb validate",
                "agentdb stats",
                "agentdb run-start <agent-id> [task-id] [backlog-id] [--run-id <id>]",
                "agentdb run-end <run-id> <success|failure>",
                "agentdb model-call start <run-id> '<json>'",
                "agentdb model-call end <call-id> '<json>'",
                "agentdb event <run-id|-|anonymous> <event-type> [key] [value] [detail]",
                "agentdb route '<task-metadata-json>'",
                "agentdb metrics [all|productive] [phase <phase>]",
                "agentdb context-packet <backlog-id> <planning|execution>"
            ]
        })),
    }
    Ok(())
}

fn open_db(path: &Path) -> Result<Connection, Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "busy_timeout", 1000_i64)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<(), Box<dyn Error>> {
    conn.execute_batch(SCHEMA)?;
    conn.execute_batch(SCHEMA_V2)?;
    conn.execute_batch(SCHEMA_V3)?;
    let has_routing_source: bool = conn
        .prepare("PRAGMA table_info(model_calls)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .any(|r| r.map(|name| name == "routing_source").unwrap_or(false));
    if !has_routing_source {
        conn.execute_batch(SCHEMA_V4)?;
    }
    let has_run_kind: bool = conn
        .prepare("PRAGMA table_info(agent_runs)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .any(|r| r.map(|name| name == "run_kind").unwrap_or(false));
    if !has_run_kind {
        conn.execute_batch(SCHEMA_V5)?;
    }
    conn.execute(
        "INSERT INTO schema_meta(key,value) VALUES('schema_version','5')
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [],
    )?;
    Ok(())
}

/// Apply additive migrations on demand so commands that read/write the new
/// semantics columns never fail on a database that predates them. Idempotent.
fn ensure_schema(conn: &Connection) -> Result<(), Box<dyn Error>> {
    let has_run_kind: bool = conn
        .prepare("PRAGMA table_info(agent_runs)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .any(|r| r.map(|name| name == "run_kind").unwrap_or(false));
    if !has_run_kind {
        init_schema(conn)?;
    }
    Ok(())
}

fn seed_all(conn: &mut Connection, root: &Path) -> Result<(), Box<dyn Error>> {
    let tx = conn.transaction()?;
    seed_instructions(&tx, &root.join("knowledge/instructions.json"))?;
    seed_problems(&tx, &root.join("knowledge/common-problems.json"))?;
    seed_knowledge_dir(&tx, &root.join("knowledge/sources"))?;
    seed_skills(&tx, &root.join("registry/skills.json"))?;
    seed_agents(&tx, &root.join("agents"))?;
    seed_backlog(&tx, &root.join("backlog/backlog_seed.json"))?;
    tx.commit()?;
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, Box<dyn Error>> {
    let data = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

fn seed_instructions(tx: &Transaction<'_>, path: &Path) -> Result<(), Box<dyn Error>> {
    let file: InstructionsFile = read_json(path)?;
    let _ = file.schema_version;
    for item in file.instructions {
        tx.execute(
            "INSERT INTO instructions(id,scope,trigger,priority,body,source_ref,enabled,updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,1,unixepoch())
             ON CONFLICT(id) DO UPDATE SET
               scope=excluded.scope, trigger=excluded.trigger, priority=excluded.priority,
               body=excluded.body, source_ref=excluded.source_ref, enabled=1,
               updated_at=unixepoch()",
            params![
                item.id,
                item.scope,
                item.trigger,
                item.priority,
                item.body,
                item.source_ref
            ],
        )?;
    }
    Ok(())
}

fn seed_knowledge_dir(tx: &Transaction<'_>, dir: &Path) -> Result<(), Box<dyn Error>> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
        .collect();
    paths.sort();

    for path in paths {
        let file: KnowledgeFile = read_json(&path)?;
        tx.execute(
            "INSERT INTO sources(source_id,channel,authority,version_policy,source_type,source_ref)
             VALUES(?1,?2,?3,?4,?5,?6)
             ON CONFLICT(source_id) DO UPDATE SET
               channel=excluded.channel, authority=excluded.authority,
               version_policy=excluded.version_policy, source_type=excluded.source_type,
               source_ref=excluded.source_ref",
            params![
                file.source_id,
                file.channel,
                file.authority,
                file.version_policy,
                file.source_type,
                file.source_ref
            ],
        )?;

        for entry in file.entries {
            tx.execute(
                "DELETE FROM knowledge_terms WHERE entry_id=?1",
                params![entry.id],
            )?;
            tx.execute(
                "INSERT INTO knowledge_entries(
                    id,channel,lookup_key,topic,content,symbols_json,keywords_json,
                    source_id,source_ref,source_version,authority,enabled,updated_at
                 ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,1,unixepoch())
                 ON CONFLICT(id) DO UPDATE SET
                    channel=excluded.channel, lookup_key=excluded.lookup_key,
                    topic=excluded.topic, content=excluded.content,
                    symbols_json=excluded.symbols_json, keywords_json=excluded.keywords_json,
                    source_id=excluded.source_id, source_ref=excluded.source_ref,
                    source_version=excluded.source_version, authority=excluded.authority,
                    enabled=1, updated_at=unixepoch()",
                params![
                    entry.id,
                    file.channel,
                    entry.lookup_key,
                    entry.topic,
                    entry.content,
                    serde_json::to_string(&entry.symbols)?,
                    serde_json::to_string(&entry.keywords)?,
                    file.source_id,
                    entry.source_ref,
                    entry.source_version,
                    file.authority,
                ],
            )?;

            insert_terms(
                tx,
                &entry.id,
                &entry.lookup_key,
                12,
                "knowledge_terms",
                "entry_id",
            )?;
            insert_terms(
                tx,
                &entry.id,
                &entry.topic,
                4,
                "knowledge_terms",
                "entry_id",
            )?;
            for symbol in &entry.symbols {
                insert_terms(tx, &entry.id, symbol, 10, "knowledge_terms", "entry_id")?;
            }
            for keyword in &entry.keywords {
                insert_terms(tx, &entry.id, keyword, 7, "knowledge_terms", "entry_id")?;
            }
        }
    }
    Ok(())
}

fn seed_problems(tx: &Transaction<'_>, path: &Path) -> Result<(), Box<dyn Error>> {
    let file: ProblemsFile = read_json(path)?;
    let _ = file.schema_version;
    for item in file.problems {
        tx.execute(
            "INSERT INTO common_problems(
               signature,domain,symptom,cause,solution,prevention,search_terms_json,
               confidence,occurrences,status,source_ref,updated_at
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,unixepoch())
             ON CONFLICT(signature) DO UPDATE SET
               domain=excluded.domain, symptom=excluded.symptom, cause=excluded.cause,
               solution=excluded.solution, prevention=excluded.prevention,
               search_terms_json=excluded.search_terms_json, confidence=excluded.confidence,
               occurrences=excluded.occurrences, status=excluded.status,
               source_ref=excluded.source_ref, updated_at=unixepoch()",
            params![
                item.signature,
                item.domain,
                item.symptom,
                item.cause,
                item.solution,
                item.prevention,
                serde_json::to_string(&item.search_terms)?,
                item.confidence,
                item.occurrences,
                item.status,
                item.source_ref
            ],
        )?;
        tx.execute(
            "DELETE FROM problem_terms WHERE signature=?1",
            params![item.signature],
        )?;
        insert_terms(
            tx,
            &item.signature,
            &item.signature,
            12,
            "problem_terms",
            "signature",
        )?;
        insert_terms(
            tx,
            &item.signature,
            &item.domain,
            5,
            "problem_terms",
            "signature",
        )?;
        for term in &item.search_terms {
            insert_terms(tx, &item.signature, term, 8, "problem_terms", "signature")?;
        }
    }
    Ok(())
}

fn insert_terms(
    tx: &Transaction<'_>,
    id: &str,
    text: &str,
    weight: i64,
    table: &str,
    id_column: &str,
) -> Result<(), Box<dyn Error>> {
    let sql = format!(
        "INSERT INTO {table}(term,{id_column},weight) VALUES(?1,?2,?3)
         ON CONFLICT(term,{id_column}) DO UPDATE SET weight=MAX(weight,excluded.weight)"
    );
    for term in tokenize(text) {
        tx.execute(&sql, params![term, id, weight])?;
    }
    Ok(())
}

fn seed_skills(tx: &Transaction<'_>, path: &Path) -> Result<(), Box<dyn Error>> {
    let file: SkillsFile = read_json(path)?;
    let _ = file.schema_version;
    for item in file.skills {
        tx.execute(
            "INSERT INTO skill_registry(id,manifest_path,triggers_json,knowledge_channels_json,risk,enabled)
             VALUES(?1,?2,?3,?4,?5,?6)
             ON CONFLICT(id) DO UPDATE SET
               manifest_path=excluded.manifest_path, triggers_json=excluded.triggers_json,
               knowledge_channels_json=excluded.knowledge_channels_json, risk=excluded.risk,
               enabled=excluded.enabled",
            params![
                item.id,
                item.manifest_path,
                serde_json::to_string(&item.triggers)?,
                serde_json::to_string(&item.knowledge_channels)?,
                item.risk,
                if item.enabled { 1 } else { 0 }
            ],
        )?;
    }
    Ok(())
}

fn seed_agents(tx: &Transaction<'_>, dir: &Path) -> Result<(), Box<dyn Error>> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
        .collect();
    paths.sort();

    for path in paths {
        let item: AgentSeed = read_json(&path)?;
        let manifest_path = format!(
            ".agents/agents/{}",
            path.file_name().unwrap().to_string_lossy()
        );
        tx.execute(
            "INSERT INTO agent_registry(
               id,role,model_tier,manifest_path,purpose,skills_json,knowledge_channels_json,enabled
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,1)
             ON CONFLICT(id) DO UPDATE SET
               role=excluded.role, model_tier=excluded.model_tier,
               manifest_path=excluded.manifest_path, purpose=excluded.purpose,
               skills_json=excluded.skills_json,
               knowledge_channels_json=excluded.knowledge_channels_json, enabled=1",
            params![
                item.id,
                item.role,
                item.model_tier,
                manifest_path,
                item.purpose,
                serde_json::to_string(&item.skills)?,
                serde_json::to_string(&item.knowledge_channels)?,
            ],
        )?;
    }
    Ok(())
}

fn seed_backlog(tx: &Transaction<'_>, path: &Path) -> Result<(), Box<dyn Error>> {
    let file: BacklogFile = read_json(path)?;
    let _ = file.schema_version;
    for item in file.items {
        tx.execute(
            "INSERT INTO backlog_items(
               id,epic,title,description,item_type,status,priority,sort_order,
               rationale,technical_risk,user_value,dependencies_json,affected_areas_json,
               acceptance_criteria_json,evidence_json,source_agent,source_context,updated_at
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,unixepoch())
             ON CONFLICT(id) DO UPDATE SET
               epic=excluded.epic, title=excluded.title, description=excluded.description,
               item_type=excluded.item_type, status=excluded.status, priority=excluded.priority,
               sort_order=excluded.sort_order, rationale=excluded.rationale,
               technical_risk=excluded.technical_risk, user_value=excluded.user_value,
               dependencies_json=excluded.dependencies_json,
               affected_areas_json=excluded.affected_areas_json,
               acceptance_criteria_json=excluded.acceptance_criteria_json,
               evidence_json=excluded.evidence_json, source_agent=excluded.source_agent,
               source_context=excluded.source_context, updated_at=unixepoch()",
            params![
                item.id,
                item.epic,
                item.title,
                item.description,
                item.item_type,
                item.status,
                item.priority,
                item.sort_order,
                item.rationale,
                item.technical_risk,
                item.user_value,
                serde_json::to_string(&item.dependencies)?,
                serde_json::to_string(&item.affected_areas)?,
                serde_json::to_string(&item.acceptance_criteria)?,
                serde_json::to_string(&item.evidence)?,
                item.source_agent,
                item.source_context,
            ],
        )?;
    }
    Ok(())
}

fn query_backlog(conn: &Connection, args: &[String]) -> Result<Value, Box<dyn Error>> {
    let mut status_filter: Option<String> = None;
    let mut epic_filter: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--status" => {
                i += 1;
                status_filter = Some(required_arg(args, i, "--status value")?.to_string());
            }
            "--epic" => {
                i += 1;
                epic_filter = Some(required_arg(args, i, "--epic value")?.to_string());
            }
            _ => {
                return Err(format!("unknown backlog option: {}", args[i]).into());
            }
        }
        i += 1;
    }

    let mut sql = String::from(
        "SELECT id,epic,title,item_type,status,priority,sort_order,
                rationale,technical_risk,user_value,dependencies_json,affected_areas_json,
                acceptance_criteria_json,evidence_json
         FROM backlog_items WHERE 1=1",
    );
    if status_filter.is_some() {
        sql.push_str(" AND status=?");
    }
    if epic_filter.is_some() {
        sql.push_str(" AND epic=?");
    }
    sql.push_str(" ORDER BY status, priority DESC, sort_order ASC");

    let mut status_ref: &str = "";
    let mut epic_ref: &str = "";
    if let Some(s) = status_filter.as_deref() {
        status_ref = s;
    }
    if let Some(e) = epic_filter.as_deref() {
        epic_ref = e;
    }
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
    if status_filter.is_some() {
        params.push(&status_ref);
    }
    if epic_filter.is_some() {
        params.push(&epic_ref);
    }
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params.as_slice(), |r| {
        let deps: String = r.get(10)?;
        let areas: String = r.get(11)?;
        let criteria: String = r.get(12)?;
        let evidence: String = r.get(13)?;
        Ok(BacklogHit {
            id: r.get(0)?,
            epic: r.get(1)?,
            title: r.get(2)?,
            item_type: r.get(3)?,
            status: r.get(4)?,
            priority: r.get(5)?,
            sort_order: r.get(6)?,
            rationale: r.get(7)?,
            technical_risk: r.get(8)?,
            user_value: r.get(9)?,
            dependencies: serde_json::from_str(&deps).unwrap_or_default(),
            affected_areas: serde_json::from_str(&areas).unwrap_or_default(),
            acceptance_criteria: serde_json::from_str(&criteria).unwrap_or_default(),
            evidence: serde_json::from_str(&evidence).unwrap_or_default(),
        })
    })?;
    Ok(json!({"ok": true, "results": rows.collect::<Result<Vec<_>, _>>()?}))
}

pub(crate) fn query_instructions(
    conn: &Connection,
    scope: &str,
    trigger: Option<&str>,
) -> Result<Value, Box<dyn Error>> {
    let mut stmt = conn.prepare(
        "SELECT id,scope,trigger,priority,body,source_ref
         FROM instructions
         WHERE enabled=1
           AND (scope='global' OR scope=?1)
           AND (?2 IS NULL OR trigger IS NULL OR trigger=?2)
         ORDER BY priority DESC, id ASC",
    )?;
    let rows = stmt.query_map(params![scope, trigger], |r| {
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "scope": r.get::<_, String>(1)?,
            "trigger": r.get::<_, Option<String>>(2)?,
            "priority": r.get::<_, i64>(3)?,
            "body": r.get::<_, String>(4)?,
            "source_ref": r.get::<_, Option<String>>(5)?
        }))
    })?;
    Ok(json!({"ok": true, "results": rows.collect::<Result<Vec<_>, _>>()?}))
}

pub(crate) fn query_knowledge(
    conn: &Connection,
    channel: &str,
    query: &str,
    limit: usize,
) -> Result<Value, Box<dyn Error>> {
    let exact: Option<KnowledgeHit> = conn
        .query_row(
            "SELECT id,channel,lookup_key,topic,content,source_ref,source_version,authority
             FROM knowledge_entries
             WHERE enabled=1 AND channel=?1 AND lower(lookup_key)=lower(?2)
             ORDER BY authority DESC LIMIT 1",
            params![channel, query],
            |r| {
                Ok(KnowledgeHit {
                    id: r.get(0)?,
                    channel: r.get(1)?,
                    lookup_key: r.get(2)?,
                    topic: r.get(3)?,
                    content: r.get(4)?,
                    source_ref: r.get(5)?,
                    source_version: r.get(6)?,
                    authority: r.get(7)?,
                    score: 1000,
                })
            },
        )
        .optional()?;

    if let Some(hit) = exact {
        return Ok(json!({"ok": true, "mode": "exact", "results": [hit]}));
    }

    let mut hits: HashMap<String, KnowledgeHit> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT ke.id,ke.channel,ke.lookup_key,ke.topic,ke.content,
                ke.source_ref,ke.source_version,ke.authority,kt.weight
         FROM knowledge_terms kt
         JOIN knowledge_entries ke ON ke.id=kt.entry_id
         WHERE ke.enabled=1 AND ke.channel=?1 AND kt.term=?2
         ORDER BY kt.weight DESC, ke.authority DESC",
    )?;
    for term in tokenize(query) {
        let rows = stmt.query_map(params![channel, term], |r| {
            Ok(KnowledgeHit {
                id: r.get(0)?,
                channel: r.get(1)?,
                lookup_key: r.get(2)?,
                topic: r.get(3)?,
                content: r.get(4)?,
                source_ref: r.get(5)?,
                source_version: r.get(6)?,
                authority: r.get(7)?,
                score: r.get(8)?,
            })
        })?;
        for row in rows {
            let hit = row?;
            hits.entry(hit.id.clone())
                .and_modify(|h| h.score += hit.score)
                .or_insert(hit);
        }
    }
    let mut values: Vec<KnowledgeHit> = hits.into_values().collect();
    values.sort_by(|a, b| b.score.cmp(&a.score).then(b.authority.cmp(&a.authority)));
    values.truncate(limit);
    Ok(json!({"ok": true, "mode": "indexed", "results": values}))
}

pub(crate) fn query_problems(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Value, Box<dyn Error>> {
    let exact: Option<ProblemHit> = conn
        .query_row(
            "SELECT signature,domain,symptom,cause,solution,prevention,
                    confidence,occurrences,source_ref
             FROM common_problems
             WHERE status='active' AND signature=?1 LIMIT 1",
            params![query],
            |r| {
                Ok(ProblemHit {
                    signature: r.get(0)?,
                    domain: r.get(1)?,
                    symptom: r.get(2)?,
                    cause: r.get(3)?,
                    solution: r.get(4)?,
                    prevention: r.get(5)?,
                    confidence: r.get(6)?,
                    occurrences: r.get(7)?,
                    source_ref: r.get(8)?,
                    score: 1000,
                })
            },
        )
        .optional()?;

    if let Some(hit) = exact {
        return Ok(json!({"ok": true, "mode": "exact", "results": [hit]}));
    }

    let mut hits: HashMap<String, ProblemHit> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT cp.signature,cp.domain,cp.symptom,cp.cause,cp.solution,cp.prevention,
                cp.confidence,cp.occurrences,cp.source_ref,pt.weight
         FROM problem_terms pt
         JOIN common_problems cp ON cp.signature=pt.signature
         WHERE cp.status='active' AND pt.term=?1
         ORDER BY pt.weight DESC, cp.confidence DESC",
    )?;
    for term in tokenize(query) {
        let rows = stmt.query_map(params![term], |r| {
            Ok(ProblemHit {
                signature: r.get(0)?,
                domain: r.get(1)?,
                symptom: r.get(2)?,
                cause: r.get(3)?,
                solution: r.get(4)?,
                prevention: r.get(5)?,
                confidence: r.get(6)?,
                occurrences: r.get(7)?,
                source_ref: r.get(8)?,
                score: r.get(9)?,
            })
        })?;
        for row in rows {
            let hit = row?;
            hits.entry(hit.signature.clone())
                .and_modify(|h| h.score += hit.score)
                .or_insert(hit);
        }
    }
    let mut values: Vec<ProblemHit> = hits.into_values().collect();
    values.sort_by(|a, b| b.score.cmp(&a.score).then(b.confidence.cmp(&a.confidence)));
    values.truncate(limit);
    Ok(json!({"ok": true, "mode": "indexed", "results": values}))
}

fn query_agent(conn: &Connection, id: &str) -> Result<Value, Box<dyn Error>> {
    let result: Option<Value> = conn
        .query_row(
            "SELECT id,role,model_tier,manifest_path,purpose,skills_json,knowledge_channels_json
             FROM agent_registry WHERE enabled=1 AND id=?1",
            params![id],
            |r| {
                let skills: String = r.get(5)?;
                let channels: String = r.get(6)?;
                Ok(json!({
                    "id": r.get::<_, String>(0)?,
                    "role": r.get::<_, String>(1)?,
                    "model_tier": r.get::<_, String>(2)?,
                    "manifest_path": r.get::<_, String>(3)?,
                    "purpose": r.get::<_, String>(4)?,
                    "skills": serde_json::from_str::<Value>(&skills).unwrap_or(json!([])),
                    "knowledge_channels": serde_json::from_str::<Value>(&channels).unwrap_or(json!([]))
                }))
            },
        )
        .optional()?;
    Ok(json!({"ok": true, "result": result}))
}

fn query_skill(conn: &Connection, id: &str) -> Result<Value, Box<dyn Error>> {
    let result: Option<Value> = conn
        .query_row(
            "SELECT id,manifest_path,triggers_json,knowledge_channels_json,risk
             FROM skill_registry WHERE enabled=1 AND id=?1",
            params![id],
            |r| {
                let triggers: String = r.get(2)?;
                let channels: String = r.get(3)?;
                Ok(json!({
                    "id": r.get::<_, String>(0)?,
                    "manifest_path": r.get::<_, String>(1)?,
                    "triggers": serde_json::from_str::<Value>(&triggers).unwrap_or(json!([])),
                    "knowledge_channels": serde_json::from_str::<Value>(&channels).unwrap_or(json!([])),
                    "risk": r.get::<_, String>(4)?
                }))
            },
        )
        .optional()?;
    Ok(json!({"ok": true, "result": result}))
}

fn cache_put(
    conn: &Connection,
    scope: &str,
    key: &str,
    payload: &str,
    ttl_seconds: i64,
) -> Result<(), Box<dyn Error>> {
    let _: Value = serde_json::from_str(payload)?;
    let expires = now_unix() + ttl_seconds.max(0);
    conn.execute(
        "INSERT INTO context_cache(cache_key,scope,payload_json,expires_at,hits,updated_at)
         VALUES(?1,?2,?3,?4,0,unixepoch())
         ON CONFLICT(cache_key) DO UPDATE SET
           scope=excluded.scope,payload_json=excluded.payload_json,
           expires_at=excluded.expires_at,hits=0,updated_at=unixepoch()",
        params![key, scope, payload, expires],
    )?;
    Ok(())
}

fn cache_get(conn: &Connection, scope: &str, key: &str) -> Result<Value, Box<dyn Error>> {
    let result: Option<String> = conn
        .query_row(
            "SELECT payload_json FROM context_cache
             WHERE cache_key=?1 AND scope=?2
               AND (expires_at IS NULL OR expires_at>=?3)",
            params![key, scope, now_unix()],
            |r| r.get(0),
        )
        .optional()?;
    if result.is_some() {
        conn.execute(
            "UPDATE context_cache SET hits=hits+1 WHERE cache_key=?1 AND scope=?2",
            params![key, scope],
        )?;
    }
    let parsed = result
        .as_deref()
        .map(serde_json::from_str::<Value>)
        .transpose()?;
    Ok(json!({"ok": true, "result": parsed}))
}

fn validate_registry(conn: &Connection, repo_root: &Path) -> Result<Value, Box<dyn Error>> {
    let mut missing = Vec::new();

    let mut skill_stmt =
        conn.prepare("SELECT id,manifest_path FROM skill_registry WHERE enabled=1 ORDER BY id")?;
    let skills =
        skill_stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for row in skills {
        let (id, path) = row?;
        if !repo_root.join(&path).exists() {
            missing.push(json!({"type":"skill","id":id,"path":path}));
        }
    }

    let mut agent_stmt =
        conn.prepare("SELECT id,manifest_path FROM agent_registry WHERE enabled=1 ORDER BY id")?;
    let agents =
        agent_stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for row in agents {
        let (id, path) = row?;
        if !repo_root.join(&path).exists() {
            missing.push(json!({"type":"agent","id":id,"path":path}));
        }
    }

    Ok(json!({"ok": missing.is_empty(), "missing": missing}))
}

fn stats(conn: &Connection) -> Result<Value, Box<dyn Error>> {
    let count = |table: &str| -> Result<i64, rusqlite::Error> {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
    };
    Ok(json!({
        "ok": true,
        "instructions": count("instructions")?,
        "knowledge_entries": count("knowledge_entries")?,
        "knowledge_terms": count("knowledge_terms")?,
        "common_problems": count("common_problems")?,
        "problem_terms": count("problem_terms")?,
        "backlog_items": count("backlog_items")?,
        "skills": count("skill_registry")?,
        "agents": count("agent_registry")?,
        "context_cache": count("context_cache")?
    }))
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

pub(crate) fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.' || c == '-'))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn print_json(value: Value) {
    println!("{}", serde_json::to_string(&value).unwrap());
}
