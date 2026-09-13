# Clean code — backlog de hallazgos diferidos

Origen: limpieza basada en `static-analysis-summary.json` (run `baseline`,
HEAD `64dab296ece6a1ea1509f54213c036ade5c867d2`).
Detalle del triage: `reports/static-analysis/baseline/TRIAGE.md`.

## Estado (2026-09-12)

- **Cerrados:** CLEAN-01, CLEAN-02, CLEAN-03, CLEAN-04, CLEAN-05, CLEAN-06,
  CLEAN-07, CLEAN-08, CLEAN-09, CLEAN-11.
- **Parcial — falta completar:** CLEAN-10 (complejidad). cargo-crap bajo de 70 a
  **15 findings** reales (Sprint 0, HEAD `5e173557`) y luego a **12** (Sprint 1).
  Resta:
  1. dividir 4 funciones con cc > 30 en rutas calientes (`mixer::render` cc72,
     `AudioPowertrainSynthesis::validate` cc41, `PowertrainState::process_shift`
     cc38, `AeroForces::step_with_kinematics` cc34) con verificacion de paridad fisica;
  2. subir cobertura o dividir los validadores con cobertura parcial
     (`VehicleSoundBank::load` cc30, `validate_brake_duct` cc26, `validate_layer`
     cc24, `load_manifest_members` cc21, `enable_synth_from_profile` cc19,
     `validate_aero_config` cc17, `SampleZone::load` cc15, `trigger_from_code`
     cc13).
- **Abierto:** CLEAN-12 (artefacto de cobertura rlib/cdylib en llvm-cov).
- **Frontera ABI y tooling:** las funciones `#[no_mangle]` (`f90_core_*`,
  `vehicle_audio_*`, `f1_94_*`, `sim_world_*`) y `bin_support.rs` quedaron
  excluidas del gate cargo-crap por politica aprobada (CLEAN-10); su 0% es un
  artefacto de atribucion de cobertura, no complejidad (CLEAN-12).
- **Ajeno a estos items (no bloquea cobertura):** `aero_test` (4 fallos de
  integracion) y `facade_parity` (3) siguen fallando; `llvm-cov` corre con
  `--ignore-run-fail`.

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

- Estado: **cerrado** (2026-09-12, HEAD `c7a9bb72`).
- Re-baseline previo en HEAD `5e173557` (2026-09-12): **197 findings**
  `formula90s-rust-unwrap-expect` (WARNING) en `src/` no-tooling. Re-baseline en
  `c7a9bb72` (run `clean09-rebaseline2`): **202** (el delta es del split
  CLEAN-10). Distribucion por crate: v10-engine-synth 84, vehicle-audio-engine
  41, vehicle-physics-engine 27, skybox-engine 26, formula90-core 16,
  game-sim 3.
- **Calibracion de alcance (seguimiento de CLEAN-03):** de los 202, **183
  estaban dentro de modulos inline `#[cfg(test)]`** de `src/` (semgrep excluye
  `**/tests/**` por ruta pero no los modulos `#[cfg(test)]` embebidos; el
  conteo de "5" del primer re-baseline era un subconteo). Semgrep no puede
  expresar esa exclusion por patron (su soporte Rust no cubre cuerpos de
  `mod`), asi que la calibracion se aplico en `normalize_sarif.py`: un lexer
  single-pass (strings raw/multilinea, char literals con lifetimes, comentarios)
  calcula los spans de los modulos `#[cfg(test)] mod` y reclasifica esos
  hallazgos semgrep a maintainability/note. Misma separacion por contexto que
  la regla tooling (INFO). Evidencia: run `clean09-fix1` reclasifica 183.
- **Trabajo (19 findings de produccion reales, tras calibracion):**
  1. `bank.rs` y `sample_layer::read_mono_pcm16` (9): los
     `slice.try_into().unwrap()` de chunks WAV (tamano garantizado por el
     control de flujo) se sustituyeron por construccion directa de arrays
     `[b[i], b[i+1], ...]`, eliminando el panic sin cambiar semantica.
  2. `telemetry.rs` (2): `partial_cmp().unwrap()` en pesos/costes de
     telemetria sustituido por `total_cmp()` (nunca devuelve None, sin NaN).
  3. `scene.rs` (1): `max().unwrap()` sobre el array fijo de 3 taps ->
     `unwrap_or(0)`.
  4. `formula90-core/lib.rs` (2): `RwLock::write()/read().expect("poisoned")`
     -> `unwrap_or_else(|p| p.into_inner())`. La recuperacion es sound: el
     valor publicado es un `Arc<CoreFrame>` completo construido antes del lock
     y no hay codigo panickeable entre lock y store (un solo escritor).
  5. `sample_layer::group_members` (4): `.find(...).expect("group must
     exist")` -> `let-else` con `Err` explicito; `next()/next_back().unwrap()`
     sobre el `BTreeSet` -> `positions.iter().copied().eq(0..indices.len())`;
     `pop_front().expect("front checked")` -> `match` con `None => break`
     (infallible sin panic).
  6. `vehicle_config::f1_2026_2008_canonical` (1): se conserva el
     `expect("checked-in f1_2026_2008 physics profile must be valid")` — caso
     demostrablemente infalible (JSON embebido con `include_str!`, perfil
     validado por tests) — justificado uno a uno en `suppressions.json`
     (nueva clave por instancia `tool|rule|file|line` en `normalize_sarif.py`,
     para no suprimir el archivo entero).
