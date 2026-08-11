. "$PSScriptRoot\_common.ps1"

$script:fail = $false
$agentdb = Assert-AgentDb
$sqlite = Assert-Sqlite

function Write-TestResult {
    param([string]$Name, [bool]$Pass, [string]$Detail = "")
    $status = if ($Pass) { "PASS" } else { "FAIL" }
    Write-Host ("{0}: {1}  {2}" -f $Name, $status, $Detail)
    if (-not $Pass) { $script:fail = $true }
}

function Invoke-AgentDbJson {
    param([string[]]$Arguments)
    $prevEAP = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $out = (& $agentdb @Arguments 2>&1 | Out-String)
        return ($out | ConvertFrom-Json)
    }
    finally {
        $ErrorActionPreference = $prevEAP
    }
}

function Get-RunField {
    param([string]$RunId, [string]$Column)
    $sql = "SELECT $Column FROM agent_runs WHERE run_id='$RunId';"
    return ((& $sqlite -readonly $DbPath $sql | Out-String).Trim().Split("`n") | Select-Object -First 1)
}

function New-TestRun {
    param([string]$Kind, [string]$PhaseFlag = "")
    $runId = "run_exp_{0}_{1}" -f $Kind, [guid]::NewGuid().ToString("N").Substring(0, 8)
    $args = @("run-start", "developer-tooling", "EXP-$Kind", "--run-id", $runId, "--run-kind", $Kind)
    if ($PhaseFlag) { $args += @("--phase", $PhaseFlag) }
    $r = Invoke-AgentDbJson -Arguments $args
    if (-not $r.ok) { throw "run-start falló: $($r.error)" }
    return $runId
}

Write-Section "TEST A - run_kind probe"
$runA = New-TestRun -Kind "probe"
$kindA = Get-RunField $runA "run_kind"
Write-TestResult "TEST A" ($kindA -eq "probe") "run_kind=$kindA"

Write-Section "TEST B - run_kind control"
$runB = New-TestRun -Kind "control"
$kindB = Get-RunField $runB "run_kind"
Write-TestResult "TEST B" ($kindB -eq "control") "run_kind=$kindB"

Write-Section "TEST C - run_kind instrumentation"
$runC = New-TestRun -Kind "instrumentation"
$kindC = Get-RunField $runC "run_kind"
Write-TestResult "TEST C" ($kindC -eq "instrumentation") "run_kind=$kindC"

Write-Section "TEST D - run_kind bootstrap"
$runD = New-TestRun -Kind "bootstrap"
$kindD = Get-RunField $runD "run_kind"
Write-TestResult "TEST D" ($kindD -eq "bootstrap") "run_kind=$kindD"

Write-Section "TEST E - run_kind productive (sintetico, sin gameplay)"
$runE = New-TestRun -Kind "productive"
$kindE = Get-RunField $runE "run_kind"
Write-TestResult "TEST E" ($kindE -eq "productive") "run_kind=$kindE"

Write-Section "TEST F - experiment_phase por defecto desde config"
$runF = New-TestRun -Kind "control"
$phaseF = Get-RunField $runF "experiment_phase"
Write-TestResult "TEST F" ($phaseF -eq "reactive-router") "phase=$phaseF (config current_phase=reactive-router)"

Write-Section "TEST G - override explicito contract-first (run de test)"
$runG = New-TestRun -Kind "probe" -PhaseFlag "contract-first"
$phaseG = Get-RunField $runG "experiment_phase"
Write-TestResult "TEST G" ($phaseG -eq "contract-first") "phase=$phaseG (config sigue en reactive-router)"

Write-Section "TEST H - capacity_source normal"
$runH = New-TestRun -Kind "control"
$metaH = @{ provider = "openai_codex"; requested_model = "gpt-5.6-luna"; requested_effort = "low"; model_tier = "fast"; purpose = "test"; routing_rule = "test_execution"; routing_source = "auto" } | ConvertTo-Json -Compress
$callH = Invoke-AgentDbJson -Arguments @("model-call", "start", $runH, (ConvertTo-HexJson $metaH))
$csH = Get-RunField $runH "1" | Out-Null
$sqlH = "SELECT capacity_source FROM model_calls WHERE call_id='$($callH.call_id)';"
$csH = ((& $sqlite -readonly $DbPath $sqlH | Out-String).Trim())
Write-TestResult "TEST H" ($csH -eq "normal") "capacity_source=$csH"

Write-Section "TEST I - capacity_source automatic_escalation"
$runI = New-TestRun -Kind "probe"
$metaI = @{ provider = "openai_codex"; requested_model = "gpt-5.6-sol"; model_tier = "strong"; purpose = "debugging"; routing_rule = "attempt_budget_exhausted"; routing_source = "auto" } | ConvertTo-Json -Compress
$callI = Invoke-AgentDbJson -Arguments @("model-call", "start", $runI, (ConvertTo-HexJson $metaI))
$sqlI = "SELECT capacity_source FROM model_calls WHERE call_id='$($callI.call_id)';"
$csI = ((& $sqlite -readonly $DbPath $sqlI | Out-String).Trim())
Write-TestResult "TEST I" ($csI -eq "automatic_escalation") "capacity_source=$csI"

Write-Section "TEST J - capacity_source manual_override"
$runJ = New-TestRun -Kind "sync"
$metaJ = @{ provider = "openai_codex"; requested_model = "gpt-5.6-terra"; model_tier = "strong"; purpose = "feature"; routing_rule = "user_override"; routing_source = "user_override" } | ConvertTo-Json -Compress
$callJ = Invoke-AgentDbJson -Arguments @("model-call", "start", $runJ, (ConvertTo-HexJson $metaJ))
$sqlJ = "SELECT capacity_source FROM model_calls WHERE call_id='$($callJ.call_id)';"
$csJ = ((& $sqlite -readonly $DbPath $sqlJ | Out-String).Trim())
Write-TestResult "TEST J" ($csJ -eq "manual_override") "capacity_source=$csJ"

