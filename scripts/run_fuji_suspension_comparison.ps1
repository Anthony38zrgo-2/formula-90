[CmdletBinding()]
param(
    [ValidateSet('standard', 'simplified')][string]$Backend = 'simplified',
    [ValidateSet('standard', 'simplified')][string]$AerodynamicsBackend = 'simplified',
    [string]$OutputDirectory,
    [string]$GodotPath,
    [ValidateRange(0, 7200)][int]$MaximumSeconds = 0
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path (Split-Path -Parent $PSScriptRoot) ('scratch/suspension_comparison_{0}_{1}' -f $Backend, (Get-Date -Format 'yyyyMMdd_HHmmss'))
}
$launchParameters = @{
    SuspensionBackend = $Backend
    AerodynamicsBackend = $AerodynamicsBackend
    AerodynamicsComparison = $true
    AerodynamicsComparisonOutput = $OutputDirectory
    AerodynamicsComparisonMaximumSeconds = $MaximumSeconds
}
if ($GodotPath) { $launchParameters.GodotPath = $GodotPath }
& (Join-Path $PSScriptRoot 'run_f1_94.ps1') @launchParameters
