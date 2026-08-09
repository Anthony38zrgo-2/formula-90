param(
    [string]$PythonExe = "py"
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$pipeline = Join-Path $repo "blender\track_pipeline"
$venv = Join-Path $pipeline ".venv"

if (-not (Test-Path $venv)) {
    & $PythonExe -3 -m venv $venv
}
$python = Join-Path $venv "Scripts\python.exe"
& $python -m pip install --upgrade pip
& $python -m pip install -r (Join-Path $pipeline "requirements.txt")
Write-Host "Track pipeline ready: $python" -ForegroundColor Green
