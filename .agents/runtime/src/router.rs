use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, error::Error, fs, path::Path};

#[derive(Deserialize, Clone)]
pub struct RoutingConfig {
    pub schema_version: u32,
    pub providers: Providers,
    pub defaults: Defaults,
    pub luna_effort: HashMap<String, String>,
    pub sol_effort: HashMap<String, String>,
    pub limits: Limits,
    #[serde(default)]
    pub codex_profiles: HashMap<String, ProfileSpec>,
}

#[derive(Deserialize, Clone)]
pub struct Providers {
    pub deepseek: DeepSeekProvider,
    pub openai_codex: OpenAiCodexProvider,
}

#[derive(Deserialize, Clone)]
pub struct DeepSeekProvider {
    pub enabled: bool,
    pub default_model: String,
}

#[derive(Deserialize, Clone)]
pub struct OpenAiCodexProvider {
    pub enabled: bool,
    pub luna_model: String,
    pub sol_model: String,
}

#[derive(Deserialize, Clone)]
pub struct Defaults {
    pub cheap_worker: String,
    pub codex_worker: String,
    pub strong_model: String,
}

#[derive(Deserialize, Clone)]
pub struct Limits {
    pub max_attempts_before_strong_escalation: i64,
    pub cross_subsystem_threshold: i64,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ProfileSpec {
    pub model: String,
    pub effort: String,
}

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
pub struct TaskMetadata {
    pub agent_id: String,
    pub role: String,
    pub task_type: String,
    pub attempt_count: i64,
    pub affected_files: i64,
    pub affected_subsystems: i64,
    pub architecture_change: bool,
    pub cross_subsystem_change: bool,
    pub technical_risk: String,
    pub knowledge_hit: bool,
    pub problem_hit: bool,
    pub root_cause_unknown: bool,
    pub conflicting_architectural_constraints: bool,
    pub ownership_boundary_refactor: bool,
    pub large_regression: bool,
    pub user_model_override: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct RoutingDecision {
    pub provider: String,
    pub requested_model: String,
    pub requested_effort: String,
    pub model_tier: String,
    pub reason: String,
    pub rule: String,
    pub profile: Option<String>,
}

pub fn load_routing_config(agents_root: &Path) -> Result<RoutingConfig, Box<dyn Error>> {
    let path = agents_root.join("config/model-routing.json");
    let data = fs::read_to_string(&path)?;
    let cfg: RoutingConfig = serde_json::from_str(&data)?;
    if cfg.schema_version != 1 {
        return Err(format!(
            "unsupported model-routing schema_version: {}",
            cfg.schema_version
        )
        .into());
    }
    if cfg.defaults.cheap_worker != cfg.providers.deepseek.default_model {
        return Err(format!(
            "model-routing policy mismatch: defaults.cheap_worker '{}' != providers.deepseek.default_model '{}'",
            cfg.defaults.cheap_worker, cfg.providers.deepseek.default_model
        )
        .into());
    }
    if cfg.defaults.codex_worker != cfg.providers.openai_codex.luna_model {
        return Err(format!(
            "model-routing policy mismatch: defaults.codex_worker '{}' != providers.openai_codex.luna_model '{}'",
            cfg.defaults.codex_worker, cfg.providers.openai_codex.luna_model
        )
        .into());
    }
    if cfg.defaults.strong_model != cfg.providers.openai_codex.sol_model {
        return Err(format!(
            "model-routing policy mismatch: defaults.strong_model '{}' != providers.openai_codex.sol_model '{}'",
            cfg.defaults.strong_model, cfg.providers.openai_codex.sol_model
        )
        .into());
    }
    Ok(cfg)
}

pub fn deepseek_available(cfg: &RoutingConfig) -> bool {
    if !cfg.providers.deepseek.enabled {
        return false;
    }
    match std::env::var("AGENT_DEEPSEEK_AVAILABLE") {
        Ok(v) => matches!(v.trim().to_lowercase().as_str(), "1" | "true" | "yes"),
        Err(_) => true,
    }
}

fn sol_decision(cfg: &RoutingConfig, reason: &str, rule: &str) -> RoutingDecision {
    if !cfg.providers.openai_codex.enabled {
        return deepseek_decision(
            cfg,
            &format!("{reason} (openai_codex disabled in policy)"),
            &format!("{rule}_codex_disabled"),
        );
    }
    RoutingDecision {
        provider: "openai_codex".into(),
        requested_model: cfg.providers.openai_codex.sol_model.clone(),
        requested_effort: cfg
            .sol_effort
            .get("default")
            .cloned()
            .unwrap_or_else(|| "medium".into()),
        model_tier: "strong".into(),
        reason: reason.into(),
        rule: rule.into(),
        profile: Some("sol_medium".into()),
    }
}

fn luna_decision(cfg: &RoutingConfig, effort: &str, reason: &str, rule: &str) -> RoutingDecision {
    if !cfg.providers.openai_codex.enabled {
        return deepseek_decision(
            cfg,
            &format!("{reason} (openai_codex disabled in policy)"),
            &format!("{rule}_codex_disabled"),
        );
    }
    RoutingDecision {
        provider: "openai_codex".into(),
        requested_model: cfg.providers.openai_codex.luna_model.clone(),
        requested_effort: effort.into(),
        model_tier: "fast".into(),
        reason: reason.into(),
        rule: rule.into(),
        profile: Some(format!("luna_{effort}")),
    }
}

fn deepseek_decision(cfg: &RoutingConfig, reason: &str, rule: &str) -> RoutingDecision {
    RoutingDecision {
        provider: "deepseek".into(),
        requested_model: cfg.providers.deepseek.default_model.clone(),
        requested_effort: "n/a".into(),
        model_tier: "fast".into(),
        reason: reason.into(),
        rule: rule.into(),
        profile: None,
    }
}

pub fn decide(
    cfg: &RoutingConfig,
    meta: &TaskMetadata,
    deepseek_avail: bool,
    role_from_registry: Option<&str>,
) -> RoutingDecision {
    let role = if !meta.role.trim().is_empty() {
        meta.role.trim()
    } else {
        role_from_registry.unwrap_or("developer-tooling")
    };
    let task_type = meta.task_type.trim().to_lowercase();
    let risk = meta.technical_risk.trim().to_lowercase();
    let max_attempts = cfg.limits.max_attempts_before_strong_escalation.max(1);
    let cross_threshold = cfg.limits.cross_subsystem_threshold.max(1);

    if let Some(override_raw) = &meta.user_model_override {
        let o = override_raw.trim().to_lowercase();
        return match o.as_str() {
            "sol" | "sol_medium" | "gpt-5.6-sol" | "gpt-5.6-sol-medium" => {
                sol_decision(cfg, "user explicitly requested Sol", "user_override")
            }
            "luna" | "luna_medium" | "gpt-5.6-luna" | "gpt-5.6-luna-medium" => luna_decision(
                cfg,
                "medium",
                "user explicitly requested Luna medium",
                "user_override",
            ),
            "luna_low" | "luna-low" => luna_decision(
                cfg,
                "low",
                "user explicitly requested Luna low",
                "user_override",
            ),
            "luna_high" | "luna-high" => luna_decision(
                cfg,
                "high",
                "user explicitly requested Luna high",
                "user_override",
            ),
            "deepseek" | "deepseek-v4-flash" | "deepseek/deepseek-v4-flash" => {
                deepseek_decision(cfg, "user explicitly requested DeepSeek", "user_override")
            }
            _ => luna_decision(
                cfg,
                "medium",
                "unrecognized user override, falling back to Luna medium",
                "user_override",
            ),
        };
    }

    struct HardRule {
        reason: String,
        rule: &'static str,
    }

    let mut hard: Option<HardRule> = None;
    macro_rules! hard_if {
        ($cond:expr, $rule:expr, $reason:expr) => {
            if hard.is_none() && $cond {
                hard = Some(HardRule {
                    reason: $reason.into(),
                    rule: $rule,
                });
            }
        };
    }

    hard_if!(
        role == "product-owner",
        "role_product_owner",
        "product-owner role requires product-level reasoning"
    );
    hard_if!(
        role == "architect",
        "role_architect",
        "architect role requires architecture-level reasoning"
    );
    hard_if!(
        role == "retrospective",
        "role_retrospective",
        "retrospective synthesis requires strong reasoning"
    );
    hard_if!(
        meta.architecture_change,
        "architecture_change",
        "architecture change requires strong tier"
    );
    hard_if!(
        meta.cross_subsystem_change && meta.affected_subsystems >= cross_threshold,
        "cross_subsystem_change",
        format!(
            "cross-subsystem change affecting {0} subsystems",
            meta.affected_subsystems
        )
    );
    hard_if!(
        meta.large_regression,
        "large_regression",
        "large regression affecting multiple systems"
    );
    hard_if!(risk == "high", "high_technical_risk", "high technical risk");
    hard_if!(
        meta.conflicting_architectural_constraints,
        "conflicting_constraints",
        "confirmed conflicting architectural constraints"
    );
    hard_if!(
        meta.ownership_boundary_refactor,
        "ownership_boundary_refactor",
        "major refactor involving ownership boundaries"
    );
    hard_if!(
        meta.attempt_count >= max_attempts,
        "attempt_budget_exhausted",
        format!(
            "same unresolved failure after {0} attempts",
            meta.attempt_count
        )
    );
    hard_if!(
        meta.root_cause_unknown
            && meta.attempt_count >= 1
            && !meta.knowledge_hit
            && !meta.problem_hit
            && risk != "low",
        "consequential_unknown_failure",
        "knowledge and problem lookup failed and root cause unknown"
    );

    if let Some(h) = hard {
        return sol_decision(cfg, &h.reason, h.rule);
    }

    if role == "reviewer" {
        return luna_decision(
            cfg,
            "medium",
            "reviewer role executes bounded reviews",
            "role_reviewer",
        );
    }

    match task_type.as_str() {
        "lookup" | "scan" | "search" | "validate" | "format" | "documentation" | "summarize" => {
            let effort = cfg
                .luna_effort
                .get("lookup")
                .cloned()
                .unwrap_or_else(|| "low".into());
            return luna_decision(cfg, &effort, "bounded read-only lookup", "bounded_lookup");
        }
        "test" => {
            return luna_decision(
                cfg,
                "low",
                "test execution and validation",
                "test_execution",
            );
        }
        "review" => {
            return luna_decision(cfg, "medium", "bounded code review", "bounded_review");
        }
        "debugging" => {
            if meta.attempt_count <= 0 {
                return cheap_decision(
                    cfg,
                    deepseek_avail,
                    "initial debugging attempt",
                    "debug_initial",
                );
            }
            if meta.attempt_count == 1 {
                return luna_decision(
                    cfg,
                    "high",
                    "bounded debugging after first failed attempt",
                    "debug_attempt_1",
                );
            }
        }
        _ => {}
    }

    if matches!(
        task_type.as_str(),
        "implementation" | "bugfix" | "refactor" | "backlog" | "planning" | ""
    ) && meta.attempt_count < max_attempts
    {
        if meta.attempt_count == 0 {
            return cheap_decision(
                cfg,
                deepseek_avail,
                "routine implementation",
                "routine_implementation",
            );
        }
        return cheap_decision(
            cfg,
            deepseek_avail,
            "routine implementation retry",
            "routine_implementation_retry",
        );
    }

    if meta.attempt_count < max_attempts {
        return cheap_decision(
            cfg,
            deepseek_avail,
            "cheap tier bounded work",
            "bounded_cheap",
        );
    }

    sol_decision(
        cfg,
        "attempt budget exhausted (safety fallback)",
        "attempt_budget_exhausted",
    )
}

fn cheap_decision(
    cfg: &RoutingConfig,
    deepseek_avail: bool,
    reason: &str,
    rule: &str,
) -> RoutingDecision {
    if deepseek_avail {
        deepseek_decision(cfg, reason, rule)
    } else {
        luna_decision(
            cfg,
            "medium",
            &format!("{reason} (DeepSeek unavailable, fallback)"),
            &format!("{rule}_fallback"),
        )
    }
}

pub fn cmd_route(
    conn: &Connection,
    args: &[String],
    agents_root: &Path,
) -> Result<Value, Box<dyn Error>> {
    let payload = args
        .get(2)
        .ok_or_else(|| "missing argument: task-metadata-json".to_string())?;
    let payload = crate::telemetry::decode_payload_arg(payload)?;
    let meta: TaskMetadata = serde_json::from_str(&payload)?;
    let cfg = load_routing_config(agents_root)?;

    let role_from_registry: Option<String> =
        if meta.role.trim().is_empty() && !meta.agent_id.trim().is_empty() {
            conn.query_row(
                "SELECT role FROM agent_registry WHERE enabled=1 AND id=?1",
                params![meta.agent_id],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten()
        } else {
            None
        };

    let decision = decide(
        &cfg,
        &meta,
        deepseek_available(&cfg),
        role_from_registry.as_deref(),
    );

    let run_id = crate::telemetry::run_id_from_env();
    if let Some(rid) = run_id.as_deref() {
        let detail = serde_json::to_string(&decision)?;
        crate::telemetry::emit_event(
            conn,
            Some(rid),
            "routing_decision",
            Some("route"),
            Some(decision.rule.as_str()),
            None,
            Some(&detail),
        )?;
        if decision.model_tier == "strong" {
            crate::telemetry::emit_event(
                conn,
                Some(rid),
                "escalation",
                Some("route"),
                Some(decision.rule.as_str()),
                Some(meta.attempt_count),
                Some(&detail),
            )?;
        }
        if meta.attempt_count >= 1 {
            crate::telemetry::emit_event(
                conn,
                Some(rid),
                "retry",
                Some("route"),
                Some("attempt_count"),
                Some(meta.attempt_count),
                None,
            )?;
        }
    }

    Ok(json!({
        "ok": true,
        "provider": decision.provider,
        "requested_model": decision.requested_model,
        "requested_effort": decision.requested_effort,
        "model_tier": decision.model_tier,
        "reason": decision.reason,
        "rule": decision.rule,
        "profile": decision.profile,
        "deepseek_available": deepseek_available(&cfg),
        "codex_profiles": cfg.codex_profiles,
        "role": role_from_registry.or_else(|| (!meta.role.is_empty()).then(|| meta.role.clone()))
    }))
}
