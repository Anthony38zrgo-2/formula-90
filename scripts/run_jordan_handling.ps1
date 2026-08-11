[CmdletBinding()]
param(
    [string]$GodotPath,
    [string]$BundlePath,
    [switch]$ForceExtract
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$game = Join-Path $root 'game'
$scene = 'res://scenes/tracks/test_field/jordan_handling_test.tscn'
$expectedBundleSha256 = '4F9602254FDF3B06CB1B06116258F4C3AFD7035A121A4C156EB2C25DB99A4AA4'

# Create diagnostics BEFORE any extraction, validation or Godot startup.
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

function Invoke-GodotNative {
    param(
        [Parameter(Mandatory = $true)] [string]$Executable,
        [Parameter(Mandatory = $true)] [string[]]$Arguments,
        [Parameter(Mandatory = $true)] [string]$StdoutPath,
        [Parameter(Mandatory = $true)] [string]$StderrPath
    )

    # Windows PowerShell 5.1 can promote native stderr output to a terminating
    # NativeCommandError when ErrorActionPreference is Stop. Godot writes
    # warnings to stderr, so temporarily relax only the native invocation and
    # trust the process exit code instead.
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & $Executable @Arguments 1> $StdoutPath 2> $StderrPath
        return $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousPreference
    }
}

try {
    Write-Host ''
    Write-Host 'Formula-90 / Jordan 1995 diagnostics' -ForegroundColor Cyan
    Write-Host "Launcher log: $launcherLog"

    if ($BundlePath) {
        if (-not (Test-Path $BundlePath -PathType Leaf)) {
            throw "BundlePath no existe: $BundlePath"
        }
        $bundle = (Resolve-Path $BundlePath).Path
    }
    else {
        $bundle = Join-Path $game 'assets\bundles\jordan_1995_runtime.zip'
    }

    $assetDir = Join-Path $game 'assets\generated\jordan_1995'

    # Canonical Formula90s vehicle visual contract: chassis + one wheel visual
    # per axle. The source bundle may still contain four side-specific wheel
    # files, but runtime intentionally materializes only the verified left-side
    # source for each axle and reuses it for both sides in Godot.
    $assetSources = @{
        'jordan_191_1995_chassis.glb' = 'jordan_191_1995_chassis.glb'
        'jordan_191_1995_wheel_front.glb' = 'jordan_191_1995_wheel_fl.glb'
        'jordan_191_1995_wheel_rear.glb' = 'jordan_191_1995_wheel_rl.glb'
    }
    $expected = @(
        'jordan_191_1995_chassis.glb',
        'jordan_191_1995_wheel_front.glb',
        'jordan_191_1995_wheel_rear.glb'
    )

    Write-Host "Bundle:   $bundle"
    Write-Host "Assets:   $assetDir"

    if (-not (Test-Path $bundle -PathType Leaf)) {
        throw "Bundle Jordan no encontrado: $bundle"
    }

    $bundleHash = (Get-FileHash -Path $bundle -Algorithm SHA256).Hash.ToUpperInvariant()
    Write-Host "SHA256:   $bundleHash"

    if ($bundleHash -ne $expectedBundleSha256) {
        throw @"
Bundle Jordan invalido o corrupto.
Esperado: $expectedBundleSha256
Actual:   $bundleHash

Use un bundle valido con:
  .\scripts\run_jordan_handling.ps1 -BundlePath "C:\ruta\jordan_1995_assets_small.zip" -ForceExtract
"@
    }

    $needsExtract = $ForceExtract
    foreach ($name in $expected) {
        if (-not (Test-Path (Join-Path $assetDir $name))) {
            $needsExtract = $true
            break
        }
    }

    if ($needsExtract) {
        Write-Host 'Materializando contrato canonico de 3 assets del Jordan 1995...' -ForegroundColor Cyan
        if (Test-Path $assetDir) {
            Remove-Item $assetDir -Recurse -Force
        }
        New-Item -ItemType Directory -Path $assetDir -Force | Out-Null

        $extractTemp = Join-Path $game 'assets\generated\.jordan_1995_extract_tmp'
        if (Test-Path $extractTemp) {
            Remove-Item $extractTemp -Recurse -Force
        }
        New-Item -ItemType Directory -Path $extractTemp -Force | Out-Null

        try {
            Expand-Archive -Path $bundle -DestinationPath $extractTemp -Force
            foreach ($runtimeName in $expected) {
                $sourceName = $assetSources[$runtimeName]
                $sourcePath = Join-Path $extractTemp $sourceName
                if (-not (Test-Path $sourcePath -PathType Leaf)) {
                    throw "Asset fuente Jordan faltante dentro del bundle: $sourceName"
                }
                Copy-Item -Path $sourcePath -Destination (Join-Path $assetDir $runtimeName) -Force
            }
        }
        finally {
            if (Test-Path $extractTemp) {
                Remove-Item $extractTemp -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }

    # Enforce the dedicated generated directory invariant even after older runs:
    # only the three canonical runtime GLBs and their Godot .import sidecars may
    # remain here. This prevents stale FR/RR assets from being scanned/imported.
    if (Test-Path $assetDir) {
        $allowedNames = @()
        foreach ($name in $expected) {
            $allowedNames += $name
            $allowedNames += "$name.import"
        }
        foreach ($file in Get-ChildItem -Path $assetDir -File -ErrorAction SilentlyContinue) {
            if ($allowedNames -notcontains $file.Name) {
                Remove-Item $file.FullName -Force -ErrorAction SilentlyContinue
            }
        }
    }

    foreach ($name in $expected) {
        $path = Join-Path $assetDir $name
        if (-not (Test-Path $path)) {
            throw "Asset Jordan faltante despues de materializar: $path"
        }
        Write-Host "OK canonical asset: $name"
    }

    $godot = Resolve-Godot $GodotPath
    $dll = Join-Path $game 'addons\formula90s\bin\libformula90s.windows.template_debug.x86_64.dll'
    if (-not (Test-Path $dll)) {
        throw 'GDExtension no compilada. Ejecute .\scripts\build_windows.ps1 -Configuration debug.'
    }

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

    $importArgs = @(
        '--headless',
        '--path', $game,
        '--import',
        '--log-file', $importGodotLog
    )
    $importExit = Invoke-GodotNative `
        -Executable $consoleGodot `
        -Arguments $importArgs `
        -StdoutPath $importStdout `
        -StderrPath $importStderr

    Write-Host "Import exit code: $importExit"

    if ($importExit -ne 0) {
        Write-Host 'La importacion de Godot fallo.' -ForegroundColor Red
        Show-LogTail $importStderr
        Show-LogTail $importStdout
        Show-LogTail $importGodotLog
        exit $importExit
    }

    if ((Test-Path $importStderr) -and ((Get-Item $importStderr).Length -gt 0)) {
        Write-Host 'Godot emitio warnings durante la importacion; se conservaron en jordan_import_stderr.log.' -ForegroundColor Yellow
    }

    Write-Host 'Importacion completada.' -ForegroundColor Green
    Write-Host '2/2 Ejecutando Jordan handling test...' -ForegroundColor Cyan

    $runArgs = @(
        '--path', $game,
        '--log-file', $runGodotLog,
        $scene
    )
    $runExit = Invoke-GodotNative `
        -Executable $consoleGodot `
        -Arguments $runArgs `
        -StdoutPath $runStdout `
        -StderrPath $runStderr

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
