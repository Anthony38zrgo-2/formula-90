# Informe final — limpieza basada en analisis estatico (2026-09-11)

- HEAD final: `24445d64e7402b35cbb8e060d6fe2d9094ac41f6` (rama `main-clean`).
- Fuente de verdad: `static-analysis-summary.json` (run `post-warnings`).
- Informes locales: `reports/static-analysis/baseline/TRIAGE.md`,
  `reports/static-analysis/final/FINAL_REPORT.md`,
  `reports/static-analysis/post-warnings/` (SARIF + raw + comparaciones).
- Backlog diferido: `docs/clean-code/BACKLOG.md`.

## Proceso

1. Herramientas provisionadas con versiones fijas:
   `scripts/setup_analysis_tools_windows.ps1` (LLVM 23.1.1, cppcheck 2.21.0,
   semgrep 1.177.0, cargo-audit 0.22.2, cargo-deny 0.20.2, cargo-geiger 0.13.0,
   cargo-crap 0.5.0, cargo-llvm-cov 0.9.1, clippy-sarif y clang-tidy-sarif 0.8.0).
2. Baseline determinista (SARIF 2.1.0 normalizado, deduplicado y consolidado en
   `static-analysis-summary.json`).
3. Correcciones en lotes con re-analisis, rebuild y comparacion estable
   (`scripts/clean-code/compare_findings.py`).
4. Validacion por lote: build (`scripts/build_windows.ps1`), unit tests C++,
   runtime parity/Godot (`scripts/run_f1_94.ps1 -ValidateRuntimeOnly`), tests Rust.

## Numeros

| Fase | Findings | Errores | Perf | Suprimidos |
|---|---:|---:|---:|---:|
| Baseline (pre-fix) | 1018 | 13 | 3 | 0 |
| Post lotes 1-3 (`final`) | 1010 | 0 | 0 | 4 |
| Post warnings mecanicos (`post-warnings`) | 814 | 0 | 0 | 4 |

En el ciclo completo: 215 findings corregidos, 11 findings pre-existentes de
`formula90-core` que quedaron visibles al corregir el runner de clippy (no son
regresiones), 4 suprimidos con justificacion y 0 regresiones introducidas.

## Resueltos (215)

- Errores: 13 (`not_unsafe_ptr_arg_deref` x12 en el ABI C de game-sim y
  formula90-core, `approx_constant` en skybox-engine).
- Performance: 3 (`performance-use-std-move`).
- RAII: 2 (copy/assign eliminados en `dsp_instance.h`).
- Warnings mecanicos C++: 196
  (`readability-uppercase-literal-suffix` 116,
  `readability-math-missing-parentheses` 12,
  `readability-braces-around-statements` 68).
- 1 `doc_lazy_continuation` resuelto de forma incidental.

## Suprimidos con justificacion (4)

Registrados por clave estable en `scripts/clean-code/suppressions.json`:

- `RUSTSEC-2025-0141` y `cargo-deny a:unmaintained`: bincode 1.3.3 sin
  mantenimiento; migracion en `docs/clean-code/BACKLOG.md` (CLEAN-01).
- `cppcheck incompleteArrayFill` x2: falso positivo en test_main.cpp (copia
  intencional de 1024 de 4096 frames).

## Pendientes (810 abiertos)

- clang-tidy 262: `readability-implicit-bool-conversion` 159,
  `bugprone-narrowing-conversions` 40, `bugprone-easily-swappable-parameters` 14,
  `readability-isolate-declaration` 10, complejidad cognitiva 7, etc.
- semgrep 278: regla de `unwrap`/`expect` (calibracion en CLEAN-03).
- clippy 144: `field_reassign_with_default` 34, `type_complexity` 9,
  `unused_variables`/`unused_mut` 14, `manual_range_contains`/`clamp` 10, etc.
- cargo-crap 70 (complejidad; CLEAN-05).
- cargo-deny 13 (licencias y metadatos; CLEAN-04).
- cargo-geiger 5 (auditoria `unsafe`; CLEAN-02).

## Revision manual / backlog

`docs/clean-code/BACKLOG.md`: CLEAN-01 (bincode), CLEAN-02 (auditoria unsafe),
CLEAN-03 (semgrep), CLEAN-04 (licencias), CLEAN-05 (complejidad), CLEAN-06
(cstyleCast), CLEAN-07 (test roto de game-sim), CLEAN-08 (perfiles cargo).
Los 3 tests que fallan (2 aero de vehicle-physics y `c_abi_roundtrip` de
game-sim) se reprodujeron en un worktree limpio en HEAD: pre-existentes.

## Commits

- `448f46e0` Limpieza estatica: errores clippy FFI, RAII del DSP y std::move
- `f0e16ce8` Analisis estatico: runner SARIF, herramientas fijadas y backlog
- `0afebe49` build: republish Windows runtime for f0e16ce8
- `f5645758` Limpieza estatica: sufijos uppercase y parentesis matematicos en C++
- `2bb8763e` Limpieza estatica: llaves en bloques de control C++
- `9d9d6b60` Analisis estatico: supresiones por clave estable
- `24445d64` build: republish Windows runtime for 9d9d6b60

## Determinismo

- 7 herramientas identicas entre corridas; clippy estabilizado aislando
  target-dir, `cargo clean -p` por paquete, `--no-deps` y `CARGO_INCREMENTAL=0`.
- Las supresiones usan clave estable (tool|regla|archivo|mensaje) para no
  romperse con desplazamientos de linea.
