[CmdletBinding()]
param(
    [ValidateSet('Runtime', 'HandlingPhysics', 'Scene', 'Import', 'RecoveryImport')]
    [string]$Mode = 'Runtime',
    [string]$GodotPath,
    [string]$Resource = 'res://assets/generated/tracks/la_chutana/la_chutana.glb',
    [string]$Scene = 'res://scenes/tracks/test_field/jordan_handling_test.tscn',
    [int]$PhysicsFrames = 300,
    [int]$QuitAfter = 30
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$game = Join-Path $repo 'game'
$godot = if ($GodotPath) {
    (Resolve-Path $GodotPath).Path
}
else {
    (Resolve-Path (Join-Path $repo '.tools\godot\Godot_v4.7.1-stable_win64.exe')).Path
}
$procdump = (Resolve-Path (Join-Path $repo '.tools\procdump\procdump64.exe')).Path
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$diagnosticRoot = Join-Path $repo "blender\generated\diagnostics\godot\${stamp}_$($Mode.ToLowerInvariant())"
$dumpDir = Join-Path $diagnosticRoot 'dumps'
$godotLog = Join-Path $diagnosticRoot 'godot.log'
$captureLog = Join-Path $diagnosticRoot 'procdump.log'
New-Item -ItemType Directory -Path $dumpDir -Force | Out-Null

$common = @(
    '--disable-crash-handler',
    '--verbose',
    '--headless',
    '--path', $game,
    '--log-file', $godotLog
)

$godotArguments = switch ($Mode) {
    'Runtime' {
        $common + @(
            '--single-threaded-scene',
            '--script', 'res://tools/diagnostics/collision_runtime_probe.gd',
            '--', $Resource
        )
    }
    'HandlingPhysics' {
        $common + @(
            '--single-threaded-scene',
            '--script', 'res://tools/diagnostics/jordan_physics_probe.gd',
            '--', $Scene, [string]$PhysicsFrames
        )
    }
    'Scene' {
        $common + @(
            '--single-threaded-scene',
            '--quit-after', [string]$QuitAfter,
            $Scene
        )
    }
    'Import' {
        $common + @('--import')
    }
    'RecoveryImport' {
        $common + @('--recovery-mode', '--import')
    }
}

$procdumpArguments = @(
    '-accepteula',
    '-ma',
    '-e',
    '-n', '1',
    '-x', $dumpDir,
    $godot
) + $godotArguments

Write-Host "Godot headless diagnostic / $Mode" -ForegroundColor Cyan
Write-Host "Godot log:   $godotLog"
Write-Host "ProcDump log: $captureLog"
Write-Host "Dump folder: $dumpDir"

$previousPreference = $ErrorActionPreference
try {
    $ErrorActionPreference = 'Continue'
    & $procdump @procdumpArguments 1> $captureLog 2>&1
    $procdumpExitCode = $LASTEXITCODE
}
finally {
    $ErrorActionPreference = $previousPreference
}

$dumps = @(Get-ChildItem -LiteralPath $dumpDir -Filter '*.dmp' -File -ErrorAction SilentlyContinue)
$probeSucceeded = $true
if ($Mode -eq 'Runtime') {
    $probeSucceeded = (Select-String -LiteralPath $godotLog, $captureLog -Pattern '^PROBE_RESULT' -Quiet -ErrorAction SilentlyContinue)
}
elseif ($Mode -eq 'HandlingPhysics') {
    $probeSucceeded = (Select-String -LiteralPath $godotLog, $captureLog -Pattern '^PHYSICS_PROBE_RESULT .*"passed":true' -Quiet -ErrorAction SilentlyContinue)
}

Write-Host "ProcDump exit: $procdumpExitCode"
Write-Host "Dumps:         $($dumps.Count)"
if ($Mode -in @('Runtime', 'HandlingPhysics')) {
    Write-Host "Probe result:  $probeSucceeded"
}

if ($dumps.Count -gt 0) {
    foreach ($dump in $dumps) {
        Write-Host "Crash dump: $($dump.FullName) ($($dump.Length) bytes)" -ForegroundColor Yellow
    }
    exit 20
}
if (-not $probeSucceeded) {
    Write-Host 'Godot did not produce a passing probe result. Inspect the phase markers and ProcDump log.' -ForegroundColor Red
    exit 21
}
if ($procdumpExitCode -ne 0) {
    exit $procdumpExitCode
}

Write-Host 'Headless diagnostic completed without a native crash.' -ForegroundColor Green
