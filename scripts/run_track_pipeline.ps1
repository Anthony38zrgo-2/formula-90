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
& $python (Join-Path $pipeline "apply_ground_cover_to_terrain.py") --config $config
if ($LASTEXITCODE -ne 0) { throw "Hybrid ground-cover terrain generation failed" }
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

& $python (Join-Path $pipeline "generate_environment.py") `
    --config $config `
    --trees-density $TreesDensity `
    --bushes-density $BushesDensity `
    --grass-density $GrassDensity `
    --buildings-density $BuildingsDensity `
    --seed $Seed
if ($LASTEXITCODE -ne 0) { throw "Environment placement failed" }
& $python (Join-Path $pipeline "validate_environment.py") --config $config
if ($LASTEXITCODE -ne 0) { throw "Environment validation failed" }
& $BlenderExe --background --python (Join-Path $pipeline "build_environment_blender.py") -- --config $config
if ($LASTEXITCODE -ne 0) { throw "Blender procedural environment build failed" }

Write-Host ""
Write-Host "Procedural racetrack generated and published with seed $Seed." -ForegroundColor Green
Write-Host "Biome: $($trackConfig.procedural_environment.biome.continent)/$($trackConfig.procedural_environment.biome.longitude)/$($trackConfig.procedural_environment.biome.altitude)" -ForegroundColor DarkGray
Write-Host "Texture Forge: $($trackConfig.materials.texture_forge_style)" -ForegroundColor DarkGray
Write-Host "Texture source: $TextureSource" -ForegroundColor DarkGray
Write-Host "Trees=$TreesDensity Bushes=$BushesDensity Grass=$GrassDensity Buildings=$BuildingsDensity" -ForegroundColor DarkGray
Write-Host "The previous track_environment.blend was backed up when present." -ForegroundColor DarkGray
