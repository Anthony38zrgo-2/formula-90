. "$PSScriptRoot\_common.ps1"

$env:AGENT_DB = $DbPath
$agentdb = Assert-AgentDb

$metricsAll = (& $agentdb metrics all | Out-String | ConvertFrom-Json)
$metricsProd = (& $agentdb metrics productive | Out-String | ConvertFrom-Json)

Write-Section "ALL RUNS"
$c = $metricsAll.counts
Write-Host ("{0,-28} {1}" -f "Runs", $c.runs)
Write-Host ("{0,-28} {1}" -f "Success", $c.successful_runs)
Write-Host ("{0,-28} {1}" -f "First attempt", $c.first_attempt_success)
Write-Host ("{0,-28} {1}" -f "Retries", $c.retries)
if ($null -ne $metricsAll.successful_run_rate) {
    Write-Host ("{0,-28} {1:P1}" -f "Successful run rate", $metricsAll.successful_run_rate)
}
if ($null -ne $metricsAll.first_attempt_success_rate) {
    Write-Host ("{0,-28} {1:P1}" -f "First-attempt success rate", $metricsAll.first_attempt_success_rate)
}

Write-Section "PRODUCTIVE RUNS"
$cp = $metricsProd.counts
Write-Host ("{0,-28} {1}" -f "Runs", $cp.runs)
Write-Host ("{0,-28} {1}" -f "Success", $cp.successful_runs)
Write-Host ("{0,-28} {1}" -f "First attempt", $cp.first_attempt_success)
Write-Host ("{0,-28} {1}" -f "Retries", $cp.retries)
if ($null -ne $metricsProd.successful_run_rate) {
    Write-Host ("{0,-28} {1:P1}" -f "Successful run rate", $metricsProd.successful_run_rate)
}
if ($null -ne $metricsProd.first_attempt_success_rate) {
    Write-Host ("{0,-28} {1:P1}" -f "First-attempt success rate", $metricsProd.first_attempt_success_rate)
}
if ($null -ne $metricsProd.input_tokens_per_success) {
    Write-Host ("{0,-28} {1:N0}" -f "Tokens per success", $metricsProd.input_tokens_per_success)
}
if ($null -ne $metricsProd.duration_per_success_ms) {
    Write-Host ("{0,-28} {1:N0} ms" -f "Duration per success", $metricsProd.duration_per_success_ms)
}

Write-Section "SYNTHETIC RUNS"
$kind = $metricsAll.run_kind_counts
$syntheticTotal = 0
foreach ($k in @("probe", "control", "instrumentation", "bootstrap", "unknown")) {
    $n = 0
    if ($kind.PSObject.Properties.Name -contains $k) { $n = $kind.$k }
    $syntheticTotal += $n
    Write-Host ("{0,-28} {1}" -f $k, $n)
}
$kc = $metricsAll.knowledge_consumption
Write-Host ("{0,-28} {1}" -f "Synthetic knowledge queries", $kc.synthetic_queries)
Write-Host ("{0,-28} {1}" -f "Synthetic knowledge hits", $kc.synthetic_hits)

Write-Section "EXPERIMENT PHASES"
$phases = $metricsAll.phase_counts
$phaseTotal = 0
foreach ($p in $phases.PSObject.Properties) {
    $phaseTotal += $p.Value
    Write-Host ("{0,-28} {1}" -f $p.Name, $p.Value)
}
Write-Host ("{0,-28} {1}" -f "Total", $phaseTotal)

Write-Section "MODEL CAPACITY"
$cap = $metricsAll.capacity
Write-Host ("{0,-28} {1}" -f "Strong calls", $cap.strong_usage)
Write-Host ("{0,-28} {1}" -f "Automatic escalation", $cap.automatic_escalation)
Write-Host ("{0,-28} {1}" -f "Manual override", $cap.manual_override)
Write-Host ("{0,-28} {1}" -f "Planned capacity", $cap.planned_capacity)
if ($null -ne $metricsAll.strong_usage_rate) {
    Write-Host ("{0,-28} {1:P1}" -f "Strong usage rate", $metricsAll.strong_usage_rate)
}
if ($null -ne $metricsAll.automatic_escalation_rate) {
    Write-Host ("{0,-28} {1:P1}" -f "Automatic escalation rate", $metricsAll.automatic_escalation_rate)
}
if ($null -ne $metricsAll.manual_override_rate) {
    Write-Host ("{0,-28} {1:P1}" -f "Manual override rate", $metricsAll.manual_override_rate)
}
if ($null -ne $metricsAll.planned_capacity_rate) {
    Write-Host ("{0,-28} {1:P1}" -f "Planned capacity rate", $metricsAll.planned_capacity_rate)
}

Write-Section "KNOWLEDGE CONSUMPTION"
Write-Host ("{0,-28} {1}" -f "Productive knowledge queries", $kc.productive_queries)
Write-Host ("{0,-28} {1}" -f "Productive knowledge hits", $kc.productive_hits)
Write-Host ("{0,-28} {1}" -f "Planning-prefetch queries", $kc.planning_prefetch_queries)
Write-Host ("{0,-28} {1}" -f "Planning-prefetch hits", $kc.planning_prefetch_hits)
Write-Host ("{0,-28} {1}" -f "Execution-prefetch queries", $kc.execution_prefetch_queries)
Write-Host ("{0,-28} {1}" -f "Execution-prefetch hits", $kc.execution_prefetch_hits)
if ($null -ne $metricsAll.productive_knowledge_usage_rate) {
    Write-Host ("{0,-28} {1:P1}" -f "Productive runs using Knowledge", $metricsAll.productive_knowledge_usage_rate)
}
if ($null -ne $metricsAll.productive_problem_usage_rate) {
    Write-Host ("{0,-28} {1:P1}" -f "Productive runs using Problems", $metricsAll.productive_problem_usage_rate)
}
Write-Host ""
Write-Host "NOTE: token counts are only reported where the provider exposes measured usage (codex exec turn.completed / opencode step_finish). NULL/unknown otherwise. Never invented."
