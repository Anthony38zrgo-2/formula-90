[CmdletBinding()]
param(
    [string]$GodotPath,
    [ValidateSet('1994', '2009', '2026')]
    [string]$VehicleVariant = '2026',
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
$is2026Variant = $VehicleVariant -eq '2026'
$is2009Variant = $VehicleVariant -eq '2009'
$scene = if ($is2026Variant) {
    'res://scenes/runtime/vehicle_test_session_2026.tscn'
} elseif ($is2009Variant) {
    'res://scenes/runtime/vehicle_test_session_2009.tscn'
} else {
    'res://scenes/runtime/vehicle_test_session.tscn'
}
$manifestPath = if ($is2026Variant) {
    Join-Path $game 'assets\models\vehicles\f1-2026-2008\manifest.json'
} elseif ($is2009Variant) {
    Join-Path $game 'assets\models\vehicles\f1_94\variants\f1_2009_fw31\manifest.json'
} else {
    Join-Path $game 'assets\models\vehicles\f1_94\decoupled\manifest.json'
}
$variantLabel = if ($is2026Variant) { 'F1 2026-2008 new base vehicle' } elseif ($is2009Variant) { 'F1 2009 Williams FW31 optional variant' } else { 'F1 1994 textured canonical' }
$trackPath = Join-Path $game 'assets\generated\tracks\la_chutana\la_chutana.glb'
$trackBuildPath = Join-Path $game 'assets\generated\tracks\la_chutana\runtime_build.json'
$trackConfigPath = Join-Path $root 'blender\track_pipeline\configs\la_chutana.json'
$barrierManifestPath = Join-Path $root 'blender\track_pipeline\manifests\la_chutana_safety_barriers_v3.json'
$barrierAssetManifestPath = Join-Path $root 'game\resources\environment\assets\barriers\barrier_manifest_v3.json'
$barrierConstructionManifestPath = Join-Path $root 'game\resources\environment\manifests\barrier_construction_manifest_v2.json'
$barrierRecipeManifestPath = Join-Path $root 'game\resources\environment\recipes\barrier_library.json'
$barrierPaletteManifestPath = Join-Path $root 'game\resources\environment\palettes\barrier_palette.json'
$barrierGeneratorPath = Join-Path $root 'game\resources\environment\tools\build_barrier_3d_library.py'
$environmentBuilderPath = Join-Path $root 'blender\track_pipeline\build_environment_blender.py'
$buildingAssetManifestPath = Join-Path $root 'game\resources\environment\assets\buildings\building_manifest.json'
$buildingSourceManifestPath = Join-Path $root 'game\resources\environment\manifests\building_sources.json'
$buildingConstructionManifestPath = Join-Path $root 'game\resources\environment\manifests\building_construction_manifest.json'
$buildingGeneratorPath = Join-Path $root 'game\resources\environment\tools\build_building_asset_manifest.py'
$buildingIntegrationValidatorPath = Join-Path $root 'blender\track_pipeline\validate_building_asset_integration.py'
$tracksideIntegrationValidatorPath = Join-Path $root 'blender\track_pipeline\validate_trackside_asset_integration.py'
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

if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) { throw "Manifest de vehiculo faltante: $manifestPath" }
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
$runtimeDir = Split-Path -Parent $manifestPath
if ($is2026Variant) {
    if ($manifest.asset -ne 'F1_2026_2008' -or $manifest.standard -ne 'Formula-90 GEVP decoupled visual asset') {
        throw 'El manifest del F1-2026-2008 no cumple el contrato esperado.'
    }
    $expectedAssets = @{
        ([string]$manifest.geometry_assets.chassis_gevp.path).Replace('/', '\') = [string]$manifest.geometry_assets.chassis_gevp.sha256
        ([string]$manifest.geometry_assets.wheel_front_canonical.path).Replace('/', '\') = [string]$manifest.geometry_assets.wheel_front_canonical.sha256
        ([string]$manifest.geometry_assets.wheel_rear_canonical.path).Replace('/', '\') = [string]$manifest.geometry_assets.wheel_rear_canonical.sha256
    }
    $physicsRelative = ([string]$manifest.physics_profile).Replace('res://', '').Replace('/', '\')
    $physicsPath = Join-Path $game $physicsRelative
    if (-not (Test-Path -LiteralPath $physicsPath -PathType Leaf)) { throw "Perfil fisico 2026 faltante: $physicsPath" }
    $physicsHash = (Get-FileHash -LiteralPath $physicsPath -Algorithm SHA256).Hash
    if (-not $physicsHash.Equals([string]$manifest.physics_sha256, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Hash inesperado para el perfil fisico 2026.'
    }
} elseif ($is2009Variant) {
    if ($manifest.variant_id -ne 'f1_2009_fw31' -or $manifest.base_vehicle_id -ne 'f1_94') {
        throw 'El manifest de la variante 2009 no cumple el contrato esperado.'
    }
    $expectedAssets = @{
        ([string]$manifest.runtime.chassis.path).Replace('/', '\') = [string]$manifest.runtime.chassis.sha256
        ([string]$manifest.runtime.wheel_front.path).Replace('/', '\') = [string]$manifest.runtime.wheel_front.sha256
        ([string]$manifest.runtime.wheel_rear.path).Replace('/', '\') = [string]$manifest.runtime.wheel_rear.sha256
    }
    $physicsRelative = ([string]$manifest.runtime.physics_profile).Replace('res://', '').Replace('/', '\')
    $physicsPath = Join-Path $game $physicsRelative
    if (-not (Test-Path -LiteralPath $physicsPath -PathType Leaf)) { throw "Perfil fisico 2009 faltante: $physicsPath" }
    $physicsHash = (Get-FileHash -LiteralPath $physicsPath -Algorithm SHA256).Hash
    if (-not $physicsHash.Equals([string]$manifest.runtime.physics_sha256, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Hash inesperado para el perfil fisico 2009.'
    }
} else {
    if ($manifest.asset -ne 'F1_94' -or $manifest.standard -ne 'Formula-90 GEVP decoupled visual asset') {
        throw 'El manifest desacoplado F1-94 no cumple el contrato esperado.'
    }
    $expectedAssets = @{
        ([string]$manifest.geometry_assets.chassis_gevp.path).Replace('/', '\') = [string]$manifest.geometry_assets.chassis_gevp.sha256
        ([string]$manifest.geometry_assets.wheel_front_canonical.path).Replace('/', '\') = [string]$manifest.geometry_assets.wheel_front_canonical.sha256
        ([string]$manifest.geometry_assets.wheel_rear_canonical.path).Replace('/', '\') = [string]$manifest.geometry_assets.wheel_rear_canonical.sha256
    }
}
foreach ($relativePath in $expectedAssets.Keys) {
    $assetPath = Join-Path $runtimeDir $relativePath
    if (-not (Test-Path -LiteralPath $assetPath -PathType Leaf)) { throw "Asset de vehiculo faltante: $assetPath" }
    $actualHash = (Get-FileHash -LiteralPath $assetPath -Algorithm SHA256).Hash
    if (-not $actualHash.Equals($expectedAssets[$relativePath], [StringComparison]::OrdinalIgnoreCase)) { throw "Hash inesperado para $relativePath" }
}

if (-not (Test-Path -LiteralPath $trackPath -PathType Leaf)) {
    throw "Circuito runtime faltante: $trackPath"
}
Write-Host "MODO NO-REBUILD: se omite la validacion de BUILD/HEAD y paridad de La Chutana." -ForegroundColor Yellow
if (Test-Path -LiteralPath $trackBuildPath -PathType Leaf) {
    $trackBuild = Get-Content -LiteralPath $trackBuildPath -Raw | ConvertFrom-Json
    $currentHead = (& git -C $root rev-parse HEAD).Trim()
    if ($LASTEXITCODE -eq 0 -and $trackBuild.head -eq $currentHead) {
        Write-Host "La Chutana BUILD/HEAD coincide (sin exigir rebuild): $currentHead" -ForegroundColor Green
    } else {
        Write-Host "Advertencia: BUILD/HEAD de La Chutana no coincide; se continua igualmente (modo no-rebuild)." -ForegroundColor Yellow
    }
} else {
    Write-Host "Advertencia: BUILD del circuito faltante; se continua igualmente (modo no-rebuild)." -ForegroundColor Yellow
}

$runtimeAppData = Join-Path $root '.tools\appdata'
$runtimeLocalAppData = Join-Path $root '.tools\localappdata'
New-Item -ItemType Directory -Force -Path $runtimeAppData, $runtimeLocalAppData | Out-Null
$env:APPDATA = (Resolve-Path -LiteralPath $runtimeAppData).Path
$env:LOCALAPPDATA = (Resolve-Path -LiteralPath $runtimeLocalAppData).Path
$godot = Resolve-Godot $GodotPath

Write-Host "Importando y validando $variantLabel..." -ForegroundColor Cyan
& $godot --headless --path $game --import
if ($LASTEXITCODE -ne 0) {
    throw "Godot no pudo importar el runtime $variantLabel ($LASTEXITCODE)."
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
    Write-Host "Runtime $variantLabel validado." -ForegroundColor Green
    return
}

Write-Host "Iniciando $variantLabel con $scene" -ForegroundColor Green
& $godot --path $game $scene
exit $LASTEXITCODE
