. "$PSScriptRoot\_common.ps1"

$env:AGENT_DB = $DbPath
$agentdb = Assert-AgentDb
$failed = $false
$controlRunId = "run_control_tests_{0}" -f ([guid]::NewGuid().ToString("N").Substring(0, 8))
$instrumentRunId = "run_instrumentation_test_{0}" -f ([guid]::NewGuid().ToString("N").Substring(0, 8))

$runJson = (& $agentdb run-start developer-tooling CONTROL-TESTS --run-id $controlRunId --run-kind control | Out-String | ConvertFrom-Json)
if (-not $runJson.ok) { throw "run-start falló: $($runJson.error)" }
$env:AGENT_RUN_ID = $controlRunId

function Invoke-RouteCase {
    param(
        [Parameter(Mandatory=$true)][string]$Name,
        [Parameter(Mandatory=$true)][hashtable]$Metadata,
        [Parameter(Mandatory=$true)][string]$ExpectedModel,
        [Parameter(Mandatory=$true)][string]$ExpectedEffort,
        [Parameter(Mandatory=$true)][string]$ExpectedRule
    )
    $json = $Metadata | ConvertTo-Json -Compress
    $decision = (& $agentdb route (ConvertTo-HexJson $json) | Out-String | ConvertFrom-Json)
    if (-not $decision.ok) {
        Write-Host ("{0}: FAIL (route error: {1})" -f $Name, $decision.error)
        return $false
    }
    $pass = ($decision.requested_model -eq $ExpectedModel) -and
            ($decision.requested_effort -eq $ExpectedEffort) -and
            ($decision.rule -eq $ExpectedRule)
    $status = if ($pass) { "PASS" } else { "FAIL" }
    Write-Host ("{0}: {1}  -> model={2} effort={3} rule={4} reason={5}" -f $Name, $status, $decision.requested_model, $decision.requested_effort, $decision.rule, $decision.reason)
    return $pass
}

Write-Section "ROUTING CONTROL CASES"

$caseA = Invoke-RouteCase -Name "CASE A (lookup -> Luna low)" -Metadata @{
    agent_id = "developer-tooling"; task_type = "lookup"; attempt_count = 0; technical_risk = "low"
} -ExpectedModel "gpt-5.6-luna" -ExpectedEffort "low" -ExpectedRule "bounded_lookup"
if (-not $caseA) { $failed = $true }

$caseB = Invoke-RouteCase -Name "CASE B (implementation -> DeepSeek Flash)" -Metadata @{
    agent_id = "developer-tooling"; task_type = "implementation"; attempt_count = 0; technical_risk = "low"
} -ExpectedModel "deepseek/deepseek-v4-flash" -ExpectedEffort "n/a" -ExpectedRule "routine_implementation"
if (-not $caseB) { $failed = $true }

$savedDeepseek = $env:AGENT_DEEPSEEK_AVAILABLE
$env:AGENT_DEEPSEEK_AVAILABLE = "0"
$caseBf = Invoke-RouteCase -Name "CASE B-fallback (no DeepSeek -> Luna medium)" -Metadata @{
    agent_id = "developer-tooling"; task_type = "implementation"; attempt_count = 0; technical_risk = "low"
} -ExpectedModel "gpt-5.6-luna" -ExpectedEffort "medium" -ExpectedRule "routine_implementation_fallback"
if ($savedDeepseek) { $env:AGENT_DEEPSEEK_AVAILABLE = $savedDeepseek } else { Remove-Item Env:AGENT_DEEPSEEK_AVAILABLE -ErrorAction SilentlyContinue }
if (-not $caseBf) { $failed = $true }

$caseC = Invoke-RouteCase -Name "CASE C (debug attempt 1 -> Luna high)" -Metadata @{
    agent_id = "developer-tooling"; task_type = "debugging"; attempt_count = 1; technical_risk = "medium"
} -ExpectedModel "gpt-5.6-luna" -ExpectedEffort "high" -ExpectedRule "debug_attempt_1"
if (-not $caseC) { $failed = $true }

$caseD = Invoke-RouteCase -Name "CASE D (attempt 2 unresolved -> Sol medium)" -Metadata @{
    agent_id = "developer-tooling"; task_type = "debugging"; attempt_count = 2; technical_risk = "medium"; root_cause_unknown = $true
} -ExpectedModel "gpt-5.6-sol" -ExpectedEffort "medium" -ExpectedRule "attempt_budget_exhausted"
if (-not $caseD) { $failed = $true }

