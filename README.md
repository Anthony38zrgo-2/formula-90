# Formula-90s

Prototipo de carreras de fórmula con una estética de consola de los años 90 y
un circuito de pruebas basado en La Chutana.

## Arquitectura actual

- **GEVP en GDScript** es la autoridad de la simulación del vehículo:
  `RigidBody3D`, ruedas raycast, suspensión, transmisión, contactos y ayudas.
- **C++20 mediante GDExtension** integra bootstrap, cámara arcade,
  presentación visual, audio DSP, reset y menú. Estos consumidores leen el
  estado de GEVP; no ejecutan física de vehículo paralela.
- **Mundo y HUD** se componen con `WorldHudCompositor`: el mundo se renderiza a
  640x360 y el HUD permanece nítido en el canvas raíz.

Consulta [docs/architecture.md](docs/architecture.md) para los límites de
responsabilidad actuales y `PROJECT_STATE.md` para el estado validado del
proyecto.

## Requisitos

- Git y el submódulo `third_party/godot-cpp`.
- Python 3.8+, SCons 4.10.0 y Godot 4.7.1 stable x86_64.
- En Windows, Visual Studio 2022 con Desktop development with C++.

## Preparación y compilación

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\bootstrap_windows.ps1
.\scripts\build_windows.ps1 -Configuration debug
```

El bootstrap inicializa `godot-cpp`. Los visuales Jordan se materializan desde
el bundle versionado mediante `scripts/run_jordan_handling.ps1`; no se guardan
como GLB generados en Git.

## Ejecución y pruebas

```powershell
.\scripts\run_windows.ps1
.\scripts\test_windows.ps1
```

La Fase 1 de REF-001 registró dos brechas de reproducibilidad de la suite:
el preflight de pruebas aún no materializa los visuales Jordan y
`native/tests/unit_tests.cpp` referencia un header inexistente. Véase
`docs/engineering/ref-001-phase-1-baseline.md` antes de interpretar esos
fallos como regresiones de gameplay.

## Documentación

- `PROJECT_STATE.md`: estado actual, restricciones y validación.
- `docs/architecture.md`: arquitectura de runtime vigente.
- `docs/game-design/`: dirección de producto y comportamiento deseado.
- `docs/decisions/`: decisiones históricas; no sustituyen el estado actual.
