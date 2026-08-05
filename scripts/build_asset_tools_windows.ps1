[CmdletBinding()]
param()
$ErrorActionPreference='Stop'; $root=Split-Path -Parent $PSScriptRoot; Set-Location $root
$python=(Get-Command python -ErrorAction SilentlyContinue).Source
if(-not $python){$python=Join-Path $env:USERPROFILE '.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe'}
$env:PYTHONPATH=(Resolve-Path '.tools\python-packages')
& $python -m SCons -f tools/sprites/SConstruct build/tools/prepare_sprite.exe build/tools/inspect_sprite.exe build/tools/build_sprite_sheet.exe build/tools/sprite_tools_tests.exe build/tools/analyze_audio.exe build/tools/wav_analyzer_tests.exe build/tools/validate_assets.exe -j4
if($LASTEXITCODE -ne 0){throw "Build de herramientas falló ($LASTEXITCODE)."}
