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
    [int]$Seed = 1995,
    [string]$BlenderExe = ""
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$pipeline = Join-Path $repo "blender\track_pipeline"
$python = Join-Path $pipeline ".venv\Scripts\python.exe"
$config = Join-Path $pipeline "configs\$Track.json"
$catalogRel = "blender/generated/$Track/asset_catalog.json"
$catalog = Join-Path $repo ($catalogRel -replace '/', '\')

if (-not (Test-Path $python)) {
    throw "Pipeline venv missing. Run .\scripts\setup_track_pipeline.ps1 first."
}
if (-not (Test-Path $config)) {
    throw "Track config missing: $config"
}

if ([string]::IsNullOrWhiteSpace($BlenderExe)) {
    $candidates = @(
        "C:\Program Files\Blender Foundation\Blender 5.2\blender.exe",
        "C:\Program Files\Blender Foundation\Blender 5.1\blender.exe",
        "C:\Program Files\Blender Foundation\Blender 5.0\blender.exe",
        "C:\Program Files\Blender Foundation\Blender 4.5\blender.exe",
        "blender"
    )
    foreach ($candidate in $candidates) {
        if ((Test-Path $candidate) -or (Get-Command $candidate -ErrorAction SilentlyContinue)) {
            $BlenderExe = $candidate
            break
        }
    }
}
if ([string]::IsNullOrWhiteSpace($BlenderExe)) {
    throw "Blender executable not found. Pass -BlenderExe explicitly."
}

& $python (Join-Path $pipeline "prepare_track.py") --config $config
if ($LASTEXITCODE -ne 0) { throw "prepare_track.py failed" }

& $python (Join-Path $pipeline "validate_track.py") --config $config
if ($LASTEXITCODE -ne 0) { throw "validate_track.py failed" }

if ($Mode -eq "Base") {
    & $BlenderExe --background --python (Join-Path $pipeline "build_track_blender.py") -- --config $config
    if ($LASTEXITCODE -ne 0) { throw "Blender base track build failed" }
    Write-Host "Base track generated. HUMAN VALIDATION REQUIRED before Procedural mode." -ForegroundColor Green
    exit 0
}

$baseBlend = Join-Path $repo "blender\generated\$Track\track_base.blend"
if (-not (Test-Path $baseBlend)) {
    throw "Base track not found. Run -Mode Base and validate it in Godot first."
}

& $BlenderExe --background --python (Join-Path $pipeline "inspect_assets_blender.py") -- --config $config --output $catalogRel
if ($LASTEXITCODE -ne 0) { throw "Asset inspection failed" }

& $python (Join-Path $pipeline "generate_environment.py") `
    --config $config --catalog $catalogRel `
    --trees-density $TreesDensity `
    --bushes-density $BushesDensity `
    --grass-density $GrassDensity `
    --seed $Seed
if ($LASTEXITCODE -ne 0) { throw "Environment placement failed" }

& $python (Join-Path $pipeline "validate_environment.py") --config $config --catalog $catalogRel
if ($LASTEXITCODE -ne 0) { throw "Environment validation failed" }

& $BlenderExe --background --python (Join-Path $pipeline "build_environment_blender.py") -- --config $config --catalog $catalogRel
if ($LASTEXITCODE -ne 0) { throw "Blender procedural environment build failed" }

Write-Host "Procedural racetrack generated with seed $Seed." -ForegroundColor Green
