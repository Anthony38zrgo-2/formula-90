. "$PSScriptRoot\_common.ps1"

Write-Section "Últimas fuentes"
Invoke-SqliteReadOnly "SELECT * FROM sources LIMIT 25;"

Write-Section "Problemas comunes"
Invoke-SqliteReadOnly "SELECT * FROM common_problems LIMIT 25;"

Write-Section "Skills registradas"
Invoke-SqliteReadOnly "SELECT * FROM skill_registry LIMIT 50;"

Write-Section "Agentes registrados"
Invoke-SqliteReadOnly "SELECT * FROM agent_registry LIMIT 50;"

Write-Section "Context cache"
Invoke-SqliteReadOnly "SELECT * FROM context_cache LIMIT 50;"

Write-Section "Telemetría: modelos usados (requested/effective)"
Invoke-SqliteReadOnly "SELECT COALESCE(effective_model, requested_model) AS model, COUNT(*) AS calls FROM model_calls GROUP BY model ORDER BY calls DESC;"

Write-Section "Telemetría: distribución por tier"
Invoke-SqliteReadOnly "SELECT model_tier, COUNT(*) AS calls FROM model_calls GROUP BY model_tier ORDER BY calls DESC;"

Write-Section "Telemetría: distribución por fuente de routing"
Invoke-SqliteReadOnly "SELECT routing_source, COUNT(*) AS calls FROM model_calls GROUP BY routing_source ORDER BY calls DESC;"

Write-Section "Telemetría: retries promedio por run exitoso"
Invoke-SqliteReadOnly "SELECT AVG(retries) AS avg_retries_per_success FROM agent_runs WHERE outcome = 'success';"

Write-Section "Telemetría: knowledge hit rate"
Invoke-SqliteReadOnly "SELECT SUM(knowledge_queries) AS queries, SUM(knowledge_hits) AS hits, ROUND(100.0 * SUM(knowledge_hits) / NULLIF(SUM(knowledge_queries), 0), 1) AS hit_rate_pct FROM agent_runs;"

Write-Section "Telemetría: problem hit rate"
Invoke-SqliteReadOnly "SELECT SUM(problem_queries) AS queries, SUM(problem_hits) AS hits, ROUND(100.0 * SUM(problem_hits) / NULLIF(SUM(problem_queries), 0), 1) AS hit_rate_pct FROM agent_runs;"

Write-Section "Telemetría: cache hit rate"
Invoke-SqliteReadOnly "SELECT SUM(cache_queries) AS queries, SUM(cache_hits) AS hits, ROUND(100.0 * SUM(cache_hits) / NULLIF(SUM(cache_queries), 0), 1) AS hit_rate_pct FROM agent_runs;"

Write-Section "Telemetría: tokens por run exitoso"
Invoke-SqliteReadOnly "SELECT ROUND(CAST(SUM(input_tokens + output_tokens) AS REAL) / NULLIF(SUM(CASE WHEN outcome='success' THEN 1 ELSE 0 END), 0), 1) AS tokens_per_successful_run FROM agent_runs;"

Write-Section "Telemetría: strong escalation rate"
Invoke-SqliteReadOnly "SELECT COUNT(*) AS strong_calls, (SELECT COUNT(*) FROM model_calls) AS total_calls, ROUND(100.0 * COUNT(*) / (SELECT COUNT(*) FROM model_calls), 1) AS escalation_rate_pct FROM model_calls WHERE model_tier = 'strong';"

Write-Section "Telemetría: runs por run_kind"
Invoke-SqliteReadOnly "SELECT run_kind, COUNT(*) AS runs FROM agent_runs GROUP BY run_kind ORDER BY runs DESC;"

Write-Section "Telemetría: runs por experiment_phase"
Invoke-SqliteReadOnly "SELECT experiment_phase, COUNT(*) AS runs FROM agent_runs GROUP BY experiment_phase ORDER BY runs DESC;"

Write-Section "Telemetría: model calls por capacity_source"
Invoke-SqliteReadOnly "SELECT capacity_source, COUNT(*) AS calls FROM model_calls GROUP BY capacity_source ORDER BY calls DESC;"

Write-Section "Telemetría: capacidad strong (planned vs automatic)"
Invoke-SqliteReadOnly "SELECT capacity_source, requested_model, COUNT(*) AS calls FROM model_calls WHERE model_tier = 'strong' GROUP BY capacity_source, requested_model ORDER BY calls DESC;"

Write-Section "Telemetría: metricas de exito solo productivas"
Invoke-SqliteReadOnly "SELECT COUNT(*) AS runs, SUM(CASE WHEN outcome='success' THEN 1 ELSE 0 END) AS success, SUM(CASE WHEN outcome='success' AND retries=0 THEN 1 ELSE 0 END) AS first_attempt, SUM(retries) AS retries FROM agent_runs WHERE run_kind='productive';"

Write-Section "Telemetría: knowledge usage productivo"
Invoke-SqliteReadOnly "SELECT SUM(knowledge_queries) AS queries, SUM(knowledge_hits) AS hits, SUM(CASE WHEN knowledge_queries >= 1 THEN 1 ELSE 0 END) AS runs_using_knowledge, COUNT(*) AS productive_runs FROM agent_runs WHERE run_kind='productive';"

Write-Section "Telemetría: knowledge usage sintetico"
Invoke-SqliteReadOnly "SELECT SUM(knowledge_queries) AS queries, SUM(knowledge_hits) AS hits FROM agent_runs WHERE run_kind IN ('probe','control','instrumentation','bootstrap','unknown');"

Write-Section "Telemetría: prefetch de contexto (planning/execution)"
Invoke-SqliteReadOnly "SELECT source, event_type, COUNT(*) AS n FROM telemetry_events WHERE source IN ('planning_prefetch','execution_prefetch') GROUP BY source, event_type ORDER BY source, event_type;"
