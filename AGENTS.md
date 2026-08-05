# Guía para agentes

## Visión y alcance

`formula-90s` es un juego de carreras arcade inspirado conceptualmente en recreativas de los 90. El circuito es 3D y los vehículos se presentan con Sprite3D. Fase 1 prioriza siempre un proyecto compilable. Godot fijado: **4.7.1-stable**. `godot-cpp`: commit **7e18e40d7591429f915035a7de7cf79457d555cc** (línea v10, API 4.7).

## Arquitectura obligatoria

- Toda la lógica de runtime (flujo, coche, transmisión, cámara, HUD y reset) vive en C++20 mediante GDExtension.
- `native/` contiene runtime; `game/` escenas y recursos declarativos; `tools/` será Python auxiliar en Fase 2.
- No sustituir errores C++ con GDScript. No añadir assets sin licencia o extraídos de juegos.
- Alcance: este archivo rige todo; los `AGENTS.md` anidados concretan normas locales.

## Comandos

- Windows: `scripts/bootstrap_windows.ps1`, `scripts/build_windows.ps1`, `scripts/run_windows.ps1`, `scripts/test_windows.ps1`.
- Linux: equivalentes `.sh`.
- SCons directo: `python -m SCons platform=windows target=template_debug arch=x86_64`.

## Convenciones y terminado

- Tipos C++ en PascalCase; métodos/archivos snake_case; nodos con nombres estables PascalCase.
- Commits pequeños en imperativo; nunca inventar identidad ni publicar. No versionar binarios.
- Terminado significa: build y pruebas ejecutados, extensión cargada, documentación sincronizada y resultados reales informados. Nunca afirmar una ejecución no realizada.

