# Clean code — backlog de hallazgos diferidos

Origen: limpieza basada en `static-analysis-summary.json` (run `baseline`,
HEAD `64dab296ece6a1ea1509f54213c036ade5c867d2`).
Detalle del triage: `reports/static-analysis/baseline/TRIAGE.md`.

## CLEAN-01 — Migrar bincode 1.3.3 (RUSTSEC-2025-0141)

- Estado: **cerrado** (2026-09-11). El codec se migro de `bincode 1.3.3` a
  `postcard 1.1` (alternativa recomendada por el propio advisory). RUSTSEC-2025-0141
  marca como no mantenido todo el proyecto bincode (`patched = []`), incluido 2.x,
  por lo que la salida es otro codec serde-compatible.
- Cambios: `Snapshot::to_bytes`/`from_bytes` (game-sim) y
  `FacadeSnapshot::to_bytes`/`from_bytes` (formula90-core) usan
  `postcard::to_allocvec`/`from_bytes`; stubs de modulos y `tests/modules.rs`
  actualizados. Las supresiones de CLEAN-01 se retiraron de `suppressions.json`.
- Impacto: el formato de bytes cambia; no hay consumidor externo (Godot usa C
  structs, no decodifica el blob) ni golden files de snapshot. Los tests de
  paridad comparan ambas rutas con el mismo codec, asi que siguen siendo validos.
- Verificacion: `cargo tree -i bincode` no encuentra el paquete; `cargo-audit` y
  `cargo-deny` ya no reportan RUSTSEC-2025-0141 / `a:unmaintained`.

## CLEAN-02 — Auditoria de `unsafe` (cargo-geiger)

- Hallazgos originales (agregados por crate): vehicle-physics 1261 usos,
  formula90-core 473, vehicle-audio 337, game-sim 330, psx-art 75.
- Estado: **cerrado** (2026-09-11). Los bloques `unsafe` de `src/` quedaron
  documentados con comentarios `SAFETY:`; los crates con FFI aplican
  `#![deny(unsafe_op_in_unsafe_fn)]` (vehicle-physics-engine, vehicle-audio-engine,
  game-sim, formula90-core, psx-art-pluggin) y los crates puros
  `#![forbid(unsafe_code)]` (skybox-engine, dsp-abi-check; v10-engine-synth ya lo
  tenia). Los 5 findings de cargo-geiger se cerraron por agregado aceptado
  explicitamente en `scripts/clean-code/suppressions.json` (clave estable
  `tool|rule|file`, no depende del conteo que embebe el mensaje).
- Commits: `c9a05b16` (lints), `a678013e` (comentarios `SAFETY:`).
- Criterio de cierre cumplido: cada bloque `unsafe` con justificacion y sin
  hallazgos geiger pendientes (agregado aceptado explicitamente).


## CLEAN-03 — Calibrar regla semgrep unwrap/expect

- Estado: **cerrado** (2026-09-11). La regla se dividio por contexto:
  `formula90s-rust-unwrap-expect` (WARNING) ahora excluye `**/bin/**` y
  `**/build.rs` ademas de tests/benches/examples; la nueva
  `formula90s-rust-unwrap-expect-tooling` (INFO) cubre bins y build scripts.
- Politica: `unwrap()`/`expect()` es fail-fast aceptable en binarios, build
  scripts y tooling (informativo); en `src/` de produccion se mantiene como
  WARNING. Tras el split: 192 findings de produccion (WARNING) y 86 de tooling
  (INFO/note).
- Seguimiento: los 192 de produccion quedan como CLEAN-09.

## CLEAN-09 — Reducir unwrap/expect en codigo de produccion

- 192 findings `formula90s-rust-unwrap-expect` (WARNING) en `src/` no-tooling.
- Trabajo: sustituir por manejo explicito de Result/Option donde el panic no
  sea infalible; los casos demostrablemente infalibles pueden documentarse con
  `expect("razon")`.
- Criterio de cierre: sin findings WARNING de la regla de produccion (o
  justificados uno a uno).

## CLEAN-04 — Licencias y metadatos de dependencias

- Estado: **cerrado** (2026-09-11).
- Licencias: `license = "proprietary"` no es SPDX valido (originaba `l:parse-error`
  y `l:unlicensed`). Se sustituyo por `license = "LicenseRef-Proprietary"` y se
  agrego `publish = false` en los 8 crates del workspace; con `[licenses.private]
  ignore = true` de `deny.toml`, cargo-deny los trata como privados. Los crates
  `dsp-abi-check` y `v10-engine-synth` no tenian campo de licencia (`l:no-license-field`).
- Duplicado `syn`: `thiserror 1.0` usaba syn 2 mientras `serde_derive` usa syn 3.
  Se subio `thiserror` a 2.0 (que usa syn 3) en los 4 crates que lo usan; el grafo
  queda con una sola version de syn (3.0.3).
- Verificacion: `cargo-deny` deja solo el finding `a:unmaintained` de bincode
  (CLEAN-01, suprimido).

## CLEAN-05 — Complejidad (cargo-crap)

