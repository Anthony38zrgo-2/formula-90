[CmdletBinding()]
param(
    [string]$OutputRoot = "reports/audio-v10/v10-007-011/reference"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location $repoRoot
$exe = Join-Path $repoRoot "game\crates\target\release\v10_render.exe"
$root = Join-Path $repoRoot $OutputRoot
$sampleDir = "game/audio/v10_gf509"

$rpm = @(3000, 5000, 8000, 11000, 14000, 15000)
$throttle = @("0.25", "0.70", "1.00")
$load = @("0.00", "0.50", "1.00")

if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    cargo build --release --manifest-path "game/crates/v10-engine-synth/Cargo.toml" --bin v10_render
    if ($LASTEXITCODE -ne 0) { throw "v10_render build failed" }
}

New-Item -ItemType Directory -Path $root -Force | Out-Null
$common = @(
    "--seconds", "6",
    "--warmup", "1",
    "--accel-seconds", "5.0",
    "--sample-rate", "48000",
    "--seed", "4035969040",
    "--acoustic-scene",
    "--sample-layer-dir", $sampleDir
)

$count = 0
foreach ($r in $rpm) {
    foreach ($t in $throttle) {
        foreach ($l in $load) {
            $count++
            $combo = "r${r}_t${t}_l${l}"
            $dir = Join-Path $root $combo
            New-Item -ItemType Directory -Path $dir -Force | Out-Null
            Write-Host ("[{0}/54] {1}" -f $count, $combo)
            & $exe `
                --rpm $r `
                --throttle $t `
                --load $l `
                @common `
                --out (Join-Path $dir "out.wav") `
                --stems-dir (Join-Path $dir "stems")
            if ($LASTEXITCODE -ne 0) { throw "render failed for $combo" }
        }
    }
}

Write-Host "Corrected reference matrix complete: $count renders in $root"
