# Handoff: archivos canónicos de Fuji 76-77

Fecha de inventario: 2026-09-04

Este documento describe el contrato mínimo vigente para integrar y limpiar el repositorio. El circuito canónico actual es **Fuji Speedway 1976-1977**, identificado como `fuji76_77`. La implementación anterior de La Chutana queda como legado hasta que otro agente complete su limpieza y valide que no quedan referencias activas.

## 1. Fuente de conversión canónica (`D:\gtr2-glb-conversor`)

Entrada original que no debe eliminarse mientras se mantenga la capacidad de regenerar Fuji:

- `gtr2-tracks/Fuji76-77/Fuji76-77.scn`
- `gtr2-tracks/Fuji76-77/Fuji76-77.AIW`
- `gtr2-tracks/Fuji76-77/Fuji76-77.tdf`
- `gtr2-tracks/Fuji76-77/Fuji76-77.cam`
- `gtr2-tracks/Fuji76-77/Fuji76-77.gdb`
- `gtr2-tracks/Fuji76-77/Fuji76-77.mas`
- `gtr2-tracks/Fuji76-77/Fuji76-77Map.mas`

Código del contrato/exportación que sí es canónico para futuros circuitos:

- `scripts/track_contract.py` — contrato v1, spawn, superficies y perfiles visuales.
- `scripts/convert_track.py` — orquestador de conversión.
- `scripts/parse_scn.py`, `scripts/parse_aiw.py`, `scripts/parse_tdf.py` — parsers de entrada.
- `scripts/blender/import_track.py` — normalización Blender; el alpha de vegetación/billboards se exporta como cutout `MASK`.
- `scripts/blender/export_glb.py` — exportación visual y de colisión.
- `scripts/build_package_manifest.py` — hashes del paquete runtime.
- `scripts/validate_output.py` — validación del contrato.
- `profiles/fuji76_77/canonical.json` — perfil visual canónico de Fuji.

Salida canónica del conversor (`tracks-glb/fuji76_77/`):

- `fuji76_77_visual.glb` — geometría/render.
- `fuji76_77_collision.glb` — geometría física.
- `metadata/package.json` — contrato v1 y hashes; es la autoridad de integridad.
- `metadata/track.json` — metadatos del circuito y spawn.
- `metadata/surfaces.json` — superficies físicas normalizadas.
- `metadata/racing_line.json` — línea de carrera.
- `metadata/scene_manifest.json` — instancias, categorías, flags físicos y `visibility_groups`.
- `metadata/visual_profile.json` — máscara visual y exclusiones del perfil canónico.

`metadata/pit_lane.json`, `conversion_report.json` y `export_stats.json` son artefactos auxiliares. El agente de limpieza debe decidir si se conservan como documentación; no forman parte de los hashes declarados en `package.json` actualmente.

El perfil canónico vigente conserva la máscara completa (`visibility_mask = 0xFFFFFFFF`) y actualmente declara `POST00`–`POST09` como exclusiones. **Pendiente de corregir antes de cerrar el handoff:** el `scene_manifest.json` todavía contiene `cone009`–`cone017` con `is_render = true`; por tanto, esos conos no están excluidos por el perfil actual. Si la exclusión solicitada sigue vigente, cambiar `profiles/fuji76_77/canonical.json` a `cone009`–`cone017`, regenerar el paquete y copiar de nuevo ambos GLB/metadata al runtime.

## 2. Archivos canónicos dentro de Formula90s

Definición del circuito:

- `game/data/tracks/fuji76_77.tres`
- `game/scenes/tracks/fuji76_77/fuji76_77.tscn`
- `game/scenes/ui/fuji76_77_hud_map.tres`
- `game/scripts/track/imported_track_collision.gd`

Sesiones que apuntan al circuito canónico:

- `game/data/race_sessions/f1_94_fuji76_77.tres`
- `game/data/race_sessions/f1_2009_fw31_fuji76_77.tres`
- `game/data/race_sessions/f1_2026_2008_fuji76_77.tres`
- `game/scenes/runtime/vehicle_test_session.tscn`
- `game/scenes/runtime/vehicle_test_session_2009.tscn`
- `game/scenes/runtime/vehicle_test_session_2026.tscn`

Launcher canónico y smoke asociado:

