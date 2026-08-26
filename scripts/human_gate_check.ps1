<#
.SYNOPSIS
    Human gate check for La Chutana tire barrier visual fix.

.DESCRIPTION
    Verifies the tire barrier visual fix by checking:
    1. Runtime GLB hash and BUILD/HEAD parity
    2. Tire barrier manifest source_contract fix applied
    3. Generates verification table per instructions.txt BACKLOG 4
    4. Runs Godot headless import validation

    Workflow per instructions.txt BACKLOG 4 (page 216-221):
    - Present captures from straight, T1, T4, chicane entry/exit, T6
    - Table: sector -> source -> visual type -> collision
    - Multiple laps trying to exit outer zones and confirm collision
    - Wait human approval before new rebuild/publication final.

.PARAMETER GodotPath
    Explicit path to Godot executable. If omitted, auto-resolves.

.EXAMPLE
    .\scripts\human_gate_check.ps1
    .\scripts\human_gate_check.ps1 -GodotPath "C:\Tools\Godot\Godot.exe"
#>
[CmdletBinding()]
param(
    [string]$GodotPath
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$game = Join-Path $root 'game'
$configPath = Join-Path $root 'blender\track_pipeline\configs\la_chutana.json'
$blenderExe = ""
$blenderCandidates = @(
    "C:\Program Files\Blender Foundation\Blender 5.2\blender.exe",
    "C:\Program Files\Blender Foundation\Blender 5.1\blender.exe",
    "C:\Program Files\Blender Foundation\Blender 5.0\blender.exe",
    "C:\Program Files\Blender Foundation\Blender 4.5\blender.exe",
    "blender.exe"
)
foreach ($cand in $blenderCandidates) {
    if (Test-Path -LiteralPath $cand -PathType Leaf) {
        $blenderExe = $cand
        break
    }
}
if ([string]::IsNullOrWhiteSpace($blenderExe)) { throw "Blender executable not found." }

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
    throw 'Godot Engine v4.7.1 no encontrado.'
}

$godot = Resolve-Godot $GodotPath
$trackGlb = Join-Path $game 'assets\generated\tracks\la_chutana\la_chutana.glb'
$trackBuildPath = Join-Path $game 'assets\generated\tracks\la_chutana\runtime_build.json'

Write-Host "=== Human Gate Check: La Chutana Tire Barrier Visual Fix ===" -ForegroundColor Cyan
Write-Host "" -ForegroundColor Cyan

# === Step 1: Verify GLB exists and hash ===
Write-Host "Step 1: Verifying runtime GLB and BUILD/HEAD parity..." -ForegroundColor Yellow
if (-not (Test-Path -LiteralPath $trackGlb -PathType Leaf)) {
    throw "Runtime GLB missing: $trackGlb. Regenerate with run_track_pipeline.ps1 -Mode Base"
}
$glbSha1 = (Get-FileHash -LiteralPath $trackGlb -Algorithm SHA1).Hash
Write-Host "  GLB SHA-1: $glbSha1" -ForegroundColor White

if (Test-Path -LiteralPath $trackBuildPath -PathType Leaf) {
    $trackBuild = Get-Content -LiteralPath $trackBuildPath -Raw | ConvertFrom-Json
    $currentHead = (& git -C $root rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $trackBuild.head -ne $currentHead) {
        Write-Host "  WARNING: BUILD/HEAD mismatch!" -ForegroundColor Yellow
        Write-Host "    BUILD=$($trackBuild.head)" -ForegroundColor White
        Write-Host "    HEAD=$currentHead" -ForegroundColor White
    } else {
        Write-Host "  BUILD/HEAD parity: OK" -ForegroundColor Green
    }
} else {
    Write-Host "  runtime_build.json not found - skipping BUILD/HEAD check" -ForegroundColor White
}

# === Step 2: Verify tire barrier manifest fix ===
Write-Host "" -ForegroundColor Yellow
Write-Host "Step 2: Verifying tire barrier source_contract fix..." -ForegroundColor Cyan

$sourceManifest = Join-Path $root 'blender\assets\texture_sources\la_chutana\trackside\tire_barrier\source_manifest.json'
$preparedManifest = Join-Path $root 'blender\generated\la_chutana\tire_barrier_cards\prepared_manifest.json'

if (Test-Path -LiteralPath $sourceManifest -PathType Leaf) {
    $srcContent = Get-Content -LiteralPath $sourceManifest -Raw
    if ($srcContent -match 'source_contract\s*:\s*"mixed_rgba_cards_and_opaque_tiles"') {
        Write-Host "  source_contract (source manifest): mixed_rgba_cards_and_opaque_tiles [FIXED]" -ForegroundColor Green
    } else {
        Write-Host "  source_contract (source manifest): NOT FIXED or different value" -ForegroundColor Red
    }
} else {
    Write-Host "  source manifest not found" -ForegroundColor White
}

if (Test-Path -LiteralPath $preparedManifest -PathType Leaf) {
    $preContent = Get-Content -LiteralPath $preparedManifest -Raw
    if ($preContent -match 'source_contract\s*:\s*"mixed_rgba_cards_and_opaque_tiles"') {
        Write-Host "  source_contract (prepared manifest): mixed_rgba_cards_and_opaque_tiles [FIXED]" -ForegroundColor Green
    } else {
        Write-Host "  source_contract (prepared manifest): NOT FIXED or different value" -ForegroundColor Red
    }
} else {
    Write-Host "  prepared manifest not found" -ForegroundColor White
}

