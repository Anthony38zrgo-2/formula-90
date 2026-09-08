[CmdletBinding()]
param(
    [string]$Output = "reports/audio-v10/v10-007-011/reference-manifest.json"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location $repoRoot
$outputPath = Join-Path $repoRoot $Output
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outputPath) | Out-Null

function Get-Sha256([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-Relative([string]$Path) {
    [IO.Path]::GetRelativePath($repoRoot, (Resolve-Path -LiteralPath $Path).Path)
}

function Invoke-Git([string[]]$Arguments) {
    $result = & git @Arguments 2>$null
    if ($LASTEXITCODE -ne 0) { return $null }
    (($result -join "`n").Trim())
}

function Get-HashMap([string[]]$Paths) {
    $map = [ordered]@{}
    foreach ($path in $Paths) {
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            $map[(Get-Relative $path)] = Get-Sha256 $path
        }
    }
    $map
}

$referenceRoot = Join-Path $repoRoot "reports/audio-v10/v10-007-011/reference"
$referenceFiles = @()
if (Test-Path -LiteralPath $referenceRoot) {
    $referenceFiles = @(Get-ChildItem -LiteralPath $referenceRoot -Recurse -File |
        Where-Object { $_.Name -notlike "*.analysis.json" } |
        ForEach-Object { $_.FullName })
}

$sourceFiles = @(
    "game/crates/vehicle-audio-engine/src/mixer.rs",
    "game/crates/vehicle-audio-engine/src/config.rs",
    "game/crates/formula90-core/src/frame.rs",
    "game/crates/formula90-core/src/audio.rs",
    "game/crates/formula90-core/src/bin/v10_physics_transient_capture.rs",
    "game/crates/v10-engine-synth/src/engine.rs",
    "game/crates/v10-engine-synth/src/runtime.rs",
    "game/crates/v10-engine-synth/src/thermodynamics/combustion.rs",
    "game/crates/v10-engine-synth/src/bin/v10_boundary_regression.rs",
    "game/crates/vehicle-audio-engine/src/bin/v10_provenance_probe.rs",
    "scripts/audio/render_v10_007_011_reference.ps1",
    "scripts/audio/analyze_v10_007_011_reference.py",
    "scripts/audio/write_v10_007_011_reference_manifest.ps1"
)
$configFiles = @(
    "game/sounds/sound_mixer_config.json",
    "game/data/vehicles/f1_2026_2008/f1_2026_2008_physics.json",
    "game/audio/v10_gf509/package.json",
    "game/audio/v10_gf509/manifest.json",
    "game/sounds/banks/v10_vehicle/bank_manifest.json"
)
$evidenceFiles = @(
    "reports/audio-v10/v10-007-011/boundary-regression.json",
    "reports/audio-v10/v10-007-011/provenance-probes.json",
    "reports/audio-v10/v10-007-011/physics-transient-capture.csv",
    "reports/audio-v10/v10-007-011/physics-transient-summary.json",
    "reports/audio-v10/v10-007-011/reference-measurements.json",
    "reports/audio-v10/v10-001-006/v10_5000_to_1800_lift_coast_10s.wav",
    "reports/audio-v10/v10-001-006/v10_5000_to_18000_then_lift_coast_10s.wav",
    "reports/audio-v10/v10-001-006/provenance.json"
)

$head = Invoke-Git @("rev-parse", "HEAD")
$buildSourcePath = Join-Path $repoRoot "game/BUILD_SOURCE"
$buildSource = if (Test-Path -LiteralPath $buildSourcePath) { (Get-Content $buildSourcePath -Raw).Trim() } else { $null }
$status = @(git status --short)
$manifest = [ordered]@{
    schema_version = "v10-011-corrected-reference-manifest-v1"
    generated_utc = [DateTime]::UtcNow.ToString("o")
    branch = Invoke-Git @("branch", "--show-current")
    head = $head
    build_source = $buildSource
    build_source_matches_head = ($buildSource -eq $head)
    status = $status
    unrelated_worktree_changes_preserved = $true
    source_sha256 = Get-HashMap ($sourceFiles | ForEach-Object { Join-Path $repoRoot $_ })
    config_asset_sha256 = Get-HashMap ($configFiles | ForEach-Object { Join-Path $repoRoot $_ })
    evidence_sha256 = Get-HashMap ($evidenceFiles | ForEach-Object { Join-Path $repoRoot $_ })
    corrected_reference_sha256 = Get-HashMap $referenceFiles
    scenario_matrix = [ordered]@{
        rpm = @(3000, 5000, 8000, 11000, 14000, 15000)
        throttle = @(0.25, 0.70, 1.00)
        load = @(0.00, 0.50, 1.00)
        cases = 54
        duration_s = 6
        warmup_s = 1
        sample_rate_hz = 48000
        seed = 4035969040
        acoustic_scene = $true
        sample_layer = "game/audio/v10_gf509"
    }
    historical_context = [ordered]@{
        e3_e4 = "Historical performance/attribution measurements only; not current rankings."
        original_audit = "reports/audio-v10/v10-001-006/original-audit"
        prior_sweeps_preserved = $true
    }
    human_decision = [ordered]@{
        status = "PENDING_HUMAN_LISTENING"
        accepted = $null
        decision_owner = "human reviewer"
        rejection_scope = $null
        note = "Correctness and DSP transfer evidence are recorded; perceptual acceptance is intentionally not inferred from metrics."
    }
    scope = "V10-007 through V10-011: boundary regression, master coloration bypass, truthful protection diagnostics, physics-driven transient capture, corrected acoustic reference."
}
$manifest | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $outputPath -Encoding UTF8
Write-Output "Wrote $outputPath"
Write-Output "BUILD_SOURCE matches HEAD: $($buildSource -eq $head)"
