[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$trackDefinitions = Join-Path $root 'game\scenes\tracks'
$vehicleDefinitions = Join-Path $root 'game\scenes\vehicles'

$violations = @()
$violations += Get-ChildItem -LiteralPath $trackDefinitions -File -Recurse |
    Select-String -Pattern 'res://scenes/vehicles/' |
    ForEach-Object { "TRACK->VEHICLE $($_.Path):$($_.LineNumber)" }
$violations += Get-ChildItem -LiteralPath $vehicleDefinitions -File -Recurse |
    Select-String -Pattern 'res://scenes/tracks/' |
    ForEach-Object { "VEHICLE->TRACK $($_.Path):$($_.LineNumber)" }

if ($violations.Count -gt 0) {
    $violations | ForEach-Object { Write-Error $_ }
    exit 1
}

Write-Host '[PASS] Track and vehicle scene trees have no cross references.' -ForegroundColor Green
