[CmdletBinding()]
param(
    [string]$GodotPath,
    [switch]$ForceExtract
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$game = Join-Path $root 'game'
$scene = 'res://scenes/tracks/test_field/jordan_handling_test.tscn'

# Create diagnostics BEFORE any extraction, validation or Godot startup.
# This guarantees that a PowerShell-side failure still leaves evidence.
$logDir = Join-Path $game 'logs'
New-Item -ItemType Directory -Path $logDir -Force | Out-Null

$launcherLog = Join-Path $logDir 'jordan_launcher.log'
$importGodotLog = Join-Path $logDir 'jordan_import_godot.log'
$importStdout = Join-Path $logDir 'jordan_import_stdout.log'
$importStderr = Join-Path $logDir 'jordan_import_stderr.log'
$runGodotLog = Join-Path $logDir 'jordan_handling_godot.log'
$runStdout = Join-Path $logDir 'jordan_handling_stdout.log'
$runStderr = Join-Path $logDir 'jordan_handling_stderr.log'

foreach ($oldLog in @(
    $launcherLog,
    $importGodotLog,
    $importStdout,
    $importStderr,
    $runGodotLog,
    $runStdout,
    $runStderr
)) {
    if (Test-Path $oldLog) {
        Remove-Item $oldLog -Force -ErrorAction SilentlyContinue
    }
}

Start-Transcript -Path $launcherLog -Force | Out-Null

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

function Show-LogTail([string]$path, [int]$lines = 80) {
    if (Test-Path $path) {
        Write-Host "--- $path ---" -ForegroundColor DarkGray
        Get-Content $path -Tail $lines -ErrorAction SilentlyContinue
    }
}

try {
    Write-Host ''
    Write-Host 'Formula-90 / Jordan 1995 diagnostics' -ForegroundColor Cyan
    Write-Host "Launcher log: $launcherLog"

    $bundle = Join-Path $game 'assets\bundles\jordan_1995_runtime.zip'
    $assetDir = Join-Path $game 'assets\generated\jordan_1995'
    $expected = @(
        'jordan_191_1995_chassis.glb',
        'jordan_191_1995_wheel_fl.glb',
        'jordan_191_1995_wheel_fr.glb',
        'jordan_191_1995_wheel_rl.glb',
        'jordan_191_1995_wheel_rr.glb'
    )

    Write-Host "Bundle:   $bundle"
    Write-Host "Assets:   $assetDir"

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
        Write-Host "OK asset: $name"
    }

    $godot = Resolve-Godot $GodotPath
    $dll = Join-Path $game 'addons\formula90s\bin\libformula90s.windows.template_debug.x86_64.dll'
    if (-not (Test-Path $dll)) {
        throw 'GDExtension no compilada. Ejecute .\scripts\build_windows.ps1 -Configuration debug.'
    }

    # Prefer console build so native crash/error output can be redirected by PowerShell.
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
    Write-Host "Console:  $consoleGodot"
    Write-Host "Proyecto: $game"
    Write-Host "Escena:   $scene"
    Write-Host ''
    Write-Host '1/2 Importando recursos Jordan...' -ForegroundColor Cyan

    # PowerShell creates stdout/stderr files itself, so they survive even if
    # Godot crashes before its own --log-file system becomes available.
    & $consoleGodot `
        --headless `
        --path $game `
        --import `
        --log-file $importGodotLog `
        1> $importStdout `
        2> $importStderr

    $importExit = $LASTEXITCODE
    Write-Host "Import exit code: $importExit"

    if ($importExit -ne 0) {
        Write-Host 'La importacion de Godot fallo.' -ForegroundColor Red
        Show-LogTail $importStderr
        Show-LogTail $importStdout
        Show-LogTail $importGodotLog
        exit $importExit
    }

    Write-Host 'Importacion completada.' -ForegroundColor Green
    Write-Host '2/2 Ejecutando Jordan handling test...' -ForegroundColor Cyan

    & $consoleGodot `
        --path $game `
        --log-file $runGodotLog `
        $scene `
        1> $runStdout `
        2> $runStderr

    $runExit = $LASTEXITCODE
    Write-Host "Run exit code: $runExit"

    if ($runExit -ne 0) {
        Write-Host 'Jordan test termino con error/crash.' -ForegroundColor Red
        Show-LogTail $runStderr
        Show-LogTail $runStdout
        Show-LogTail $runGodotLog
        exit $runExit
    }

    Write-Host 'Jordan test termino normalmente.' -ForegroundColor Green
}
catch {
    Write-Host ''
    Write-Host 'FALLO DEL LAUNCHER ANTES O DURANTE GODOT:' -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    Write-Host $_.ScriptStackTrace -ForegroundColor DarkGray
    throw
}
finally {
    try {
        Stop-Transcript | Out-Null
    }
    catch {
        # Never mask the original failure because transcript shutdown failed.
    }
}
