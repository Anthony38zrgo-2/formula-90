[CmdletBinding()]
param(
    [ValidateRange(1, 65535)]
    [int]$Port = 5173,

    [ValidateRange(1, 65535)]
    [int]$ApiPort = 8765,

    [string]$ListenAddress = "127.0.0.1"
)

$ErrorActionPreference = "Stop"
$appDirectory = $PSScriptRoot
$packageManifest = Join-Path $appDirectory "package.json"
$dependencyDirectory = Join-Path $appDirectory "node_modules"

if (-not (Test-Path -LiteralPath $packageManifest -PathType Leaf)) {
    throw "No se encontró package.json en $appDirectory"
}

$nodeCommand = Get-Command node.exe -ErrorAction SilentlyContinue
if (-not $nodeCommand) { $nodeCommand = Get-Command node -ErrorAction SilentlyContinue }
if (-not $nodeCommand) { throw "Node.js no está disponible." }
$npmCommand = Get-Command npm.cmd -ErrorAction SilentlyContinue
if (-not $npmCommand) { $npmCommand = Get-Command npm -ErrorAction SilentlyContinue }
if (-not $npmCommand) { throw "npm no está disponible." }
$pythonCommand = Get-Command python.exe -ErrorAction SilentlyContinue
if (-not $pythonCommand) { $pythonCommand = Get-Command python -ErrorAction SilentlyContinue }
if (-not $pythonCommand) { throw "Python no está disponible." }

Push-Location $appDirectory
try {
    if (-not (Test-Path -LiteralPath $dependencyDirectory -PathType Container)) {
        Write-Host "Instalando dependencias desde package-lock.json..."
        & $npmCommand.Source ci
        if ($LASTEXITCODE -ne 0) { throw "npm ci terminó con código $LASTEXITCODE" }
    }

    & $nodeCommand.Source ".\dev_runner.mjs" "--port" "$Port" "--api-port" "$ApiPort" "--host" "$ListenAddress" "--python" "$($pythonCommand.Source)"
    if ($LASTEXITCODE -ne 0) { throw "Vehicle Studio terminó con código $LASTEXITCODE" }
}
finally {
    Pop-Location
}
