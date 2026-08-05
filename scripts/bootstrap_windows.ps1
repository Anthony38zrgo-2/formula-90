[CmdletBinding()]
param([string]$GodotPath)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
function Find-Python {
    $cmd = Get-Command python -ErrorAction SilentlyContinue; if ($cmd) { return $cmd.Source }
    $bundled = Join-Path $env:USERPROFILE '.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe'
    if (Test-Path $bundled) { return $bundled }
    throw 'Python 3.8+ no encontrado. Instálelo desde https://python.org y reabra la terminal.'
}
$python = Find-Python
Write-Host "Repositorio: $root"
Write-Host "Python: $python"
if (-not (Get-Command git -ErrorAction SilentlyContinue)) { throw 'Git no encontrado.' }
git -c "safe.directory=$($root -replace '\\','/')" submodule update --init --recursive
$packages = Join-Path $root '.tools\python-packages'; New-Item -ItemType Directory -Force $packages | Out-Null
& $python -m pip install --disable-pip-version-check --target $packages 'scons==4.10.0'
$godotDir = Join-Path $root '.tools\godot'; New-Item -ItemType Directory -Force $godotDir | Out-Null
$godotExe = Join-Path $godotDir 'Godot_v4.7.1-stable_win64.exe'
if ($GodotPath) { if (-not (Test-Path $GodotPath)) { throw "Godot no existe: $GodotPath" }; $godotExe=$GodotPath }
elseif (-not (Test-Path $godotExe)) {
    $zip=Join-Path $godotDir 'godot.zip'; Invoke-WebRequest 'https://github.com/godotengine/godot-builds/releases/download/4.7.1-stable/Godot_v4.7.1-stable_win64.exe.zip' -OutFile $zip
    Expand-Archive $zip $godotDir -Force
}
Write-Host "Godot: $godotExe"
Write-Host 'Bootstrap completado.'