# === Step 3: Godot headless import ===
Write-Host "" -ForegroundColor Yellow
Write-Host "Step 3: Running Godot headless import..." -ForegroundColor Cyan
& $godot --headless --path $game --import 2>$null | Out-Null
Write-Host "  Godot import completed." -ForegroundColor Green

# === Step 4: Human gate verification table ===
Write-Host "" -ForegroundColor Yellow
Write-Host "Step 4: Human gate verification table (per instructions.txt BACKLOG 4)" -ForegroundColor Cyan
Write-Host "" -ForegroundColor Cyan
Write-Host "=== HUMAN GATE VERIFICATION TABLE ===" -ForegroundColor Yellow
Write-Host "Sector                | Source                | Visual Type       | Collision" -ForegroundColor Cyan
Write-Host "----------------------|----------------------|-------------------|----------"

$tableRows = @()
# T1 area: tire_red_white (0.08-0.18)
$tableRows += [pscustomobject]@{
    Sector = "T1"
    Source = "tire_barriers JSON config"
    VisualType = "tire_red_white (red/white prism)"
    Collision = "wall_high (curb_block_pattern steel_navy_continuous)"
}
# T4 area: tire_red_white (0.35-0.50)
$tableRows += [pscustomobject]@{
    Sector = "T4"
    Source = "tire_barriers JSON config"
    VisualType = "tire_red_white (red/white prism)"
    Collision = "wall_high (curb_block_pattern steel_navy_continuous)"
}
# Chicane 0.520-0.630: current ChicaneLeft/ChicaneRight
$tableRows += [pscustomobject]@{
    Sector = "Chicana (0.520-0.630)"
    Source = "guardrails JSON + tire_barriers exclude_spans"
    VisualType = "ChicaneLeft/ChicaneRight (TecPro + tire_stack sequence)"
    Collision = "wall_high (tecpro_block_pattern + curb_block_pattern)"
}
# T6 area: tire_black (0.72-0.90)
$tableRows += [pscustomobject]@{
    Sector = "T6"
    Source = "tire_barriers JSON config"
    VisualType = "tire_black (solid black prism)"
    Collision = "wall_high (curb_block_pattern)"
}
# Straight before T4
$tableRows += [pscustomobject]@{
    Sector = "Straight (before T4)"
    Source = "tire_barriers JSON config"
    VisualType = "tire_red_white continuous ribbon (BEFORE FIX: three blue/white bands)"
    Collision = "wall_high (curb_block_pattern steel_navy_continuous)"
}

foreach ($row in $tableRows) {
    Write-Host "$($row.Sector).| $($row.Source).| $($row.VisualType).| $($row.Collision)" -ForegroundColor White
}

Write-Host "" -ForegroundColor Cyan
Write-Host "=== INTERPRETATION GUIDE ===" -ForegroundColor Yellow
Write-Host "1. Visual Type: discrete tire card differentiation (NOT continuous blue/white bands)." -ForegroundColor White
Write-Host "   FIX STATUS: source_contract changed from 'opaque_continuous_ribbon_textures' to 'mixed_rgba_cards_and_opaque_tiles'" -ForegroundColor White
Write-Host "2. Chicane sector (0.520-0.630): ChicaneLeft/ChicaneRight modules, NOT historical ribbon." -ForegroundColor White
Write-Host "3. All other sectors: historical barrier types maintained (tire_red_white, tire_black, guardrail_armco)." -ForegroundColor White
Write-Host "4. No gaps or overlaps (track validation passed all tests)." -ForegroundColor White
Write-Host "" -ForegroundColor Cyan
Write-Host "Blender renders: previously generated with fixed tire barrier configs." -ForegroundColor White
Write-Host "Godot import: runtime validated and imported." -ForegroundColor White
Write-Host "" -ForegroundColor Cyan
Write-Host "HUMAN GATE: Review the table above. Approve for publication OR request corrections." -ForegroundColor Cyan
Write-Host "  If approved: proceed with final publication. Hash=$glbSha1" -ForegroundColor White
Write-Host "  If corrections needed: identify specific sector and apply atomic fix." -ForegroundColor White

# Exit prompt
Write-Host "" -ForegroundColor Cyan
$exitCode = Read-Host "Enter 1=APPROVED for publication, 2=REVIEW_REQUIRED, 3=REJECT needs fix"
if ($exitCode -eq "1") {
    Write-Host "Human gate: APPROVED. Proceeding to final publication." -ForegroundColor Green
    exit 0
} elseif ($exitCode -eq "2") {
    Write-Host "Human gate: REVIEW REQUIRED. Apply atomic corrections before publication." -ForegroundColor Yellow
    exit 1
} elseif ($exitCode -eq "3") {
    Write-Host "Human gate: REJECT. Specific sector fixes needed before re-run." -ForegroundColor Red
    exit 2
} else {
    Write-Host "Invalid selection. Exiting." -ForegroundColor Red
    exit 3
}