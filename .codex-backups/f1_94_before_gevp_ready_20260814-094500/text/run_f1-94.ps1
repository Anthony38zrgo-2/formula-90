[CmdletBinding()]
param(
    [string]$GodotPath,
    [switch]$ValidateRuntimeOnly,
    [switch]$Smoke
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$game = Join-Path $root 'game'
$scene = 'res://scenes/runtime/f1_94_la_chutana.tscn'
$manifestPath = Join-Path $game 'assets\models\vehicles\f1_94\vehicle_runtime_manifest.json'
$smokeScript = 'res://tests/smoke_test_f1_94_la_chutana_hud.gd'

function Resolve-Godot([string]$ExplicitPath) {
    if ($ExplicitPath) {
        if (-not (Test-Path -LiteralPath $ExplicitPath -PathType Leaf)) {
            throw "Godot no existe: $ExplicitPath"
        }
        return (Resolve-Path -LiteralPath $ExplicitPath).Path
    }
    if ($env:GODOT_BIN -and (Test-Path -LiteralPath $env:GODOT_BIN -PathType Leaf)) {
        return (Resolve-Path -LiteralPath $env:GODOT_BIN).Path
    }
    $candidate = Join-Path $root '.tools\godot\Godot_v4.7.1-stable_win64.exe'
    if (Test-Path -LiteralPath $candidate -PathType Leaf) {
        return (Resolve-Path -LiteralPath $candidate).Path
    }
    throw 'Godot 4.7.1 no encontrado.'
}

if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Manifest runtime F1-94 faltante: $manifestPath"
}
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.vehicle_id -ne 'f1_94' -or $manifest.assembly_equivalence.status -ne 'PASS') {
    throw 'El runtime F1-94 no tiene equivalencia de ensamblado PASS.'
}
$sourcePath = Join-Path $root ($manifest.source.asset -replace '/', '\')
if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) {
    throw "Fuente normalizada F1-94 faltante: $sourcePath"
}
$sourceHash = (Get-FileHash -LiteralPath $sourcePath -Algorithm SHA256).Hash
if (-not $sourceHash.Equals($manifest.source.sha256, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'El hash de la fuente normalizada F1-94 no coincide con el manifest.'
}
if ($manifest.source.original_asset -and $manifest.source.original_sha256) {
    $originalPath = Join-Path $root ($manifest.source.original_asset -replace '/', '\')
    if (-not (Test-Path -LiteralPath $originalPath -PathType Leaf)) {
        throw "Fuente OBJ original F1-94 faltante: $originalPath"
    }
    $originalHash = (Get-FileHash -LiteralPath $originalPath -Algorithm SHA256).Hash
    if (-not $originalHash.Equals($manifest.source.original_sha256, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'El hash del OBJ original F1-94 no coincide con el manifest.'
    }
}
foreach ($assetProperty in $manifest.assets.PSObject.Properties) {
    $assetResPath = [string]$assetProperty.Value
    $assetDiskPath = Join-Path $game (($assetResPath -replace '^res://', '') -replace '/', '\')
    if (-not (Test-Path -LiteralPath $assetDiskPath -PathType Leaf)) {
        throw "Asset runtime F1-94 faltante: $assetDiskPath"
    }
}

$runtimeAppData = Join-Path $root '.tools\appdata'
$runtimeLocalAppData = Join-Path $root '.tools\localappdata'
New-Item -ItemType Directory -Force -Path $runtimeAppData, $runtimeLocalAppData | Out-Null
$env:APPDATA = (Resolve-Path -LiteralPath $runtimeAppData).Path
$env:LOCALAPPDATA = (Resolve-Path -LiteralPath $runtimeLocalAppData).Path
$godot = Resolve-Godot $GodotPath

Write-Host 'Importando y validando F1-94...' -ForegroundColor Cyan
& $godot --headless --path $game --import
if ($LASTEXITCODE -ne 0) {
    throw "Godot no pudo importar el runtime F1-94 ($LASTEXITCODE)."
}

if ($Smoke) {
    Write-Host 'Ejecutando smoke aislado de F1-94 + HUD + mapa...' -ForegroundColor Cyan
    & $godot --headless --path $game --script $smokeScript
    exit $LASTEXITCODE
}
if ($ValidateRuntimeOnly) {
    Write-Host 'Runtime F1-94 e importacion validados.' -ForegroundColor Green
    return
}

Write-Host "Iniciando $scene" -ForegroundColor Green
& $godot --path $game $scene
exit $LASTEXITCODE
