# formula-90s

Prototipo de carreras arcade inspirado en la claridad y respuesta de recreativas de los 90. El circuito y el V10 jugable son 3D; la simulación, transmisión, cámara, HUD, audio y reset viven íntegramente en C++20 mediante GDExtension. No hay runtime Python ni GDScript.

La presentación del jugador usa un GLB real importado mediante una escena contenedora. Los sprites direccionales continúan disponibles para árboles, marshals, público, decoración, pruebas y el placeholder estático del circuito.

## Estado

Fase 2. Godot fijado en **4.7.1-stable** (`a13da4feb`). `godot-cpp` v10 está fijado al commit **`7e18e40d7591429f915035a7de7cf79457d555cc`**, construido con `api_version=4.7` y `build_profile.json`.

## Requisitos

- Git.
- Python 3.8+ y SCons 4.10.0.
- Godot 4.7.1 estable x86_64.
- Windows: Visual Studio 2022 con Desktop development with C++.
- Linux: GCC o Clang y herramientas de desarrollo.

## Preparación y compilación

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\bootstrap_windows.ps1
.\scripts\build_windows.ps1 -Configuration debug
```

```bash
./scripts/bootstrap_linux.sh
./scripts/build_linux.sh debug
```

No se requiere CMake ni recompilar Godot.

## Ejecución

```powershell
.\scripts\run_windows.ps1
```

```bash
GODOT_BIN=/ruta/godot ./scripts/run_linux.sh
```

## Controles

| Acción | Teclado | Mando |
|---|---|---|
| Acelerar | ↑ | gatillo derecho |
| Frenar / solicitar reversa | ↓ | gatillo izquierdo |
| Dirección | ←/→ | stick izquierdo |
| Subir marcha | A | — |
| Bajar marcha | Z | — |
| Cambio automático | 1 | — |
| Freno fuerte | Espacio | A |
| Reiniciar | R | Y |
| Volver al menú | Escape | Back |

## Arquitectura

- `native/`: runtime C++20 y pruebas deterministas.
- `game/`: proyecto Godot, escenas, recursos y assets procesados.
- `references/`: fuentes intactas y auditorías de procedencia.
- `docs/`: arquitectura, dirección de arte y decisiones.
- `scripts/`: bootstrap, build, ejecución y pruebas.
- `tools/`: pipelines offline; nunca runtime.

`ArcadeCarController` conserva la física existente. `VehicleVisual3DController` consume su pose interpolada y aplica únicamente escala, offset, giro de ruedas opcionales, roll, pitch y vibración visual. El GLB se escala bajo `VehicleVisualRoot`; la raíz física y su `BoxShape3D` no se escalan ni se sustituyen por la malla.

## V10 3D

- Fuente intacta: `references/vehicles/v10_3d/source/formula-v10.glb`.
- Asset importado: `game/assets/models/vehicles/v10/v10.glb`.
- SHA-256: `4cb73cdc216224cd869d8e43e74450cd54ee1d64bfc06f42d7da720673de1a72`.
- Escala visual: `4.0`, aproximadamente `1.77 × 1.05 × 3.99 m`.
- Convención: `+Y` arriba, `-Z` frente, `+X` derecha.

El archivo contiene una sola malla y no separa las ruedas; por tanto, la animación individual de ruedas queda desactivada de forma segura hasta recibir un GLB con nodos y pivotes independientes.

## Pruebas

```powershell
.\scripts\test_windows.ps1
```

```bash
GODOT_BIN=/ruta/godot ./scripts/test_linux.sh
```

La suite compila el runtime, ejecuta pruebas C++ deterministas, valida assets y carga en headless menú, campo, coche, HUD, visual 3D y escenas direccionales. La interacción completa se revisa con `tests/smoke/manual_checklist.md`.

## Solución de problemas

- Si falta `godot-cpp`, ejecute el bootstrap o `git submodule update --init --recursive`.
- Si falta SCons, ejecute el bootstrap; no es obligatorio instalarlo globalmente.
- Si la GDExtension no carga, construya la misma plataforma/configuración/arquitectura que Godot.
- Después de cambiar `build_profile.json`, reconstruya para regenerar los bindings.
- No edite la escena importada del GLB; ajuste `v10_visual_3d.tres` o la escena contenedora.

## Limitaciones actuales

La física es deliberadamente arcade y no simula neumáticos ni suspensión avanzada. El GLB actual no incluye UV, texturas, animaciones ni ruedas separadas. No hay IA, vueltas ni contenido completo de carrera.
