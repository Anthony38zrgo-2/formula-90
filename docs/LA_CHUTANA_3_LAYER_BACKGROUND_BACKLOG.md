# Backlog: migracion del background de La Chutana a tres layers

Estado: `IN_PROGRESS` (BG3-001 a BG3-006, BG3-009 y BG3-010 completados y validados; BG3-007 pendiente de ajuste fino visual)  
Owner: race-presentation  
Alcance: solo presentacion visual; no fisicas, track mesh, vehiculo ni HUD.

## Resultado buscado

Reemplazar gradualmente el background actual de La Chutana por una composicion
configurable de tres capas independientes:

```text
back -> Sky -> FarMountains -> NearMountains -> world/track -> front
```

Cada preset debe declarar sus texturas, orden, parallax, escala, offset,
repeticion y pixel snap sin hardcodear esos valores en la escena del circuito.
La camara sera la unica fuente de movimiento visual.

## Inventario confirmado

Fuente canonica de arte congelada con manifiesto de procedencia formal:

| Layer | Asset v3 | Formato | SHA-256 |
|---|---|---:|---|
| `sky` | `assets-lowpoly-python/background/background_layering_codex/layers/la_chutana_sky_layer_v3_reference_edit.png` | 1280x720 RGB | `5fd5cc0574b1330c4e96a650fd2dd498736856a1801a8278ae528dc5dda2c036` |
| `far_mountains` | `assets-lowpoly-python/background/background_layering_codex/layers/la_chutana_far_mountains_layer_v3_reference_edit.png` | 1280x720 RGBA | `0deb3e705c9f75116af89fc680d8161bf86792917fd9c8c52c6979d40f195159` |
| `near_mountains` | `assets-lowpoly-python/background/background_layering_codex/layers/la_chutana_near_mountains_layer_v3_reference_edit.png` | 1280x720 RGBA | `3566b213c4079d9ad981918a934d595b498bb1615f0e31e534c6071a6d414937` |

Manifiesto de procedencia activo:
- `assets-lowpoly-python/background/background_layering_codex/generative_asset_manifest.json`

Assets de runtime promovidos:
- `game/assets/backgrounds/la_chutana_snes_day/background.json`
- `game/assets/backgrounds/la_chutana_snes_day/sky.png`
- `game/assets/backgrounds/la_chutana_snes_day/far_mountains.png`
- `game/assets/backgrounds/la_chutana_snes_day/near_mountains.png`

El sistema legacy permanece funcional como fallback durante la transicion:

- `game/scenes/tracks/test_field/la_chutana_generated.tscn`: panorama y la
  instancia `SourceSkyboxRig` (ocultado automaticamente en runtime si hay preset activo).
- `game/scenes/visuals/la_chutana_source_skybox.tscn`: ocho `Sprite3D` de
  montana y dieciseis cascadas animadas.
- `game/addons/formula90s/scripts/source_skybox_rig.gd`: seguimiento de camara
  en X/Z, solo visual.
- `game/tests/smoke_test_la_chutana_skybox.gd`: suite de regresion legacy (`PASS`).

## Restricciones de migracion

- No convertir estos PNG planos en panorama ni asumir que son tileables: la
  repeticion horizontal empieza desactivada hasta validar una version seam-safe.
- Usar filtrado `Nearest`, conservar alfa, evitar blur y mipmaps destructivos.
- No usar fisicas, velocidad, RPM ni suspension como entrada de parallax.
- Mantener el legacy como fallback hasta que el preset de tres layers supere
  pruebas estructurales y capturas visuales.
- La nueva arquitectura debe poder servir a otro circuito sin duplicar codigo.

## Backlog ordenado y estado

