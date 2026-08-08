<#
.SYNOPSIS
    Open the F1 2026_B vehicle in the Godot 4.7 editor.

.DESCRIPTION
    Shortcut: opens Godot with the player_car.tscn scene loaded,
    which uses the new f1_2026_b model with decimated meshes and embedded textures.

.PARAMETER GodotPath
    Explicit path to a Godot executable.

.EXAMPLE
    .\scripts\open_f1_2026_b.ps1
#>
[CmdletBinding()]
param([string]$GodotPath)

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
$scene = 'scenes/vehicles/player_car.tscn'

Write-Host "Godot  : $godot"
Write-Host "Project: $projectPath"
Write-Host "Scene  : $scene"

& $godot --path $projectPath $scene
