# Brake-duct estático — split HUB y 4 GLB L/R (f1_2026_2008)

Fecha: 2026-09-07
Rama: `main-clean` · HEAD `8fbcedd1f1f07d66c3cbf126ea8a21ed5dbf7d7f`
Estado inicial: rama con 2 cambios preexistentes ajenos (`.agents/AGENTS.md`, `third_party/godot-cpp`) — inventariados, no se tocan ni se incluyen en commits de este item.

## Problema

1. `GEO_WHEEL_HUB` contiene el conducto de freno fusionado (105 vértices asimétricos inboard, cara interna, sesgo adelante/arriba; `avg (-0.205, +0.066, +0.043)` en ejes Blender). Como `Visual` cuelga de `Spinner` (`spinner.rotation.x = -ángulo`), el conducto orbita con la rueda.
2. Reutilizar 2 GLB en 4 ruedas con `rotation.y = PI` en izquierdas (`tscn:92,115`) canjea inboard/outboard pero también adelante/atrás → la toma queda atrás en un lado. La fuente Blender ya trae ese defecto (`WHEEL_*_L_CONTROL` con `rot.z = PI`).

## Decisión (usuario)

- Split en Blender: `RIM_SPIN` + `DUCT_STATIC`, reexportar.
- 4 GLB L/R dedicados (toma adelante en las 4, inboard correcto por lado), sin mirrors en escena.
- Alcance: las 4 ruedas.

## Diseño

- Entrada: `f1_2026_2008_wheel_front.glb` / `_rear.glb` actuales (se conservan intactos para rollback y para que `validate_2026_3500.py` siga en verde).
- Corte: polígonos con todos sus vértices en `x < -0.12` → ducto (delantero: 780 polys/751 verts; trasero: 636/636); resto (incl. 128 caras de transición por eje) → spin. Sin vértices en el plano de corte (hueco natural del histograma).
- Nuevos ficheros (6):
  - `f1_2026_2008_wheel_front_spin.glb`, `f1_2026_2008_wheel_rear_spin.glb` (RIM+TIRE, simétricos, compartidos L/R).
  - `f1_2026_2008_duct_FL.glb`, `_FR.glb`, `_RL.glb`, `_RR.glb` (L = mirror X exacto a nivel buffer: `x→-x`, `nx→-nx`, winding invertido).
- Escena: por rueda `CamberPivot -> DuctStatic -> DuctVisual (ducto L/R, identidad)` y `CamberPivot -> Spinner -> Visual (spin, identidad)`. Se eliminan los `rotation.y = PI`. `F1WheelVisualController` no cambia (sigue rotando solo `Spinner`).
- Tests: extender `smoke_test_f1_2026_2008_wheel_visual.gd`, nuevo `test_f1_brake_duct_static.gd`, nuevo validador Blender `validate_duct_split.py`.

## Criterio de aceptación

- Ducto inmóvil ante spin (`<1e-4 rad`), llanta gira.
- Tomas adelante (`-Z` Godot) e inboard correcto en las 4; 0 `rotation.y==PI`.
- `tris(SPIN)+tris(DUCT) == tris(HUB_orig)` por eje; spin `miss_rotX == 0`.
- Tests en verde + lanzamiento canónico `run_f1_94.ps1`.

## Resultado (2026-09-07, sin commit — gate humano pendiente)

- `tools/blender/split_wheel_duct_static.py` → 6 GLB nuevos; legacy intactos
  (hashes coinciden con `manifest.json`: `2D7C9CAD…`, `DAC334FC…`).
- Frente `5898 = 5118 + 780`; trasero `5754 = 5118 + 636`.
- `tools/blender/validate_duct_split.py` → `DUCT_VALIDATE_PASS`
  (spin simétrico, scoop delantero adelante en FL/FR, inboard correcto L/R).
- `game/tests/test_f1_brake_duct_static.gd` (nuevo) → 0 fallos.
- `test_f1_wheel_visual_positions.gd` → 0 fallos (sin cambios).
- `smoke_test_f1_2026_2008_wheel_visual.gd` (extendido) → PASS en escena real.
- `run_f1_94.ps1 -ValidateRuntimeOnly` → paridad BUILD/HEAD + manifest + Fuji OK.
- `F1WheelVisualController`: sin cambios (solo escribe `Spinner`; `DuctStatic`
  hereda steer/camber/suspensión por jerarquía).
- Preexistentes ajenos no tocados: `.agents/AGENTS.md`, `third_party/godot-cpp`.

## Corrección: solo la toma es freno; las tapas son rin y giran (2026-09-07)

Revisión visual (Blender, tapa en rojo) demostró que el corte por plano era
incorrecto: el HUB contiene tapas de rin simétricas + UNA toma de freno
desconectada (105 verts / 144 tris, toda la asimetría). Trasero sin toma.
`split_wheel_duct_static.py` ahora aísla por componentes/conectividad:
estático = toma delantera FL/FR (toma adelante, inboard por lado, sin compresión
— el saliente inboard es propio de la toma); giro = resto incl. tapas.
Traseros `DuctStatic` vacíos por diseño (sin `DuctVisual`). Se eliminó
`tuck_front_ducts.py` (obsoleto) y los ductos RL/RR. Frente `5898=5754+144`,
trasero `5754` íntegro. Validador, escena y tests actualizados; todo verde.
