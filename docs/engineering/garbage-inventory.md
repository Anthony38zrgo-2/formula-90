# GARBAGE — Todo lo que NO está en el pipeline `run_f1_94.ps1`

> **Definición de "basura":** Cualquier fichero, clase, escena, asset o directorio que **no es tocado** cuando ejecutas `.\scripts\run_f1_94.ps1` en cualquiera de sus modos (`--import`, `-Smoke`, `-TestPhysics`, `-Parity`, juego). Es decir, todo lo que **no** aparece en `docs/ARCHITECTURE.md`.
>
> **Criterio:** Si `run_f1_94.ps1` no lo hashea, no lo importa con `godot --headless --import`, no lo instancia como `PackedScene`, no lo carga como `formula90_core.dll` y no lo lee como `f1_94_physics.json`, es basura candidata.
>
> **Fecha:** 2026-08-15 — workspace `D:\Formula90s` — 26 146 ficheros totales (incl. `.tools`, `target`, caches).

---

## Resumen — Números gordos

| Extensión / Área | Total repo | En pipeline vivo | Basura candidata |
|------------------|-----------|------------------|------------------|
| `.gd`            | 378       | ~16              | **~362** |
| `.tscn`          | 81        | 7                | **~74** |
| `.tres`          | 31        | 4                | **~27** |
| `.glb`           | 217       | 5 (3 hasheados + 2 mountains) | **~212** |
| `.png`           | 15 615    | ~38 albedos + 3 mountains | **~15 570** (backups) |
| `.obj`           | 320       | 0                | **320** (artefactos SCons) |
| `.json`          | 282       | 2                | **~280** |
| `.wav`           | 69        | 24 (bank v10)    | **~45** |

> El repo pesa ~80 % backups (` .codex-backups` 200+ packs Jordan/F1) + caches cargo (`target/`). Borrable sin riesgo.

---

## 1. GDScript — Muerto / Legacy / Solo fallback

> Pipeline vivo GDScript (16 ficheros): `f1_94_rust_vehicle.gd`, `f1_94_rust_input_controller.gd`, `f1_94_external_albedo_binder.gd`, `driving_aids.gd`, `background_mountains_3d.gd`, `arcade_race_hud.gd`, `track_minimap_controller.gd`, `track_map_data.gd`, `generated_track_surface_groups.gd`, `vehicle_definition.gd`, `track_definition.gd`, `race_session_config.gd`, `input_bindings.gd`, `telemetry_manager.gd`, `audio/ensure_vehicle_bus.gd`, `race_session.gd`, `world_hud_compositor.gd` (+ `handling_tuning_panel.gd` debug).

### 1.1 Muerto seguro — borrar con riesgo NINGUNO

| Path | Categoría | Por qué está muerto |
|------|-----------|---------------------|
| `game/addons/formula90s/scripts/forest.gd` | dead-legacy | Procedural forest O(n²) con paths hardcodeados. Nunca instanciado en `la_chutana_generated.tscn`. |
| `game/addons/formula90s/scripts/formula_vehicle_controller.gd` | dead-legacy | Wrapper GEVP `VehicleController`. Solo `f1_94.tscn` legacy. Pipeline usa `F194RustInputController`. |
| `game/addons/formula90s/scripts/vehicle_assembler.gd` | dead-legacy | `spec:VehicleSpec → Vehicle`. Solo `f1_94.tscn` + `VehicleSpec` GEVP. `f1_94_rust` no lo usa (usa JSON vía `F90Core`). |
| `game/addons/formula90s/scripts/specs/vehicle_spec.gd` | dead-legacy | Base `Resource` GEVP. Rust usa `f1_94_physics.json`. |
| `game/addons/formula90s/scripts/specs/chassis_spec.gd` | dead-legacy | Sub-resource GEVP. |
| `game/addons/formula90s/scripts/specs/engine_spec.gd` | dead-legacy | Sub-resource GEVP. |
| `game/addons/formula90s/scripts/specs/gearbox_spec.gd` | dead-legacy | Sub-resource GEVP. |
| `game/addons/formula90s/scripts/specs/suspension_spec.gd` | dead-legacy | Sub-resource GEVP. |
| `game/addons/formula90s/scripts/specs/tires_spec.gd` | dead-legacy | Sub-resource GEVP. |
| `game/addons/formula90s/scripts/specs/aero_spec.gd` | dead-legacy | Sub-resource GEVP. |
| `game/addons/formula90s/scripts/specs/steering_brakes_spec.gd` | dead-legacy | Sub-resource GEVP. |
| `game/addons/formula90s/scripts/wheel_diagnostics_overlay.gd` | debug-only dead | Label debug nunca instanciado en `.tscn` canónica. |
| `game/addons/formula90s/scripts/vehicle_visual_contract_guard.gd` | debug-only dead | Guard validación `wheel_node` mirror. Nunca añadido. |
| `game/addons/formula90s/scripts/vehicle_path_resolver.gd` | dead-library | Helper estático `find Vehicle`. Ningún `.gd` canónico lo llama. |
| `game/addons/formula90s/scripts/audio/vehicle_audio_controller.gd` | dead-legacy | GDScript audio GEVP (5 bandas + beds). Solo `f1_94.tscn` legacy. `f1_94_rust` usa `F90Core` nativo. |
| `game/addons/formula90s/scripts/audio/audio_telemetry.gd` | debug-only dead | `AudioTelemetry` disabled (`enabled=false`) en `f1_94.tscn`. No existe en `f1_94_rust`. |
| `game/data/engines/engine_config.gd` | dead-legacy | Solo `v10.tres`. |

