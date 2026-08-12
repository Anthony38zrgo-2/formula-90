[CmdletBinding()]
param(
    [switch]$UnitOnly,
    [switch]$SkipLaChutana,
    [switch]$SkipBlender,
    [switch]$SkipGodot,
    [string]$FixtureSource = "blender/track_pipeline/tests/fixtures/compile_track.svg",
    [string]$FixtureRegistry = "blender/track_pipeline/tests/fixtures/asset_registry_blender_test.json",
    [string]$BlenderExe = "C:\Program Files\Blender Foundation\Blender 5.2\blender.exe",
    [int]$BlenderTimeout = 1800,
    [switch]$Interactive,
    [string]$Track = "la_chutana",
    [switch]$OpenBrowser,
    [string]$TracksDir = "tracks",
    [int]$Port = 8412
)

# End-to-end test of the F90 SVG Track Authoring System.
#
# Runs every stage in sequence against temporary output roots so the real
# game runtime (game/assets/generated/) and the legacy raster pipeline are
# never touched:
#
#   1. unittest suite (authoring, compile, revisions, gateway, importer, godot)
#   2. authoring editor server smoke (start /api/validate / kill on port 8412)
#   3. SVG compile of the fixture (canonical + normalized + manifest)
#   4. [default] La Chutana importer -> canonical SVG with exact parity counts
#   5. [default] end-to-end build: Blender headless + atomic activation +
#      Godot-load gate, against a temporary runtime root
#
# -Interactive opens the editor for manual use instead of running the automated
# stages: it serves the tip revision of -Track (requires a revision already
# bootstrapped under -TracksDir, e.g. via bootstrap_la_chutana_track.py) and
# keeps the server running until Ctrl+C. With -OpenBrowser it opens the URL in
# the default browser.
#
# Switches:
#   -UnitOnly            run only the unittest suite, then exit
#   -SkipLaChutana       skip the La Chutana importer step
#   -SkipBlender         skip the Blender build step
#   -SkipGodot           skip the Godot-load gate (build still validates)
#   -FixtureSource       SVG used for the compile/build steps
#   -FixtureRegistry     registry used for the compile/build steps
#   -Interactive         open the editor at the tip revision of -Track (manual)
#   -Track               track id served in -Interactive mode
#   -OpenBrowser         open the editor URL in the default browser
#   -TracksDir           tracks revision root (default: tracks)
#   -Port                server port for -Interactive mode
#
# Usage:
#   .\scripts\run-circuit-editor.ps1
#   .\scripts\run-circuit-editor.ps1 -UnitOnly
#   .\scripts\run-circuit-editor.ps1 -SkipBlender -SkipGodot
#   .\scripts\run-circuit-editor.ps1 -Interactive -Track la_chutana -OpenBrowser

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$pipeline = Join-Path $repo "blender\track_pipeline"
$python = Join-Path $pipeline ".venv\Scripts\python.exe"
$work = Join-Path $env:TEMP "f90_circuit_editor"
$serverUrl = "http://127.0.0.1:$Port"

if (-not (Test-Path $python)) { throw "Pipeline venv missing: $python" }

