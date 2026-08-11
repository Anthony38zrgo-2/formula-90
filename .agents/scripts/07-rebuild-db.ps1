param(
    [switch]$ConfirmRebuild
)

. "$PSScriptRoot\_common.ps1"

if (-not $ConfirmRebuild) {
    throw "Operación destructiva controlada. Repite con -ConfirmRebuild."
}

$agentdb = Assert-AgentDb
$dataDir = Join-Path $AgentsRoot "data"
New-Item -ItemType Directory -Force -Path $dataDir | Out-Null

if (Test-Path $DbPath) {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $backup = Join-Path $dataDir "agents.$stamp.bak.db"
    Copy-Item $DbPath $backup
    Write-Host "Backup: $backup"
    Remove-Item $DbPath -Force
}

Write-Section "init"
Invoke-AgentDb -Arguments @("init")

Write-Section "seed"
Invoke-AgentDb -Arguments @("seed")

Write-Section "validate"
Invoke-AgentDb -Arguments @("validate")

Write-Section "conteos"
& "$PSScriptRoot\03-db-counts.ps1"

Write-Host ""
Write-Host "DB reconstruida correctamente: $DbPath"
