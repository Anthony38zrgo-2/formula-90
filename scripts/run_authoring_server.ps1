[CmdletBinding()]
param(
    [int]$Port = 8400,
    [string]$Address = "127.0.0.1"
)

# Launches the local F90 Track Authoring web tool (Vue 3 + SVG.js editor
# served by the Python backend). The SVG document is canonical authority;
# the server sanitizes every save and reports validation without Blender.
#
# Usage:
#   .\scripts\run_authoring_server.ps1
#   .\scripts\run_authoring_server.ps1 -Port 9000

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$pipeline = Join-Path $repo "blender\track_pipeline"
$python = Join-Path $pipeline ".venv\Scripts\python.exe"
$server = Join-Path $pipeline "authoring\server.py"

if (-not (Test-Path $python)) { throw "Pipeline venv missing: $python" }
if (-not (Test-Path $server)) { throw "Authoring server missing: $server" }
if ($Port -lt 1 -or $Port -gt 65535) { throw "Port must be in 1..65535, got $Port" }

Write-Host "F90 Track Authoring"
Write-Host "  venv    : $python"
Write-Host "  server  : $server"
Write-Host ("  url     : http://{0}:{1}/" -f $Address, $Port)
Write-Host "  Stop    : Ctrl+C"

try {
    & $python $server "--host" $Address "--port" $Port
    exit $LASTEXITCODE
}
catch {
    Write-Host ("ERROR: failed to launch authoring server: {0}" -f $_.Exception.Message) -ForegroundColor Red
    exit 1
}
