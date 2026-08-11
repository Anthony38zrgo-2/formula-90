. "$PSScriptRoot\_common.ps1"

$failed = $false

Write-Section "Sintaxis PowerShell"
$files = Get-ChildItem -Path $PSScriptRoot -Filter *.ps1 -File
foreach ($file in $files) {
    $tokens = $null
    $errors = $null
    [System.Management.Automation.Language.Parser]::ParseFile(
        $file.FullName,
        [ref]$tokens,
        [ref]$errors
    ) | Out-Null

    if ($errors.Count -gt 0) {
        $failed = $true
        Write-Host ("FAIL {0}" -f $file.Name)
        foreach ($error in $errors) {
            Write-Host ("  Linea {0}: {1}" -f $error.Extent.StartLineNumber, $error.Message)
        }
    }
    else {
        Write-Host ("PASS {0}" -f $file.Name)
    }
}

Write-Section "Repositorio"
Write-Host ("Repo Root      : {0}" -f $RepoRoot)
Write-Host ("Agents Root    : {0}" -f $AgentsRoot)

if (-not (Test-Path -LiteralPath $AgentsRoot)) {
    Write-Host ".agents        : FALTA"
    $failed = $true
}

$cargoToml = Join-Path $RuntimeRoot "Cargo.toml"
$runtimeState = if (Test-Path -LiteralPath $cargoToml) { "PRESENTE" } else { "FALTA" }
Write-Host ("Runtime source : {0}" -f $runtimeState)
if (-not (Test-Path -LiteralPath $cargoToml)) {
    $failed = $true
}

Write-Section "AgentDB bootstrap"
$bootstrap = Resolve-Bootstrap -NoCache

if ($bootstrap.ok) {
    Write-Host ("agentdb path   : {0}" -f $bootstrap.agentdb.path)
    Write-Host ("agentdb source : {0}" -f $bootstrap.agentdb.source)
    Write-Host ("built now      : {0}" -f $(if ($bootstrap.agentdb.built_now) { "YES" } else { "NO" }))
    Write-Host ("DB path        : {0}" -f $bootstrap.database.path)
    Write-Host ("DB available   : {0}" -f $(if ($bootstrap.database.available) { "YES" } else { "NO" }))
}
else {
    Write-Host "agentdb path   : UNAVAILABLE"
    Write-Host ("agentdb source : {0}" -f $bootstrap.agentdb.source)
    Write-Host ("DB path        : {0}" -f $bootstrap.database.path)
    Write-Host "DB available   : NO"
    $failed = $true
}

if (-not $failed) {
    Write-Section "Validacion"
    try {
        Invoke-AgentDb -Arguments @("validate")
    } catch {
        Write-Error $_
        $failed = $true
    }

    $sqlite = Get-SqliteExe
    if ($sqlite) {
        $integrity = (& $sqlite -readonly $DbPath "PRAGMA integrity_check;" 2>$null | Out-String).Trim()
        Write-Host ("SQLite integrity: {0}" -f $integrity)
        if ($integrity -ne "ok") { $failed = $true }
    }
    else {
        Write-Warning "sqlite3 no disponible; se omite PRAGMA integrity_check."
    }
}

if ($failed) {
    Write-Error "AGENT INFRASTRUCTURE UNAVAILABLE"
    Write-Error "PREFLIGHT: FAIL"
    exit 1
}

Write-Host ""
Write-Host "PREFLIGHT: PASS"
exit 0
