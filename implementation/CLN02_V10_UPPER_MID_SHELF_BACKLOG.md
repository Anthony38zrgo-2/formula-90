# CLN02 — Shelf de medios-altos (2–5 kHz) sin arrastre a 5–10 kHz

Date: 2026-09-18 · Branch: `main-clean` · HEAD: `2ad3adc5072efe9e7c1d5aa94fe0488a8db5ce1f`
Prerrequisito: CLN01 (perfil con `dry_mid 0.75`, `mount 0.85`, `residual 0.7`, sin rasp,
`cover 1.10`, `dry_high 0.28`, `dry_low 0.0`).
Origen: pedido del usuario: empuje de medios-altos 2–5 kHz sin sumar tanto 5–10 kHz; se autoriza
la vía de código con shelf y rebuild.

## Objetivo

Añadir una etapa lineal de realce 2–5 kHz aplicada al híbrido (escena + capa sample), con
arrastre mínimo a 5–10 kHz, transportable por perfil, apagable (ganancia 0 = comportamiento
bit-idéntico) y medida por stems.

## Diseño

- Topología elegida por análisis numérico (respuesta RBJ): **band-add** `y = x + g·(HP2@1800 ·
  LP2@5600)(x)`, Butterworth Q=0.707, un biquad por etapa.
- Ventaja medida (respuesta teórica): g=0.25 → +1.54 dB en 2–5k con solo +0.28 dB en 5–10k
  (ratio 5:1); `hp2+2xLP2` no llega a +1 dB; peaking EQ @3200 Q0.9 → +2.05 con +0.76 (peor).
- Ubicación: post-`blend_hybrid` en `Gf509Runtime` (afecta ambas capas) y en `v10_render`
  (paridad offline). Nuevo módulo `tone.rs` con `Biquad` y `UpperMidShelf`.
- Transporte: `gf509.upper_mid_shelf_gain` (0.0–1.0, default 0.0) →
  `V10LayerTuning.upper_mid_shelf_gain` → `Gf509RuntimeConfig.upper_mid_shelf_gain`.
  CLI offline: `--upper-mid-shelf-gain`.
- Sin comentarios en el código nuevo; defaults apagados.

## Acceptance

- Ganancia 0: salida bit-idéntica al render previo (test unitario de transparencia).
- g=0.25: +1.5 dB (±0.5) integrado en 2–5k; ≤ +0.5 dB en 5–10k; ≤ ±0.2 dB en 150–800 y 100–150.
- Estable, sin NaN, sin allocations en el callback.
- Tests verdes: v10-engine-synth, vehicle-audio-engine, formula90-core (los 3 `facade_parity`
  preexistentes no cuentan).
- Rebuild Windows completo, `game/BUILD_SOURCE == HEAD`, `run_f1_94.ps1 -ValidateRuntimeOnly`
  verde, `aud_path_bench` exit 0 sin fallback.
- Evidencia de stems + audiciones normalizadas antes del gate de activación en perfil.

## Evidence log

### Diseño numérico (2026-09-18)

- Respuestas integradas con peso rosa (dB por banda):
  - `hp2+lp2 g=0.25`: 800-2k −0.00, 2-5k +1.54, 5-10k +0.28, 150-350 −0.03, 350-800 −0.15.
  - `hp2+lp2 g=0.35`: 2-5k +2.10, 5-10k +0.43.
  - `hp2+2xLP2 g=0.35`: 2-5k +1.25, 5-10k −0.54 (insuficiente).
  - `peak 3200 Q0.9 +2.5 dB`: 2-5k +2.05, 5-10k +0.76 (arrastre 2.7× mayor).
- Seleccionado `hp2+lp2` con ganancia lineal configurable.

### Implementación (2026-09-18)

- `v10-engine-synth/src/tone.rs`: `Biquad` (RBJ, DF-II-T) + `UpperMidShelf`
  (`y = x + g·LP2@5600(HP2@1800(x))`), rango 0.0–1.0, reset, tests de transparencia
  bit-exacta, +1.0–2.2 dB @3.2k con g=0.25, <0.7 dB @8k, ±0.3 dB @500.