- `scripts/run_f1_94.ps1`
- `game/tests/smoke_test_f1_94_fuji76_77.gd`

El launcher debe validar `game/tracks/fuji76_77/metadata/package.json` antes de iniciar Godot y debe seguir comprobando paridad `BUILD_SOURCE`/`HEAD`.

Paquete runtime canónico (`game/tracks/fuji76_77/`):

- `fuji76_77_visual.glb`
- `fuji76_77_collision.glb`
- `metadata/package.json`
- `metadata/track.json`
- `metadata/surfaces.json`
- `metadata/racing_line.json`
- `metadata/scene_manifest.json`
- `metadata/visual_profile.json`

Los archivos `.import` y las texturas `.png` extraídas por el importador de Godot son derivados regenerables del GLB. Antes de hacer un commit de limpieza, verificar si la política del repositorio los requiere; no tratarlos como otra fuente de verdad ni editarlos manualmente.

## 3. Vehículo canónico usado para la prueba rápida

La variante de vehículo canónica actual es `f1_2026_2008`:

- `game/data/vehicles/f1_2026_2008.tres`
- `game/scenes/vehicles/f1_2026_2008/f1_2026_2008_rust.tscn`
- `game/data/vehicles/f1_2026_2008/f1_2026_2008_physics.json`
- `game/assets/models/vehicles/f1-2026-2008/f1_2026_2008_chassis.glb`
- `game/assets/models/vehicles/f1-2026-2008/f1_2026_2008_wheel_front.glb`
- `game/assets/models/vehicles/f1-2026-2008/f1_2026_2008_wheel_rear.glb`
- `game/assets/models/vehicles/f1-2026-2008/manifest.json`
- `game/assets/models/vehicles/f1-2026-2008/vehicle_metadata.json`

Los `.png`, `.import`, `export_report.json` y `validation_report.json` del directorio del vehículo son derivados o reportes, salvo que una política posterior los declare explícitamente necesarios.

## 4. PSX temporalmente desactivado

- `game/scenes/runtime/world_hud_compositor.gd` contiene `psx_enabled = false` por defecto.
- `game/scenes/runtime/world_hud_compositor.tscn` mantiene `auto_apply = false` y no referencia el material PSX de presentación.
- Las tres escenas `vehicle_test_session*.tscn` fijan `psx_enabled = false`.

Esto es reversible: para reactivar PSX se debe restaurar el material/preset de presentación y cambiar el flag de forma coordinada. No eliminar todavía el addon ni los presets PSX.

## 5. Guía para el agente de limpieza antes del push de Formula90s

1. Registrar rama, `HEAD` y `git status`; no usar `reset --hard`, `checkout --` ni `git add -A`.
2. Mantener los archivos canónicos listados arriba y cualquier cambio de usuario no relacionado fuera del commit.
3. Buscar referencias activas a `la_chutana`, `la_chutana_generated`, `la_chutana_source_skybox` y assets de montañas. Clasificarlas como legado, test histórico o dependencia todavía usada; eliminar sólo las que ya no tengan consumidores.
4. No eliminar `game/scenes/tracks/test_field/`, `game/assets/generated/tracks/la_chutana/` ni sus metadatos hasta confirmar que ningún test, menú o sesión los carga.
5. Revisar si los `.import`, PNG extraídos y binarios generados de Fuji deben permanecer versionados. Si son regenerables y no están requeridos por el runtime limpio, retirarlos junto con sus referencias, nunca sólo uno de los pares.
6. Validar que los hashes de `game/tracks/fuji76_77/metadata/package.json` coincidan después de cualquier limpieza. Si cambia un archivo del paquete, regenerar el manifest; no editar hashes a mano.
7. Ejecutar como mínimo:

   ```powershell
   .\scripts\run_f1_94.ps1 -VehicleVariant 2026 -Smoke
   ```

   Debe terminar con `PASS` y validar spawn, superficies, cámara, HUD y contacto de ruedas.
8. Hacer un commit atómico sólo con la limpieza revisada. El `push` de Formula90s requiere gate humano posterior.

## 6. Estado de commits relacionados

- `D:\Formula90s`: `6c7a37e1` — integración Fuji y PSX temporalmente desactivado.
- `D:\gtr2-glb-conversor`: `6578973` — contrato GLB/perfil visual Fuji; ya publicado en `origin/main`.
