# Renders the PHY-001 frozen hybrid baseline: 6 RPM x 3 throttle x 3 load = 54
# steady-state renders of the current v10-engine-synth at HEAD.
#
# Grid choices (per backlog PHY-001):
#   rpm      3000 5000 8000 11000 14000 15000
#   throttle 0.25 0.70 1.00
#   load     0.00 0.50 1.00  (coast / partial / high)
#
# Renderer    game/crates/target/release/v10_render.exe  (builds once)
# Seed        0xF090_0010 (4035969040) -- the EngineConfig default
# Output      reports/audio/physical-refactor/baseline/r<rpm>_t<throttle>_l<load>/
#             {out.wav, out.metadata.json, out.events.csv, stems/*.wav}
#
# The renderer is invoked from the repo root so git provenance and the
# game/audio/v10_gf509 sample-layer asset path both resolve.

$ErrorActionPreference = "Stop"

$repoRoot = "D:\Formula90s"
$exe      = Join-Path $repoRoot "game\crates\target\release\v10_render.exe"
$outRoot  = Join-Path $repoRoot "reports\audio\physical-refactor\baseline"
$sampleDir = "game/audio/v10_gf509"

$rpm      = @(3000, 5000, 8000, 11000, 14000, 15000)
$throttle = @("0.25", "0.70", "1.00")
$load     = @("0.00", "0.50", "1.00")

# Individually include agnostic flags; accel-seconds is ignored for steady
# renders but must still be inside the render duration for validation.
$common = @(
    "--seconds", "6",
    "--warmup", "1",
    "--accel-seconds", "5.0",
    "--sample-rate", "48000",
    "--seed", "4035969040",
    "--acoustic-scene",
    "--sample-layer-dir", $sampleDir
)

if (-not (Test-Path -LiteralPath $exe)) {
    Write-Host "Building v10_render (release) ..."
    Push-Location $repoRoot
    try {
        cargo build --release --manifest-path "game/crates/v10-engine-synth/Cargo.toml" --bin v10_render
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    } finally {
        Pop-Location
    }
}

New-Item -ItemType Directory -Path $outRoot -Force | Out-Null

$count = 0
foreach ($r in $rpm) {
    foreach ($t in $throttle) {
        foreach ($l in $load) {
            $count++
            $combo = "r${r}_t${t}_l${l}"
            $dir   = Join-Path $outRoot $combo
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

Write-Host "Baseline complete: $count renders in $outRoot"
