[CmdletBinding()]
param(
    [string]$GodotPath,
    [switch]$ValidateRuntimeOnly,
    [switch]$Smoke,
    [switch]$SmokeAudio,
    [switch]$SmokeBackground,
    [switch]$TestPhysics,
    [switch]$Parity
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$game = Join-Path $root 'game'
$scene = 'res://scenes/runtime/vehicle_test_session.tscn'
$manifestPath = Join-Path $game 'assets\models\vehicles\f1_94\decoupled\manifest.json'
$smokeScript = 'res://tests/smoke_test_f1_94_la_chutana_hud.gd'
$smokeBackgroundScript = 'res://tests/smoke_test_mountains_3d.gd'

function Resolve-Godot([string]$ExplicitPath) {
    if ($ExplicitPath -and (Test-Path -LiteralPath $ExplicitPath -PathType Leaf)) {
        return (Resolve-Path -LiteralPath $ExplicitPath).Path
    }
    if ($env:GODOT_BIN -and (Test-Path -LiteralPath $env:GODOT_BIN -PathType Leaf)) {
        return (Resolve-Path -LiteralPath $env:GODOT_BIN).Path
    }
    $candidate = Join-Path $root '.tools\godot\Godot_v4.7.1-stable_win64_console.exe'
    if (Test-Path -LiteralPath $candidate -PathType Leaf) {
        return (Resolve-Path -LiteralPath $candidate).Path
    }
    throw 'Godot 4.7.1 no encontrado.'
}

if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Manifest desacoplado F1-94 faltante: $manifestPath"
}
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.asset -ne 'F1_94' -or $manifest.standard -ne 'Formula-90 GEVP decoupled visual asset') {
    throw 'El manifest desacoplado F1-94 no cumple el contrato esperado.'
}
$expectedAssets = @{
    'geometry\F1_94_chassis_geometry.glb' = '5C5CE50E0B94B027F1D8A835E860DC67F87DECAA4AEB17DFD87674FBF22F4AB2'
    'geometry\F1_94_wheel_front_geometry.glb' = '0F9923994E8AC8E328EBC621358D3AA7BE86C61BCDB0C65FD5B0F4676B8F2D53'
    'geometry\F1_94_wheel_rear_geometry.glb' = '32A9F4E78F9B7CCEDFFF72ACA851E54BA650A89D639B08622390CE451A2E1D75'
}
$runtimeDir = Split-Path -Parent $manifestPath
foreach ($relativePath in $expectedAssets.Keys) {
    $assetPath = Join-Path $runtimeDir $relativePath
    if (-not (Test-Path -LiteralPath $assetPath -PathType Leaf)) {
        throw "Asset desacoplado F1-94 faltante: $assetPath"
    }
    $actualHash = (Get-FileHash -LiteralPath $assetPath -Algorithm SHA256).Hash
    if (-not $actualHash.Equals($expectedAssets[$relativePath], [StringComparison]::OrdinalIgnoreCase)) {
        throw "Hash inesperado para $relativePath"
    }
}

$runtimeAppData = Join-Path $root '.tools\appdata'
$runtimeLocalAppData = Join-Path $root '.tools\localappdata'
New-Item -ItemType Directory -Force -Path $runtimeAppData, $runtimeLocalAppData | Out-Null
$env:APPDATA = (Resolve-Path -LiteralPath $runtimeAppData).Path
$env:LOCALAPPDATA = (Resolve-Path -LiteralPath $runtimeLocalAppData).Path
$godot = Resolve-Godot $GodotPath

Write-Host 'Importando y validando F1-94 desacoplado...' -ForegroundColor Cyan
& $godot --headless --path $game --import
if ($LASTEXITCODE -ne 0) {
    throw "Godot no pudo importar el runtime F1-94 ($LASTEXITCODE)."
}

if ($SmokeAudio) {
    Write-Host 'Ejecutando smoke de F1-94 audio (v10_vehicle + GEVP)...' -ForegroundColor Cyan
    & $godot --headless --path $game --script 'res://tests/smoke_test_f1_94_audio.gd'
    exit $LASTEXITCODE
}
if ($SmokeBackground) {
    Write-Host 'Ejecutando smoke de background (skybox gradient + 3 capas parallax)...' -ForegroundColor Cyan
    & $godot --headless --path $game --script $smokeBackgroundScript
    exit $LASTEXITCODE
}
if ($Smoke) {
    Write-Host 'Ejecutando smoke de F1-94 + La Chutana + HUD + Background...' -ForegroundColor Cyan
    & $godot --headless --path $game --script $smokeScript
    $hudExit = $LASTEXITCODE
    if ($hudExit -ne 0) {
        Write-Host "Smoke HUD fallo ($hudExit)." -ForegroundColor Red
        exit $hudExit
    }
    & $godot --headless --path $game --script $smokeBackgroundScript
    $bgExit = $LASTEXITCODE
    if ($bgExit -ne 0) {
        Write-Host "Smoke Background fallo ($bgExit)." -ForegroundColor Red
        exit $bgExit
    }
    Write-Host 'Smoke completo: HUD + Background.' -ForegroundColor Green
    exit 0
}
if ($TestPhysics) {
    Write-Host 'Ejecutando suite determinista Rust + test de integraciÃ³n Godot (12 raycasts)...' -ForegroundColor Cyan
    & cargo test --manifest-path (Join-Path $root 'game\crates\vehicle-physics-engine\Cargo.toml')
    if ($LASTEXITCODE -ne 0) { throw "Rust vehicle physics unit tests fallaron ($LASTEXITCODE)." }
    & $godot --headless --path $game --script 'res://tests/test_f1_94_rust_physics.gd'
    exit $LASTEXITCODE
}
if ($Parity) {
    Write-Host 'Ejecutando test de paridad en pista La Chutana (PHY-010)...' -ForegroundColor Cyan
    & $godot --headless --path $game --script 'res://tests/test_f1_94_la_chutana_parity.gd'
    exit $LASTEXITCODE
}
if ($ValidateRuntimeOnly) {
    Write-Host 'Runtime F1-94 desacoplado validado.' -ForegroundColor Green
    return
}

Write-Host "Iniciando $scene" -ForegroundColor Green
& $godot --path $game $scene
exit $LASTEXITCODE