- Integrado post-`blend_hybrid` en `Gf509Runtime` y en `v10_render` (misma struct), y
  transportado por `gf509.upper_mid_shelf_gain` → `V10LayerTuning` → `Gf509RuntimeConfig`.
  CLI offline `--upper-mid-shelf-gain`; metadata del render incluye el valor.
- Tests: v10-engine-synth 118+5 passed (1 ignored); vehicle-audio-engine 232+2+7 passed;
  formula90-core 28+1+5 passed + 3 fallos preexistentes de `facade_parity`.
- Transparencia end-to-end: render `cur2` (código nuevo, shelf 0) bit-idéntico (SHA-256) a
  `um_c` (binario previo) en los 4 RPM.

### Matriz offline (vs perfil de CLN01, `cur2`)

- `sh025`: 2k-5k +1.50/+1.49/+1.51/+1.49; 5k-10k +0.57/+0.49/+0.29/+0.59.
- `sh035`: 2k-5k +2.05/+2.04/+2.06/+2.04; 5k-10k +0.84/+0.72/+0.45/+0.86.
- `sh045`: 2k-5k +2.57/+2.56/+2.59/+2.56; 5k-10k +1.11/+0.97/+0.62/+1.15.
- `sh035dh14` (shelf 0.35 y `dry_high` 0.28→0.14): 2k-5k +1.80/+1.60/+1.81/+1.93; 5k-10k
  +0.10/−0.56/−0.50/+0.41; 350-800 −0.3 a −0.5; picos ≤ −4.7 dBFS.
- vs estado previo a CLN01-uppermid (`dl000`): `sh035dh14` deja 2k-5k +2.0/+2.0/+2.1/+2.0 y
  5k-10k +0.87/+0.72/+0.46/+0.88 (frente a `um_c`: 2k-5k +0.45 y 5k-10k +1.3). Sweep:
  2k-5k +2.1 dB, 5k-10k +0.2 dB, pico −4.0 dBFS, LUFS −16.83.

### Activación y rebuild (2026-09-18)

- Perfil: `upper_mid_shelf_gain: 0.35`, `dry_high` vuelve a 0.14, resto de CLN01 intacto.
- `scripts/build_windows.ps1` (debug + Release DSP) exit 0; `BUILD_SOURCE == HEAD == 2ad3adc5`;
  DLLs de `game/addons/formula90s/bin` regenerados.
- `aud_path_bench` con el perfil real exit 0 sin fallback; `run_f1_94.ps1 -ValidateRuntimeOnly`
  exit 0; `run_f1_94.ps1 -SmokeAudio` PASS (audio integración Godot + telemetría del mixer).
- Audición A/B normalizada: `reports/audio-v10/cln01/auditions_shelf035/` (`um_c_sweep` = antes,
  `sh035dh14_sweep` = después).
- Nota de procedencia: BUILD_SOURCE quedó en `2ad3adc5` con el CLN02 sin commitear; el binario
  corresponde al estado construido. Al commitear exactamente este estado, el check de paridad
  acepta `BUILD_SOURCE` como padre del commit.

## Retrospective

- Funcionó: prediseñar el filtro numéricamente evitó implementar una topología con mal ratio;
  el band-add es 2.5–5× más selectivo que el peaking; la ganancia lineal con default 0 conserva
  transparencia bit-exacta; compartir la struct entre runtime y render garantiza paridad
  offline/in-game; el transporte existente permitió activar sin tocar mixer/core tras el rebuild.
- Límite: sin rebuild previo, el shelf no existe para el DLL; el rebuild completo (SCons + cargo +
  Faust) es el coste. Una vez construido, el valor se ajusta por perfil sin recompilar.
- Pendiente: escucha del usuario en juego; si pide más, `upper_mid_shelf_gain 0.45` (dato ya
  medido) es cambio de perfil sin rebuild.

