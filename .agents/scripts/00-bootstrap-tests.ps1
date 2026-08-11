. "$PSScriptRoot\_common.ps1"

$script:fail = $false
$expectedRoot = [System.IO.Path]::GetFullPath($RepoRoot)
$expectedAgents = [System.IO.Path]::GetFullPath((Join-Path $expectedRoot ".agents"))
$expectedDb = [System.IO.Path]::GetFullPath((Join-Path $expectedAgents "data\agents.db"))
$preflight = Join-Path $PSScriptRoot "00-preflight.ps1"

function Write-TestResult {
    param([string]$Name, [bool]$Pass, [string]$Detail = "")
    $status = if ($Pass) { "PASS" } else { "FAIL" }
    Write-Host ("{0}: {1}  {2}" -f $Name, $status, $Detail)
    if (-not $Pass) { $script:fail = $true }
}

function Invoke-Preflight {
    param([string]$FromDir)
    $prevEAP = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    Push-Location $FromDir
    try {
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $preflight 2>&1 | Out-Null
        return ($LASTEXITCODE -eq 0)
    }
    finally {
        Pop-Location
        $ErrorActionPreference = $prevEAP
    }
}

function Invoke-RunStart {
    param([string]$RunId, [string]$Suffix, [string]$RunKind = "bootstrap")
    $agentdb = Assert-AgentDb
    $prevEAP = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        & $agentdb run-start developer-tooling "BOOTSTRAP-$Suffix" --run-id $RunId --run-kind $RunKind 2>$null | Out-Null
    }
    finally {
        $ErrorActionPreference = $prevEAP
    }
}

function Get-TelemetryCount {
    param([string]$RunId, [string]$EventType)
    $sqlite = Assert-Sqlite
    $sql = "SELECT COUNT(*) FROM telemetry_events WHERE run_id='$RunId' AND event_type='$EventType';"
    $out = (& $sqlite -readonly $DbPath $sql | Out-String).Trim()
    return [int]($out.Split("`n") | Select-Object -First 1)
}

Write-Section "TEST A - repository root"
$ok = Invoke-Preflight -FromDir $expectedRoot
Write-TestResult "TEST A" $ok "preflight desde repo root"

Write-Section "TEST B - nested game folder"
$gameDir = Join-Path $expectedRoot "game"
if (-not (Test-Path -LiteralPath $gameDir)) {
    Write-TestResult "TEST B" $false "no existe <repo>\game"
}
else {
    Push-Location $gameDir
    try {
        . "$PSScriptRoot\_common.ps1"
        $sameRoot = [System.IO.Path]::GetFullPath($RepoRoot) -eq $expectedRoot
        $sameDb = [System.IO.Path]::GetFullPath($DbPath) -eq $expectedDb
    }
    finally {
        Pop-Location
    }
    $ok = $sameRoot -and $sameDb -and (Invoke-Preflight -FromDir $gameDir)
    Write-TestResult "TEST B" $ok "same RepoRoot=$sameRoot same AgentDbPath=$sameDb"
}

Write-Section "TEST C - .agents/scripts"
$ok = Invoke-Preflight -FromDir $PSScriptRoot
Write-TestResult "TEST C" $ok "preflight desde .agents/scripts"

Write-Section "TEST D - PATH independence"
$savedPath = $env:PATH
$env:PATH = "$env:WINDIR\System32"
try {
    $r = Resolve-AgentDb -NoCache
    $ok = $null -ne $r -and ($r.source -eq "project_release" -or $r.source -eq "built_release")
    $detail = if ($r) { "source=$($r.source)" } else { "sin resolucion" }
}
finally {
    $env:PATH = $savedPath
}
Write-TestResult "TEST D" $ok $detail

Write-Section "TEST E - explicit env AGENTDB_EXE"
$release = Join-Path $RuntimeRoot "target\release\agentdb.exe"
if (-not (Test-Path -LiteralPath $release)) {
    Write-TestResult "TEST E" $false "falta release binary; ejecuta cargo build --release"
}
else {
    $savedExe = [System.Environment]::GetEnvironmentVariable("AGENTDB_EXE")
    $env:AGENTDB_EXE = $release
    try {
        $r = Resolve-AgentDb -NoCache
        $ok = $null -ne $r -and $r.source -eq "explicit_env" -and
              ([System.IO.Path]::GetFullPath($r.path) -eq [System.IO.Path]::GetFullPath($release))
        $detail = if ($r) { "source=$($r.source)" } else { "sin resolucion" }
    }
    finally {
        if ($null -eq $savedExe) { Remove-Item Env:AGENTDB_EXE -ErrorAction SilentlyContinue }
        else { $env:AGENTDB_EXE = $savedExe }
    }
    Write-TestResult "TEST E" $ok $detail
}

Write-Section "TEST F - invalid env override"
$savedExe = [System.Environment]::GetEnvironmentVariable("AGENTDB_EXE")
$env:AGENTDB_EXE = Join-Path $env:TEMP "agentdb-nonexistent.exe"
try {
    $r = Resolve-AgentDb -NoCache
    $ok = $null -ne $r -and ($r.source -eq "project_release" -or $r.source -eq "built_release" -or $r.source -eq "system_path")
    $detail = if ($r) { "continua con source=$($r.source)" } else { "sin resolucion" }
}
finally {
    if ($null -eq $savedExe) { Remove-Item Env:AGENTDB_EXE -ErrorAction SilentlyContinue }
    else { $env:AGENTDB_EXE = $savedExe }
}
Write-TestResult "TEST F" $ok $detail