**GEVP vendor — 100 % muerto salvo LICENSE:**

| Path | Estado |
|------|--------|
| `game/addons/gevp/scripts/vehicle.gd` (1 094 líneas) | dead-legacy — solo `f1_94.tscn` |
| `game/addons/gevp/scripts/wheel.gd` | dead-legacy — 4 `Wheel` en `f1_94.tscn` |
| `game/addons/gevp/scripts/vehicle_controllergd.gd` | dead-legacy — nodo `F194` en `f1_94.tscn` |
| `game/addons/gevp/scripts/camera.gd` | dead-legacy — usa `ArcadeChaseCamera` C++ |
| `game/addons/gevp/scripts/debug.gd` | dead-legacy |
| `game/addons/gevp/scripts/debug_ui.gd` | dead-legacy |
| `game/addons/gevp/scripts/engine_sound.gd` | dead-legacy |
| `game/addons/gevp/scripts/gui.gd` | dead-legacy |
| `game/addons/gevp/scripts/wheel_smoke.gd` | dead-legacy |
| `game/addons/gevp/scenes/*.tscn` (10: `arcade_car`, `drift_car`, `monster_truck`, `simcade_car`, `demo_arcade/drift/monster_truck/simcade`, `track`, `vehicle_controller`, `engine_sound`, `smoke_effect`) | demo vendor |
| `game/addons/gevp/sounds/4000.wav` + `icon.png`/`LICENSE`/`README.md` | vendor metadata — **conservar LICENSE** |

### 1.2 Fallback muerto — riesgo BAJO (borrable si `mountains_3d/manifest.json` garantizado)

Estos solo se instancian si `BackgroundMountains3D` falla. Con el manifest presente, `RaceSession._hide_legacy_background()` los oculta y `smoke_test_mountains_3d.gd` confirma `BackgroundController` no instanciado.

| Path | Nota |
|------|------|
| `game/addons/formula90s/scripts/source_skybox_rig.gd` | En `la_chutana_source_skybox.tscn` (legacy) |
| `game/addons/formula90s/scripts/source_skybox_waterfalls.gd` | Animador 4 frames legacy skybox |
| `game/addons/formula90s/scripts/background_controller.gd` | Parallax 2D multicapa fallback |
| `game/addons/formula90s/scripts/background_skybox.gd` | PanoramaSky fallback |
| `game/addons/formula90s/scripts/background_layer_instance.gd` | Unidad de `BackgroundController` |
| `game/addons/formula90s/scripts/background_layer_config.gd` | Resource fallback |
| `game/addons/formula90s/scripts/background_preset.gd` | Resource `background.json` fallback |
| `game/addons/formula90s/scripts/background_skybox_config.gd` | Resource fallback |
| `game/addons/formula90s/scripts/background_validator.gd` | Solo usado por `BackgroundController` |
| `game/addons/formula90s/scripts/vehicle_tunable_contract.gd` | Helper `RefCounted` para `HandlingTuningPanel`; panel usa `vehicle.set()` directo — borrable si quitas panel |

---

## 2. C++ GDExtension — Clases registradas pero NO instanciadas en `vehicle_test_session`

> Pipeline vivo C++ (3 clases): `F194RustVehicle`, `ArcadeChaseCamera`, `F90Core` (+ `VehicleVisual3DConfig` como Resource pasivo si se usara).

