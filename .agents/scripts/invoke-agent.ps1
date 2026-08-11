param(
    [Parameter(Mandatory=$true)][string]$Prompt,
    [string]$AgentId = "developer-tooling",
    [string]$TaskId = "",
    [string]$BacklogId = "",
    [string]$TaskType = "implementation",
    [int]$AttemptCount = 0,
    [switch]$HypothesisChanged,
    [switch]$NewEvidence,
    [switch]$FailureSignatureChanged,
    [switch]$SameFailureSignature,
    [switch]$DiagnosticMode,
    [int]$AffectedFiles = 0,
    [int]$AffectedSubsystems = 1,
    [switch]$ArchitectureChange,
    [switch]$CrossSubsystemChange,
    [string]$TechnicalRisk = "low",
    [switch]$KnowledgeHit,
    [switch]$ProblemHit,
    [switch]$RootCauseUnknown,
    [switch]$LargeRegression,
    [string]$UserModelOverride = "",
    [string]$Purpose = "",
    [string]$RunId = "",
    [string]$RunKind = "productive"
)

. "$PSScriptRoot\_common.ps1"

$ErrorActionPreference = "Stop"

# AGENTS_ROOT / AGENT_DB absolutos ya configurados por _common.ps1 (dot-source).

# Bootstrap obligatorio: sin infraestructura .agents verificada no se inicia
# trabajo de ingenieria (fail-closed para tareas significativas).
$bootstrap = Resolve-Bootstrap
if (-not $bootstrap.ok) {
    $agentdbState = if ($bootstrap.agentdb.available) { "OK" } else { "UNAVAILABLE" }
    $dbState = if ($bootstrap.database.available) { "OK" } else { "UNAVAILABLE" }
    throw ("AGENT INFRASTRUCTURE UNAVAILABLE: agentdb={0}, database={1}. " +
        "Bloqueador de infraestructura: no se inicia la ejecucion sin acceso a la " +
        "base de conocimiento del proyecto (.agents)." -f $agentdbState, $dbState)
}
$agentdb = $bootstrap.agentdb.path
$builtNow = $bootstrap.agentdb.built_now

& $agentdb validate 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "agentdb validate fallo; infraestructura .agents no verificada."
}

if ([string]::IsNullOrWhiteSpace($Prompt)) {
    throw "Prompt vacio: se requiere una instruccion para el agente."
}

$runId = $RunId
if ([string]::IsNullOrWhiteSpace($runId)) { $runId = $env:AGENT_RUN_ID }
if ([string]::IsNullOrWhiteSpace($runId)) {
    $runArgs = @("run-start", $AgentId)
    if ($TaskId) { $runArgs += $TaskId }
    if ($BacklogId) { $runArgs += $BacklogId }
    $runArgs += @("--run-kind", $RunKind)
    $prevEAP = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $runOut = (& $agentdb @runArgs 2>&1 | Out-String)
    $ErrorActionPreference = $prevEAP
    $runJson = $null
    try { $runJson = $runOut | ConvertFrom-Json } catch { $runJson = $null }
    if (-not $runJson -or -not $runJson.ok) {
        $errText = if ($runJson) { "$($runJson.error)" } else { $runOut.Trim() }
        throw "run-start fallo: $errText"
    }
    $runId = $runJson.run_id
}
$env:AGENT_RUN_ID = $runId

# Telemetria retroactiva: agentdb fue reconstruido localmente durante bootstrap.
if ($builtNow) {
    & $agentdb event $runId infrastructure_bootstrap built_release 0 "agentdb was absent and successfully rebuilt locally" --source agentdb 2>$null | Out-Null
}

Set-DeepSeekAvailableEnv

$callStarted = $false
$callEnded = $false
$runEnded = $false
$success = $false
$outcome = "failure"
$executionError = $null
$callId = $null
$provider = ""
$requestedModel = ""
$requestedEffort = ""
$profile = $null
$rule = ""
$reason = ""
$effectiveModel = $null
$effectiveEffort = $null
$verificationStatus = "unverified"
$durationMs = 0
$inputTokens = $null
$cachedTokens = $null
$outputTokens = $null

