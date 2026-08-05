[CmdletBinding()]
param()
$ErrorActionPreference='Stop';$root=Split-Path -Parent $PSScriptRoot;Set-Location $root
$python=(Get-Command python -ErrorAction SilentlyContinue).Source;if(-not $python){$python=Join-Path $env:USERPROFILE '.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe'}
if(-not(Test-Path $python)){throw 'Python 3.10+ no encontrado.'}
if(-not(Test-Path '.venv\Scripts\python.exe')){& $python -m venv .venv}
& .\.venv\Scripts\python.exe -m pip install --disable-pip-version-check -r requirements-dev.txt