| ID | Pri. | Dependencia | Estado | Entregable y criterio de aceptacion |
|---|---|---|---|---|
| `BG3-001` | P0 | - | `DONE` | Congelar la procedencia de los tres PNG v3: crear `generative_asset_manifest.json`, registrar hashes, roles y revision visual; no regenerar ni sobrescribir assets. |
| `BG3-002` | P0 | `BG3-001` | `DONE` | Spike de render minimo: comparar compositor desacoplado compatible con Godot 4 contra tarjetas 3D camera-followed. Debe decidir un owner unico de movimiento, demostrar que el fondo queda detras del mundo y conservar pixel art. No tocar el legacy en esta fase. |
| `BG3-003` | P1 | `BG3-002` | `DONE` | Definir `BackgroundPreset` externo y un validador: ids unicos, depth unico, rutas existentes, escala positiva, `parallax_x/y`, offsets, `repeat_x/y`, `pixel_snap`, orden de dibujo. Fallos deben ser legibles, no un fondo negro silencioso. |
| `BG3-004` | P1 | `BG3-003` | `DONE` | Implementar `BackgroundController` y unidades de layer reutilizables (`BackgroundLayerInstance`). API minima: cargar preset, enlazar camara, activar/desactivar. Admite N layers futuros sin cambiar la logica base. |
| `BG3-005` | P1 | `BG3-004` | `DONE` | Crear preset `la_chutana_snes_day` con los tres assets v3 y parallax inicial configurable (`sky < far < near`). Integrarlo mediante configuracion de pista (`track_definition.gd` -> `la_chutana.tres`), no paths hardcodeados en `la_chutana_generated.tscn`. Conservar fallback legacy en `race_session.gd`. |
| `BG3-006` | P1 | `BG3-005` | `DONE` | Smoke tests de configuracion, carga, orden de los tres layers, filtros nearest, alfa y ausencia de nodos fisicos (`smoke_test_la_chutana_3_layer_background.gd`). Capturas de recta, giro izquierda (+30 deg) y giro derecha (-30 deg) con F1-94. Resultado: `PASS`. |
| `BG3-007` | P2 | `BG3-006` | `PROPOSED` | Ajuste visual contra `reference/00_original_reference.jpg`: horizonte, escala, offsets y diferencia perceptible de parallax. Evaluar seams y shimmering; si hay costura, crear una nueva version seam-safe editada desde los inputs, nunca estirar automaticamente la v3. |
| `BG3-008` | P3 | `BG3-007` | `PROPOSED` | Deprecar panorama, tarjetas y cascadas legacy solo despues de aceptacion visual explicita y regresion de La Chutana. Eliminar el contrato legacy y documentar rollback por preset. |
| `BG3-009` | P0 | - | `DONE` | Sustituir el skybox 2D por geometria 3D topografica: anillos `far_mountains_ring.glb` (1600m) y `near_mountains_ring.glb` (1150m) + `sky_dome.glb` con vertex colors (azul-dominante), generados desde SVG semantico (`data-elevation` como unica fuente de altura) via `generate_topo_terrain.py`. Modulo Rust offline `game/graphics/engine/skybox` (44 tests) como autoridad de validacion. Smoke test `smoke_test_mountains_3d.gd` `PASS` (8/8). |
| `BG3-010` | P0 | `BG3-009` | `DONE` | Corregir la geometria de las montanas 3D topograficas: redisenado SVG semantico v3 con cobertura 360° en 8 sectores y 42 contornos jerarquicos (25m a 150m); perfil facetado de 6 filas (apron, talus, cliff, summit, crest, skirt); escala Near (hasta 100m) y Far (hasta 225m); deteccion de 3 cascadas en canadas naturales; exportacion GLB y manifest.json actualizados; smoke test `smoke_test_mountains_3d.gd` `PASS` (8/8) y `cargo test` `PASS` (44/44). |

## Resumen de validacion y suites ejecutadas

- `BG3-001`: Validación de manifiesto generativo y hashes SHA-256 (`PASS`).
- `BG3-002`: `game/tests/spike_3_layer_background.gd` (`PASS`).
- `BG3-003`: `game/tests/test_background_preset_validator.gd` (9 tests unitarios, `PASS`).
- `BG3-004`: `game/tests/test_background_controller.gd` (6 tests unitarios, `PASS`).
- `BG3-005`: `game/tests/test_la_chutana_snes_day_preset.gd` (5 tests de integración, `PASS`).
- `BG3-006`: `game/tests/smoke_test_la_chutana_3_layer_background.gd` (`PASS`).
- `BG3-009`: `game/tests/smoke_test_mountains_3d.gd` (`PASS`, 8/8); `cargo test` en `game/graphics/engine/skybox` (`PASS`, 44 tests); validacion Python de normales, perfil de pendiente y gradiente del domo (`PASS`).
- `BG3-010`: Cobertura 360° y 42 contornos validados; Far range $[0.0, 225.0]\text{m}$, Near range $[0.0, 100.0]\text{m}$; perfil de 6 filas facetado; `smoke_test_mountains_3d.gd` (`PASS`, 8/8); `cargo test` (`PASS`, 44/44); análisis de elevación y perfil angular (`reports/background/mountains_3d_profile.png`).
- Regresión Legacy: `game/tests/smoke_test_la_chutana_skybox.gd` (`PASS`) y `smoke_test_race_session_matrix.gd` (`PASS`).

## Regresion y rollback

La baseline es el estado actual de `la_chutana_generated.tscn` y sus smokes.
Hasta `BG3-007`, el fallback legacy sigue activo cuando una pista no define
preset o si la carga falla.

La aceptacion final exige: tres texturas distintas, alpha correcto, nearest,
configuracion externa, parallax `sky < far < near`, sin seam inesperado en el
encuadre validado, y una comparacion visual aceptada por el usuario.
