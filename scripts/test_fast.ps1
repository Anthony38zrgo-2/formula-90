[CmdletBinding()]
param(
    [ValidateSet('Architecture', 'Audio', 'Vehicle', 'Track', 'Runtime')]
    [string[]]$Area = @('Architecture'),
    [string]$GodotPath,
    [switch]$List
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

if ($List) {
    Write-Host 'FAST areas:' -ForegroundColor Cyan
    Write-Host '  Architecture  output policy and architecture helpers'
    Write-Host '  Audio         offline audio tooling tests'
    Write-Host '  Vehicle       f1_2030_v10 five-GLB runtime asset validation'
    Write-Host '  Track         focused SVG/track compiler tests'
    Write-Host '  Runtime       canonical runtime and BUILD/HEAD validation'
    exit 0
}

function Invoke-FastStep {
    param(
        [string]$Name,
        [scriptblock]$Action
    )

    Write-Host "FAST [$Name]" -ForegroundColor Cyan
    & $Action
    if (-not $?) {
        throw "FAST [$Name] failed."
    }
}

Push-Location $root
try {
    foreach ($selectedArea in $Area | Select-Object -Unique) {
        switch ($selectedArea) {
            'Architecture' {
                Invoke-FastStep 'Architecture' {
                    & python -m unittest discover -s tools/common/tests -p 'test_*.py' -v
                }
            }
            'Audio' {
                Invoke-FastStep 'Audio' {
                    & (Join-Path $PSScriptRoot 'test_audio_tools_windows.ps1')
                }
            }
            'Vehicle' {
                Invoke-FastStep 'Vehicle' {
					$vehicleRoots = @('game/assets/models/vehicles/f1-2030')
                    foreach ($relativeRoot in $vehicleRoots) {
                        $manifest = Join-Path $root $relativeRoot
                        if (-not (Test-Path (Join-Path $manifest 'manifest.json'))) {
                            throw "Vehicle manifest missing: $relativeRoot"
                        }
                    }
                    $vehicleDataDir = Join-Path $root 'game/data/vehicles'
                    $vehicleDefinitions = @(Get-ChildItem -LiteralPath $vehicleDataDir -Filter '*.tres' -File)
                    if ($vehicleDefinitions.Count -ne 1 -or $vehicleDefinitions[0].Name -ne 'f1_2030_v10.tres') {
                        throw 'Vehicle registry must contain only f1_2030_v10.tres'
                    }
                    foreach ($forbidden in @('f1_94', 'f1_2009_fw31')) {
                        foreach ($relativePath in @(
                            "game/data/vehicles/$forbidden",
                            "game/data/vehicles/$forbidden.tres",
                            "game/scenes/vehicles/$forbidden",
                            "game/assets/models/vehicles/$forbidden"
                        )) {
                            if (Test-Path -LiteralPath (Join-Path $root $relativePath)) {
                                throw "Forbidden vehicle path remains: $relativePath"
                            }
                        }
                    }
                }
            }
            'Track' {
                Invoke-FastStep 'Published track package' {
                    $track = Join-Path $root 'game/assets/generated/tracks/la_chutana/la_chutana.glb'
                    if (-not (Test-Path $track)) { throw "Published track GLB missing: $track" }
                }
            }
            'Runtime' {
                Invoke-FastStep 'Runtime' {
                    $runtimeArgs = @{ ValidateRuntimeOnly = $true }
                    if ($GodotPath) { $runtimeArgs.GodotPath = $GodotPath }
                    & (Join-Path $root 'run_f1_94.ps1') @runtimeArgs
                }
            }
        }
    }
} finally {
    Pop-Location
}

Write-Host "FAST passed: $($Area -join ', ')" -ForegroundColor Green