Write-Section "TEST K - capacity_source planned_capacity (via metadata)"
$runK = New-TestRun -Kind "probe"
$metaK = @{ provider = "openai_codex"; requested_model = "gpt-5.6-sol"; model_tier = "strong"; purpose = "planning"; routing_rule = "role_product_owner"; routing_source = "auto"; capacity_source = "planned_capacity" } | ConvertTo-Json -Compress
$callK = Invoke-AgentDbJson -Arguments @("model-call", "start", $runK, (ConvertTo-HexJson $metaK))
$sqlK = "SELECT capacity_source FROM model_calls WHERE call_id='$($callK.call_id)';"
$csK = ((& $sqlite -readonly $DbPath $sqlK | Out-String).Trim())
Write-TestResult "TEST K" ($csK -eq "planned_capacity") "capacity_source=$csK"

Write-Section "TEST L - context-packet planning (con telemetria)"
$runL = New-TestRun -Kind "productive"
$prevRunId = $env:AGENT_RUN_ID
$env:AGENT_RUN_ID = $runL
try {
    $packet = Invoke-AgentDbJson -Arguments @("context-packet", "PHY-003", "planning")
    $okPacket = $packet.ok -and $packet.phase -eq "planning" -and $packet.knowledge.Count -gt 0
    $srcSql = "SELECT COUNT(*) FROM telemetry_events WHERE run_id='$runL' AND source='planning_prefetch';"
    $srcCount = ((& $sqlite -readonly $DbPath $srcSql | Out-String).Trim())
    Write-TestResult "TEST L" ($okPacket -and [int]$srcCount -ge 1) "knowledge=$($packet.knowledge.Count) planning_prefetch_events=$srcCount"
}
finally {
    if ($null -eq $prevRunId) { Remove-Item Env:AGENT_RUN_ID -ErrorAction SilentlyContinue }
    else { $env:AGENT_RUN_ID = $prevRunId }
}

Write-Section "TEST M - context-packet execution"
$runM = New-TestRun -Kind "productive"
$prevRunId = $env:AGENT_RUN_ID
$env:AGENT_RUN_ID = $runM
try {
    $packet = Invoke-AgentDbJson -Arguments @("context-packet", "PHY-003", "execution")
    $bounded = $packet.ok -and $packet.knowledge.Count -le 8 -and $packet.common_problems.Count -le 5
    $srcSql = "SELECT COUNT(*) FROM telemetry_events WHERE run_id='$runM' AND source='execution_prefetch';"
    $srcCount = ((& $sqlite -readonly $DbPath $srcSql | Out-String).Trim())
    Write-TestResult "TEST M" ($bounded -and [int]$srcCount -ge 1) "knowledge=$($packet.knowledge.Count) problems=$($packet.common_problems.Count) execution_prefetch_events=$srcCount"
}
finally {
    if ($null -eq $prevRunId) { Remove-Item Env:AGENT_RUN_ID -ErrorAction SilentlyContinue }
    else { $env:AGENT_RUN_ID = $prevRunId }
}

Write-Section "TEST N - backlog sin common problem relevante (miss, no fallo)"
$packetN = Invoke-AgentDbJson -Arguments @("context-packet", "PHY-001", "planning")
$miss = $packetN.ok -and $packetN.common_problems.Count -eq 0
Write-TestResult "TEST N" $miss "problems=$($packetN.common_problems.Count) ok=$($packetN.ok)"

Write-Section "TEST O - context-packet sin AGENT_RUN_ID"
$prevRunId = $env:AGENT_RUN_ID
Remove-Item Env:AGENT_RUN_ID -ErrorAction SilentlyContinue
try {
    $packetO = Invoke-AgentDbJson -Arguments @("context-packet", "PHY-003", "planning")
    Write-TestResult "TEST O" $packetO.ok "ok=$($packetO.ok) knowledge=$($packetO.knowledge.Count)"
}
finally {
    if ($null -ne $prevRunId) { $env:AGENT_RUN_ID = $prevRunId }
}

Write-Section "METRICS FILTERS"
$mAll = Invoke-AgentDbJson -Arguments @("metrics", "all")
$mProd = Invoke-AgentDbJson -Arguments @("metrics", "productive")
$mPhase = Invoke-AgentDbJson -Arguments @("metrics", "phase", "reactive-router")
$mBoth = Invoke-AgentDbJson -Arguments @("metrics", "productive", "phase", "reactive-router")
$filtersOk = $mAll.ok -and $mProd.ok -and $mPhase.ok -and $mBoth.ok -and
              $mProd.scope -eq "productive" -and $mProd.counts.runs -le $mAll.counts.runs
Write-TestResult "METRICS FILTERS" $filtersOk "all=$($mAll.counts.runs) productive=$($mProd.counts.runs) phase=$($mPhase.counts.runs) both=$($mBoth.counts.runs)"

foreach ($runId in @($runA, $runB, $runC, $runD, $runE, $runF, $runG, $runH, $runI, $runJ, $runK, $runL, $runM)) {
    & $agentdb run-end $runId success 2>$null | Out-Null
}

Write-Section "RESULTADO"
if ($script:fail) {
    Write-Error "EXPERIMENT SEMANTICS TESTS: FAIL"
    exit 1
}
Write-Host "EXPERIMENT SEMANTICS TESTS: PASS"
exit 0
