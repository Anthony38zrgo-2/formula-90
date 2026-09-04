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
$variantLabel = if ($is2026Variant) { 'F1 2026-2008 canonical vehicle' } elseif ($is2009Variant) { 'F1 2009 Williams FW31 optional variant' } else { 'F1 1994 legacy variant' }
$vehicleId = if ($is2026Variant) { 'f1_2026_2008' } elseif ($is2009Variant) { 'f1_2009_fw31' } else { 'f1_94' }
$trackDir = Join-Path $game 'tracks\fuji76_77'
$trackPath = Join-Path $trackDir 'fuji76_77_visual.glb'
$trackCollisionPath = Join-Path $trackDir 'fuji76_77_collision.glb'
$trackBuildPath = Join-Path $trackDir 'metadata\package.json'
$smokeScript = 'res://tests/smoke_test_f1_94_fuji76_77.gd'

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

function Get-Sha256Hex([string]$Path) {
    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $sha256 = [System.Security.Cryptography.SHA256]::Create()
        try {
            return ([System.BitConverter]::ToString($sha256.ComputeHash($stream))).Replace('-', '')
        } finally {
            $sha256.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
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
    $physicsHash = Get-Sha256Hex $physicsPath
    if (-not $physicsHash.Equals([string]$manifest.physics_sha256, [StringComparison]::OrdinalIgnoreCase)) {
        Write-Warning ('Perfil fisico 2026 modificado: hashes difieren del manifest. Si el cambio es intencional, refresca physics_sha256 en el manifest. actual=' + $physicsHash)
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
    $physicsHash = Get-Sha256Hex $physicsPath
    if (-not $physicsHash.Equals([string]$manifest.runtime.physics_sha256, [StringComparison]::OrdinalIgnoreCase)) {
        Write-Warning ('Perfil fisico 2009 modificado: hashes difieren del manifest. Si el cambio es intencional, refresca physics_sha256 en el manifest. actual=' + $physicsHash)
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
    $actualHash = Get-Sha256Hex $assetPath
    if (-not $actualHash.Equals($expectedAssets[$relativePath], [StringComparison]::OrdinalIgnoreCase)) { throw "Hash inesperado para $relativePath" }
}

if (-not (Test-Path -LiteralPath $trackPath -PathType Leaf)) {
    throw "Circuito runtime faltante: $trackPath"
}
if (-not (Test-Path -LiteralPath $trackCollisionPath -PathType Leaf)) { throw "Colision runtime faltante: $trackCollisionPath" }
if (-not (Test-Path -LiteralPath $trackBuildPath -PathType Leaf)) { throw "Manifest del circuito faltante: $trackBuildPath" }
$trackBuild = Get-Content -LiteralPath $trackBuildPath -Raw | ConvertFrom-Json
if ([int]$trackBuild.contract_version -ne 1 -or [string]$trackBuild.track_id -ne 'fuji76_77') { throw 'Contrato de Fuji ausente o incompatible.' }
foreach ($fileEntry in $trackBuild.files.PSObject.Properties) {
    $assetPath = Join-Path $trackDir ([string]$fileEntry.Name).Replace('/', '\')
    if (-not (Test-Path -LiteralPath $assetPath -PathType Leaf)) { throw "Archivo de paquete faltante: $assetPath" }
    $actualHash = Get-Sha256Hex $assetPath
    if (-not $actualHash.Equals([string]$fileEntry.Value.sha256, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Hash de paquete invalido: $($fileEntry.Name)"
    }
}
Write-Host 'Paquete runtime de Fuji 76-77 validado.' -ForegroundColor Green

$runtimeAppData = Join-Path $root '.tools\appdata'
$runtimeLocalAppData = Join-Path $root '.tools\localappdata'
New-Item -ItemType Directory -Force -Path $runtimeAppData, $runtimeLocalAppData | Out-Null
$env:APPDATA = (Resolve-Path -LiteralPath $runtimeAppData).Path
$env:LOCALAPPDATA = (Resolve-Path -LiteralPath $runtimeLocalAppData).Path
$godot = Resolve-Godot $GodotPath

$buildSourcePath = Join-Path $game 'BUILD_SOURCE'
if (-not (Test-Path -LiteralPath $buildSourcePath -PathType Leaf)) {
    throw 'game/BUILD_SOURCE ausente. Ejecute scripts/build_windows.ps1.'
}
$headSha = (& git rev-parse HEAD).Trim()
$buildSource = (Get-Content -LiteralPath $buildSourcePath -Raw).Trim()
$buildCommit = (& git log -1 --format=%H -- $buildSourcePath).Trim()
$buildParent = (& git rev-parse "$buildCommit~1" 2>$null)
if ($buildParent) { $buildParent = $buildParent.Trim() }
if ($buildSource -ne $headSha -and $buildSource -ne $buildCommit -and $buildSource -ne $buildParent) {
    throw "Paridad BUILD/HEAD rota: BUILD_SOURCE=$buildSource HEAD=$headSha. Ejecute scripts/build_windows.ps1."
}
$requiredDlls = @(
    'formula90_core.dll',
    'game_sim.dll',
    'vehicle_audio_engine.dll',
    'vehicle_physics_engine.dll',
    'psx_art_plugin.dll',
    'f90_audio_dsp.dll'
)
$binDir = Join-Path $game 'addons\formula90s\bin'
foreach ($dll in $requiredDlls) {
    if (-not (Test-Path -LiteralPath (Join-Path $binDir $dll) -PathType Leaf)) {
        throw "DLL faltante: $dll. Ejecute scripts/build_windows.ps1."
    }
}
Write-Host "Paridad BUILD/HEAD validada: $headSha" -ForegroundColor Green

Write-Host "Importando y validando $variantLabel..." -ForegroundColor Cyan
& $godot --headless --path $game --import
if ($LASTEXITCODE -ne 0) {
    throw "Godot no pudo importar el runtime $variantLabel ($LASTEXITCODE)."
}

if ($SmokeAudio) {
    Write-Host "Ejecutando smoke de audio $variantLabel (v10_vehicle + GEVP)..." -ForegroundColor Cyan
    & $godot --headless --path $game --script 'res://tests/smoke_test_f1_94_audio.gd' -- "--scene=$scene"
    exit $LASTEXITCODE
}
if ($SmokeBackground) {
    Write-Host 'Fuji no usa skybox ni montanas externas; ejecutando smoke del circuito.' -ForegroundColor Cyan
    & $godot --headless --path $game --script $smokeScript -- "--scene=$scene" "--vehicle-id=$vehicleId"
    exit $LASTEXITCODE
}
if ($Smoke) {
    Write-Host "Ejecutando smoke de $variantLabel + Fuji 76-77 + camara + HUD..." -ForegroundColor Cyan
    & $godot --headless --path $game --script $smokeScript -- "--scene=$scene" "--vehicle-id=$vehicleId"
    exit $LASTEXITCODE
}
if ($TestPhysics) {
    Write-Host 'Ejecutando suite determinista Rust + test de integraciÃ³n Godot (12 raycasts)...' -ForegroundColor Cyan
    & cargo test --manifest-path (Join-Path $root 'game\crates\vehicle-physics-engine\Cargo.toml')
    if ($LASTEXITCODE -ne 0) { throw "Rust vehicle physics unit tests fallaron ($LASTEXITCODE)." }
    & $godot --headless --path $game --script 'res://tests/test_f1_94_rust_physics.gd'
    exit $LASTEXITCODE
}
if ($Parity) {
    Write-Host 'Ejecutando smoke de paridad minima en Fuji 76-77...' -ForegroundColor Cyan
    & $godot --headless --path $game --script $smokeScript -- "--scene=$scene" "--vehicle-id=$vehicleId"
    exit $LASTEXITCODE
}
if ($ValidateRuntimeOnly) {
    Write-Host "Runtime $variantLabel validado." -ForegroundColor Green
    return
}

Write-Host "Iniciando $variantLabel con $scene" -ForegroundColor Green
& $godot --path $game $scene
exit $LASTEXITCODE
