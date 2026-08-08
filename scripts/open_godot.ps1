<#
.SYNOPSIS
    Open the Formula90s project in the Godot 4.7 editor.

.DESCRIPTION
    Resolves the Godot 4.7 executable and opens the project at game/.
    Optionally opens a specific scene file directly.

.PARAMETER GodotPath
    Explicit path to a Godot executable. If omitted, auto-resolves via
    .tools/godot/, GODOT_BIN env var, or system install.

.PARAMETER Scene
    Scene path relative to game/ to open on launch (e.g. scenes/vehicles/player_car.tscn).

.EXAMPLE
    .\scripts\open_godot.ps1
    .\scripts\open_godot.ps1 -Scene scenes/vehicles/player_car.tscn
    .\scripts\open_godot.ps1 -Scene scenes/tracks/test_field/test_field.tscn
#>
[CmdletBinding()]
param(
    [string]$GodotPath,
    [string]$Scene
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

function Resolve-Godot([string]$explicit) {
    if ($explicit) {
        if (Test-Path $explicit) { return (Resolve-Path $explicit).Path }
        throw "Godot not found at: $explicit"
    }
    if ($env:GODOT_BIN -and (Test-Path $env:GODOT_BIN)) {
        return (Resolve-Path $env:GODOT_BIN).Path
    }
    $candidates = @(
        (Join-Path $root '.tools\godot\Godot_v4.7.1-stable_win64.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs\Godot\Godot.exe'),
        (Join-Path $env:ProgramFiles 'Godot\Godot.exe')
    )
    foreach ($item in $candidates) {
        if (Test-Path $item) { return (Resolve-Path $item).Path }
    }
    throw "Godot 4.7.1 not found. Pass -GodotPath, set GODOT_BIN, or run bootstrap_windows.ps1."
}

$godot = Resolve-Godot $GodotPath
$projectPath = Join-Path $root 'game'

Write-Host "Godot  : $godot"
Write-Host "Project: $projectPath"

if ($Scene) {
    $fullScene = Join-Path $projectPath $Scene
    if (-not (Test-Path $fullScene)) {
        Write-Warning "Scene not found: $fullScene"
    }
    Write-Host "Scene  : $Scene"
    & $godot --path $projectPath $Scene
} else {
    & $godot --path $projectPath
}
