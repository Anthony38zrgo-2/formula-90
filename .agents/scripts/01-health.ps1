. "$PSScriptRoot\_common.ps1"

$failed = $false

Write-Section "Entorno"
& "$PSScriptRoot\00-env.ps1"

Write-Section "Integridad SQLite"
try {
    Assert-Database
    $sqlite = Assert-Sqlite
    $integrity = (& $sqlite -readonly $DbPath "PRAGMA integrity_check;").Trim()
    Write-Host "integrity_check = $integrity"
    if ($integrity -ne "ok") { $failed = $true }

    Write-Host "journal_mode:"
    & $sqlite -readonly $DbPath "PRAGMA journal_mode;"

    Write-Host "foreign_keys:"
    & $sqlite -readonly $DbPath "PRAGMA foreign_keys;"
} catch {
    Write-Error $_
    $failed = $true
}

Write-Section "Tablas y conteos"
try {
    & "$PSScriptRoot\03-db-counts.ps1"
} catch {
    Write-Error $_
    $failed = $true
}

Write-Section "Validación agentdb"
try {
    $agentdb = Get-AgentDbExe
    if ($agentdb) {
        Invoke-AgentDb -Arguments @("validate")
    } else {
        Write-Warning "agentdb no encontrado; se omite validate."
        $failed = $true
    }
} catch {
    Write-Error $_
    $failed = $true
}

if ($failed) {
    Write-Host ""
    Write-Error "HEALTHCHECK: FAIL"
    exit 1
}

Write-Host ""
Write-Host "HEALTHCHECK: PASS"
exit 0