- Verificacion: `cargo test --lib` de vehicle-audio-engine (224), v10-engine
  -synth (97) y formula90-core (15) en verde; run `clean09-fix1`: 0 warnings
  abiertos de la regla de produccion (queda 1 warning, suprimido y
  justificado).
- Criterio de cierre cumplido: sin findings WARNING abiertos de la regla de
  produccion (el unico restante esta justificado uno a uno).

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

- Estado: **parcial** (2026-09-12, re-baselinado en HEAD `5e173557`).
- Divididos por responsabilidad 4 validadores de alta complejidad:
  `EngineConfig::validate` (cc56), `JsonVehicleSpec::validate` (cc57),
  `AeroModelConfig::validate` (cc53) y `apply_runtime_config_to_sim` (cc46), en
  helpers de cc <= 25.
- Sprint 0 — calibracion de falsos positivos: los 8 FFI `#[no_mangle]` con
  cobertura 0% no eran deuda de codigo. Cada crate compila como
  `["rlib", "cdylib"]`; `cargo llvm-cov` registra dos instanciaciones del modulo
  (`formula90-core/src/ffi.rs`: las copias `CslR8...` del cdylib dan `FNDA:0` y
  las `Cs35CT...` del rlib con hits). cargo-crap puntuaba la copia exportada sin
  manglar del cdylib. `bin_support.rs` (`godot_project_version`) es tooling de
  los bins de calibracion. Se excluyeron por nombre de funcion y por archivo en
  `game/crates/.cargo-crap.toml` (aprobado 2026-09-12); ver CLEAN-12 para el
  artefacto de cobertura rlib/cdylib.
- Sprint 1 — cobertura y division (2026-09-12): tests para los dos helpers cc6
  (`SoundConfig::merge_with_defaults`, `AeroModelConfig::validate_underfloor_map_axes`)
  y division de `validate_experimental_manifest` (cc30) en
  `validate_experimental_header` / `validate_experimental_entry` /
  `validate_experimental_variants`, con tests que cubren el bank schema-2 valido
  y las rutas de rechazo (header, entradas, variantes).
- Resultado: 42 -> 24 (CLEAN-05..11) -> **15** reales (Sprint 0) -> **12**
  (Sprint 1). De los 12: 4 rutas calientes cc > 30 y 8 validadores/loaders con
  cobertura parcial.
- Criterio de cierre: sin findings cargo-crap en `src/` de produccion.

## CLEAN-11 — Arreglar tests que bloquean la cobertura

- Estado: **cerrado** (2026-09-11).
- `vehicle-physics-engine --lib` (aero): expectativas desactualizadas respecto al
  modelo por elementos. El bottoming localizado (un probe) solo colapsa el elemento
  delantero del suelo (total ~58% del caso limpio, no <45%), y el rake se
  reconstruye desde el plano de los probes, no desde `AeroEnvironment.rake_rad`
  (campo que el modelo ya no lee). Tests actualizados para reflejar el modelo.
- `vehicle-audio-engine --lib` (alloc): **regresion real**. `ThreeZoneSampleLayer`
  asignaba `rpm_anchors()` y un `Vec` de pesos por muestra en la ruta legacy,
  rompiendo el contrato zero-alloc del callback. Se anadio
  `process_into(&mut SampleLayerFrame)` sin asignaciones (buffer reutilizado en
  `Gf509Runtime`) y se elimino el `Vec` de anchors.
- Efecto: `cargo llvm-cov` ya registra la cobertura de ambos crates; cargo-crap
  pasa de 39 a 24 findings.
- Pendiente ajeno: `aero_test` (4 fallos de integracion) y `facade_parity` (3)
  siguen fallando; no bloquean la cobertura de los crates y quedan fuera de este item.


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

## CLEAN-12 — Artefacto de cobertura rlib/cdylib en llvm-cov

- Estado: **abierto** (2026-09-12).
- Sintoma: en los crates con `crate-type = ["rlib", "cdylib"]`, `cargo llvm-cov
  --workspace` emite dos instanciaciones del mismo modulo en `lcov.info`. Las
  funciones `#[no_mangle] pub extern "C"` aparecen con `FNDA:0` (copia exportada
  del cdylib) aunque la copia manglada del rlib registre hits, porque los tests
  enlazan el rlib. Evidencia: `formula90-core/src/ffi.rs` (`f90_core_*` FNDA:0
  frente a `_RNvNtCs35CT...ffi...` con FNDA:1/5/...); `lcov` genera un solo
  bloque `SF:` por archivo, por lo que las cuentas compiten.
- Impacto: el CRAP del perimetro ABI queda inflado (0% artificial) y penaliza el
  gate. Se mitigo con la politica de `allow` (CLEAN-10), pero el artefacto sigue
  afectando a cualquier metrica de cobertura por funcion sobre el ABI.
- Trabajo: evaluar instrumentar solo el rlib en la corrida de cobertura, o
  filtrar las copias del cdylib durante la normalizacion del LCOV. Verificar sin
  enmascarar cobertura real del resto de `ffi.rs`/`c_abi.rs`.
- Criterio de cierre: `lcov` no reporta doble instanciacion para estos modulos y
  la politica de `allow` de CLEAN-10 puede retirarse.
