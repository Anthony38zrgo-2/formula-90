[CmdletBinding()]
param(
    [string]$GodotPath,
    [switch]$Plan
)

$ErrorActionPreference = 'Stop'
$fullSuite = Join-Path $PSScriptRoot 'scripts\test_windows.ps1'
$canon = Join-Path $PSScriptRoot 'run_f1_94.ps1'

if ($Plan) {
    Write-Host 'FULL plan:' -ForegroundColor Cyan
    Write-Host '  1. Build C++ and Rust runtime binaries'
    Write-Host '  2. Run native, asset, Rust, Godot and GdUnit tests'
    Write-Host '  3. Validate canonical runtime BUILD/HEAD parity'
    exit 0
}

$fullArgs = @{}
if ($GodotPath) { $fullArgs.GodotPath = $GodotPath }
& $fullSuite @fullArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$canonArgs = @{ ValidateRuntimeOnly = $true }
if ($GodotPath) { $canonArgs.GodotPath = $GodotPath }
& $canon @canonArgs
exit $LASTEXITCODE
