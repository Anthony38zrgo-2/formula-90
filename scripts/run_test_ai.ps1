[CmdletBinding()]
param(
    [string]$DriverTuningPath,
    [string]$OutputDirectory,
    [string]$GodotPath,
    [switch]$Headless
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$repositoryDirectory = Split-Path -Parent $PSScriptRoot
$driverProcess = $null
Push-Location $repositoryDirectory
try {
    $sourceIdentifier = (& git rev-parse HEAD).Trim()
    if ((Get-Content -LiteralPath (Join-Path $repositoryDirectory 'game/BUILD_SOURCE') -Raw).Trim() -ne $sourceIdentifier) {
        throw 'BUILD/HEAD mismatch. Rebuild native binaries with scripts/build_windows.ps1 before launching.'
    }
    if (-not $DriverTuningPath) {
        $DriverTuningPath = Join-Path $repositoryDirectory 'game/data/race_intelligence/unrestricted_driver_tuning.json'
    }
    $DriverTuningPath = (Resolve-Path -LiteralPath $DriverTuningPath).Path
    if (-not $OutputDirectory) {
        $OutputDirectory = Join-Path $repositoryDirectory ('scratch/test_ai_fuji_' + (Get-Date -Format 'yyyyMMdd_HHmmss_fff'))
    }
    $OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)
    if (Test-Path -LiteralPath $OutputDirectory) {
        throw 'The output directory must be new to preserve earlier test evidence.'
    }
    New-Item -ItemType Directory -Path $OutputDirectory | Out-Null
    git status --porcelain=v1 --untracked-files=all | Set-Content (Join-Path $OutputDirectory 'status_before.txt')
    $sourceIdentifier | Set-Content (Join-Path $OutputDirectory 'head_before.txt')
    git branch --show-current | Set-Content (Join-Path $OutputDirectory 'branch_before.txt')
    $driverManifest = Join-Path $repositoryDirectory 'game/crates/race-intelligence-engine/Cargo.toml'
    Write-Host 'Comprobando el ejecutable de IA con Cargo incremental...'
    & cargo build --release --manifest-path $driverManifest --bin autonomous_driver_bridge *> (Join-Path $OutputDirectory 'driver_build.log')
    if ($LASTEXITCODE -ne 0) {
        throw "Artificial driver build failed. See $OutputDirectory/driver_build.log"
    }
    $sourceTuningPath = $DriverTuningPath
    $DriverTuningPath = Join-Path $OutputDirectory 'driver_tuning.json'
    Copy-Item -LiteralPath $sourceTuningPath -Destination $DriverTuningPath
    $driverExecutable = Join-Path $repositoryDirectory 'game/crates/target/release/autonomous_driver_bridge.exe'
    & $driverExecutable --validate-tuning $DriverTuningPath
    if ($LASTEXITCODE -ne 0) { throw 'Invalid artificial-driver tuning JSON.' }
    $racingLinePath = Join-Path $repositoryDirectory 'game/tracks/fuji76_77/metadata/racing_line.json'
    $vehicleProfilePath = Join-Path $repositoryDirectory 'game/data/vehicles/f1_2030/profiles/geometric/vehicle.json'
    $steeringMappingPath = Join-Path $OutputDirectory 'steering_mapping.json'
    & $driverExecutable --write-steering-mapping $steeringMappingPath $repositoryDirectory $vehicleProfilePath
    if ($LASTEXITCODE -ne 0) { throw 'Could not derive the canonical steering conversion.' }
    $configuration = Get-Content -LiteralPath $DriverTuningPath -Raw | ConvertFrom-Json
    @{
        head = $sourceIdentifier
        driver_tuning_path = $DriverTuningPath
        driver_tuning_sha256 = (Get-FileHash -LiteralPath $DriverTuningPath -Algorithm SHA256).Hash
        driver_executable_sha256 = (Get-FileHash -LiteralPath $driverExecutable -Algorithm SHA256).Hash
        steering_mapping_sha256 = (Get-FileHash -LiteralPath $steeringMappingPath -Algorithm SHA256).Hash
        configuration = $configuration
    } | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $OutputDirectory 'launch_configuration.json')
    $driverArguments = @("`"$racingLinePath`"", '39194', "`"$DriverTuningPath`"", "`"$repositoryDirectory`"", "`"$vehicleProfilePath`"")
    $driverStandardError = Join-Path $OutputDirectory 'bridge_stderr.log'
    $driverProcess = Start-Process -FilePath $driverExecutable -ArgumentList $driverArguments -WorkingDirectory $repositoryDirectory -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $OutputDirectory 'bridge_stdout.log') -RedirectStandardError $driverStandardError
    $readyDeadline = [DateTime]::UtcNow.AddSeconds(10)
    while ($true) {
        $driverProcess.Refresh()
        $driverLog = if (Test-Path -LiteralPath $driverStandardError) { Get-Content -LiteralPath $driverStandardError -Raw } else { '' }
        if ($driverProcess.HasExited) { throw "Artificial driver exited before listening: $driverLog" }
        if ($driverLog -and $driverLog.Contains('AUTONOMOUS_DRIVER_LISTENING')) { break }
        if ([DateTime]::UtcNow -ge $readyDeadline) { throw 'Artificial driver did not become ready.' }
        Start-Sleep -Milliseconds 50
    }
    Write-Host "IA: utilizacion=$($configuration.grip_utilization_fraction), configuracion=$DriverTuningPath"
    Write-Host 'Abriendo Fuji con las fisicas simplificadas. Pulsa C para cambiar de camara.'
    Write-Host "Registros: $OutputDirectory"
    $powerShellExecutable = (Get-Process -Id $PID).Path
    $canonicalLauncher = Join-Path $PSScriptRoot 'run_f1_94.ps1'
    $canonicalArguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $canonicalLauncher, '-AutonomousDriverOutputDirectory', $OutputDirectory)
    if (-not $Headless) { $canonicalArguments += '-ObserveAutonomousDriver' }
    if ($GodotPath) { $canonicalArguments += @('-GodotPath', $GodotPath) }
    & $powerShellExecutable @canonicalArguments *> (Join-Path $OutputDirectory 'canonical_observation.log')
    $runtimeExitCode = $LASTEXITCODE
    @{ exit_code = $runtimeExitCode; head = $sourceIdentifier } | ConvertTo-Json | Set-Content (Join-Path $OutputDirectory 'exit_result.json')
    if ($runtimeExitCode -ne 0) { throw "Fuji AI test exited with code $runtimeExitCode. See $OutputDirectory/canonical_observation.log" }
} finally {
    if ($null -ne $driverProcess) {
        $driverProcess.Refresh()
        if (-not $driverProcess.HasExited) { Stop-Process -Id $driverProcess.Id -ErrorAction SilentlyContinue }
    }
    Pop-Location
}
