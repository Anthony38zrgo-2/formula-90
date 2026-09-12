# Clean code — backlog de hallazgos diferidos

Origen: limpieza basada en `static-analysis-summary.json` (run `baseline`,
HEAD `64dab296ece6a1ea1509f54213c036ade5c867d2`).
Detalle del triage: `reports/static-analysis/baseline/TRIAGE.md`.

## CLEAN-01 — Migrar bincode 1.3.3 (RUSTSEC-2025-0141)

- Hallazgos: `cargo-audit RUSTSEC-2025-0141` y `cargo-deny a:unmaintained`.
- Estado: suprimidos con justificacion (`scripts/clean-code/suppressions.json`) por
  decision humana del 2026-09-11; no es vulnerabilidad explotable.
- Trabajo: evaluar reemplazo del codec (bincode 2.x, postcard, etc.), impacto en
  snapshots serializados (`f90_core_snapshot`), tests de paridad y Cargo.lock.
- Criterio de cierre: la supresion se retira y ambos findings desaparecen del informe.

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

- 278 findings `formula90s-rust-unwrap-expect`; muchos en bins, benches y tests.
- Trabajo: dividir la regla por contexto (produccion vs tooling) y decidir politica.

## CLEAN-04 — Licencias y metadatos de dependencias

- cargo-deny: 8 crates propios sin `license`/`publish=false`, 1 `l:parse-error`
  SPDX, 1 `b:duplicate` (`syn` 2.0 y 3.0).
- Trabajo: definir licencia de los crates del workspace y revisar el origen del
  parse error y del duplicado.

## CLEAN-05 — Complejidad (cargo-crap)

- 70 funciones sobre umbral; top en bins (`core_cli::main` 380,
  `v10_physics_transient_capture::main` 380) y FFI sin cobertura.
- Trabajo: tests para bins/FFI o division de funciones segun valor.

## CLEAN-06 — cppcheck cstyleCast

- 37 conversiones C-style en C++ propio.
- Trabajo: decidir refactor a casts nombrados o supresion justificada por lote.

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
