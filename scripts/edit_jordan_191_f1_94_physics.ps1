[CmdletBinding()]
param(
    [string]$GodotPath,
    [switch]$Wait
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$game = Join-Path $root 'game'
$scene = 'res://scenes/vehicles/jordan_191/jordan_191_f1_94_physics.tscn'
$sceneDiskPath = Join-Path $game 'scenes\vehicles\jordan_191\jordan_191_f1_94_physics.tscn'

function Resolve-Godot([string]$ExplicitPath) {
    if ($ExplicitPath) {
        if (Test-Path -LiteralPath $ExplicitPath -PathType Leaf) {
            return (Resolve-Path -LiteralPath $ExplicitPath).Path
        }
        throw "Godot no encontrado en: $ExplicitPath"
    }

    if ($env:GODOT_BIN -and (Test-Path -LiteralPath $env:GODOT_BIN -PathType Leaf)) {
        return (Resolve-Path -LiteralPath $env:GODOT_BIN).Path
    }

    $candidates = @(
        (Join-Path $root '.tools\godot\Godot_v4.7.1-stable_win64.exe'),
        (Join-Path $root '.tools\godot\Godot_v4.7.1-stable_win64_console.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs\Godot\Godot.exe'),
        (Join-Path $env:ProgramFiles 'Godot\Godot.exe')
    )
    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }

    throw 'Godot 4.7.1 no encontrado. Use -GodotPath o defina GODOT_BIN.'
}

if (-not (Test-Path -LiteralPath $sceneDiskPath -PathType Leaf)) {
    throw "Escena aislada Jordan 191 + fisica F1-94 no encontrada: $sceneDiskPath"
}

# Keep user:// writable and isolated inside the workspace.
$runtimeAppData = Join-Path $root '.tools\appdata'
$runtimeLocalAppData = Join-Path $root '.tools\localappdata'
New-Item -ItemType Directory -Force -Path $runtimeAppData, $runtimeLocalAppData | Out-Null
$env:APPDATA = (Resolve-Path -LiteralPath $runtimeAppData).Path
$env:LOCALAPPDATA = (Resolve-Path -LiteralPath $runtimeLocalAppData).Path

$godot = Resolve-Godot $GodotPath
$arguments = @('--editor', '--path', $game, $scene)

Write-Host 'Formula-90 - Editor Jordan 191 / fisica F1-94' -ForegroundColor Cyan
Write-Host "Godot:   $godot" -ForegroundColor Green
Write-Host "Proyecto: $game"
Write-Host "Escena:   $scene" -ForegroundColor Yellow

if ($Wait) {
    & $godot $arguments
    exit $LASTEXITCODE
}

Start-Process -FilePath $godot -ArgumentList $arguments -WorkingDirectory $game
Write-Host 'Editor de Godot iniciado con la escena aislada.' -ForegroundColor Green
