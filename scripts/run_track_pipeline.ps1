param(
    [ValidateSet("Base","Procedural")]
    [string]$Mode = "Base",
    [string]$Track = "la_chutana",
    [ValidateSet("none","very_low","low","medium","high")]
    [string]$TreesDensity = "low",
    [ValidateSet("none","very_low","low","medium","high")]
    [string]$BushesDensity = "low",
    [ValidateSet("none","very_low","low","medium","high")]
    [string]$GrassDensity = "medium",
    [ValidateSet("none","very_low","low","medium","high")]
    [string]$BuildingsDensity = "very_low",
    [int]$Seed = 1995,
    [ValidateSet("Curated","Procedural")]
    [string]$TextureSource = "Curated",
    [ValidateSet(1,2)]
    [int]$VegetationPass = 1,
    [switch]$SkipSigns,
    [switch]$SkipPeople,
    [switch]$SkipTrees,
    [switch]$SkipBushes,
    [switch]$SkipGrass,
    [ValidateSet("None", "Vegetation", "SafetyBarrier", "Guardrail", "Signs", "People", "Buildings")]
    [string]$ReviewMode = "None",
    [string]$BlenderExe = ""
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$pipeline = Join-Path $repo "blender\track_pipeline"
$python = Join-Path $pipeline ".venv\Scripts\python.exe"
$config = Join-Path $pipeline "configs\$Track.json"

if (-not (Test-Path $python)) { throw "Pipeline venv missing. Run .\scripts\setup_track_pipeline.ps1 first." }
if (-not (Test-Path $config)) { throw "Track config missing: $config" }
$trackConfig = Get-Content -Path $config -Raw | ConvertFrom-Json

$barrierPythonCommand = Get-Command python -ErrorAction SilentlyContinue
if (-not $barrierPythonCommand) { throw "System Python missing; required for validated barrier asset generation." }
$barrierPython = $barrierPythonCommand.Source
$barrierRecipe = Join-Path $repo $trackConfig.safety_barriers.recipe_manifest
$barrierGenerator = Join-Path $repo $trackConfig.safety_barriers.generator
& $barrierPython $barrierGenerator --recipes $barrierRecipe --repo $repo
if ($LASTEXITCODE -ne 0) { throw "Barrier asset library generation failed" }
& $barrierPython -m unittest game.resources.environment.tests.test_barrier_3d_library -v
if ($LASTEXITCODE -ne 0) { throw "Barrier asset library validation failed" }

$buildingGenerator = Join-Path $repo $trackConfig.procedural_environment.fake_buildings.generator
$buildingSources = Join-Path $repo $trackConfig.procedural_environment.fake_buildings.source_manifest
& $barrierPython $buildingGenerator --sources $buildingSources --repo $repo
if ($LASTEXITCODE -ne 0) { throw "Building asset manifest generation failed" }
& $barrierPython -m unittest game.resources.environment.tests.test_building_asset_library -v
if ($LASTEXITCODE -ne 0) { throw "Building asset library validation failed" }

if ([string]::IsNullOrWhiteSpace($BlenderExe)) {
    $candidates = @(
        "C:\Program Files\Blender Foundation\Blender 5.2\blender.exe",
        "C:\Program Files\Blender Foundation\Blender 5.1\blender.exe",
        "C:\Program Files\Blender Foundation\Blender 5.0\blender.exe",
        "C:\Program Files\Blender Foundation\Blender 4.5\blender.exe",
        "blender"
    )
    foreach ($candidate in $candidates) {
        if ((Test-Path $candidate) -or (Get-Command $candidate -ErrorAction SilentlyContinue)) { $BlenderExe = $candidate; break }
    }
}
if ([string]::IsNullOrWhiteSpace($BlenderExe)) { throw "Blender executable not found. Pass -BlenderExe explicitly." }

if ($trackConfig.tire_barriers.procedural) {
    $barrierSource = Join-Path $repo "blender\assets\texture_sources\$Track\trackside\tire_barrier\source_manifest.json"
    $barrierOutput = Join-Path $repo "blender\generated\$Track\tire_barrier_cards"
    & $python (Join-Path $pipeline "prepare_tire_barrier_cards.py") --manifest $barrierSource --output-dir $barrierOutput
    if ($LASTEXITCODE -ne 0) { throw "Textured barrier preparation failed" }
    & $python (Join-Path $pipeline "validate_tire_barrier_cards.py") `
        --source-manifest $barrierSource `
        --prepared-manifest (Join-Path $barrierOutput "prepared_manifest.json")
    if ($LASTEXITCODE -ne 0) { throw "Textured barrier validation failed" }
}

& $python (Join-Path $pipeline "prepare_track.py") --config $config
if ($LASTEXITCODE -ne 0) { throw "prepare_track.py failed" }
& $python (Join-Path $pipeline "validate_track.py") --config $config
if ($LASTEXITCODE -ne 0) { throw "validate_track.py failed" }
& $python (Join-Path $pipeline "document_vegetation_palette.py") --config $config
if ($LASTEXITCODE -ne 0) { throw "Vegetation documentation generation failed" }

$textureRoot = Join-Path $repo (Join-Path $trackConfig.generated_dir "textures")
$activeManifestPath = Join-Path $textureRoot "active_manifest.json"
if ($TextureSource -eq "Procedural") {
    & $python (Join-Path $pipeline "generate_procedural_textures.py") --config $config --seed $Seed
    if ($LASTEXITCODE -ne 0) { throw "Procedural texture generation failed" }
    Write-Host "Texture source: freshly generated procedural bank" -ForegroundColor DarkGray
} else {
    if (-not (Test-Path $activeManifestPath)) {
        throw "Curated texture bank manifest missing: $activeManifestPath"
    }
    $activeManifest = Get-Content -Path $activeManifestPath -Raw | ConvertFrom-Json
    $activeSeed = [int]$activeManifest.forge.seed
    if ($activeSeed -ne $Seed) {
        throw "Curated texture bank seed=$activeSeed does not match requested seed=$Seed. Pass -TextureSource Procedural or use the matching seed."
    }
    Write-Host "Texture source: existing curated bank ($textureRoot)" -ForegroundColor DarkGray
    & $python (Join-Path $pipeline "rebuild_curated_vegetation.py") `
        --config $config `
        --pass-index $VegetationPass
    if ($LASTEXITCODE -ne 0) { throw "Source-backed vegetation rebuild failed" }
    & $python (Join-Path $pipeline "recut_vegetation_textures.py") `
        --config $config `
        --pass-index $VegetationPass `
        --skip-source-backed
    if ($LASTEXITCODE -ne 0) { throw "Vegetation bottom-anchor validation failed" }
    & $python (Join-Path $pipeline "analyze_vegetation_textures.py") `
        --config $config `
        --strict
    if ($LASTEXITCODE -ne 0) { throw "Vegetation analysis failed" }
    Write-Host "Vegetation gate: pre-cut RGBA source rebuild + read-only bottom-anchor validation + per-asset alpha validation pass $VegetationPass" -ForegroundColor DarkGray
}
& $python (Join-Path $pipeline "apply_asphalt_texture.py") --config $config
if ($LASTEXITCODE -ne 0) { throw "Curated seamless asphalt generation failed" }
& $python (Join-Path $pipeline "generate_environment.py") `
    --config $config `
    --trees-density $TreesDensity `
    --bushes-density $BushesDensity `
    --grass-density $GrassDensity `
    --buildings-density $BuildingsDensity `
    --seed $Seed
if ($LASTEXITCODE -ne 0) { throw "Semantic environment placement failed" }
& $python (Join-Path $pipeline "apply_ground_cover_to_terrain.py") --config $config
if ($LASTEXITCODE -ne 0) { throw "Semantic ground-cover terrain generation failed" }
& $python (Join-Path $pipeline "validate_texture_forge.py") --config $config
if ($LASTEXITCODE -ne 0) { throw "Texture Forge validation failed" }

if ($Mode -eq "Base") {
    & $BlenderExe --background --python (Join-Path $pipeline "build_track_blender.py") -- --config $config
    if ($LASTEXITCODE -ne 0) { throw "Blender base track build failed" }
    & $BlenderExe --background --python (Join-Path $pipeline "validate_curbs_blender.py") -- --config $config
    if ($LASTEXITCODE -ne 0) { throw "Generated curb profile validation failed" }
    Write-Host ""
    Write-Host "Base track generated and published to the canonical runtime GLB." -ForegroundColor Green
    Write-Host "Texture Forge: $($trackConfig.materials.texture_forge_style), seed=$Seed" -ForegroundColor DarkGray
    Write-Host "Texture source: $TextureSource" -ForegroundColor DarkGray
    Write-Host "The previous track_base.blend was backed up when present." -ForegroundColor DarkGray
    Write-Host "HUMAN VALIDATION REQUIRED before Procedural mode." -ForegroundColor Yellow
    exit 0
}

$baseBlend = Join-Path $repo "blender\generated\$Track\track_base.blend"
if (-not (Test-Path $baseBlend)) { throw "Base track not found. Run -Mode Base and validate it in Godot first." }

& $python (Join-Path $pipeline "validate_environment.py") --config $config
if ($LASTEXITCODE -ne 0) { throw "Environment validation failed" }

$blenderArgs = @("--config", $config)
if ($ReviewMode -eq "Vegetation") { $blenderArgs += "--vegetation-review" }
elseif ($ReviewMode -eq "SafetyBarrier") { $blenderArgs += "--safety-barrier-review" }
elseif ($ReviewMode -eq "Guardrail") { $blenderArgs += "--guardrail-review" }
elseif ($ReviewMode -eq "Signs") { $blenderArgs += "--signs-review" }
elseif ($ReviewMode -eq "People") { $blenderArgs += "--people-review" }
elseif ($ReviewMode -eq "Buildings") { $blenderArgs += "--buildings-review" }

& $BlenderExe --background --python (Join-Path $pipeline "build_environment_blender.py") -- $blenderArgs
if ($LASTEXITCODE -ne 0) { throw "Blender procedural environment build failed" }

if ($ReviewMode -ne "Vegetation") {
    $builtTrackGlb = if ($ReviewMode -eq "None") {
        Join-Path $repo "game\assets\generated\tracks\$Track\${Track}_environment.glb"
    } else {
        Join-Path $repo "game\assets\generated\tracks\$Track\${Track}_$($ReviewMode.ToLowerInvariant().Replace('safetybarrier','safety_barrier'))_review.glb"
    }
    $assetManifestForValidation = Join-Path $repo $trackConfig.safety_barriers.asset_library_manifest
    $barrierReport = Join-Path $repo "blender\generated\$Track\review\safety_barrier_report.json"
    & $python (Join-Path $pipeline "validate_barrier_asset_integration.py") `
        --glb $builtTrackGlb --asset-manifest $assetManifestForValidation --report $barrierReport
    if ($LASTEXITCODE -ne 0) { throw "Barrier asset integration validation failed" }
    $buildingAssetManifestForValidation = Join-Path $repo $trackConfig.procedural_environment.fake_buildings.asset_manifest
    $buildingReport = Join-Path $repo "blender\generated\$Track\review\building_report.json"
    & $python (Join-Path $pipeline "validate_building_asset_integration.py") `
        --glb $builtTrackGlb --asset-manifest $buildingAssetManifestForValidation --report $buildingReport
    if ($LASTEXITCODE -ne 0) { throw "Building asset integration validation failed" }
    & $python (Join-Path $pipeline "validate_trackside_asset_integration.py") `
        --glb $builtTrackGlb --placements (Join-Path $repo "blender\generated\$Track\placements.json")
    if ($LASTEXITCODE -ne 0) { throw "Trackside asset integration validation failed" }
}

if ($ReviewMode -ne "None") {
    Write-Host "Review build validated; runtime and runtime_build.json were not published." -ForegroundColor Yellow
    exit 0
}

$runtimeTrack = Join-Path $repo "game\assets\generated\tracks\$Track\$Track.glb"
$runtimeBuild = Join-Path $repo "game\assets\generated\tracks\$Track\runtime_build.json"
$barrierManifest = Join-Path $repo $trackConfig.safety_barriers.manifest
$barrierAssetManifest = Join-Path $repo $trackConfig.safety_barriers.asset_library_manifest
$barrierConstructionManifest = Join-Path $repo $trackConfig.safety_barriers.construction_manifest
$barrierRecipeManifest = Join-Path $repo $trackConfig.safety_barriers.recipe_manifest
$barrierPaletteManifest = Join-Path $repo $trackConfig.safety_barriers.palette_manifest
$barrierGenerator = Join-Path $repo $trackConfig.safety_barriers.generator
$environmentBuilder = Join-Path $pipeline "build_environment_blender.py"
$buildingAssetManifest = Join-Path $repo $trackConfig.procedural_environment.fake_buildings.asset_manifest
$buildingSourceManifest = Join-Path $repo $trackConfig.procedural_environment.fake_buildings.source_manifest
$buildingConstructionManifest = Join-Path $repo $trackConfig.procedural_environment.fake_buildings.construction_manifest
$buildingGenerator = Join-Path $repo $trackConfig.procedural_environment.fake_buildings.generator
$buildingIntegrationValidator = Join-Path $pipeline "validate_building_asset_integration.py"
$tracksideIntegrationValidator = Join-Path $pipeline "validate_trackside_asset_integration.py"
if (-not (Test-Path -LiteralPath $barrierAssetManifest -PathType Leaf)) {
    throw "Barrier asset manifest missing: $barrierAssetManifest"
}
if (-not (Test-Path -LiteralPath $runtimeTrack -PathType Leaf)) {
    throw "Published runtime track missing: $runtimeTrack"
}
$head = (& git -C $repo rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $head -notmatch '^[0-9a-f]{40}$') {
    throw "Unable to resolve repository HEAD for runtime BUILD parity."
}
$buildRecord = [ordered]@{
    schema_version = 3
    track = $Track
    head = $head
    glb_sha256 = (Get-FileHash -LiteralPath $runtimeTrack -Algorithm SHA256).Hash.ToLowerInvariant()
    config_sha256 = (Get-FileHash -LiteralPath $config -Algorithm SHA256).Hash.ToLowerInvariant()
    safety_barrier_manifest_sha256 = (Get-FileHash -LiteralPath $barrierManifest -Algorithm SHA256).Hash.ToLowerInvariant()
    barrier_asset_manifest_sha256 = (Get-FileHash -LiteralPath $barrierAssetManifest -Algorithm SHA256).Hash.ToLowerInvariant()
    barrier_construction_manifest_sha256 = (Get-FileHash -LiteralPath $barrierConstructionManifest -Algorithm SHA256).Hash.ToLowerInvariant()
    barrier_recipe_manifest_sha256 = (Get-FileHash -LiteralPath $barrierRecipeManifest -Algorithm SHA256).Hash.ToLowerInvariant()
    barrier_palette_manifest_sha256 = (Get-FileHash -LiteralPath $barrierPaletteManifest -Algorithm SHA256).Hash.ToLowerInvariant()
    barrier_generator_sha256 = (Get-FileHash -LiteralPath $barrierGenerator -Algorithm SHA256).Hash.ToLowerInvariant()
    environment_builder_sha256 = (Get-FileHash -LiteralPath $environmentBuilder -Algorithm SHA256).Hash.ToLowerInvariant()
    building_asset_manifest_sha256 = (Get-FileHash -LiteralPath $buildingAssetManifest -Algorithm SHA256).Hash.ToLowerInvariant()
    building_source_manifest_sha256 = (Get-FileHash -LiteralPath $buildingSourceManifest -Algorithm SHA256).Hash.ToLowerInvariant()
    building_construction_manifest_sha256 = (Get-FileHash -LiteralPath $buildingConstructionManifest -Algorithm SHA256).Hash.ToLowerInvariant()
    building_generator_sha256 = (Get-FileHash -LiteralPath $buildingGenerator -Algorithm SHA256).Hash.ToLowerInvariant()
    building_integration_validator_sha256 = (Get-FileHash -LiteralPath $buildingIntegrationValidator -Algorithm SHA256).Hash.ToLowerInvariant()
    trackside_integration_validator_sha256 = (Get-FileHash -LiteralPath $tracksideIntegrationValidator -Algorithm SHA256).Hash.ToLowerInvariant()
    seed = $Seed
    generated_utc = [DateTime]::UtcNow.ToString('o')
}
$buildRecord | ConvertTo-Json | Set-Content -LiteralPath $runtimeBuild -Encoding utf8
Write-Host "Runtime BUILD manifest: $runtimeBuild" -ForegroundColor DarkGray

Write-Host ""
Write-Host "Procedural racetrack generated and published with seed $Seed." -ForegroundColor Green
Write-Host "Biome: $($trackConfig.procedural_environment.biome.continent)/$($trackConfig.procedural_environment.biome.longitude)/$($trackConfig.procedural_environment.biome.altitude)" -ForegroundColor DarkGray
Write-Host "Texture Forge: $($trackConfig.materials.texture_forge_style)" -ForegroundColor DarkGray
Write-Host "Texture source: $TextureSource" -ForegroundColor DarkGray
Write-Host "Trees=$TreesDensity Bushes=$BushesDensity Grass=$GrassDensity Buildings=$BuildingsDensity" -ForegroundColor DarkGray
Write-Host "The previous track_environment.blend was backed up when present." -ForegroundColor DarkGray
