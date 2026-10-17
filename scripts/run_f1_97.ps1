[CmdletBinding()]
param(
    [string]$GodotPath,
    [switch]$ValidateRuntimeOnly
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$game = Join-Path $root 'game'
$scene = 'res://scenes/runtime/williams_hockenheim_villenueve_fuji_test_session.tscn'
$vehicleId = 'f1_1997_williams_hockenheim_villenueve'
$variantLabel = 'Williams FW19 1997 Hockenheim / Villeneuve'
$manifestPath = Join-Path $game 'assets\models\vehicles\f1-1997\williams\variants\hockenheim\villenueve\manifest.json'

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
if ([string]$manifest.vehicle_id -ne $vehicleId) { throw "vehicle_id inesperado: $($manifest.vehicle_id)" }
if ([string]$manifest.asset -ne 'F1_1997_WILLIAMS_HOCKENHEIM_VILLENUEVE') { throw "asset inesperado: $($manifest.asset)" }
if ([string]$manifest.standard -ne 'Formula-90 GVEP decoupled visual asset') { throw "standard inesperado: $($manifest.standard)" }

$runtimeDir = Split-Path -Parent $manifestPath
$expectedAssets = @{}
foreach ($assetEntry in $manifest.geometry_assets.PSObject.Properties) {
    $relative = ([string]$assetEntry.Value.path).Replace('/', '\')
    if ($expectedAssets.ContainsKey($relative)) { throw "Asset duplicado en manifest: $relative" }
    $expectedAssets[$relative] = [string]$assetEntry.Value.sha256
}
if ($expectedAssets.Count -ne 6) { throw "Williams debe declarar exactamente 6 GLB; encontrados: $($expectedAssets.Count)" }
foreach ($relativePath in $expectedAssets.Keys) {
    $assetPath = Join-Path $runtimeDir $relativePath
    if (-not (Test-Path -LiteralPath $assetPath -PathType Leaf)) { throw "Asset de vehiculo faltante: $assetPath" }
    $actualHash = Get-Sha256Hex $assetPath
    if (-not $actualHash.Equals($expectedAssets[$relativePath], [StringComparison]::OrdinalIgnoreCase)) { throw "Hash inesperado para $relativePath" }
}

$physicsRelative = ([string]$manifest.physics_profile).Replace('res://', '').Replace('/', '\')
$physicsPath = Join-Path $game $physicsRelative
if (-not (Test-Path -LiteralPath $physicsPath -PathType Leaf)) { throw "Perfil fisico faltante: $physicsPath" }
if ([string]$manifest.physics_digest_contract -ne 'vehicle_profile_provenance_v1') { throw 'Contrato de digest del perfil versionado ausente o incompatible.' }
$previousErrorPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
$profileResolutionOutput = & cargo run --quiet --manifest-path (Join-Path $game 'crates\Cargo.toml') --package vehicle_physics_engine --bin resolve_vehicle_configuration -- $root $physicsPath 2>$null
$cargoExitCode = $LASTEXITCODE
$ErrorActionPreference = $previousErrorPreference
if ($cargoExitCode -ne 0) { throw "No se pudo resolver el perfil fisico: $physicsPath" }
$resolvedProfile = ($profileResolutionOutput -join "`n") | ConvertFrom-Json
$physicsHash = [string]$resolvedProfile.provenance.global_digest_sha256
if (-not $physicsHash.Equals([string]$manifest.physics_sha256, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Digest del perfil fisico distinto del manifest: actual=$physicsHash manifest=$($manifest.physics_sha256)"
}
Write-Host 'Manifest, seis GLB y digest del perfil Williams validados.' -ForegroundColor Green

$trackDir = Join-Path $game 'tracks\fuji76_77'
$trackPath = Join-Path $trackDir 'fuji76_77_visual.glb'
$trackCollisionPath = Join-Path $trackDir 'fuji76_77_collision.glb'
$trackBuildPath = Join-Path $trackDir 'metadata\package.json'
if (-not (Test-Path -LiteralPath $trackPath -PathType Leaf)) { throw "Circuito runtime faltante: $trackPath" }
if (-not (Test-Path -LiteralPath $trackCollisionPath -PathType Leaf)) { throw "Colision runtime faltante: $trackCollisionPath" }
if (-not (Test-Path -LiteralPath $trackBuildPath -PathType Leaf)) { throw "Manifest del circuito faltante: $trackBuildPath" }
$trackBuild = Get-Content -LiteralPath $trackBuildPath -Raw | ConvertFrom-Json
if ([int]$trackBuild.contract_version -ne 1 -or [string]$trackBuild.track_id -ne 'fuji76_77') { throw 'Contrato de Fuji ausente o incompatible.' }
foreach ($fileEntry in $trackBuild.files.PSObject.Properties) {
    $assetPath = Join-Path $trackDir ([string]$fileEntry.Name).Replace('/', '\')
    if (-not (Test-Path -LiteralPath $assetPath -PathType Leaf)) { throw "Archivo de paquete faltante: $assetPath" }
    $actualHash = Get-Sha256Hex $assetPath
    if (-not $actualHash.Equals([string]$fileEntry.Value.sha256, [StringComparison]::OrdinalIgnoreCase)) { throw "Hash de paquete invalido: $($fileEntry.Name)" }
}
Write-Host 'Paquete runtime de Fuji 76-77 validado.' -ForegroundColor Green

$buildSourcePath = Join-Path $game 'BUILD_SOURCE'
if (-not (Test-Path -LiteralPath $buildSourcePath -PathType Leaf)) { throw 'game/BUILD_SOURCE ausente. Ejecute scripts/build_windows.ps1.' }
$headSha = (& git -C $root rev-parse HEAD).Trim()
$buildSource = (Get-Content -LiteralPath $buildSourcePath -Raw).Trim()
if ($buildSource -ne $headSha) {
    throw "Paridad BUILD/HEAD rota: BUILD_SOURCE=$buildSource HEAD=$headSha. Ejecute scripts/build_windows.ps1."
}
$requiredDlls = @('formula90_core.dll', 'game_sim.dll', 'vehicle_audio_engine.dll', 'vehicle_physics_engine.dll', 'psx_art_plugin.dll', 'f90_audio_dsp.dll')
$binDir = Join-Path $game 'addons\formula90s\bin'
foreach ($dll in $requiredDlls) {
    if (-not (Test-Path -LiteralPath (Join-Path $binDir $dll) -PathType Leaf)) { throw "DLL faltante: $dll. Ejecute scripts/build_windows.ps1." }
}
Write-Host "Paridad BUILD/HEAD validada: $headSha" -ForegroundColor Green

$runtimeAppData = Join-Path $root '.tools\appdata'
$runtimeLocalAppData = Join-Path $root '.tools\localappdata'
New-Item -ItemType Directory -Force -Path $runtimeAppData, $runtimeLocalAppData | Out-Null
$env:APPDATA = (Resolve-Path -LiteralPath $runtimeAppData).Path
$env:LOCALAPPDATA = (Resolve-Path -LiteralPath $runtimeLocalAppData).Path
$godot = Resolve-Godot $GodotPath

Write-Host "Importando y validando $variantLabel..." -ForegroundColor Cyan
& $godot --headless --path $game --import
if ($LASTEXITCODE -ne 0) { throw "Godot no pudo importar el runtime $variantLabel ($LASTEXITCODE)." }

if ($ValidateRuntimeOnly) {
    Write-Host "Runtime $variantLabel validado." -ForegroundColor Green
    return
}

Write-Host "Iniciando $variantLabel con $scene" -ForegroundColor Green
& $godot --path $game $scene
exit $LASTEXITCODE