- Estado: **cerrado** (2026-09-11) con calibracion por contexto + cobertura parcial.
- Politica: `game/crates/.cargo-crap.toml` excluye tooling (`**/src/bin/**`,
  `**/build.rs`) del informe. Los binarios y build scripts no se ejecutan bajo
  `cargo llvm-cov`, asi que su CRAP (0% cobertura) no es senal de calidad. Misma
  separacion por contexto que la regla semgrep de CLEAN-03.
- Cobertura: tests nuevos para el ABI C (`formula90-core::ffi`: create/spawn/step/
  snapshot/audio/destroy + `parse_opts`/`surface_from_u32`) y helpers puros
  (`game_sim::c_abi::surface_from_u32`, `vehicle_config::surface_type_name`,
  `wav::write_mono_pcm16`).
- Resultado: 70 findings -> **42** (24 tooling + 4 por cobertura). Los 42 restantes
  son funciones de complejidad ciclomatica > 30 que exigen division; quedan como
  CLEAN-10.
- Nota: los tests de `vehicle-physics-engine` y `vehicle-audio-engine` no registran
  cobertura porque sus binarios de test fallan (aero/alloc), pendiente de corregir
  esos fallos.

## CLEAN-10 — Dividir funciones de alta complejidad (cargo-crap)

- Estado: **parcial** (2026-09-11). Se dividieron por responsabilidad 4 validadores
  de alta complejidad: `EngineConfig::validate` (cc56), `JsonVehicleSpec::validate`
  (cc57), `AeroModelConfig::validate` (cc53) y `apply_runtime_config_to_sim` (cc46),
  en helpers de cc <= 25.
- Resultado: 42 -> 39 findings.
- Bloqueo identificado: la mayoria de los 39 restantes tienen 0% de cobertura
  porque los binarios de test de `vehicle-physics-engine` (aero) y
  `vehicle-audio-engine` (alloc) fallan, y `cargo llvm-cov` no registra su
  cobertura. Con 0% de cobertura, cualquier funcion con cc > 5 supera el umbral,
  por lo que dividir mas no basta: hay que arreglar esos tests primero (CLEAN-11).
- Restan ademas 4 funciones con cc > 30 en rutas calientes (`mixer::render` cc72,
  `AudioPowertrainSynthesis::validate` cc41, `PowertrainState::process_shift` cc38,
  `AeroForces::step_with_kinematics` cc34) que requieren division con verificacion
  de paridad fisica.
- Criterio de cierre: sin findings cargo-crap en `src/` de produccion.

## CLEAN-11 — Arreglar tests que bloquean la cobertura

- Los binarios de test `vehicle-physics-engine --lib` (aero:
  `localized_bottoming_collapses_floor_before_wings`, `floor_ignores_probe_5_and_responds_to_rake`)
  y `vehicle-audio-engine --lib` (`gf509_render_has_zero_rust_allocations`) fallan,
  y `cargo llvm-cov` no registra su cobertura. Eso mantiene en 0% decenas de
  funciones y bloquea el cierre de CLEAN-10 (y la senal de cobertura en general).
- Trabajo: determinar si son expectativas desactualizadas o regresiones reales
  (aero/bottoming y asignaciones en render) y corregir.
- Criterio de cierre: `cargo test --workspace` sin fallos en esos binarios y
  cobertura registrada para vehicle-physics-engine/vehicle-audio-engine.


## CLEAN-06 — cppcheck cstyleCast

- Estado: **cerrado** (2026-09-11). Las 37 conversiones C-style se refactorizaron a
  casts nombrados `reinterpret_cast<...>` en `native/src/core/f90_core.cpp` (14),
  `native/src/vehicle/f1_94_rust_vehicle.cpp` (19) y
  `native/src/presentation/psx_art_controller.cpp` (4). Son casts de punteros ABI
  (`GetProcAddress` -> funcion tipada, `HMODULE` -> `void *`, buffer -> `const char *`),
  por lo que `reinterpret_cast` es el cast correcto.
- Verificacion: SCons compila los 3 archivos; cppcheck ya no reporta `cstyleCast`.

## CLEAN-07 — Test pre-existente roto en game-sim

- `cargo test -p game-sim --lib c_abi_roundtrip` fallaba con "TC active during
  launch" (`game-sim/src/c_abi.rs`), anterior a esta limpieza.
- Diagnostico: expectativa desactualizada. El perfil `f1_2026_2008` trae
  `traction_control_default_enabled: false`, asi que `AidsMask::from_config`
  desactiva TC; el test asumia TC activo por defecto. No es regresion del solver:
  con TC habilitado, `tc_active` se enciende.
- Estado: **cerrado** (2026-09-11). El test habilita TC una vez via el toggle
  edge-triggered y verifica que la telemetria `tc_active` se enciende.

## CLEAN-08 — Warning de arranque en cargo (perfiles)

- `cargo` avisaba "profiles for the non root package will be ignored" (5 crates).
- Estado: **cerrado** (2026-09-11). Los `[profile.release]` de
  vehicle-physics-engine, formula90-core, psx-art-pluggin, skybox-engine y
  vehicle-audio-engine se consolidaron en `game/crates/Cargo.toml`
  (`lto = true`, `opt-level = 3`). Al aplicarse por fin, el release del workspace
  usa LTO (antes los perfiles por crate se ignoraban). `cargo build --workspace
  --release` validado.
