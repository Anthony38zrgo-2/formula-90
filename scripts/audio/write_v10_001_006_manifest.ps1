[CmdletBinding()]
param(
    [string]$Output = "reports/audio-v10/v10-001-006/provenance.json"
)

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location $repoRoot

$outputPath = Join-Path $repoRoot $Output
$outputDir = Split-Path -Parent $outputPath
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

function Get-Sha256([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $null }
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-Relative([string]$Path) {
    return [IO.Path]::GetRelativePath($repoRoot, (Resolve-Path -LiteralPath $Path).Path)
}

function Get-HashMap([string[]]$Paths) {
    $map = [ordered]@{}
    foreach ($path in $Paths) {
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            $map[(Get-Relative $path)] = Get-Sha256 $path
        }
    }
    return $map
}

function Invoke-Git([string[]]$Arguments) {
    $result = & git @Arguments 2>$null
    if ($LASTEXITCODE -ne 0) { return $null }
    return (($result -join "`n").Trim())
}

$head = Invoke-Git @("rev-parse", "HEAD")
$branch = Invoke-Git @("branch", "--show-current")
$status = @(git status --short)
$buildSourcePath = Join-Path $repoRoot "game/BUILD_SOURCE"
$buildSource = if (Test-Path -LiteralPath $buildSourcePath) {
    (Get-Content -LiteralPath $buildSourcePath -Raw).Trim()
} else { $null }
$buildMatchesHead = $buildSource -eq $head

$sourcePaths = @(
    "game/crates/v10-engine-synth/src/acoustics.rs",
    "game/crates/v10-engine-synth/src/config.rs",
    "game/crates/v10-engine-synth/src/crank.rs",
    "game/crates/v10-engine-synth/src/cylinder.rs",
    "game/crates/v10-engine-synth/src/engine.rs",
    "game/crates/v10-engine-synth/src/geometry.rs",
    "game/crates/v10-engine-synth/src/thermodynamics/combustion.rs",
    "game/crates/v10-engine-synth/src/thermodynamics/gas.rs",
    "game/crates/v10-engine-synth/src/thermodynamics/polytrope.rs",
    "game/crates/v10-engine-synth/src/thermodynamics/exhaust_runner.rs",
    "game/crates/v10-engine-synth/src/thermodynamics/exhaust_valve.rs",
    "game/crates/v10-engine-synth/src/sample_layer.rs",
    "game/crates/v10-engine-synth/src/scene.rs",
    "game/crates/v10-engine-synth/src/runtime.rs",
    "game/crates/v10-engine-synth/src/bin/v10_render.rs"
)

$assetPaths = @(
    "game/sounds/sound_mixer_config.json",
    "game/data/vehicles/f1_2026_2008/f1_2026_2008_physics.json",
    "game/audio/v10_gf509/package.json",
    "game/audio/v10_gf509/manifest.json",
    "game/sounds/banks/v10_vehicle/manifest.json"
)

$binaryPaths = @(
    "game/crates/target/release/v10_render.exe",
    "game/crates/target/release/v10_provenance_probe.exe"
)

$capturePaths = @(
    "reports/audio-v10/v10-001-006/provenance-probes.json",
    "reports/audio-v10/v10-001-006/v10_5000_to_1800_lift_coast_10s.wav",
    "reports/audio-v10/v10-001-006/v10_5000_to_1800_lift_coast_10s.metadata.json",
    "reports/audio-v10/v10-001-006/v10_5000_to_1800_lift_coast_10s.analysis.json",
    "reports/audio-v10/v10-001-006/v10_5000_to_1800_lift_coast_10s.physical.csv",
    "reports/audio-v10/v10-001-006/v10_5000_to_18000_then_lift_coast_10s.wav",
    "reports/audio-v10/v10-001-006/v10_5000_to_18000_then_lift_coast_10s.metadata.json",
    "reports/audio-v10/v10-001-006/v10_5000_to_18000_then_lift_coast_10s.analysis.json",
    "reports/audio-v10/v10-001-006/v10_5000_to_18000_then_lift_coast_10s.physical.csv",
    "reports/audio-v10/v10-001-006/aud_path_bench.json",
    "reports/audio-v10/v10-001-006/va_bench.md",
    "reports/audio-v10/v10-001-006/va_bench_run2.md",
    "reports/audio-v10/v10-001-006/va_bench_run3.md"
)

$tempRoot = [IO.Path]::GetTempPath()
$auditCandidates = @(Get-ChildItem -LiteralPath $tempRoot -Directory -Filter "f90-v10-audit-*" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending)
$originalAudit = if ($auditCandidates.Count -gt 0) { $auditCandidates[0].FullName } else { $null }
$preservedAudit = @()
if ($originalAudit) {
    $auditDestination = Join-Path $outputDir "original-audit"
    New-Item -ItemType Directory -Force -Path $auditDestination | Out-Null
    foreach ($relative in @("Cargo.lock", "Cargo.toml", "evidencia.txt", "informe.md", "src/main.rs", "src/bin/wiring.rs")) {
        $source = Join-Path $originalAudit $relative
        if (Test-Path -LiteralPath $source -PathType Leaf) {
            $destination = Join-Path $auditDestination $relative
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
            Copy-Item -LiteralPath $source -Destination $destination -Force
            $preservedAudit += [ordered]@{
                source = $source
                preserved = Get-Relative $destination
                sha256 = Get-Sha256 $destination
            }
        }
    }
}

$rustc = (& rustc --version 2>$null) -join " "
$cargo = (& cargo --version 2>$null) -join " "
$lockfiles = @(Get-ChildItem -LiteralPath "game/crates" -Filter "Cargo.lock" -File -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName })

$manifest = [ordered]@{
    schema = "v10-001-006-provenance-v1"
    generated_utc = [DateTime]::UtcNow.ToString("o")
    repo_root = $repoRoot
    branch = $branch
    head = $head
    origin_main_clean = Invoke-Git @("rev-parse", "origin/main-clean")
    status = $status
    unrelated_worktree_changes_preserved = $true
    build_source = $buildSource
    build_source_matches_head = $buildMatchesHead
    toolchain = [ordered]@{ rustc = $rustc; cargo = $cargo }
    lockfiles_sha256 = Get-HashMap $lockfiles
    source_sha256 = Get-HashMap ($sourcePaths | ForEach-Object { Join-Path $repoRoot $_ })
    asset_manifest_sha256 = Get-HashMap ($assetPaths | ForEach-Object { Join-Path $repoRoot $_ })
    binary_sha256 = Get-HashMap ($binaryPaths | ForEach-Object { Join-Path $repoRoot $_ })
    capture_sha256 = Get-HashMap ($capturePaths | ForEach-Object { Join-Path $repoRoot $_ })
    original_audit = [ordered]@{
        source_directory = $originalAudit
        preserved_files = $preservedAudit
        note = "Original V10 audit evidence preserved without copying target artifacts."
    }
    scope = "V10-001 through V10-006: provenance, deterministic probes, ownership contract, benchmark declaration, bounded chamber model, corrected chamber implementation."
}

$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $outputPath -Encoding UTF8
Write-Output "Wrote $outputPath"
Write-Output "BUILD_SOURCE matches HEAD: $buildMatchesHead"
