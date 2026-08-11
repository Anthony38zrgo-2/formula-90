[CmdletBinding()]
param(
    [string]$GodotPath
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$game = Join-Path $root 'game'
$scene = 'res://scenes/tracks/test_field/jordan_handling_test.tscn'
$logDir = Join-Path $game 'logs'
$runGodotLog = Join-Path $logDir 'jordan_handling_godot.log'
$runStdout = Join-Path $logDir 'jordan_handling_stdout.log'
$runStderr = Join-Path $logDir 'jordan_handling_stderr.log'

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
    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path $candidate)) {
            return (Resolve-Path $candidate).Path
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

New-Item -ItemType Directory -Path $logDir -Force | Out-Null
Remove-Item -LiteralPath $runGodotLog, $runStdout, $runStderr -Force -ErrorAction SilentlyContinue

try {
    $k3Assets = @(
        (Join-Path $game 'assets\models\vehicles\f1_90s_canonical_1997\candidate_k3_historical\jordan_191_candidate_chassis.glb'),
        (Join-Path $game 'assets\models\vehicles\f1_90s_canonical_1997\candidate_k3_historical\jordan_191_candidate_wheel_front.glb'),
        (Join-Path $game 'assets\models\vehicles\f1_90s_canonical_1997\candidate_k3_historical\jordan_191_candidate_wheel_rear.glb')
    )
    foreach ($asset in $k3Assets) {
        if (-not (Test-Path $asset -PathType Leaf)) {
            throw "Asset K3 historico faltante: $asset"
        }
    }

    $godot = Resolve-Godot $GodotPath
    $dll = Join-Path $game 'addons\formula90s\bin\libformula90s.windows.template_debug.x86_64.dll'
    if (-not (Test-Path $dll)) {
        throw 'GDExtension no compilada. Ejecute .\scripts\build_windows.ps1 -Configuration debug.'
    }

    Write-Host 'Formula-90 / Jordan 191 K3 historical handling' -ForegroundColor Cyan
    Write-Host "Godot:    $godot"
    Write-Host "Proyecto: $game"
    Write-Host "Escena:   $scene"
    Write-Host 'Iniciando La Chutana con FormulaVehicleController/VehicleRigidBody...' -ForegroundColor Cyan

    $previousPreference = $ErrorActionPreference
    try {
        # Godot writes normal warnings to stderr; use the native exit code.
        $ErrorActionPreference = 'Continue'
        & $godot --path $game --log-file $runGodotLog $scene 1> $runStdout 2> $runStderr
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousPreference
    }

    if ($exitCode -ne 0) {
        Write-Host "Godot termino con error ($exitCode)." -ForegroundColor Red
        Show-LogTail $runStderr
        Show-LogTail $runStdout
        Show-LogTail $runGodotLog
        exit $exitCode
    }
}
catch {
    Write-Host 'Fallo al preparar o ejecutar el Jordan 191 K3:' -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    throw
}
