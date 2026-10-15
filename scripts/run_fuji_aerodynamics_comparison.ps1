[CmdletBinding()]
param(
    [ValidateSet('standard', 'simplified')][string]$Backend = 'simplified',
    [string]$OutputDirectory,
    [string]$GodotPath,
    [ValidateRange(0, 7200)][int]$MaximumSeconds = 0
)

$ErrorActionPreference = 'Stop'
$launchParameters = @{
    AerodynamicsBackend = $Backend
    AerodynamicsComparison = $true
    AerodynamicsComparisonMaximumSeconds = $MaximumSeconds
}
if ($OutputDirectory) { $launchParameters.AerodynamicsComparisonOutput = $OutputDirectory }
if ($GodotPath) { $launchParameters.GodotPath = $GodotPath }
& (Join-Path $PSScriptRoot 'run_f1_94.ps1') @launchParameters