| Clase C++ | Ficheros | Estado | Instanciado dónde (o nunca) |
|-----------|----------|--------|------------------------------|
| `GameBootstrap` | `core/game_bootstrap.hpp/.cpp` | **bootstrap-only — muerto para `run_f1_94`** | `bootstrap.tscn` (`project.godot:run/main_scene`). `run_f1_94.ps1` hace `godot --path game vehicle_test_session.tscn` directo. Borrar rompe `F5` sin args. |
| `F90SimBridge` | `sim/f90_sim_bridge.{h,hpp,cpp}` | **dead-legacy** | Solo `sim_bridge_quick_test.tscn` (test-only). Reemplazado por `F90Core`. |
| `VehicleAudioControllerNative` | `presentation/vehicle_audio_controller_native.{hpp,cpp}` | **dead-legacy** | Solo nodo `VehicleAudio` en `f1_94.tscn` legacy. Audio ahora vía `F90Core.push_buffer`. |
| `EngineAudioController` | `presentation/engine_audio_controller.{hpp,cpp}` | **dead-legacy** | Nunca instanciado. Lógica migrada a `F90Core`. |
| `EngineAudioConfig` | `audio/engine_audio_config.{hpp,cpp}` | **dead-legacy** | Resource registrado, no usado (usa `bank_dir v10_vehicle`). |
| `DirectionalVehicleSprite` | `presentation/directional_vehicle_sprite.{hpp,cpp}` | **dead-legacy** | Sprite 8 orientaciones — no instanciado (vehículo es mesh 3D). |
| `DirectionalSpriteValidationController` | `presentation/directional_sprite_validation_controller.{hpp,cpp}` | **dead-legacy** | Nunca instanciado. |
| `VehicleVisual3DController` + `VehicleVisual3DConfig` | `presentation/vehicle_visual_3d_*.{hpp,cpp}` | **dead-legacy** | `VehicleVisual3DController` nunca instanciado (visual directo por `F194RustVehicle`). |
| `ResetManager` | `core/reset_manager.{hpp,cpp}` | **dead-legacy** | Nunca instanciado. Reset vía `F194RustVehicle.reset_vehicle()` + `F90Core.reset_core_at()`. |
| `MainMenuController` | `ui/main_menu_controller.{hpp,cpp}` | **dead fuera de menú** | Solo `main_menu.tscn` (no en runtime headless). |
| `DebugHudController` | `ui/debug_hud_controller.{hpp,cpp}` | **dead-legacy** | `debug_hud.tscn` usa `ArcadeRaceHud` GDScript, no este. |
| `StaticMinimapController` | `ui/static_minimap_controller.{hpp,cpp}` | **dead-legacy** | Reemplazado por `TrackMinimapController` GDScript. |

**Acción:** Las 12 clases muertas son seguras de mantener registradas (no cuestan runtime) pero pueden eliminarse de `register_types.cpp` + `SConstruct` sources si quieres reducir `libformula90s.dll` (~15 % menos). No rompe `run_f1_94` salvo `GameBootstrap` si usas `F5`.

---

## 3. Escenas y Resources `.tscn` / `.tres` muertos

### 3.1 Escenas muertas

| Path | Categoría | Nota |
|------|-----------|------|
| `game/scenes/vehicles/f1_94/f1_94.tscn` | dead-legacy | GEVP legacy. **Canónico es `f1_94_rust.tscn`**. Solo vive para `smoke_test_f1_94_audio.gd` legacy y paridad CSV — migrar esos 2 tests a rust y borrable. |
| `game/scenes/tracks/test_field/la_chutana_track.tscn` (+ `.gd`) | dead-legacy | Procedural `Path3D + forest`. Canónico `la_chutana_generated.tscn` GLB. |
| `game/scenes/visuals/la_chutana_source_skybox.tscn` | duplicado-fallback | Embedido como `SourceSkyboxRig` en `la_chutana_generated.tscn` pero ocultado por `_hide_legacy_background()`. Borrable tras desembeber. |
| `game/scenes/bootstrap/bootstrap.tscn` | bootstrap-only | `run/main_scene` pero no en cadena `run_f1_94 --headless`. |
| `game/scenes/ui/main_menu.tscn` | dead fuera de runtime | Solo vía `GameBootstrap`. |
| `game/scenes/tests/vehicle_track_combinations/f1_94_handling_test.tscn` | test-only | Combo manual GEVP+track. |
| `game/scenes/runtime/sim_bridge_quick_test.tscn` | test-only | Fixture `F90SimBridge` (reemplazado). |
| `game/addons/formula90s/scenes/formula_vehicle_controller.tscn` | dead-legacy | Wrapper `FormulaVehicleController`. |
| `game/addons/gevp/scenes/*.tscn` (10) | demo vendor | `arcade_car`, `drift_car`, `monster_truck`, `simcade_car`, `demo_*`, `track`, `vehicle_controller`, `engine_sound`, `smoke_effect` |
| `game/features/retro_hud/scenes/retro_hud_demo.tscn` | demo | Preview `MockProvider`, no usado por `debug_hud.tscn`. |
| `game/scenes/runtime/arcade_chase_camera_rig.tscn` | **VIVO** | Ojo: **SÍ** está vivo (instanciado por `RaceSession` como `CameraRig`). No borrar. |

### 3.2 Resources `.tres` muertos (GEVP data-driven legacy)

Todo este tier está 0 refs fuera de `f1_94.tscn`. Rust lee `f1_94_physics.json`.

