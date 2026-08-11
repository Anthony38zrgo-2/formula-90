[CmdletBinding()]
param(
    [string]$GodotPath,
    [string]$TestPath = 'res://tests/gdunit'
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$game = Join-Path $repo 'game'
$godot = if ($GodotPath) {
    (Resolve-Path $GodotPath).Path
}
elseif ($env:GODOT_BIN) {
    (Resolve-Path $env:GODOT_BIN).Path
}
else {
    $console = Join-Path $repo '.tools\godot\Godot_v4.7.1-stable_win64_console.exe'
    $standard = Join-Path $repo '.tools\godot\Godot_v4.7.1-stable_win64.exe'
    if (Test-Path -LiteralPath $console) { $console } else { (Resolve-Path $standard).Path }
}

$env:APPDATA = Join-Path $repo '.tools\appdata'
$env:LOCALAPPDATA = Join-Path $repo '.tools\localappdata'
New-Item -ItemType Directory -Path $env:APPDATA, $env:LOCALAPPDATA -Force | Out-Null

Write-Host "GdUnit4: $TestPath" -ForegroundColor Cyan
& $godot `
    --headless `
    --path $game `
    --script 'res://addons/gdUnit4/bin/GdUnitCmdTool.gd' `
    --ignoreHeadlessMode `
    -a $TestPath `
    -c `
    -rd 'res://reports/gdunit'
$exitCode = $LASTEXITCODE
if ($exitCode -ne 0) {
    throw "GdUnit4 fallo con codigo $exitCode. Revisa game/reports/gdunit/report_*/results.xml."
}

Write-Host 'GdUnit4 completo: todas las pruebas pasaron.' -ForegroundColor Green
