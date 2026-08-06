[CmdletBinding()]
param([string]$GodotPath)
$ErrorActionPreference='Stop'; $root=Split-Path -Parent $PSScriptRoot; Set-Location $root
& "$PSScriptRoot\build_windows.ps1"
$vsdev='C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat'
if(-not(Test-Path $vsdev)){throw 'Visual Studio 2022 C++ toolchain no encontrado.'}
$unit=Join-Path $root 'build\tests'; New-Item -ItemType Directory -Force $unit | Out-Null
$command='"'+$vsdev+'" -arch=x64 -host_arch=x64 >nul && cl /nologo /std:c++20 /EHsc /I"'+(Join-Path $root 'native\include')+'" "'+(Join-Path $root 'native\tests\unit_tests.cpp')+'" /Fo:"'+(Join-Path $unit 'unit_tests.obj')+'" /Fe:"'+(Join-Path $unit 'unit_tests.exe')+'" && "'+(Join-Path $unit 'unit_tests.exe')+'"'
cmd.exe /d /s /c $command; if($LASTEXITCODE -ne 0){throw 'Unit tests fallaron.'}
& "$PSScriptRoot\build_asset_tools_windows.ps1"
& "$root\build\tools\sprite_tools_tests.exe"; if($LASTEXITCODE -ne 0){throw 'Pruebas de sprites fallaron.'}
& "$root\build\tools\wav_analyzer_tests.exe"; if($LASTEXITCODE -ne 0){throw 'Pruebas WAV fallaron.'}
& "$root\build\tools\validate_assets.exe" $root; if($LASTEXITCODE -ne 0){throw 'Validación de assets falló.'}
$godot=if($GodotPath){$GodotPath}elseif($env:GODOT_BIN){$env:GODOT_BIN}else{Join-Path $root '.tools\godot\Godot_v4.7.1-stable_win64_console.exe'}
if(-not(Test-Path $godot)){throw 'Godot console no encontrado para smoke tests.'}
$env:APPDATA=Join-Path $root '.tools\appdata'; $env:LOCALAPPDATA=Join-Path $root '.tools\localappdata'; New-Item -ItemType Directory -Force $env:APPDATA,$env:LOCALAPPDATA|Out-Null
& $godot --headless --editor --path (Join-Path $root 'game') --quit; if($LASTEXITCODE -ne 0){throw 'Smoke editor falló.'}
foreach($scene in @('scenes/ui/main_menu.tscn','scenes/tracks/test_field/test_field.tscn','scenes/vehicles/player_car.tscn','scenes/vehicles/static_directional_car.tscn','scenes/ui/debug_hud.tscn','scenes/tests/directional_sprite_validation.tscn','scenes/tests/v10_3d_validation.tscn')){Write-Host "Smoke: $scene";& $godot --headless --path (Join-Path $root 'game') $scene --quit-after 2;if($LASTEXITCODE -ne 0){throw "Smoke falló: $scene"}}
Write-Host 'Todas las pruebas automáticas pasaron.'