$caseE = Invoke-RouteCase -Name "CASE E (product-owner -> Sol medium)" -Metadata @{
    agent_id = "product-owner"; role = "product-owner"; task_type = "implementation"; attempt_count = 0
} -ExpectedModel "gpt-5.6-sol" -ExpectedEffort "medium" -ExpectedRule "role_product_owner"
if (-not $caseE) { $failed = $true }

$caseF = Invoke-RouteCase -Name "CASE F (user override Luna medium wins)" -Metadata @{
    agent_id = "developer-tooling"; task_type = "implementation"; attempt_count = 0; technical_risk = "low";
    user_model_override = "luna_medium"
} -ExpectedModel "gpt-5.6-luna" -ExpectedEffort "medium" -ExpectedRule "user_override"
if (-not $caseF) { $failed = $true }

$caseG = Invoke-RouteCase -Name "CASE G (NO escalate: lookup, 50 files)" -Metadata @{
    agent_id = "developer-tooling"; task_type = "lookup"; attempt_count = 0; technical_risk = "low"; affected_files = 50
} -ExpectedModel "gpt-5.6-luna" -ExpectedEffort "low" -ExpectedRule "bounded_lookup"
if (-not $caseG) { $failed = $true }

$caseH = Invoke-RouteCase -Name "CASE H (NO escalate: documentation search)" -Metadata @{
    agent_id = "developer-tooling"; task_type = "documentation"; attempt_count = 0; technical_risk = "low"
} -ExpectedModel "gpt-5.6-luna" -ExpectedEffort "low" -ExpectedRule "bounded_lookup"
if (-not $caseH) { $failed = $true }

$caseI = Invoke-RouteCase -Name "CASE I (NO escalate: cross-subsystem < threshold)" -Metadata @{
    agent_id = "developer-tooling"; task_type = "implementation"; attempt_count = 0; technical_risk = "low";
    cross_subsystem_change = $true; affected_subsystems = 2
} -ExpectedModel "deepseek/deepseek-v4-flash" -ExpectedEffort "n/a" -ExpectedRule "routine_implementation"
if (-not $caseI) { $failed = $true }

$caseJ = Invoke-RouteCase -Name "CASE J (NO escalate: reviewer, 40 files)" -Metadata @{
    agent_id = "reviewer"; role = "reviewer"; task_type = "review"; attempt_count = 0; technical_risk = "low"; affected_files = 40
} -ExpectedModel "gpt-5.6-luna" -ExpectedEffort "medium" -ExpectedRule "role_reviewer"
if (-not $caseJ) { $failed = $true }

(& $agentdb run-end $controlRunId success | Out-String | ConvertFrom-Json) | Out-Null

Write-Section "INSTRUMENTATION"

$runJson2 = (& $agentdb run-start developer-tooling INSTRUMENTATION-TEST --run-id $instrumentRunId --run-kind instrumentation | Out-String | ConvertFrom-Json)
if (-not $runJson2.ok) { throw "run-start falló: $($runJson2.error)" }
$env:AGENT_RUN_ID = $instrumentRunId

& $agentdb knowledge godot RigidBody3D | Out-Null
& $agentdb knowledge godot zzzunknown-term-xyz | Out-Null
& $agentdb problem node_paths | Out-Null
& $agentdb problem zzzunknown-problem-xyz | Out-Null
& $agentdb cache-get test-scope non-existent-key | Out-Null
& $agentdb cache-put test-scope instrumentation-key (ConvertTo-HexJson '{"ok":true}') | Out-Null
& $agentdb cache-get test-scope instrumentation-key | Out-Null

$metrics = (& $agentdb metrics | Out-String | ConvertFrom-Json)
$c = $metrics.counts
Write-Host ("knowledge_queries={0} knowledge_hits={1}" -f $c.knowledge_queries, $c.knowledge_hits)
Write-Host ("problem_queries={0} problem_hits={1}" -f $c.problem_queries, $c.problem_hits)
Write-Host ("cache_queries={0} cache_hits={1}" -f $c.cache_queries, $c.cache_hits)

$instrumentationOk = ($c.knowledge_queries -ge 2) -and ($c.knowledge_hits -ge 1) -and
                     ($c.problem_queries -ge 2) -and ($c.problem_hits -ge 1) -and
                     ($c.cache_queries -ge 2) -and ($c.cache_hits -ge 1)
if ($instrumentationOk) {
    Write-Host "INSTRUMENTATION: PASS"
} else {
    Write-Host "INSTRUMENTATION: FAIL"
    $failed = $true
}

(& $agentdb run-end $instrumentRunId success | Out-String | ConvertFrom-Json) | Out-Null

Write-Host ""
if ($failed) {
    Write-Error "ROUTING CONTROL: FAIL"
    exit 1
}
Write-Host "ROUTING CONTROL: PASS"
exit 0
