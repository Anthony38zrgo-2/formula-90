<#
.SYNOPSIS
    Open Godot F1-94 La Chutana project.

.DESCRIPTION
    Launches Godot Engine v4.7.1 with the Formula-90s project.
    Opens the main runtime scene for visual inspection of tire barriers,
    chicana modules, and track environment.

    This is a read-only inspection script - does not validate BUILD/HEAD
    parity or run smoke tests. For runtime validation, use run_f1_94.ps1 -ValidateRuntimeOnly.

.PARAMETER GodotPath
    Explicit path to Godot executable. If omitted, auto-resolves.

.EXAMPLE
    .\scripts\open_godot_la_chutana.ps1
    .\scripts\open_godot_la_chutana.ps1 -GodotPath "C:\Tools\Godot\Godot.exe"
#>
[CmdletBinding()]
param(
    [string]$GodotPath
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

# Resolve Godot executable
$godotExe = $GodotPath
if ([string]::IsNullOrWhiteSpace($godotExe)) {
    $candidates = @(
        (Join-Path $root '.tools\godot\Godot_v4.7.1-stable_win64_console.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs\Godot\Godot.exe'),
        (Join-Path $env:ProgramFiles 'Godot\Godot.exe')
    )
    foreach ($cand in $candidates) {
        if (Test-Path -LiteralPath $cand -PathType Leaf) {
            $godotExe = $cand
            break
        }
    }
    if ([string]::IsNullOrWhiteSpace($godotExe)) {
        throw "Godot 4.7.1 executable not found. Pass -GodotPath or run bootstrap_windows.ps1."
    }
}

$projectPath = Join-Path $root 'game'

Write-Host "Launching Godot F1-94 La Chutana project..."
Write-Host "  Godot: $godotExe"
Write-Host "  Project: $projectPath"

& $godotExe --path $projectPath
Write-Host "Godot closed."