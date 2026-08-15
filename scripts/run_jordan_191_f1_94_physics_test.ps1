[CmdletBinding()]
param(
    [string]$GodotPath,
    [switch]$ValidateOnly,
    [switch]$Smoke
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$game = Join-Path $root 'game'
$scene = 'res://scenes/tests/vehicle_track_combinations/jordan_191_f1_94_physics_test.tscn'
$smokeScript = 'res://tests/smoke_test_jordan_191_f1_94_physics.gd'
$assets = @(
    'scenes\vehicles\jordan_191\jordan_191_chassis.glb',
    'scenes\vehicles\jordan_191\jordan_191_wheel_fl.glb',
    'scenes\vehicles\jordan_191\jordan_191_wheel_fr.glb',
    'scenes\vehicles\jordan_191\jordan_191_wheel_rl.glb',
    'scenes\vehicles\jordan_191\jordan_191_wheel_rr.glb'
)

function Resolve-Godot([string]$ExplicitPath) {
    if ($ExplicitPath -and (Test-Path -LiteralPath $ExplicitPath -PathType Leaf)) { return (Resolve-Path -LiteralPath $ExplicitPath).Path }
    if ($env:GODOT_BIN -and (Test-Path -LiteralPath $env:GODOT_BIN -PathType Leaf)) { return (Resolve-Path -LiteralPath $env:GODOT_BIN).Path }
    $candidate = Join-Path $root '.tools\godot\Godot_v4.7.1-stable_win64_console.exe'
    if (Test-Path -LiteralPath $candidate -PathType Leaf) { return (Resolve-Path -LiteralPath $candidate).Path }
    throw 'Godot 4.7.1 no encontrado. Indique -GodotPath o defina GODOT_BIN.'
}

foreach ($relativePath in $assets) {
    $assetPath = Join-Path $game $relativePath
    if (-not (Test-Path -LiteralPath $assetPath -PathType Leaf)) { throw "Asset Jordan 191 faltante: $assetPath" }
}
foreach ($relativePath in @(
    'scenes\vehicles\jordan_191\jordan_191_f1_94_physics.tscn',
    'scenes\tests\vehicle_track_combinations\jordan_191_f1_94_physics_test.tscn'
)) {
    if (-not (Test-Path -LiteralPath (Join-Path $game $relativePath) -PathType Leaf)) { throw "Ruta aislada faltante: $relativePath" }
}

$runtimeAppData = Join-Path $root '.tools\appdata'
$runtimeLocalAppData = Join-Path $root '.tools\localappdata'
New-Item -ItemType Directory -Force -Path $runtimeAppData, $runtimeLocalAppData | Out-Null
$env:APPDATA = (Resolve-Path -LiteralPath $runtimeAppData).Path
$env:LOCALAPPDATA = (Resolve-Path -LiteralPath $runtimeLocalAppData).Path
$godot = Resolve-Godot $GodotPath

Write-Host 'Importando la ruta aislada Jordan 191 + fisica F1-94...' -ForegroundColor Cyan
& $godot --headless --path $game --import
if ($LASTEXITCODE -ne 0) { throw "Godot no pudo importar la ruta aislada ($LASTEXITCODE)." }
if ($Smoke) {
    & $godot --headless --path $game --script $smokeScript
    exit $LASTEXITCODE
}
if ($ValidateOnly) { Write-Host 'Ruta aislada importada y validada.' -ForegroundColor Green; return }
Write-Host "Iniciando $scene" -ForegroundColor Green
& $godot --path $game $scene
exit $LASTEXITCODE