try {
    $metadata = [ordered]@{
        agent_id = $AgentId
        task_type = $TaskType
        attempt_count = $AttemptCount
        hypothesis_changed = [bool]$HypothesisChanged
        new_evidence = [bool]$NewEvidence
        failure_signature_changed = [bool]$FailureSignatureChanged
        same_failure_signature = [bool]$SameFailureSignature
        diagnostic_mode = [bool]$DiagnosticMode
        affected_files = $AffectedFiles
        affected_subsystems = $AffectedSubsystems
        architecture_change = [bool]$ArchitectureChange
        cross_subsystem_change = [bool]$CrossSubsystemChange
        technical_risk = $TechnicalRisk
        knowledge_hit = [bool]$KnowledgeHit
        problem_hit = [bool]$ProblemHit
        root_cause_unknown = [bool]$RootCauseUnknown
        large_regression = [bool]$LargeRegression
        user_model_override = $(if ($UserModelOverride) { $UserModelOverride } else { $null })
    }
    $metadataJson = $metadata | ConvertTo-Json -Compress

    $routeJson = (& $agentdb route (ConvertTo-HexJson $metadataJson) | Out-String | ConvertFrom-Json)
    if (-not $routeJson.ok) { throw "route fallo: $($routeJson.error)" }

    $provider = $routeJson.provider
    $requestedModel = $routeJson.requested_model
    $requestedEffort = $routeJson.requested_effort
    $profile = $routeJson.profile
    $rule = $routeJson.rule
    $reason = $routeJson.reason

    $callStart = [ordered]@{
        provider = $provider
        requested_model = $requestedModel
        requested_effort = $requestedEffort
        model_tier = $routeJson.model_tier
        purpose = $(if ($Purpose) { $Purpose } else { $TaskType })
        routing_rule = $rule
        routing_reason = $reason
        routing_source = $(if ($UserModelOverride) { "user_override" } else { "auto" })
    }
    $callStartJson = $callStart | ConvertTo-Json -Compress
    $callJson = (& $agentdb model-call start $runId (ConvertTo-HexJson $callStartJson) | Out-String | ConvertFrom-Json)
    if (-not $callJson.ok) { throw "model-call start fallo: $($callJson.error)" }
    $callStarted = $true
    $callId = $callJson.call_id

    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    Push-Location $RepoRoot
    try {
        $prevEAP = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        switch ($provider) {
            "deepseek" {
                $opencode = (Get-Command opencode -ErrorAction SilentlyContinue)
                if (-not $opencode) { throw "opencode no esta disponible; adaptador deepseek unsupported" }
                $ocArgs = @("run", "-m", $requestedModel, "--format", "json", "--dir", $RepoRoot)
                if ($TaskId) { $ocArgs += @("--title", $TaskId) }
                $ocArgs += $Prompt
                $raw = (& opencode @ocArgs 2>$null | Out-String)
                $exitCode = $LASTEXITCODE
                if ($exitCode -ne 0) { throw "opencode termino con codigo $exitCode" }

                $sessionId = $null
                foreach ($line in ($raw -split "`r?`n")) {
                    if ($line -match '"type":"step_finish"') {
                        $evt = $line | ConvertFrom-Json
                        if ($evt.part.tokens) {
                            $inputTokens = $evt.part.tokens.input
                            $cachedTokens = $evt.part.tokens.cache.read
                            $outputTokens = $evt.part.tokens.output
                        }
                    }
                    if ($line -match '"type":"step_start"' -and -not $sessionId) {
                        $evt = $line | ConvertFrom-Json
                        $sessionId = $evt.sessionID
                    }
                }

                if ($sessionId) {
                    $exported = (& opencode export $sessionId 2>$null | ConvertFrom-Json)
                    if ($exported.info -and $exported.info.model -and $exported.info.model.id) {
                        $effectiveModel = "$($exported.info.model.providerID)/$($exported.info.model.id)"
                        $verificationStatus = "verified"
                        if ($exported.info.model.variant -and $exported.info.model.variant -ne "default") {
                            $effectiveEffort = $exported.info.model.variant
                        }
                    }
                }
                $success = $true
            }
            "openai_codex" {
                $codex = Get-CodexExe
                if (-not $codex) { throw "codex CLI no encontrado; adaptador openai_codex unsupported" }
                if (-not $profile) { throw "routing sin perfil codex; adaptador openai_codex unsupported" }
                $lastMsgFile = Join-Path $env:TEMP ("opencode-codex-lastmsg-{0}.txt" -f ([guid]::NewGuid().ToString("N")))
                $raw = ($Prompt | & $codex exec -p $profile -s read-only --json -o $lastMsgFile 2>$null | Out-String)
                $exitCode = $LASTEXITCODE
                if ($exitCode -ne 0) { throw "codex exec termino con codigo $exitCode" }

                foreach ($line in ($raw -split "`r?`n")) {
                    if ($line -match '"type":"turn.completed"') {
                        $evt = $line | ConvertFrom-Json
                        if ($evt.usage) {
                            $inputTokens = $evt.usage.input_tokens
                            $cachedTokens = $evt.usage.cached_input_tokens
                            $outputTokens = $evt.usage.output_tokens
                        }
                    }
                }

                $effectiveModel = $null
                $effectiveEffort = $null
                $verificationStatus = "unverified"
                $success = $true
            }
            default {
                throw "provider '$provider' sin adaptador implementado; unsupported"
            }
        }
        $ErrorActionPreference = $prevEAP
    } finally {
        Pop-Location
        $stopwatch.Stop()
    }

    $durationMs = [long]$stopwatch.Elapsed.TotalMilliseconds
    $outcome = if ($success) { "success" } else { "failure" }

    $callEnd = [ordered]@{
        success = $success
        duration_ms = $durationMs
        effective_model = $effectiveModel
        effective_effort = $effectiveEffort
        verification_status = $verificationStatus
        input_tokens = $inputTokens
        cached_input_tokens = $cachedTokens
        output_tokens = $outputTokens
    }
    $callEndJson = $callEnd | ConvertTo-Json -Compress
    $callEndResult = (& $agentdb model-call end $callId (ConvertTo-HexJson $callEndJson) | Out-String | ConvertFrom-Json)
    if (-not $callEndResult.ok) { throw "model-call end fallo: $($callEndResult.error)" }
    $callEnded = $true
} catch {
    $executionError = $_.Exception.Message
} finally {
    if ($callStarted -and -not $callEnded) {
        $failedCallEnd = [ordered]@{
            success = $false
            duration_ms = $durationMs
            effective_model = $effectiveModel
            effective_effort = $effectiveEffort
            verification_status = $verificationStatus
            input_tokens = $inputTokens
            cached_input_tokens = $cachedTokens
            output_tokens = $outputTokens
        }
        $failedCallJson = $failedCallEnd | ConvertTo-Json -Compress
        & $agentdb model-call end $callId (ConvertTo-HexJson $failedCallJson) 2>$null | Out-Null
    }
    if (-not $runEnded) {
        $runEnd = (& $agentdb run-end $runId $outcome | Out-String | ConvertFrom-Json)
        if ($runEnd.ok) { $runEnded = $true }
    }
}

$result = [ordered]@{
    ok = $success
    run_id = $runId
    call_id = $callId
    provider = $provider
    requested_model = $requestedModel
    requested_effort = $requestedEffort
    profile = $profile
    rule = $rule
    reason = $reason
    effective_model = $effectiveModel
    effective_effort = $effectiveEffort
    verification_status = $verificationStatus
    duration_ms = $durationMs
    input_tokens = $inputTokens
    cached_input_tokens = $cachedTokens
    output_tokens = $outputTokens
    error = $executionError
}
$result | ConvertTo-Json -Compress

if (-not $success -and $executionError) {
    Write-Error "Ejecucion del proveedor fallo: $executionError"
    exit 1
}
exit 0