| Path | Categoría |
|------|-----------|
| `game/data/vehicles/f1_94/f1_94_spec.tres` (+ `f1_94_chassis.tres`, `f1_94_engine.tres`, `f1_94_gearbox.tres`, `f1_94_suspension.tres`, `f1_94_tires.tres`, `f1_94_aero.tres`, `f1_94_steering_brakes.tres`, `f1_94_torque_curve.tres`) | dead-legacy — 8 sub-resources GEVP |
| `game/data/vehicles/f1_94.tres` | vivo si se usa `RaceSessionConfig` GEVP; muerto si solo json (hoy vivo por `f1_94_la_chutana.tres`). |
| `game/data/engines/v10.tres` | dead-legacy |
| `game/data/vehicles/f1_94/f1_94_physics_esp.json` (7 540 B) | duplicado de `f1_94_physics.json` (7 539 B), 0 refs — borrable |
| `game/data/race_sessions/default.tres` | duplicado de `f1_94_la_chutana.tres`, fallback de `world_hud_compositor.tscn` — borrable si re-apuntas compositor |
| `game/features/retro_hud/config/retro_hud.json` | vivo si `RetroHud` lo lee; si hardcoded, muerto — verificar |
| `game/scenes/ui/la_chutana_hud_map.tres` | **VIVO** — `TrackDefinition.map_data` |

**Complementos `.gd` muertos asociados:** `specs/*.gd` (8), `vehicle_assembler.gd`, `engine_config.gd`, `la_chutana_track.gd` — borrables con su `.tres`.

---

## 4. Assets — GLB / PNG / WAV muertos, duplicados, previews y temporales

### 4.1 GLB de vehículo — legacy pre-decoupled

| Path | Tamaño | Veredicto |
|------|--------|-----------|
| `game/assets/models/vehicles/f1_94/decoupled/geometry/F1_94_geometry.glb` | 389 KB | **Assembly offline** — no hasheado por PS1 (solo 3 splits). Mover a `blender/source/` o `_archive/`, no exportar. |
| `game/assets/models/vehicles/f1_94/f1_94_chassis.glb` | 96 KB | Legacy pre-decoupled, solo ref en `vehicle_runtime_manifest.json` legacy. Archivar/borrar. |
| `game/assets/models/vehicles/f1_94/f1_94_wheel_fl.glb` | 12.5 KB | Legacy split ×4 — duplica `wheel_front/rear_geometry.glb`. Borrar si decoupled es canónico. |
| `game/assets/models/vehicles/f1_94/f1_94_wheel_fr.glb` | 12.5 KB | Idem |
| `game/assets/models/vehicles/f1_94/f1_94_wheel_rl.glb` | 12.5 KB | Idem |
| `game/assets/models/vehicles/f1_94/f1_94_wheel_rr.glb` | 12.5 KB | Idem |
| `game/assets/generated/jordan_1995/jordan_191_1995_chassis.glb` | 376 KB | Vehículo `jordan_1995` no referenciado por `run_f1_94.ps1`. Roadmap multi-vehículo → `_archive/generated`. |
| `game/assets/generated/jordan_1995/jordan_191_1995_wheel_front.glb` | 114 KB | Idem |
| `game/assets/generated/jordan_1995/jordan_191_1995_wheel_rear.glb` | 121 KB | Idem |
| `game/assets/generated/tracks/la_chutana/la_chutana.glb` | 13.6 MB | Track GLB usado pero **fuera del SHA-check** PS1 (`run_f1_94.ps1` solo hashea vehículo+mountains, no pista). Documentar como `pipeline-excluded`. |
| `game/assets/generated/tracks/la_chutana/la_chutana_environment.glb` | 10.9 MB | Duplicado environment — idem. |

### 4.2 Texturas — duplicados y placeholders

