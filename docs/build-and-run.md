# Compilar y ejecutar

Para Fase 2, compile también las herramientas con `scripts/build_asset_tools_windows.ps1` (o su equivalente Linux). `scripts/test_windows.ps1` ejecuta unit tests del runtime, tests de sprites/WAV, `validate_assets` y smoke tests de Godot. El generador offline se prueba con `scripts/test_audio_tools_windows.ps1`.

Ejecute primero el bootstrap de su plataforma, luego build. Los scripts buscan Godot por argumento, `GODOT_BIN` y ubicaciones comunes. Windows usa MSVC x64; Linux usa GCC/Clang x86_64. Consulte README para comandos exactos.
