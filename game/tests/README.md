# Pruebas automatizadas de Godot

El proyecto fija GdUnit4 6.2.0 en `game/addons/gdUnit4`. Las pruebas nuevas viven
en `game/tests/gdunit` y se ejecutan en modo headless, con error ante fallos,
advertencias o nodos huérfanos.

## Ejecución

Windows:

```powershell
.\scripts\test_gdunit.ps1
```

Linux:

```bash
GODOT_BIN=/ruta/a/godot bash ./scripts/test_gdunit.sh
```

Ambos runners forman parte de `test_windows.ps1` y `test_linux.sh`. Los reportes
HTML y JUnit XML se generan bajo `game/reports/gdunit/`, que está ignorado por Git.

## Cobertura crítica inicial

- Contrato físico del Jordan Phase B: tres colisionadores, cuatro raycasts activos,
  separación válida entre ejes y valores canónicos del setup.
- Runtime generado de La Chutana: colisiones completas, vegetación visual sin
  colisión y glow desactivado.
- Clasificación recursiva y precedencia determinista de Road, Curb, Grass y Wall.
- Esquema de telemetría: 25 columnas, setup/procedencia en la misma fila y JSON
  válido con chasis, neumáticos, suspensión y motor.

La estabilidad dinámica del mundo completo no se ejecuta dentro del runner de
unit tests. Se valida por separado con el probe headless y ProcDump para conservar
una señal física y de crash nativo reproducible:

```powershell
.\scripts\debug_godot_headless.ps1 -Mode HandlingPhysics -PhysicsFrames 300
```