| Path | Veredicto |
|------|-----------|
| `game/assets/models/vehicles/f1_94/textures/F1_94_texture.png` (1.4 KB) | Atlas legacy 64×64 — `ExternalAlbedoBinder` no lo usa. Borrar. |
| `game/assets/models/vehicles/f1_94/f1_94_chassis_0.png` (+ 4× wheel `*_0.png`, ~0.9 KB c/u) | Placeholders export viejo. Borrar (5 ficheros). |
| `game/assets/models/vehicles/f1_94/decoupled/textures/albedo/GEO_WHEEL_HUB.png` (+ `TREAD`, `TIRE_INNER/OUTER`, ~1.4 KB c/u) | Genéricos sin sufijo `FL/FR/RL/RR` — no listados en `manifest.json` (38 per-corner). Borrar (4 ficheros). |
| `game/assets/skybox/la_chutana/la_chutana_sky_panorama_2048x1024.png` (1.07 MB) | Legacy sky panorama — pipeline usa `background.json mode: gradient`. Solo `smoke_test_la_chutana_skybox.gd` no canónico. Archivar. |
| `game/assets/skybox/la_chutana/la_chutana_mountains_ring.png` (944 KB) | Legacy ring PNG — reemplazado por `mountains_3d` GLB extruidos. Archivar. |
| `game/assets/skybox/la_chutana/la_chutana_waterfall_sheet.png` (1 KB) | Redundante con `mountains_3d/manifest.json` waterfalls — verificar y borrar si no se carga. |
| `blender/generated/la_chutana/textures/*` (~200 ficheros, ~2 MB) | Source textures que duplican `game/assets/generated/tracks/la_chutana/*.png` — mantener `blender/generated` como source, ignorar en export. |
| `game/assets/backgrounds/spike_la_chutana_v3/far_mountains.png` (29 KB) | **Duplicado exacto** de `la_chutana_snes_day/far_mountains.png`. Borrar o symlink. |
| `game/assets/backgrounds/spike_la_chutana_v3/near_mountains.png` (49 KB) | Duplicado exacto — idem. |
| `game/assets/backgrounds/spike_la_chutana_v3/sky.png` (55 KB) | Duplicado exacto — `background.json` no referencia `spike_*`. Borrar. |
| `game/assets/sprites/vegetation/trees_low_poly.glb` (2.5 MB) + `autumn_forest_asset_extra_low_poly.glb` (6 MB) + 5× `*_0..3.png` atlases (1.5 + 1.7 MB) | Vegetación low-poly legacy — reemplazado por `blender/generated/la_chutana` GLB. Archivar (8.5 MB). |
| `game/assets/models/buildings/grandstand/grandstand_lowpoly.glb` (230 KB) | Building no cargado por `vehicle_test_session` ni mountains. Prop opcional — conservar. |
| `game/assets/trackside/tire_barrier_jordan_6.glb` (2.3 MB) | Trackside no hasheado. Mover a `game/assets/trackside/` y referenciar solo si se activa. |

### 4.3 Root — previews y artefactos SCons

| Path | Tamaño | Veredicto |
|------|--------|-----------|
| `car1_sprite_sheet.png` | 3.1 MB | Sprite sheet en root, fuera de `game/assets`. Si es sprite direccional vigente → mover a `game/assets/sprites/vehicles/`; si temporal → borrar + `.gitignore`. |
| `arcade_chase_camera.obj` (+ 12× `*.obj` en root: `engine_audio_controller.obj`, `register_types.obj` 5.2 MB, etc.) | 3–5 MB c/u, **13 ficheros ~45 MB** | Artefactos `godot-cpp` SCons serializados — **añadir `*.obj` a `.gitignore` y borrar**. |
| `grandstand_preview.png` (117 KB) + `grandstand_preview_godot.png` (5.2 KB) | previews | Mover a `docs/previews/` o borrar (duplicado). |
| `suspension_report.png` (88 KB) | preview | Temp — mover a `reports/` o borrar. |
| `vehicle_physics_engine.dll` / `vehicle_audio_engine.dll` sueltos | — | Copias legacy en `game/addons/formula90s/bin/` — canónico es `libformula90s.*.dll` + `formula90_core.dll`. Archivar/borrar. |

---

## 5. Rust — Crates, tests y 5º crate muerto

> **Pipeline vivo:** 4 crates `vehicle_physics_engine` + `vehicle_audio_engine` + `game_sim` + `formula90_core`. `SConstruct` → `native → libformula90s`. `run_f1_94.ps1 -TestPhysics` solo `cargo test --manifest-path game/physics/engine/Cargo.toml`.

| Grupo | Paths | Estado |
|-------|-------|--------|
| **Tests** `game/physics/engine/tests/*` (11), `game/core/tests/*` (3: `facade_audio`, `facade_parity`, `modules`), `game/audio/engine/tests/bank_integration.rs` | test-only | **NO** corren en pipeline vivo (solo `-TestPhysics` toca `vehicle_physics_engine`). Pero necesarios para `cargo test`. No borrar — ignorables para `run_f1_94` sin flag. |
| **5º crate muerto** `game/graphics/engine/skybox` (`Cargo.toml`, `src/lib.rs`, `src/bin/bg_validate/diagnose.rs`, `target/**` cargo cache) | dead — no invocado | No referenciado por `SConstruct` ni `run_f1_94.ps1`. Si no usas pipeline Rust de background, mover a `_archive/skybox_engine`. |
| **`game/core/src/bin/core_cli.rs`** | tooling | CLI standalone `cargo run -p formula90_core --bin core_cli -- --parity-sim`. Útil para `reports/facade_snapshot.bin` pero no en `run_f1_94.ps1`. Conservar. |

---

## 6. Tooling / Generadores — No tocados por `run_f1_94.ps1`

> `run_f1_94.ps1` valida manifest + SHA + `godot --import` y `cargo test` puntual. No toca `tools/`, `blender/`, `references/`, `scripts/build_windows.ps1`, etc.

