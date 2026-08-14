[CmdletBinding()]
param(
    [string]$GodotPath,
    [switch]$ValidateRuntimeOnly,
    [string]$VehicleId = "jordan_197",
    [string]$Scene = "",
    [string]$Manifest = ""
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$game = Join-Path $root 'game'
function Resolve-VehicleScene([string]$VehicleId, [string]$Explicit) {
    if ($Explicit) { return $Explicit }
    $candidates = @(
        "res://scenes/vehicles/$VehicleId/${VehicleId}_handling_test.tscn",
        "res://scenes/tracks/test_field/${VehicleId}_handling_test.tscn",
        "res://scenes/tests/vehicle_track_combinations/jordan_197_handling_test.tscn",
        "res://scenes/tests/vehicle_track_combinations/jordan_191_handling_test.tscn",
        "res://scenes/tests/vehicle_track_combinations/jordan_handling_test.tscn"
    )
    foreach ($c in $candidates) {
        $disk = Join-Path $game ($c -replace 'res://','' -replace '/','\')
        if (Test-Path $disk) { return $c }
    }
    return $candidates[0]
}
function Resolve-VehicleManifest([string]$VehicleId, [string]$Explicit) {
    if ($Explicit) { return $Explicit }
    $candidates = @(
        "game\assets\models\vehicles\$VehicleId\vehicle_manifest.json",
        "game\scenes\vehicles\$VehicleId\vehicle_manifest.json"
    )
    foreach ($c in $candidates) {
        if (Test-Path (Join-Path $root $c)) { return Join-Path $root $c }
    }
    return $null
}
$scene = Resolve-VehicleScene $VehicleId $Scene
$manifestPath = Resolve-VehicleManifest $VehicleId $Manifest
$logDir = Join-Path $game 'logs'
$runGodotLog = Join-Path $logDir 'jordan_handling_godot.log'
$runStdout = Join-Path $logDir 'jordan_handling_stdout.log'
$runStderr = Join-Path $logDir 'jordan_handling_stderr.log'
$importGodotLog = Join-Path $logDir 'jordan_handling_import_godot.log'
$importStdout = Join-Path $logDir 'jordan_handling_import_stdout.log'
$importStderr = Join-Path $logDir 'jordan_handling_import_stderr.log'
$trackRuntimes = @(
    (Join-Path $game 'assets\generated\tracks\la_chutana\la_chutana.glb'),
    (Join-Path $game 'assets\generated\tracks\la_chutana\la_chutana_vegetation.glb')
)
$vehicleRuntimes = @()

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

function Resolve-ImportedRuntime([string]$runtime) {
    $importConfig = "$runtime.import"
    if (-not (Test-Path $importConfig -PathType Leaf)) {
        return $null
    }
    $descriptor = Get-Content -LiteralPath $importConfig -Raw
    $match = [regex]::Match($descriptor, 'path="res://([^\"]+\.scn)"')
    if (-not $match.Success) {
        return $null
    }
    $imported = Join-Path $game ($match.Groups[1].Value -replace '/', '\')
    if (-not (Test-Path $imported -PathType Leaf)) {
        return $null
    }
    return $imported
}

function Test-ImportedRuntimeFresh([string]$runtime, [string]$imported) {
    if (-not $imported) {
        return $false
    }
    $md5Path = [System.IO.Path]::ChangeExtension($imported, '.md5')
    if (-not (Test-Path $md5Path -PathType Leaf)) {
        return $false
    }
    $md5Descriptor = Get-Content -LiteralPath $md5Path -Raw
    $sourceMatch = [regex]::Match($md5Descriptor, 'source_md5="([0-9a-fA-F]{32})"')
    if (-not $sourceMatch.Success) {
        return $false
    }
    $runtimeMd5 = (Get-FileHash -Algorithm MD5 -LiteralPath $runtime).Hash
    return $sourceMatch.Groups[1].Value.Equals($runtimeMd5, [System.StringComparison]::OrdinalIgnoreCase)
}

New-Item -ItemType Directory -Path $logDir -Force | Out-Null
Remove-Item -LiteralPath $runGodotLog, $runStdout, $runStderr, $importGodotLog, $importStdout, $importStderr -Force -ErrorAction SilentlyContinue

try {
    if ($manifestPath -and (Test-Path $manifestPath)) {
        $manifestData = Get-Content $manifestPath -Raw | ConvertFrom-Json
        $vehicleRuntimes = @($manifestData.assets.PSObject.Properties | ForEach-Object {
            $rel = $_.Value.path
            $asset = Join-Path $root ($rel -replace '/','\')
            if (-not (Test-Path $asset)) { throw "Asset $VehicleId faltante: $asset (manifest $manifestPath)" }
            $expectedHash = $_.Value.sha256
            if ($expectedHash) {
                $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $asset).Hash
                if (-not $actualHash.Equals($expectedHash, [System.StringComparison]::OrdinalIgnoreCase)) {
                    throw "Hash SHA256 invalido para $asset (manifest $manifestPath)"
                }
            }
            $asset
        })
        Write-Verbose "Runtimes de vehiculo resueltos: $($vehicleRuntimes.Count)"
    } else {
        $k3Assets = @(
            (Join-Path $game 'assets\models\vehicles\f1_90s_canonical_1997\candidate_k3_historical\jordan_191_candidate_chassis.glb'),
            (Join-Path $game 'assets\models\vehicles\f1_90s_canonical_1997\candidate_k3_historical\jordan_191_candidate_wheel_front.glb'),
            (Join-Path $game 'assets\models\vehicles\f1_90s_canonical_1997\candidate_k3_historical\jordan_191_candidate_wheel_rear.glb')
        )
        foreach ($asset in $k3Assets) {
            if (-not (Test-Path $asset -PathType Leaf)) {
                Write-Warning "Asset K3 historico faltante (legacy fallback): $asset -- continuando con $VehicleId"
            }
        }
    }

    $godot = Resolve-Godot $GodotPath
    foreach ($trackRuntime in $trackRuntimes) {
        if (-not (Test-Path $trackRuntime -PathType Leaf)) {
            throw "Runtime canonico de La Chutana faltante: $trackRuntime"
        }
    }
    $dll = Join-Path $game 'addons\formula90s\bin\libformula90s.windows.template_debug.x86_64.dll'
    if (-not (Test-Path $dll)) {
        throw 'GDExtension no compilada. Ejecute .\scripts\build_windows.ps1 -Configuration debug.'
    }

    # Provenance consumed by TelemetryManager when it opens the session CSV.
    # Keep this best-effort so launching a source archive remains possible.
    $env:FORMULA90S_GIT_COMMIT = ''
    $env:FORMULA90S_GIT_BRANCH = ''
    try {
        $env:FORMULA90S_GIT_COMMIT = (git -C $root rev-parse HEAD 2>$null).Trim()
        $env:FORMULA90S_GIT_BRANCH = (git -C $root branch --show-current 2>$null).Trim()
    }
    catch {
        Write-Verbose 'No se pudo obtener la procedencia Git para la telemetria.'
    }

    Write-Host "Formula-90 / $VehicleId handling" -ForegroundColor Cyan
    Write-Host "Godot:    $godot"
    Write-Host "Proyecto: $game"
    Write-Host "Escena:   $scene"
    if ($manifestPath) { Write-Host "Manifest: $manifestPath" -ForegroundColor DarkGray }
    Write-Host 'Sincronizando importacion de La Chutana y del vehiculo...' -ForegroundColor Cyan

    $runtimeAssets = @($trackRuntimes) + @($vehicleRuntimes)
    Write-Verbose "Runtimes totales a validar: $($runtimeAssets.Count)"
    $requiresImport = $false
    foreach ($runtimeAsset in $runtimeAssets) {
        $importedRuntime = Resolve-ImportedRuntime $runtimeAsset
        if (-not $importedRuntime) {
            $requiresImport = $true
            break
        }
        if (-not (Test-ImportedRuntimeFresh $runtimeAsset $importedRuntime)) {
            $requiresImport = $true
            break
        }
    }

    if ($requiresImport) {
        $previousPreference = $ErrorActionPreference
        try {
            # Opening the scene directly can reuse a stale .godot/imported PackedScene
            # after either the track or vehicle pipeline replaces a canonical GLB.
            # Import first and wait for Godot before starting the handling session.
            $ErrorActionPreference = 'Continue'
            & $godot --headless --path $game --log-file $importGodotLog --import 1> $importStdout 2> $importStderr
            $importExitCode = $LASTEXITCODE
        }
        finally {
            $ErrorActionPreference = $previousPreference
        }

        if ($importExitCode -ne 0) {
            Write-Host "Godot no pudo importar los runtimes de pista y vehiculo ($importExitCode)." -ForegroundColor Red
            Show-LogTail $importStderr
            Show-LogTail $importStdout
            Show-LogTail $importGodotLog
            exit $importExitCode
        }
    }
    else {
        Write-Host 'Caches de pista y vehiculo vigentes; no se requiere reimportar.' -ForegroundColor DarkGreen
    }

    foreach ($runtimeAsset in $runtimeAssets) {
        $runtimeImportConfig = "$runtimeAsset.import"
        if (-not (Test-Path $runtimeImportConfig -PathType Leaf)) {
            throw "Godot no genero el descriptor de importacion: $runtimeImportConfig"
        }
        $importedRuntime = Resolve-ImportedRuntime $runtimeAsset
        if (-not $importedRuntime) {
            throw "No se pudo resolver el PackedScene importado desde: $runtimeImportConfig"
        }
        if (-not (Test-ImportedRuntimeFresh $runtimeAsset $importedRuntime)) {
            throw "La cache importada no corresponde al GLB canonico: $importedRuntime"
        }
        Write-Host "Runtime importado: $importedRuntime" -ForegroundColor Green
    }
    if ($ValidateRuntimeOnly) {
        Write-Host 'Validacion del runtime canonico completada; lanzamiento omitido.' -ForegroundColor Green
        return
    }
    Write-Host "Iniciando La Chutana con $VehicleId (via VehiclePathResolver)..." -ForegroundColor Cyan

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
    Write-Host "Fallo al preparar o ejecutar $VehicleId :" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    throw
}
