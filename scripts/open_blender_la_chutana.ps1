<#
.SYNOPSIS
    Open Blender to La Chutana track scene.

.DESCRIPTION
    Launches Blender 5.2 (or installed version) with the La Chutana track blend file.
    Used for visual inspection of tire barriers, chicana modules, and environment layout.

    This is a read-only inspection script - does not regenerate or modify any assets.
    For pipeline rebuild, use run_track_pipeline.ps1.

.PARAMETER BlenderPath
    Explicit path to Blender executable. If omitted, auto-resolves to common install paths.

.EXAMPLE
    .\scripts\open_blender_la_chutana.ps1
    .\scripts\open_blender_la_chutana.ps1 -BlenderPath "C:\Program Files\Blender Foundation\Blender 5.2\blender.exe"
#>
[CmdletBinding()]
param(
    [string]$BlenderPath
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

# Resolve Blender executable
$blenderExe = $BlenderPath
if ([string]::IsNullOrWhiteSpace($blenderExe)) {
    $candidates = @(
        "C:\Program Files\Blender Foundation\Blender 5.2\blender.exe",
        "C:\Program Files\Blender Foundation\Blender 5.1\blender.exe",
        "C:\Program Files\Blender Foundation\Blender 5.0\blender.exe",
        "C:\Program Files\Blender Foundation\Blender 4.5\blender.exe",
        "blender.exe"
    )
    foreach ($cand in $candidates) {
        if (Test-Path -LiteralPath $cand -PathType Leaf) {
            $blenderExe = $cand
            break
        }
    }
}
if ([string]::IsNullOrWhiteSpace($blenderExe)) {
    throw "ERROR: Blender executable not found. Pass -BlenderPath or install Blender."
}

$blendPath = Join-Path $root "blender\generated\la_chutana\track_base.blend"

# Check if blend file exists before attempting to launch
if (-not (Test-Path -LiteralPath $blendPath -PathType Leaf)) {
    Write-Warning "WARNING: Blend file not found: $blendPath"
    Write-Warning "Generate with: run_track_pipeline.ps1 -Mode Base"
    Write-Host "Attempting to launch Blender anyway..." -ForegroundColor Yellow
}

Write-Host "Launching Blender La Chutana track..." -ForegroundColor Cyan
Write-Host "  Blender: $blenderExe" -ForegroundColor White
Write-Host "  Track blend: $blendPath" -ForegroundColor White

try {
    & $blenderExe $blendPath
    Write-Host "Blender process started. Use Blender to inspect the La Chutana track." -ForegroundColor Green
    Write-Host "Close Blender window to exit this script." -ForegroundColor Gray
} catch {
    throw "ERROR: Failed to launch Blender: $_"
}