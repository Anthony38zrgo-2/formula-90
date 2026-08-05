[CmdletBinding()]
param()
$ErrorActionPreference='Stop';$root=Split-Path -Parent $PSScriptRoot;Set-Location $root
if(-not(Test-Path '.venv\Scripts\python.exe')){throw 'Ejecute setup_audio_tools_windows.ps1.'}
$env:TEMP=Join-Path $root 'build\audio_test_tmp';$env:TMP=$env:TEMP;New-Item -ItemType Directory -Force $env:TEMP|Out-Null
& .\.venv\Scripts\python.exe -m pytest tools\audio\tests --basetemp (Join-Path $env:TEMP 'pytest');if($LASTEXITCODE -ne 0){throw 'pytest falló'}
& .\.venv\Scripts\python.exe -m ruff check tools\audio;if($LASTEXITCODE -ne 0){throw 'ruff falló'}