Write-Section "TEST G - absolute DB path (CWD change)"
$tmp = Join-Path $env:TEMP ("agentdb-cwd-test-{0}" -f [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmp -Force | Out-Null
$statsOk = $false
$sameDb = $false
$absolute = $false
$envOk = $false
try {
    Push-Location $tmp
    try {
        $agentdb = Assert-AgentDb
        & $agentdb stats 2>$null | Out-Null
        $statsOk = ($LASTEXITCODE -eq 0)
        $sameDb = ([System.IO.Path]::GetFullPath($DbPath) -eq $expectedDb)
        $absolute = [System.IO.Path]::IsPathRooted($DbPath)
        $envOk = ($env:AGENT_DB -eq $expectedDb) -and ($env:AGENTS_ROOT -eq $expectedAgents)
    }
    finally {
        Pop-Location
    }
}
finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
$ok = $statsOk -and $sameDb -and $absolute -and $envOk
Write-TestResult "TEST G" $ok "statsOk=$statsOk sameDb=$sameDb absolute=$absolute env=$envOk"

Write-Section "TEST H - knowledge query (hit + telemetry)"
$runIdH = "run_boottest_h_{0}" -f [guid]::NewGuid().ToString("N").Substring(0, 8)
Invoke-RunStart -RunId $runIdH -Suffix "TEST-H"
$prevRunId = $env:AGENT_RUN_ID
$env:AGENT_RUN_ID = $runIdH
$hit = $false
$telemetryOk = $false
try {
    $agentdb = Assert-AgentDb
    $out = (& $agentdb knowledge godot RigidBody3D 2>$null | Out-String)
    $json = $out | ConvertFrom-Json
    $hit = $json.ok -and $json.results.Count -gt 0
    $telemetryOk = (Get-TelemetryCount $runIdH "knowledge_hit") -ge 1
}
finally {
    & $agentdb run-end $runIdH success 2>$null | Out-Null
    if ($null -eq $prevRunId) { Remove-Item Env:AGENT_RUN_ID -ErrorAction SilentlyContinue }
    else { $env:AGENT_RUN_ID = $prevRunId }
}
Write-TestResult "TEST H" ($hit -and $telemetryOk) "hit=$hit telemetry_knowledge_hit=$telemetryOk"

Write-Section "TEST I - missing knowledge term (miss, NOT unavailable)"
$runIdI = "run_boottest_i_{0}" -f [guid]::NewGuid().ToString("N").Substring(0, 8)
Invoke-RunStart -RunId $runIdI -Suffix "TEST-I"
$prevRunId = $env:AGENT_RUN_ID
$env:AGENT_RUN_ID = $runIdI
$miss = $false
$missTelemetry = $false
$unavailableTelemetry = $false
try {
    $agentdb = Assert-AgentDb
    $out = (& $agentdb knowledge godot zzz_bootstrap_missing_term_xyz 2>$null | Out-String)
    $json = $out | ConvertFrom-Json
    $miss = $json.ok -and $json.results.Count -eq 0
    $missTelemetry = (Get-TelemetryCount $runIdI "knowledge_miss") -ge 1
    $unavailableTelemetry = (Get-TelemetryCount $runIdI "knowledge_unavailable") -ge 1
}
finally {
    & $agentdb run-end $runIdI success 2>$null | Out-Null
    if ($null -eq $prevRunId) { Remove-Item Env:AGENT_RUN_ID -ErrorAction SilentlyContinue }
    else { $env:AGENT_RUN_ID = $prevRunId }
}
$ok = $miss -and $missTelemetry -and (-not $unavailableTelemetry)
Write-TestResult "TEST I" $ok "miss=$miss knowledge_miss_telemetry=$missTelemetry knowledge_unavailable=$unavailableTelemetry"

Write-Section "TEST J - resolver diagnostics"
$savedExe = [System.Environment]::GetEnvironmentVariable("AGENTDB_EXE")
$d = Get-BootstrapDiagnostics -NoCache
$env:AGENTDB_EXE = Join-Path $env:TEMP "agentdb-does-not-exist.exe"
try {
    $d2 = Get-BootstrapDiagnostics -NoCache
}
finally {
    if ($null -eq $savedExe) { Remove-Item Env:AGENTDB_EXE -ErrorAction SilentlyContinue }
    else { $env:AGENTDB_EXE = $savedExe }
}
$ok = $d.runtime_source_present -and $d.cargo_available -and $d.build_available -and
      $d.binary_available -and $d.database_available -and $d2.binary_available
$detail = "binary_source=$($d.binary_source) build_available=$($d.build_available) db_available=$($d.database_available) invalid_env_ok=$($d2.binary_available)"
Write-TestResult "TEST J" $ok $detail

Write-Section "RESULTADO"
if ($script:fail) {
    Write-Error "BOOTSTRAP TESTS: FAIL"
    exit 1
}
Write-Host "BOOTSTRAP TESTS: PASS"
exit 0