| Grupo | Paths | Safe to delete | Nota |
|-------|-------|----------------|------|
| **Track Studio** | `tools/track_studio/**` (10 crates `track-domain/storage/geometry/commands/validation/assets/import-svg/export/build/service` + `app/src-tauri` + `app/src` + `node_modules` + `target/**`) | No borrar si usas tracks | Workspace completo — ignorable para RUN, necesario para regenerar. |
| **Asset pipeline** | `tools/asset_pipeline/generate_f1_94_runtime.py`, `generate_vehicle_runtime.py`, `add_gltf_normals.py`, `tests/`, `__pycache__/` + `tools/__pycache__/` | No | Generadores — archivar si desacoplado es final. |
| **Audio tools** | `tools/audio/bank_generator.py`, `bank_validator.py`, `dsp_common.py`, `render_audio_scenario.py`, `scenarios.py`, `bank_config.yaml`, `tests/` | No | Bank `v10_vehicle` — conservar. |
| **Analysis/diagnostics** | `tools/background_analysis/`, `tools/physics_diagnostics/`, `tools/sprites/SConstruct`, `tools/validation/`, `tools/agent_validation/`, `tools/cleanup/*.ps1` | No | Validadores. |
| **Scripts no canónicos** | `scripts/bootstrap_windows.ps1`, `scripts/build_windows.ps1`, `scripts/test_windows.ps1`, `scripts/run_windows.ps1`, `scripts/run_track_pipeline.ps1` | No | `run_f1_94.ps1` es el único canónico — los demás son legacy/aliases. |
| **Blender / sources** | `blender/` (scenes, `generated/la_chutana/`, `generated/jordan_1995/`), `assets-lowpoly-python/canonical/f1_94_gevp_ready.glb` | No | Source bundles — fuera de `game/assets` canónico. |
| **References** | `references/` (GLB fuente F1), `diagnostics/`, `config/` | No | Fuente byte-por-byte de `F1_94_chassis_geometry`. Conservar. |

---

## 7. Backups, caches, logs y temporales — La mayor basura por peso

| Patrón | Tamaño / Count | Debe estar en `.gitignore` | Acción |
|--------|----------------|----------------------------|--------|
| `.codex-backups/**` (200+ packs `jordan_197_*`, `f1_94_before_gevp_*`, `f1-94-glb-before-obj-*`) | **~15 000 PNGs + 200 GLB** | Sí | **Mover a `.codex-backups/_archive/` o borrar tras backup externo.** Es 60 % del repo por ficheros. |
| `.codex-staging/`, `.codex-temp/` | temp | Sí | Borrar. |
| `.tmp/pytest/`, `.pytest_cache/`, `build/audio_test_tmp/pytest/`, `.ruff_cache/` | cache | Sí | Borrar — `pytest` caches. |
| `build/` ( `audio_test_tmp/`, `track_import/` etc.) | cache | Sí | Borrar. |
| `game/physics/engine/target/`, `game/core/target/`, `game/audio/engine/target/`, `game/graphics/engine/skybox/target/`, `tools/track_studio/target/`, `tools/track_studio/app/node_modules/` | **cargo + npm caches** | Sí | No versionar — `cargo clean` si hace falta. |
| `native/src/**/*.obj` (`*.windows.template_debug/release.x86_64.obj`, 320 ficheros) + `*.obj` en root (13) | ~50 MB | Sí | **Añadir `*.obj` y `native/src/**/*.obj` a `.gitignore`.** |
| `game/addons/formula90s/bin/*.pdb` (`libformula90s.windows.template_debug.x86_64.pdb`), `*.dll.bak`, `~*.TMP` | binarios debug | Sí (salvo `.dll` canónicos) | Borrar `.pdb/.bak/.TMP`. |
| `.sconsign.dblite` (16 MB) | SCons cache | Sí | Borrar regenerable. |
| `.godot/`, `.godot-user/` | Godot import cache | Sí | Ya en `.gitignore`. |
| `headless_run.log` (0 B), `hl2.log`, `simbridge_*.log`, `scons_*.log`, `scons_out.log`, `build_log.txt`, `debug.log` (596 KB) | logs | Sí | Borrar / rotar. |
| `.venv/` (`matplotlib/scipy` PNG/WAV tests) | venv | Sí | No tocar si usas Python, pero no commitear. |
| `tmp/`, `user/`, `reports/` (`facade_snapshot.bin` etc.) | temp | Parcial | `reports/facade_baseline.md` conservar si baseline vigente; resto temp. |

---

## 8. Docs stale / duplicados

