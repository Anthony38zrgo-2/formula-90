[CmdletBinding()]
param(
    [string]$OutputRoot = "reports/audio-v10/v10-018-saturation"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location $repoRoot
$exe = Join-Path $repoRoot "game\crates\target\release\v10_render.exe"
$root = Join-Path $repoRoot $OutputRoot

if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    cargo build --release --manifest-path "game/crates/Cargo.toml" --package v10-engine-synth --bin v10_render
    if ($LASTEXITCODE -ne 0) { throw "v10_render build failed" }
}

$cases = @(
    [pscustomobject]@{ Name = "steady_partial_load"; Rpm = 9000; Throttle = "0.55"; Load = "0.50"; Seconds = "6"; AccelSeconds = "3"; SweepEnd = $null; CoastEnd = $null },
    [pscustomobject]@{ Name = "steady_full_load"; Rpm = 9000; Throttle = "1.00"; Load = "1.00"; Seconds = "6"; AccelSeconds = "3"; SweepEnd = $null; CoastEnd = $null },
    [pscustomobject]@{ Name = "steady_coast"; Rpm = 9000; Throttle = "0.00"; Load = "0.00"; Seconds = "6"; AccelSeconds = "3"; SweepEnd = $null; CoastEnd = $null },
    [pscustomobject]@{ Name = "lift_coast_5000_to_18000"; Rpm = 5000; Throttle = "0.90"; Load = "0.90"; Seconds = "10"; AccelSeconds = "5"; SweepEnd = 18000; CoastEnd = 1800 }
)

New-Item -ItemType Directory -Path $root -Force | Out-Null
$seed = "4035969040"
$caseIndex = 0

foreach ($case in $cases) {
    $variantIndex = 0
    foreach ($variant in @("full", "excised")) {
        $dir = Join-Path (Join-Path $root $case.Name) $variant
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        $arguments = @(
            "--rpm", "$($case.Rpm)",
            "--seconds", $case.Seconds,
            "--accel-seconds", $case.AccelSeconds,
            "--warmup", "1",
            "--throttle", $case.Throttle,
            "--load", $case.Load,
            "--sample-rate", "48000",
            "--seed", $seed,
            "--acoustic-scene",
            "--out", (Join-Path $dir "out.wav"),
            "--stems-dir", (Join-Path $dir "stems")
        )
        if ($null -ne $case.SweepEnd) {
            $arguments += @("--sweep-end-rpm", "$($case.SweepEnd)", "--coast-end-rpm", "$($case.CoastEnd)")
        }
        if ($variant -eq "excised") {
            $arguments += "--excise-load-saturation"
        }
        $variantIndex++
        $ordinal = $caseIndex * 2 + $variantIndex
        Write-Host ("{0}/{1} {2} {3}" -f $ordinal, ($cases.Count * 2), $case.Name, $variant)
        & $exe @arguments
        if ($LASTEXITCODE -ne 0) { throw "render failed for $($case.Name)/$variant" }
    }
    $caseIndex++
}

Write-Host "V10-018 paired renders complete under $root"
