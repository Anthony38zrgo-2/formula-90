[CmdletBinding()]
param(
    [string]$GodotPath,
    [switch]$ForceExtract
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$game = Join-Path $root 'game'
$scene = 'res://scenes/tracks/test_field/jordan_handling_test.tscn'
$logDir = Join-Path $game 'logs'
$importLog = Join-Path $logDir 'jordan_import.log'
$runLog = Join-Path $logDir 'jordan_handling.log'

function Resolve-Godot([string]$explicit) {
    if ($explicit) {
        if (Test-Path $explicit) {
            return (Resolve-Path $explicit).Path
        }
        throw "Godot no existe: $explicit"
    }

    if ($env:GODOT_BIN -and (Test-Path $env:GODOT_BIN)) {
        return (Resolve-Path $env:GODOT_BIN).Path
    }

    $candidates = @(
        (Join-Path $root '.tools\godot\Godot_v4.7.1-stable_win64.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs\Godot\Godot.exe'),
        (Join-Path $env:ProgramFiles 'Godot\Godot.exe')
    )

    foreach ($item in $candidates) {
        if ($item -and (Test-Path $item)) {
            return (Resolve-Path $item).Path
        }
    }

    throw 'Godot 4.7.1 no encontrado.'
}

$bundle = Join-Path $game 'assets\bundles\jordan_1995_runtime.zip'
$assetDir = Join-Path $game 'assets\generated\jordan_1995'
$expected = @(
    'jordan_191_1995_chassis.glb',
    'jordan_191_1995_wheel_fl.glb',
    'jordan_191_1995_wheel_fr.glb',
    'jordan_191_1995_wheel_rl.glb',
    'jordan_191_1995_wheel_rr.glb'
)

if (-not (Test-Path $bundle)) {
    throw "Bundle Jordan no encontrado: $bundle"
}

$needsExtract = $ForceExtract
foreach ($name in $expected) {
    if (-not (Test-Path (Join-Path $assetDir $name))) {
        $needsExtract = $true
        break
    }
}

if ($needsExtract) {
    Write-Host 'Materializando assets runtime del Jordan 1995...' -ForegroundColor Cyan
    if (Test-Path $assetDir) {
        Remove-Item $assetDir -Recurse -Force
    }
    New-Item -ItemType Directory -Path $assetDir -Force | Out-Null
    Expand-Archive -Path $bundle -DestinationPath $assetDir -Force
}

foreach ($name in $expected) {
    $path = Join-Path $assetDir $name
    if (-not (Test-Path $path)) {
        throw "Asset Jordan faltante despues de extraer: $path"
    }
}

$godot = Resolve-Godot $GodotPath
$dll = Join-Path $game 'addons\formula90s\bin\libformula90s.windows.template_debug.x86_64.dll'
if (-not (Test-Path $dll)) {
    throw 'GDExtension no compilada. Ejecute .\scripts\build_windows.ps1 -Configuration debug.'
}

if (-not (Test-Path $logDir)) {
    New-Item -ItemType Directory -Path $logDir -Force | Out-Null
}

# Prefer the console executable when available so importer/runtime errors
# are visible in the current PowerShell window.
$consoleGodot = $godot
if ($godot -notmatch '_console\.exe$') {
    $candidateConsole = [System.IO.Path]::Combine(
        [System.IO.Path]::GetDirectoryName($godot),
        ([System.IO.Path]::GetFileNameWithoutExtension($godot) + '_console.exe')
    )
    if (Test-Path $candidateConsole) {
        $consoleGodot = $candidateConsole
    }
}

Write-Host "Godot:    $godot"
Write-Host "Proyecto: $game"
Write-Host "Escena:   $scene"
Write-Host 'Perfil:   Jordan Phase A / GEVP clean baseline'
Write-Host ''
Write-Host '1/2 Importando recursos Jordan en Godot...' -ForegroundColor Cyan

# Godot documents --import as: start editor, wait for resources to import, then quit.
# This prevents launching the Jordan scene while freshly extracted GLBs are still unimported.
& $consoleGodot `
    --headless `
    --path $game `
    --import `
    --log-file $importLog

$importExit = $LASTEXITCODE
if ($importExit -ne 0) {
    Write-Host "La importacion de Godot fallo con codigo $importExit." -ForegroundColor Red
    if (Test-Path $importLog) {
        Write-Host 'Ultimas lineas del log de importacion:' -ForegroundColor Yellow
        Get-Content $importLog -Tail 100
    }
    exit $importExit
}

Write-Host 'Importacion completada.' -ForegroundColor Green
Write-Host '2/2 Ejecutando Jordan handling test...' -ForegroundColor Cyan

# Pass the scene as the positional scene argument. This is the documented
# command-line form for running a specific scene.
& $consoleGodot `
    --path $game `
    --log-file $runLog `
    $scene

$runExit = $LASTEXITCODE
if ($runExit -ne 0) {
    Write-Host "Jordan test termino con codigo $runExit." -ForegroundColor Red
    if (Test-Path $runLog) {
        Write-Host 'Ultimas lineas del log de ejecucion:' -ForegroundColor Yellow
        Get-Content $runLog -Tail 100
    }
    exit $runExit
}

exit 0
