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
    Write-Host '  Vehicle       F1-94 runtime asset validation'
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
    if ($LASTEXITCODE -ne 0) {
        throw "FAST [$Name] failed with exit code $LASTEXITCODE."
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
                    & python (Join-Path $PSScriptRoot 'validate_f1_94_decoupled.py')
                }
            }
            'Track' {
                Invoke-FastStep 'Track SVG profile' {
                    & python -m unittest blender.track_pipeline.tests.test_svg_profile -v
                }
                Invoke-FastStep 'Track compiler' {
                    & python -m unittest blender.track_pipeline.tests.test_compile_svg_track -v
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
