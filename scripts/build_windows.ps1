[CmdletBinding()]
param([ValidateSet('debug','release')][string]$Configuration='debug',[switch]$CompileCommands)
$ErrorActionPreference='Stop'; $root=Split-Path -Parent $PSScriptRoot; Set-Location $root
$PSNativeCommandUseErrorActionPreference = $false
if (-not (Test-Path 'third_party\godot-cpp\SConstruct')) { throw 'godot-cpp ausente. Ejecute scripts/bootstrap_windows.ps1.' }
$python=(Get-Command python -ErrorAction SilentlyContinue).Source
if (-not $python) { $python=Join-Path $env:USERPROFILE '.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' }
if (-not (Test-Path $python)) { throw 'Python no encontrado.' }
$env:PYTHONPATH=(Resolve-Path '.tools\python-packages')
if (-not (Test-Path (Join-Path $env:PYTHONPATH 'SCons'))) { throw 'SCons ausente. Ejecute bootstrap_windows.ps1.' }
$target=if($Configuration -eq 'debug'){'template_debug'}else{'template_release'}
$args=@('-m','SCons','platform=windows',"target=$target",'arch=x86_64','api_version=4.7','build_profile=build_profile.json','-j4')
if($CompileCommands){$args+='compiledb=yes'}
Write-Host "Python: $python"; Write-Host "Target: $target / x86_64"; $pyOut = & $python @args 2>&1
if($LASTEXITCODE -ne 0){throw "Build fallÃ³ ($LASTEXITCODE).`n$pyOut"}

Write-Host "Compilando Crate Rust Vehicle Physics Engine ($Configuration)..." -ForegroundColor Cyan
$cargoArgs = @('build', '--manifest-path', 'game/crates/vehicle-physics-engine/Cargo.toml')
if ($Configuration -eq 'release') { $cargoArgs += '--release' }
& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) { throw "Build de vehicle_physics_engine fallo ($LASTEXITCODE)." }
$rustDllDir = if ($Configuration -eq 'release') { 'game/crates/target/release' } else { 'game/crates/target/debug' }
$destDllName = "vehicle_physics_engine.windows.$target.x86_64.dll"
Copy-Item (Join-Path $rustDllDir 'vehicle_physics_engine.dll') (Join-Path 'game/addons/formula90s/bin' $destDllName) -Force
Write-Host "Vehicle physics DLL copiada a game/addons/formula90s/bin/$destDllName" -ForegroundColor Green

Write-Host "Compilando Crate Rust game_sim (core autoritativo) ($Configuration)..." -ForegroundColor Cyan
$simArgs = @('build', '--manifest-path', 'game/crates/game-sim/Cargo.toml')
if ($Configuration -eq 'release') { $simArgs += '--release' }
& cargo @simArgs
if ($LASTEXITCODE -ne 0) { throw "Build de game_sim fallo ($LASTEXITCODE)." }
$simDllDir = if ($Configuration -eq 'release') { 'game/crates/target/release' } else { 'game/crates/target/debug' }
$simDest = "game_sim.windows.$target.x86_64.dll"
Copy-Item (Join-Path $simDllDir 'game_sim.dll') (Join-Path 'game/addons/formula90s/bin' $simDest) -Force
Copy-Item (Join-Path $simDllDir 'game_sim.dll') (Join-Path 'game/addons/formula90s/bin' 'game_sim.dll') -Force
Write-Host "game_sim DLL copiada a game/addons/formula90s/bin/$simDest (+ game_sim.dll)" -ForegroundColor Green

Write-Host "Compilando Crate Rust Vehicle Audio Engine ($Configuration)..." -ForegroundColor Cyan
$audioArgs = @('build', '--lib', '--manifest-path', 'game/crates/vehicle-audio-engine/Cargo.toml')
if ($Configuration -eq 'release') { $audioArgs += '--release' }
& cargo @audioArgs
if ($LASTEXITCODE -ne 0) { throw "Build de vehicle_audio_engine fallo ($LASTEXITCODE)." }
$audioDllDir = if ($Configuration -eq 'release') { 'game/crates/target/release' } else { 'game/crates/target/debug' }
$audioDest = "vehicle_audio_engine.windows.$target.x86_64.dll"
Copy-Item (Join-Path $audioDllDir 'vehicle_audio_engine.dll') (Join-Path 'game/addons/formula90s/bin' $audioDest) -Force
Copy-Item (Join-Path $audioDllDir 'vehicle_audio_engine.dll') (Join-Path 'game/addons/formula90s/bin' 'vehicle_audio_engine.dll') -Force
Write-Host "vehicle_audio_engine DLL copiada a game/addons/formula90s/bin/$audioDest (+ vehicle_audio_engine.dll)" -ForegroundColor Green

Write-Host "Compilando Crate Rust formula90_core (fachada-orquestador) ($Configuration)..." -ForegroundColor Cyan
$coreArgs = @('build', '--manifest-path', 'game/crates/formula90-core/Cargo.toml')
if ($Configuration -eq 'release') { $coreArgs += '--release' }
& cargo @coreArgs
if ($LASTEXITCODE -ne 0) { throw "Build de formula90_core fallo ($LASTEXITCODE)." }
$coreDllDir = if ($Configuration -eq 'release') { 'game/crates/target/release' } else { 'game/crates/target/debug' }
$coreDest = "formula90_core.windows.$target.x86_64.dll"
Copy-Item (Join-Path $coreDllDir 'formula90_core.dll') (Join-Path 'game/addons/formula90s/bin' $coreDest) -Force
Copy-Item (Join-Path $coreDllDir 'formula90_core.dll') (Join-Path 'game/addons/formula90s/bin' 'formula90_core.dll') -Force
Write-Host "formula90_core DLL copiada a game/addons/formula90s/bin/$coreDest (+ formula90_core.dll)" -ForegroundColor Green

Write-Host "Compilando Crate Rust psx_art_plugin ($Configuration)..." -ForegroundColor Cyan
$psxArgs = @('build', '--manifest-path', 'game/crates/psx-art-pluggin/Cargo.toml')
if ($Configuration -eq 'release') { $psxArgs += '--release' }
& cargo @psxArgs
if ($LASTEXITCODE -ne 0) { throw "Build de psx_art_plugin fallo ($LASTEXITCODE)." }
$psxDllDir = if ($Configuration -eq 'release') { 'game/crates/target/release' } else { 'game/crates/target/debug' }
$psxDest = "psx_art_plugin.windows.$target.x86_64.dll"
Copy-Item (Join-Path $psxDllDir 'psx_art_plugin.dll') (Join-Path 'game/addons/formula90s/bin' $psxDest) -Force
Copy-Item (Join-Path $psxDllDir 'psx_art_plugin.dll') (Join-Path 'game/addons/formula90s/bin' 'psx_art_plugin.dll') -Force
Write-Host "psx_art_plugin DLL copiada a game/addons/formula90s/bin/$psxDest (+ psx_art_plugin.dll)" -ForegroundColor Green


