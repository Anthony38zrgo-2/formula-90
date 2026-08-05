# formula-90s

Prototipo de carreras arcade con circuito 3D y vehículos Sprite3D, inspirado en la claridad y respuesta de recreativas de los 90 sin reutilizar sus recursos. La Fase 1 implementa menú, campo de pruebas, coche arcade, transmisión automática de seis marchas, cámara, HUD y reset íntegramente en C++20 mediante GDExtension.

## Estado

Fase 1, versión `0.1.0`. Godot fijado en **4.7.1-stable** (`a13da4feb`). `godot-cpp` v10 fijado al commit **`7e18e40d7591429f915035a7de7cf79457d555cc`**, construido con `api_version=4.7` y un perfil de clases reproducible. No se recompila el motor.

## Requisitos

- Git.
- Python 3.8+ y SCons 4.10.0 (el bootstrap instala SCons localmente).
- Godot 4.7.1 estable, edición estándar.
- Windows x86_64: Visual Studio 2022 con Desktop development with C++.
- Linux x86_64: GCC o Clang, Python venv y herramientas de desarrollo.

No se requiere CMake. No hay runtime Python ni GDScript.

## Preparación y compilación

PowerShell:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\bootstrap_windows.ps1
.\scripts\build_windows.ps1 -Configuration debug
```

El bootstrap descarga Godot oficial a `.tools/godot` si no se proporciona `-GodotPath`; inicializa el submódulo y fija SCons. Para release use `-Configuration release`. `-CompileCommands` solicita `compile_commands.json` cuando la toolchain lo admite.

Linux:

```bash
./scripts/bootstrap_linux.sh
./scripts/build_linux.sh debug
```

Instale Godot 4.7.1 estable con el gestor de su distribución o defina `GODOT_BIN=/ruta/a/godot`.

## Ejecución

```powershell
.\scripts\run_windows.ps1
# o: .\scripts\run_windows.ps1 -GodotPath C:\ruta\Godot_v4.7.1-stable_win64.exe
```

```bash
GODOT_BIN=/ruta/godot ./scripts/run_linux.sh
```

Resolución de Godot, en orden: argumento explícito, `GODOT_BIN`, binario local del bootstrap y rutas comunes limitadas. Los scripts fallan claramente si falta la biblioteca o la arquitectura no es x86_64.

## Controles

| Acción | Teclado | Mando |
|---|---|---|
| Acelerar | W / ↑ | gatillo derecho |
| Frenar / reversa | S / ↓ | gatillo izquierdo |
| Dirección | A/D / ←/→ | stick izquierdo |
| Freno fuerte | Espacio | A |
| Reiniciar | R | Y |
| Volver al menú | Escape | Back |

El menú toma foco inicial en `Comenzar` y usa navegación UI estándar de teclado/mando.

## Arquitectura

- `native/`: runtime C++20 y pruebas deterministas.
- `game/`: proyecto Godot, escenas, recursos y assets originales.
- `config/`: valores de referencia legibles fuera de Godot.
- `docs/`: diseño, modelo físico, ADR y roadmap.
- `scripts/`: bootstrap, build, ejecución y pruebas repetibles.
- `tools/`: placeholders de pipelines de Fase 2; nunca runtime.
- `third_party/godot-cpp`: submódulo fijado.

`GameBootstrap` mantiene el flujo. Las escenas describen composición; `ArcadeCarController`, `AutomaticTransmission`, `ArcadeChaseCamera`, `DirectionalVehicleSprite`, `DebugHudController` y `ResetManager` ejecutan el juego. El recurso `default_car_physics.tres` centraliza el ajuste.

## Pruebas

```powershell
.\scripts\test_windows.ps1
```

```bash
GODOT_BIN=/ruta/godot ./scripts/test_linux.sh
```

Se compilan tests C++ de marcha, RPM, histéresis, tiempo entre cambios, reversa/límites, dirección y drag. Los smoke tests cargan la extensión, abren el proyecto e instancian menú, campo, coche y HUD en headless. La interacción completa se verifica con `tests/smoke/manual_checklist.md`.

## Solución de problemas

- `godot-cpp ausente`: ejecute el bootstrap o `git submodule update --init --recursive`.
- `No module named SCons`: ejecute bootstrap; no instale globalmente por obligación.
- `cl/g++/clang++ no encontrado`: instale la carga C++ de VS 2022 o `build-essential`/Clang.
- `Godot no encontrado`: pase ruta explícita o defina `GODOT_BIN`.
- `GDExtension no compilada`: ejecute el build de la misma plataforma/configuración/arquitectura.
- Error de arquitectura: use Godot x86_64 y `arch=x86_64`.
- Después de cambiar `build_profile.json`, reconstruya; SCons regenerará bindings recortados.

## Limitaciones de Fase 1

Física plana y deliberadamente arcade; no hay suspensión, neumáticos avanzados, IA, vueltas ni contenido de carrera. El coche usa una silueta SVG original provisional de una sola vista. `EngineAudioController` funciona sin samples. `car1_sprite_sheet.png`, preexistente, no se usa ni versiona porque su licencia/procedencia no está establecida.

## Siguiente fase

Diseñar herramientas Python aisladas para sprites direccionales y preparación de audio: staging, validación de licencias, pivotes, escalas, transparencia y metadatos antes de promover assets al juego.

