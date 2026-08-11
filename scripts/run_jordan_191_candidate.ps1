[CmdletBinding()]
param(
    [string]$GodotPath,
    [switch]$HeadlessSmoke
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$game = Join-Path $root 'game'
$scene = 'res://scenes/vehicles/f1_90s_canonical_1997/jordan_191_candidate_validation.tscn'
$smoke = 'res://tests/smoke_test_jordan_191_candidate_validation.gd'
$candidateDir = Join-Path $game 'assets\models\vehicles\f1_90s_canonical_1997\candidate_k3_historical'
$manifest = Join-Path $candidateDir 'vehicle_manifest.json'
$assets = @(
    'jordan_191_candidate_chassis.glb',
    'jordan_191_candidate_wheel_front.glb',
    'jordan_191_candidate_wheel_rear.glb'
)

function Resolve-Godot([string]$explicit) {
    $candidates = @($explicit, $env:GODOT_BIN, (Join-Path $root '.tools\godot\Godot_v4.7.1-stable_win64_console.exe'))
    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path $candidate -PathType Leaf)) { return (Resolve-Path $candidate).Path }
    }
    throw 'Godot 4.7.1 no encontrado. Use -GodotPath o GODOT_BIN.'
}

if (-not (Test-Path $manifest -PathType Leaf)) { throw "Manifest candidato faltante: $manifest" }
foreach ($asset in $assets) {
    $path = Join-Path $candidateDir $asset
    if (-not (Test-Path $path -PathType Leaf)) { throw "GLB candidato faltante: $path" }
    Write-Host "OK: $asset  $((Get-FileHash $path -Algorithm SHA256).Hash)"
}

$godot = Resolve-Godot $GodotPath
Write-Host "Candidato: $candidateDir" -ForegroundColor Cyan
Write-Host "Escena aislada: $scene" -ForegroundColor Cyan

& $godot --headless --path $game --import
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if ($HeadlessSmoke) {
    & $godot --headless --path $game --script $smoke
    exit $LASTEXITCODE
}

Write-Host 'Abriendo la escena candidata. El Jordan activo no se modifica.' -ForegroundColor Green
Start-Process -FilePath $godot -ArgumentList @('--path', $game, $scene)