# --- Interactive mode: serve the tip revision for manual editing ----------
if ($Interactive) {
    if (-not (Test-Path (Join-Path $repo $TracksDir))) {
        throw "Tracks dir not found: $TracksDir. Bootstrap the track first (bootstrap_la_chutana_track.py)."
    }
    $url = "http://127.0.0.1:$Port/"
    Write-Host "F90 Circuit Editor - interactive ($Track)" -ForegroundColor Magenta
    Write-Host "  track dir : $(Join-Path $repo $TracksDir)"
    Write-Host ("  url       : {0}" -f $url)
    Write-Host "  Stop      : Ctrl+C"
    if ($OpenBrowser) {
        Start-Process $url
    }
    & $python (Join-Path $pipeline "authoring\server.py") `
        --host 127.0.0.1 `
        --port $Port `
        --track $Track `
        --tracks-dir (Join-Path $repo $TracksDir) `
        --git-repo-root $repo
    exit $LASTEXITCODE
}

function Invoke-Step {
    param(
        [string]$Name,
        [ScriptBlock]$Body,
        [switch]$Optional
    )
    Write-Host ""
    Write-Host ("===== {0} =====" -f $Name) -ForegroundColor Cyan
    try {
        & $Body
        if ($LASTEXITCODE -ne 0) { throw "$Name failed with exit code $LASTEXITCODE" }
        Write-Host ("[PASS] {0}" -f $Name) -ForegroundColor Green
    }
    catch {
        if ($Optional) {
            Write-Warning "[SKIP] $Name : $($_.Exception.Message)"
        } else {
            Write-Host ("[FAIL] {0}" -f $Name) -ForegroundColor Red
            throw
        }
    }
}

# Fresh temporary work root (never touches the real game runtime).
New-Item -ItemType Directory -Force -Path $work | Out-Null
Get-ChildItem -Path $work -Recurse -ErrorAction SilentlyContinue | Remove-Item -Force -Recurse -ErrorAction SilentlyContinue

Write-Host "F90 Circuit Editor (Track Authoring) - end-to-end" -ForegroundColor Magenta
Write-Host "  repo : $repo"
Write-Host ("  work : {0}" -f $work)

# --- 1. unittest suite ----------------------------------------------------
Invoke-Step "Unit tests (expect 169 OK)" {
    & $python -m unittest discover -s (Join-Path $pipeline "tests")
}

if ($UnitOnly) {
    Write-Host ""
    Write-Host "Unit tests only. Skipped editor/build stages (-UnitOnly)." -ForegroundColor Yellow
    exit 0
}

# --- 2. authoring editor server smoke ------------------------------------
Invoke-Step "Authoring editor server smoke (port 8412)" {
    $server = Start-Process -FilePath $python -ArgumentList @(
        (Join-Path $pipeline "authoring\server.py"),
        "--host", "127.0.0.1", "--port", "8412"
    ) -PassThru -NoNewWindow
    try {
        $ok = $false
        for ($i = 0; $i -lt 40; $i++) {
            Start-Sleep -Milliseconds 250
            try {
                $r = Invoke-WebRequest -Uri "$serverUrl/api/validate" -UseBasicParsing -TimeoutSec 3
                if ($r.StatusCode -eq 200 -and ($r.Content | ConvertFrom-Json).ok) { $ok = $true; break }
            } catch { }
        }
        if (-not $ok) { throw "editor server did not report ok=true" }
        $assets = Invoke-WebRequest -Uri "$serverUrl/api/assets" -UseBasicParsing -TimeoutSec 5
        if ($assets.StatusCode -ne 200) { throw "GET /api/assets failed" }
        Write-Host "  GET /api/validate ok=true; GET /api/assets $($assets.StatusCode)"
    }
    finally {
        if ($server -and -not $server.HasExited) { Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue }
    }
}

# --- 3. SVG compile of the fixture ---------------------------------------
$fixtureOut = Join-Path $work "compile"
$compileManifest = Join-Path $fixtureOut "manifest.json"
Invoke-Step "SVG compile (fixture -> canonical + normalized + manifest)" {
    & $python (Join-Path $pipeline "compile_svg_track.py") `
        --source (Join-Path $repo $FixtureSource) `
        --registry (Join-Path $repo $FixtureRegistry) `
        --output-dir $fixtureOut
}

# --- 4. La Chutana importer (optional) ------------------------------------
$lacOut = Join-Path $work "la_chutana"
if (-not $SkipLaChutana) {
    Invoke-Step "La Chutana importer (parity counts + collision validation)" {
        & $python (Join-Path $pipeline "import_la_chutana.py") `
            --output-dir $lacOut `
            --check
    }
} else {
    Write-Host ""
    Write-Host "Skipped La Chutana importer (-SkipLaChutana)." -ForegroundColor Yellow
}

# --- 5. end-to-end build + activate + godot gate --------------------------
if (-not $SkipBlender) {
    $buildOut = Join-Path $work "builds"
    $runtimeOut = Join-Path $work "runtime"
    New-Item -ItemType Directory -Force -Path $runtimeOut | Out-Null

    $godotArg = @()
    if (-not $SkipGodot) { $godotArg = @("--godot") }

    Invoke-Step "End-to-end build: Blender + atomic activation + Godot-load gate" {
        & $python (Join-Path $pipeline "build_svg_track.py") `
            --source (Join-Path $repo $FixtureSource) `
            --registry (Join-Path $repo $FixtureRegistry) `
            --output-root $buildOut `
            --approved `
            --activate `
            --runtime-root $runtimeOut `
            --blender-exe $BlenderExe `
            --blender-timeout $BlenderTimeout `
            @godotArg
    }

    $envGlb = Get-ChildItem -Path $runtimeOut -Filter "*.glb" -ErrorAction SilentlyContinue | Where-Object { $_.Name -notmatch "vegetation" } | Select-Object -First 1
    $vegGlb = Get-ChildItem -Path $runtimeOut -Filter "*vegetation.glb" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($envGlb -and $vegGlb) {
        Invoke-Step "Godot-load direct validation of activated pair" {
            & $python (Join-Path $pipeline "validate_godot_load.py") `
                --env-glb $envGlb.FullName `
                --veg-glb $vegGlb.FullName
        } -Optional
    }
} else {
    Write-Host ""
    Write-Host "Skipped Blender build (-SkipBlender)." -ForegroundColor Yellow
}

Write-Host ""
Write-Host ("All stages passed. Temp outputs: {0}" -f $work) -ForegroundColor Green
exit 0
