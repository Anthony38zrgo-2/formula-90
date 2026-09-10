[CmdletBinding()]
param([string]$GodotPath)
$ErrorActionPreference='Stop'; $root=Split-Path -Parent $PSScriptRoot; Set-Location $root
& "$PSScriptRoot\build_windows.ps1"
$vsdev='C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat'
if(-not(Test-Path $vsdev)){throw 'Visual Studio 2022 C++ toolchain no encontrado.'}
$unit=Join-Path $root 'build\tests'; New-Item -ItemType Directory -Force $unit | Out-Null
$command='"'+$vsdev+'" -arch=x64 -host_arch=x64 >nul && cl /nologo /std:c++20 /EHsc /I"'+(Join-Path $root 'native\include')+'" "'+(Join-Path $root 'native\tests\unit_tests.cpp')+'" /Fo:"'+(Join-Path $unit 'unit_tests.obj')+'" /Fe:"'+(Join-Path $unit 'unit_tests.exe')+'" && "'+(Join-Path $unit 'unit_tests.exe')+'"'
cmd.exe /d /s /c $command; if($LASTEXITCODE -ne 0){throw 'Unit tests fallaron.'}

$godot=if($GodotPath){$GodotPath}elseif($env:GODOT_BIN){$env:GODOT_BIN}else{Join-Path $root '.tools\godot\Godot_v4.7.1-stable_win64_console.exe'}
if(-not(Test-Path $godot)){throw 'Godot console no encontrado para smoke tests.'}
$env:APPDATA=Join-Path $root '.tools\appdata'; $env:LOCALAPPDATA=Join-Path $root '.tools\localappdata'; New-Item -ItemType Directory -Force $env:APPDATA,$env:LOCALAPPDATA|Out-Null
& $godot --headless --editor --path (Join-Path $root 'game') --quit; if($LASTEXITCODE -ne 0){throw 'Smoke editor falló.'}
foreach($scene in @('scenes/ui/main_menu.tscn','scenes/runtime/world_hud_compositor.tscn','scenes/runtime/vehicle_test_session.tscn','scenes/vehicles/f1_2030_v10/f1_2030_v10_rust.tscn','scenes/ui/debug_hud.tscn')){Write-Host "Smoke: $scene";& $godot --headless --path (Join-Path $root 'game') $scene --quit-after 2;if($LASTEXITCODE -ne 0){throw "Smoke falló: $scene"}}
Write-Host "Ejecutando tests de Rust Vehicle Physics Engine..." -ForegroundColor Cyan
& cargo test --manifest-path (Join-Path $root 'game\crates\vehicle-physics-engine\Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw 'Rust vehicle physics tests fallaron.' }

foreach($script in @('res://tests/test_f1_2030_v10_rust_physics.gd', 'res://tests/smoke_test_f1_2030_v10_fuji76_77.gd', 'res://tests/smoke_test_f1_2030_v10_wheel_visual.gd', 'res://tests/smoke_test_mountains_3d.gd', 'res://tests/smoke_test_vehicle_audio.gd')){Write-Host "Smoke/Test script: $script";& $godot --headless --path (Join-Path $root 'game') --script $script;if($LASTEXITCODE -ne 0){throw "Test script failed: $script"}}
& "$PSScriptRoot\gate_hud_removed_telemetry_keys.ps1"
& "$PSScriptRoot\test_gdunit.ps1" -GodotPath $godot
Write-Host 'Todas las pruebas automáticas pasaron.' -ForegroundColor Green