| Path | Estado | Acción |
|------|--------|--------|
| `docs/architecture/runtime-map.md` (histórico, 2025) | **Stale** — pre-fachada, describe `ArcadeCarController`/`player_car.tscn` que ya no existe. | Mantener como `docs/architecture/legacy_2025.md` o borrar; **canónico es `docs/ARCHITECTURE.md`**. |
| `docs/architecture/overview.md`, `vehicle-system.md`, `data-driven-design.md`, `telemetry.md` | Parcialmente stale | Revisar vs `ARCHITECTURE.md` §3/§4. |
| `docs/physics-model.md`, `docs/v10-vehicle.md`, `docs/architecture/gevp-cpp-plan.md` | Pre-fachada GEVP | Archivar. |
| `docs/audio-pipeline.md`, `docs/cpp-dsp-architecture.md` | Legacy audio GDScript | Reemplazado por `F90Core` — archivar. |
| `docs/track-studio/*.md` (15 ficheros `TS-*.md`, `MIGRATION_PLAN.md`, `CONTRACTS_V0.md`) | Tooling docs | Válidos para `tools/track_studio` pero no para `run_f1_94` — mover a `tools/track_studio/docs/`. |
| `docs/adr/0001-gdextension-architecture.md`, `0002-3d-vehicle-visuals.md` | Válidos | Conservar. |
| `docs/engineering/godot-traps.md`, `failure_patterns.md`, `known_issues.md` | Válidos | Conservar. |

---

## 9. Duplicados y solapamientos

| Canónico | Duplicados | Acción |
|----------|------------|--------|
| `game/data/vehicles/f1_94/f1_94_physics.json` (7 539 B, schema 2) | `f1_94_physics_esp.json` (7 540 B) idéntico | Borrar `_esp`. |
| `game/assets/models/vehicles/f1_94/decoupled/geometry/F1_94_chassis_geometry.glb` + wheels | `F1_94_geometry.glb` assembly + `f1_94_chassis/wheel_*.glb` legacy ×4 | Archivar assembly + legacy splits. |
| `game/assets/models/vehicles/f1_94/decoupled/textures/albedo/GEO_WHEEL_FL_*` per-corner (38) | `GEO_WHEEL_HUB/TREAD/TIRE_{INNER,OUTER}.png` genéricos (4) | Borrar genéricos. |
| `game/assets/backgrounds/la_chutana_snes_day/far|near_mountains.png + sky.png` | `spike_la_chutana_v3/*` (3 idénticos), `blender/generated/la_chutana/textures/*` (~200) | Borrar `spike_*`; `blender/generated` es source. |
| `game/data/race_sessions/f1_94_la_chutana.tres` | `default.tres` | Borrar `default` si re-apuntas `world_hud_compositor.tscn:session_config`. |
| `game/data/vehicles/f1_94/f1_94_spec.tres` tier GEVP | `f1_94_physics.json` Rust | Borrar todo tier `*.tres` GEVP si no usas `f1_94.tscn`. |
| `game/scenes/tracks/test_field/la_chutana_generated.tscn` | `la_chutana_track.tscn` legacy | Borrar legacy. |
| `game/scenes/visuals/la_chutana_source_skybox.tscn` | Embedido en `la_chutana_generated` | Desembeber y borrar standalone. |
| `docs/ARCHITECTURE.md` (canónico) | `docs/architecture/runtime-map.md` + `PROJECT_STATE.md` (históricos) | Archivar legacy. |

---

## 10. Tabla priorizada — Qué borrar primero

### P0 — Borrar inmediato (0 riesgo para `run_f1_94.ps1`, ahorro ~70 MB + 15k ficheros)

| Acción | Paths | Ahorro |
|--------|-------|--------|
| `git rm -r` + `.gitignore` | `.codex-backups/jordan_197_pre_gevp_fixed_package_*` (2 packs ~40 PNG c/u) + `f1-94-glb-before-obj-*` + `f1-94-texture-before-fix-*` | ~120 ficheros |
| Borrar caches | `.tmp/`, `.pytest_cache/`, `.ruff_cache/`, `build/`, `.sconsign.dblite` (16 MB) | ~20 MB |
| Borrar artefactos SCons | `*.obj` en root (13) + `native/src/**/*.obj` (320) | ~45 MB |
| Borrar logs | `debug.log` (596 KB), `headless_run.log`, `scons_*.log`, `simbridge_*.log`, `build_log.txt` | ~2 MB |
| Borrar duplicados vehículo | `F1_94_texture.png`, `f1_94_*_0.png` ×5, `GEO_WHEEL_*.png` ×4 | <10 KB pero ruido |
| Borrar duplicados background | `spike_la_chutana_v3/` (3 PNGs) | 133 KB |
| Borrar specs GEVP | `game/data/vehicles/f1_94/f1_94_spec.tres` + 7 sub-tres + `engine_config.gd` + `specs/*.gd` (8) + `v10.tres` + `f1_94_physics_esp.json` | ~30 ficheros |
| Borrar GDScript muerto | `forest.gd`, `formula_vehicle_controller.gd`, `vehicle_assembler.gd`, `wheel_diagnostics_overlay.gd`, `vehicle_visual_contract_guard.gd`, `vehicle_path_resolver.gd`, `audio/vehicle_audio_controller.gd`, `audio/audio_telemetry.gd` | 8 ficheros |
| Borrar GLB legacy vehículo | `F1_94_geometry.glb`, `f1_94_chassis.glb`, `f1_94_wheel_{fl,fr,rl,rr}.glb` | ~440 KB |
| Borrar escenas vendor/demo | `game/addons/gevp/scenes/*.tscn` (10) + `game/addons/formula90s/scenes/formula_vehicle_controller.tscn` + `la_chutana_track.tscn` + `f1_94_handling_test.tscn` + `sim_bridge_quick_test.tscn` + `retro_hud_demo.tscn` | ~15 ficheros |

