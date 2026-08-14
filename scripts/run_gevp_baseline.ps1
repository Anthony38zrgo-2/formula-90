[CmdletBinding()]
param([string]$GodotPath)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

function Resolve-Godot([string]$explicit) {
    if ($explicit) {
        if (Test-Path $explicit) {
            return (Resolve-Path $explicit).Path
        }

        throw "Godot no existe: $explicit"
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
        if (Test-Path $item) {
            return (Resolve-Path $item).Path
        }
    }

    throw 'Godot 4.7.1 no encontrado.'
}

$godot = Resolve-Godot $GodotPath

$dll = Join-Path $root `
    'game\addons\formula90s\bin\libformula90s.windows.template_debug.x86_64.dll'

if (-not (Test-Path $dll)) {
    throw 'GDExtension no compilada. Ejecute build_windows.ps1.'
}

$game = Join-Path $root 'game'
$scene = 'res://scenes/tests/vehicle_track_combinations/gevp_baseline.tscn'

Write-Host "Godot: $godot"
Write-Host "Proyecto: $game"
Write-Host "Escena: $scene"

& $godot `
    --path $game `
    --scene $scene
