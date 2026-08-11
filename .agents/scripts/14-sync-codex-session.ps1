param(
    [string]$SessionFile = "",
    [string]$AgentId = "developer-godot",
    [string]$TaskId = "SKYBOX-FEATURE",
    [string]$Purpose = "feature implementation (skybox + mountains + waterfall)",
    [string]$RunId = ""
)

. "$PSScriptRoot\_common.ps1"

$env:AGENT_DB = $DbPath
$agentdb = Assert-AgentDb

if ([string]::IsNullOrWhiteSpace($SessionFile)) {
    $today = Get-Date -Format "yyyy/MM/dd"
    $sessionsDir = Join-Path $env:USERPROFILE ".codex\sessions\$today"
    $candidate = Get-ChildItem $sessionsDir -Filter "rollout-*.jsonl" -File -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $candidate) { throw "No se encontró ninguna sesión rollout para hoy en $sessionsDir" }
    $SessionFile = $candidate.FullName
}
if (-not (Test-Path $SessionFile)) { throw "No existe la sesión: $SessionFile" }

$sessionId = $null
$startMs = $null
$endMs = $null
$model = $null
$sessionStartRaw = $null
$sessionEndRaw = $null

foreach ($line in (Get-Content -LiteralPath $SessionFile)) {
    if ($line -match '"timestamp":"([^"]+)"') {
        $tsRaw = $matches[1]
        $tsMs = ([DateTimeOffset]::Parse($tsRaw)).ToUnixTimeMilliseconds()
        if (-not $startMs) { $startMs = $tsMs; $sessionStartRaw = $tsRaw }
        $endMs = $tsMs
        $sessionEndRaw = $tsRaw
    }
    if ($line -match '"type":"session_meta"' -and $line -match '"session_id":"([^"]+)"') {
        $sessionId = $matches[1]
    }
    if ($line -match '"type":"thread_settings_applied"' -and $line -match '"model":"([^"]+)"') {
        $model = $matches[1]
    }
}

if (-not $sessionId) { throw "session_id no encontrado en $SessionFile" }
if (-not $model) { throw "model no encontrado (thread_settings_applied) en $SessionFile" }
if (-not $startMs) { throw "timestamp de inicio no encontrado en $SessionFile" }

if ([string]::IsNullOrWhiteSpace($RunId)) {
    $RunId = "run_codex_desktop_{0}" -f $sessionId.Replace("-", "").Substring(0, [Math]::Min(16, $sessionId.Replace("-", "").Length))
}

$prevEAP = $ErrorActionPreference
$ErrorActionPreference = "Continue"
$startOut = (& $agentdb run-start $AgentId $TaskId --run-id $RunId --run-kind productive --ingestion-source desktop_sync 2>&1 | Out-String)
$ErrorActionPreference = $prevEAP
$exists = $null
try { $exists = $startOut | ConvertFrom-Json } catch { $exists = $null }
if (-not $exists -or -not $exists.ok) {
    $errText = if ($exists) { "$($exists.error)" } else { $startOut.Trim() }
    if ($errText -match "UNIQUE") {
        Write-Host "run_id ya registrado (idempotente): $RunId - nada que hacer."
        exit 0
    }
    throw "run-start falló: $errText"
}

$durationMs = $endMs - $startMs

$callStart = [ordered]@{
    provider = "openai_codex"
    requested_model = $model
    requested_effort = $null
    model_tier = "strong"
    purpose = $Purpose
    routing_rule = "user_override"
    routing_reason = "user manual selection in Codex Desktop"
    routing_source = "user_override"
}
$callJson = (& $agentdb model-call start $RunId (ConvertTo-HexJson ($callStart | ConvertTo-Json -Compress)) | Out-String | ConvertFrom-Json)
if (-not $callJson.ok) { throw "model-call start falló: $($callJson.error)" }
$callId = $callJson.call_id

$callEnd = [ordered]@{
    success = $true
    duration_ms = $durationMs
    effective_model = $model
    effective_effort = $null
    verification_status = "verified"
    input_tokens = $null
    cached_input_tokens = $null
    output_tokens = $null
}
$callEndJson = $callEnd | ConvertTo-Json -Compress
$callEndResult = (& $agentdb model-call end $callId (ConvertTo-HexJson $callEndJson) | Out-String | ConvertFrom-Json)
if (-not $callEndResult.ok) { throw "model-call end falló: $($callEndResult.error)" }

$runEnd = (& $agentdb run-end $RunId success | Out-String | ConvertFrom-Json)
if (-not $runEnd.ok) { throw "run-end falló: $($runEnd.error)" }

[ordered]@{
    ok = $true
    run_id = $RunId
    call_id = $callId
    session_id = $sessionId
    session_file = $SessionFile
    requested_model = $model
    effective_model = $model
    verification_status = "verified"
    routing_source = "user_override"
    duration_ms = $durationMs
    session_start = $sessionStartRaw
    session_end = $sessionEndRaw
    note = "effective model from thread_settings_applied (runtime evidence); tokens not observable in Desktop session format"
} | ConvertTo-Json -Compress