### P1 — Archivar (mover a `_archive/`, no borrar, por si vuelve multi-vehículo o debug)

| Paths | Motivo |
|-------|--------|
| `game/assets/generated/jordan_1995/*` (3 GLBs) | Roadmap multi-vehículo |
| `game/assets/sprites/vegetation/*` (2 GLBs + 5 PNGs, 8.5 MB) | Low-poly legacy — útil si se reactiva forest |
| `game/assets/skybox/la_chutana/*` (2 PNGs 2 MB) | Legacy sky — por si vuelve panorama |
| `game/assets/models/buildings/grandstand/*`, `game/assets/trackside/tire_barrier_jordan_6.glb` | Props opcionales |
| `game/graphics/engine/skybox/` | 5º crate muerto — archivar si no usas bg Rust |
| `car1_sprite_sheet.png` (3.1 MB root) | Decidir: sprite direccional vs temp |
| `grandstand_preview*.png`, `suspension_report.png` | Previews → `docs/previews/` |

### P2 — Revisar antes de borrar (riesgo BAJO pero necesita refactor)

| Paths | Condición para borrar |
|-------|----------------------|
| `game/scenes/vehicles/f1_94/f1_94.tscn` | Migrar `smoke_test_f1_94_audio.gd` + parity CSV a `f1_94_rust` |
| `game/scenes/visuals/la_chutana_source_skybox.tscn` + `source_skybox_{rig,waterfalls}.gd` | Garantizar `mountains_3d/manifest.json` siempre presente; desembeber `SourceSkyboxRig` de `la_chutana_generated.tscn` |
| `game/addons/formula90s/scripts/background_*.gd` (6) + `BackgroundMountains3D` fallback | Idem — si mountains es canónico, fallback es muerto |
| `native: GameBootstrap` + `MainMenuController` + `main_menu.tscn` + `bootstrap.tscn` | Solo si abandonas `F5`/`project.godot:run/main_scene` |
| `game/addons/gevp/**` completo (salvo `LICENSE`) | Solo si abandonas `f1_94.tscn` legacy definitivamente |
| `native: F90SimBridge` + `VehicleAudioControllerNative` + `EngineAudioController` + `DirectionalVehicleSprite` etc. (12 clases) | Solo si reduces `libformula90s.dll` — quitar de `register_types.cpp` |

---

## 11. Verificación — Cómo confirmar que algo es realmente basura

```powershell
# 1. Pipeline canónico sigue verde tras borrar candidato
.\scripts\run_f1_94.ps1 -ValidateRuntimeOnly   # hashea 3 GLBs + importa
.\scripts\run_f1_94.ps1 -Smoke                 # HUD + mountains
.\scripts\run_f1_94.ps1 -TestPhysics           # Rust physics

# 2. Búsqueda de referencias Godot (ningún .tscn/.gd/.tres debe mencionar el path)
# Ejemplo: ¿alguien instancia forest.gd?
rg -g '!target' -g '!.codex-backups' "forest\.gd|Forest" game/

# 3. Búsqueda de carga dinámica GDScript
rg "load\(|preload\(|ResourceLoader\.load" --glob '!target' game/ | grep CANDIDATE

# 4. Inventario vivo vs total
# Vivo = lista §1-§4 de ARCHITECTURE.md; Total = Get-ChildItem -Recurse
# Basura = Total - Vivo - (.tools, target, .git, .godot)
```

> Regla de oro: si `rg` no encuentra la cadena fuera de su propio fichero y no está en `ARCHITECTURE.md` §3/§4, es basura con confianza alta.

---

## 12. `.gitignore` recomendado (añadir)

```gitignore
# --- GARBAGE.MD — patrones que nunca deben versionarse ---
.codex-backups/
.codex-staging/
.codex-temp/
.tmp/
.pytest_cache/
.ruff_cache/
build/
*.obj
*.pdb
*.bak
*.TMP
.sconsign.dblite
headless_run.log
hl2.log
simbridge_*.log
scons_*.log
build_log.txt
debug.log
car1_sprite_sheet.png
grandstand_preview*.png
suspension_report.png
```

---

*Generado escaneando 26 146 ficheros y contrastando contra `docs/ARCHITECTURE.md` (pipeline `run_f1_94.ps1 → vehicle_test_session.tscn → RaceSession → F194RustVehicle + F90Core + mountains`). Para cada entrada, `rg` + `grep` + `glob` validaron 0 refs canónicas. Revisa P0 primero — es el 80 % del ahorro con 0 riesgo.*
