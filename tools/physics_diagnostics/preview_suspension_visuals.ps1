[CmdletBinding()]
param(
    [string]$OutputDirectory,
    [switch]$Animate
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$godot = Join-Path $repo '.tools\godot\Godot_v4.7.1-stable_win64_console.exe'
if (-not (Test-Path -LiteralPath $godot)) { throw 'Godot 4.7.1 is required.' }
$isolated = Join-Path ([IO.Path]::GetTempPath()) ('f90-suspension-' + [guid]::NewGuid().ToString('N'))
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $isolated 'captures' }
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$OutputDirectory = (Resolve-Path -LiteralPath $OutputDirectory).Path
$files = @(
    'scripts/vehicle/suspension_geometry.gd',
    'scripts/vehicle/suspension_link_visual.gd',
    'scripts/vehicle/f1_wheel_visual_controller.gd',
    'data/vehicles/f1_2030/f1_2030_v10_physics.json',
    'data/vehicles/f1_2030/f1_2030_suspension_meshes.json',
    'tests/test_f1_2030_suspension_geometry.gd',
    'tests/test_f1_2030_suspension_visual_pose.gd',
    'tests/visual_preview_f1_2030_suspension.gd'
)
foreach ($part in @('chassis', 'wheel_FL', 'wheel_FR', 'wheel_RL', 'wheel_RR')) {
    $files += "assets/models/vehicles/f1-2030/f1_2030_v10_$part.glb"
}
$hashes = [ordered]@{}
foreach ($relative in $files) {
    $source = Join-Path (Join-Path $repo 'game') $relative
    $destination = Join-Path $isolated $relative
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
    Copy-Item -LiteralPath $source -Destination $destination
    $hashes[$relative] = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
}
@'
[application]
config/name="F90 isolated suspension review"
[rendering]
renderer/rendering_method="gl_compatibility"
'@ | Set-Content -LiteralPath (Join-Path $isolated 'project.godot')
& $godot --headless --path $isolated --editor --import --quit *> (Join-Path $OutputDirectory 'import.log')
if ($LASTEXITCODE -ne 0) { throw 'Isolated import failed; see import.log.' }
foreach ($test in @('test_f1_2030_suspension_geometry', 'test_f1_2030_suspension_visual_pose')) {
    & $godot --headless --path $isolated --script "res://tests/$test.gd" *> (Join-Path $OutputDirectory "$test.log")
    if ($LASTEXITCODE -ne 0) { throw "Regression failed: $test" }
}
$captureArgs = @('--path', $isolated, '--rendering-method', 'gl_compatibility', '--script', 'res://tests/visual_preview_f1_2030_suspension.gd', '--', $OutputDirectory)
if ($Animate) { $captureArgs += 'animate' }
& $godot @captureArgs *> (Join-Path $OutputDirectory 'capture.log')
if ($LASTEXITCODE -ne 0) { throw 'Visual capture failed; see capture.log.' }
[ordered]@{
    head = (& git -C $repo rev-parse HEAD).Trim()
    branch = (& git -C $repo branch --show-current).Trim()
    source = 'Working-tree visual-only snapshot; no native DLLs loaded'
    isolated_project = $isolated
    sha256 = $hashes
} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $OutputDirectory 'provenance.json')
Write-Host "Visual tests and captures completed: $OutputDirectory"
