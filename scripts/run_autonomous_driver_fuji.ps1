[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$PhysicalWorldPackage,
    [string]$GodotPath,
    [string]$OutputDirectory,
    [string]$DriverTuningPath,
    [ValidateRange(1, 10)][int]$Laps = 1,
    [ValidateRange(30, 7200)][int]$MaximumSeconds = 600,
    [ValidateRange(1024, 65535)][int]$Port = 39194,
    [switch]$Rendered
)

$ErrorActionPreference = 'Stop'
$repositoryDirectory = Split-Path -Parent $PSScriptRoot
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $repositoryDirectory ('scratch/autonomous_driver_fuji_' + (Get-Date -Format 'yyyyMMdd_HHmmss'))
}
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $OutputDirectory) { throw 'Output directory must be new to preserve previous evidence.' }
$PhysicalWorldPackage = (Resolve-Path -LiteralPath $PhysicalWorldPackage).Path
if (-not $DriverTuningPath) { $DriverTuningPath = Join-Path $repositoryDirectory 'game/data/race_intelligence/solitary_driver_tuning.json' }
$DriverTuningPath = (Resolve-Path -LiteralPath $DriverTuningPath).Path
if (-not $GodotPath) { $GodotPath = Join-Path $repositoryDirectory '.tools/godot/Godot_v4.7.1-stable_win64_console.exe' }
New-Item -ItemType Directory -Path $OutputDirectory | Out-Null
git -C $repositoryDirectory status --porcelain=v1 --untracked-files=all | Set-Content (Join-Path $OutputDirectory 'status_before.txt')
git -C $repositoryDirectory rev-parse HEAD | Set-Content (Join-Path $OutputDirectory 'head_before.txt')
git -C $repositoryDirectory branch --show-current | Set-Content (Join-Path $OutputDirectory 'branch_before.txt')
& (Join-Path $PSScriptRoot 'run_f1_94.ps1') -GodotPath $GodotPath -ValidateRuntimeOnly *> (Join-Path $OutputDirectory 'canonical_preflight.log')
if ($LASTEXITCODE -ne 0) { throw 'Canonical runtime preflight failed.' }
$previousTargetDirectory = $env:CARGO_TARGET_DIR
$bridgeProcess = $null
try {
    $env:CARGO_TARGET_DIR = Join-Path $OutputDirectory 'cargo_target'
    $crateManifest = Join-Path $repositoryDirectory 'game/crates/race-intelligence-engine/Cargo.toml'
    & cargo build --release --manifest-path $crateManifest --bin autonomous_driver_bridge *> (Join-Path $OutputDirectory 'driver_build.log')
    if ($LASTEXITCODE -ne 0) { throw 'Artificial driver build failed.' }
    & cargo build --release --manifest-path (Join-Path $repositoryDirectory 'game/crates/vehicle-physics-engine/Cargo.toml') --lib *> (Join-Path $OutputDirectory 'physics_build.log')
    if ($LASTEXITCODE -ne 0) { throw 'Candidate physical world build failed.' }
    $bridgeExecutable = Join-Path $env:CARGO_TARGET_DIR 'release/autonomous_driver_bridge.exe'
    $physicsLibrary = Join-Path $env:CARGO_TARGET_DIR 'release/vehicle_physics_engine.dll'
    $racingLinePath = Join-Path $repositoryDirectory 'game/tracks/fuji76_77/metadata/racing_line.json'
    $sourceFiles = @(Get-ChildItem -LiteralPath (Join-Path $repositoryDirectory 'game/crates/race-intelligence-engine') -Recurse -File)
    $vehicleProfilePath = Join-Path $repositoryDirectory 'game/data/vehicles/f1_2030/profiles/geometric/vehicle.json'
    $sourceFiles += Get-ChildItem -LiteralPath (Split-Path -Parent $vehicleProfilePath) -Recurse -File
    $sourceFiles += Get-Item -LiteralPath $bridgeExecutable, $physicsLibrary, $PhysicalWorldPackage, $racingLinePath, $driverTuningPath
    $sourceFiles | Get-FileHash -Algorithm SHA256 | Select-Object Path, Hash | ConvertTo-Json | Set-Content (Join-Path $OutputDirectory 'source_and_binary_hashes.json')
    $bridgeArguments = @(('"' + $racingLinePath + '"'), [string]$Port, ('"' + $driverTuningPath + '"'), ('"' + $repositoryDirectory + '"'), ('"' + $vehicleProfilePath + '"'))
    $bridgeProcess = Start-Process -FilePath $bridgeExecutable -ArgumentList $bridgeArguments -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $OutputDirectory 'bridge_stdout.log') -RedirectStandardError (Join-Path $OutputDirectory 'bridge_stderr.log')
    $godotArguments = @('--path', (Join-Path $repositoryDirectory 'game'), '--script', 'res://tests/test_autonomous_driver_fuji.gd')
    if (-not $Rendered) { $godotArguments += '--headless' }
    $godotArguments += @('--', "--package=$PhysicalWorldPackage", "--physics-library=$physicsLibrary", "--output=$OutputDirectory", "--laps=$Laps", "--maximum-seconds=$MaximumSeconds", "--port=$Port")
    & $GodotPath @godotArguments 2>&1 | Tee-Object -FilePath (Join-Path $OutputDirectory 'godot_fuji.log')
    if ($LASTEXITCODE -ne 0) { throw "Fuji validation failed. Inspect $OutputDirectory" }
    Write-Host "Fuji validation passed. Evidence: $OutputDirectory"
} finally {
    if ($bridgeProcess -and -not $bridgeProcess.HasExited) { Stop-Process -Id $bridgeProcess.Id }
    $env:CARGO_TARGET_DIR = $previousTargetDirectory
}
