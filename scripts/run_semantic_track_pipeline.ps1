[CmdletBinding()]
param(
    [string]$Track = "la_chutana",
    [switch]$Bootstrap,
    [switch]$Publish,
    [string]$BlenderExe = "C:\Program Files\Blender Foundation\Blender 5.2\blender.exe"
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$pipeline = Join-Path $repo "blender\track_pipeline"
$python = Join-Path $pipeline ".venv\Scripts\python.exe"
$layout = Join-Path $pipeline "layouts\$Track\layout_config.json"
$rawConfig = Join-Path $pipeline "configs\${Track}_raw.json"
$v2SourceManifest = Join-Path $repo "blender\vegetation_v2_upload_bundle\source_manifest.json"
$v2Output = Join-Path $repo "blender\generated\$Track\raw_vegetation\assets_v2"
$v2PreparedManifest = Join-Path $v2Output "prepared_manifest.json"
$v2AssetManifest = Join-Path $v2Output "asset_manifest.json"

if (-not (Test-Path $python)) { throw "Pipeline venv missing: $python" }
if (-not (Test-Path $layout)) { throw "Semantic layout config missing: $layout" }
if (-not (Test-Path $rawConfig)) { throw "Raw config missing: $rawConfig" }
if (-not (Test-Path $BlenderExe)) { throw "Blender missing: $BlenderExe" }
if (-not (Test-Path $v2SourceManifest)) { throw "Vegetation v2 source manifest missing: $v2SourceManifest" }

& $python (Join-Path $pipeline "prepare_raw_vegetation_v2.py") --manifest $v2SourceManifest --output-dir $v2Output
if ($LASTEXITCODE -ne 0) { throw "Vegetation v2 source preparation failed" }

& $BlenderExe --background --python (Join-Path $pipeline "build_raw_vegetation_assets_blender.py") -- --prepared-manifest $v2PreparedManifest --repo $repo
if ($LASTEXITCODE -ne 0) { throw "Vegetation v2 reusable GLB build failed" }

& $python (Join-Path $pipeline "validate_raw_vegetation_v2.py") --source-manifest $v2SourceManifest --asset-manifest $v2AssetManifest
if ($LASTEXITCODE -ne 0) { throw "Vegetation v2 asset validation failed" }

if ($Bootstrap) {
    Write-Warning "Bootstrap recreates the semantic PNGs and overwrites manual map edits."
    & $python (Join-Path $pipeline "bootstrap_semantic_layout.py") --layout-config $layout
    if ($LASTEXITCODE -ne 0) { throw "Semantic layout bootstrap failed" }
}

& $python (Join-Path $pipeline "compile_semantic_layout.py") --layout-config $layout
if ($LASTEXITCODE -ne 0) { throw "Semantic layout compilation failed" }

& $python (Join-Path $pipeline "validate_semantic_layout.py") --layout-config $layout --raw-config $rawConfig
if ($LASTEXITCODE -ne 0) { throw "Semantic layout validation failed; Blender was not executed" }

& $BlenderExe --background --python (Join-Path $pipeline "build_raw_vegetation_blender.py") -- --config $rawConfig
if ($LASTEXITCODE -ne 0) { throw "Semantic Blender build failed" }

$rawBlend = Join-Path $repo "blender\generated\$Track\raw_vegetation\${Track}_raw_environment.blend"
$compiledLayout = Join-Path $repo "blender\generated\$Track\semantic_layout\compiled_layout.json"
& $BlenderExe --background --python (Join-Path $pipeline "validate_raw_vegetation_blender.py") -- --blend $rawBlend --compiled $compiledLayout
if ($LASTEXITCODE -ne 0) { throw "Raw vegetation Blender validation failed; runtime was not published" }

if ($Publish) {
    $source = Join-Path $repo "blender\generated\$Track\raw_vegetation\${Track}_raw_environment.glb"
    $runtime = Join-Path $repo "game\assets\generated\tracks\$Track\$Track.glb"
    $backupDir = Join-Path $repo "blender\generated\$Track\raw_vegetation\backups"
    New-Item -ItemType Directory -Path $backupDir -Force | Out-Null
    if (Test-Path $runtime) {
        $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
        Copy-Item -LiteralPath $runtime -Destination (Join-Path $backupDir "${Track}_runtime_before_semantic_$stamp.glb") -Force
    }
    Copy-Item -LiteralPath $source -Destination $runtime -Force
    $sourceHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $source).Hash
    $runtimeHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $runtime).Hash
    if ($sourceHash -ne $runtimeHash) { throw "Published runtime hash does not match semantic GLB" }
    Write-Host "Published semantic runtime: $runtime" -ForegroundColor Green
}

Write-Host "Semantic layout pipeline completed for $Track" -ForegroundColor Green
