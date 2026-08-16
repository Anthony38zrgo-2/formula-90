[CmdletBinding()]
param([ValidateSet('debug','release')][string]$Configuration='debug',[switch]$CompileCommands)
$ErrorActionPreference='Stop'; $root=Split-Path -Parent $PSScriptRoot; Set-Location $root
if (-not (Test-Path 'third_party\godot-cpp\SConstruct')) { throw 'godot-cpp ausente. Ejecute scripts/bootstrap_windows.ps1.' }
$python=(Get-Command python -ErrorAction SilentlyContinue).Source
if (-not $python) { $python=Join-Path $env:USERPROFILE '.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' }
if (-not (Test-Path $python)) { throw 'Python no encontrado.' }
$env:PYTHONPATH=(Resolve-Path '.tools\python-packages')
if (-not (Test-Path (Join-Path $env:PYTHONPATH 'SCons'))) { throw 'SCons ausente. Ejecute bootstrap_windows.ps1.' }
$target=if($Configuration -eq 'debug'){'template_debug'}else{'template_release'}
$args=@('-m','SCons','platform=windows',"target=$target",'arch=x86_64','api_version=4.7','build_profile=build_profile.json','-j4')
if($CompileCommands){$args+='compiledb=yes'}
Write-Host "Python: $python"; Write-Host "Target: $target / x86_64"; & $python @args
if($LASTEXITCODE -ne 0){throw "Build falló ($LASTEXITCODE)."}

Write-Host "Compilando Crate Rust Vehicle Physics Engine ($Configuration)..." -ForegroundColor Cyan
$cargoArgs = @('build', '--manifest-path', 'game/physics/engine/Cargo.toml')
if ($Configuration -eq 'release') { $cargoArgs += '--release' }
& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) { throw "Build de vehicle_physics_engine falló ($LASTEXITCODE)." }
$rustDllDir = if ($Configuration -eq 'release') { 'game/physics/engine/target/release' } else { 'game/physics/engine/target/debug' }
$destDllName = "vehicle_physics_engine.windows.$target.x86_64.dll"
Copy-Item (Join-Path $rustDllDir 'vehicle_physics_engine.dll') (Join-Path 'game/addons/formula90s/bin' $destDllName) -Force
Write-Host "Vehicle physics DLL copiada a game/addons/formula90s/bin/$destDllName" -ForegroundColor Green

