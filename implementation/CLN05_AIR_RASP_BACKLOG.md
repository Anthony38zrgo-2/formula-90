# CLN05 — Rasp procedural en `engine_air`

Date: 2026-09-18 · Branch: `main-clean` · Base: CLN04 (working tree)
Origen: pedido del usuario — "¿qué se puede modificar en engine_air para que tenga un poco más
de rasp?", elegidas las opciones 1+2.

## Diagnóstico

- `engine_air` sale de `AirPath` (comb de 3 taps 2.11/3.91/6.31 ms, sin señal directa) y hoy es
  oscuro: 800-2k domina, 2-5k a −14.0 y 5-10k a −20.4 (steady 11000).
- El único lever de perfil era `dry_high` (0.14): 0.14→0.30 (+6.6 dB) solo movía +0.3 dB en 2-5k
  y +1.2 en 5-10k, porque el comb cancela detalle de agudos.

## Implementación (código + transporte, defaults apagados)

- `scene.rs`:
  - `air_high_tilt_db` (0.0–12.0, default 0): high-shelf RBJ @ 2500 Hz aplicado a `filtered_dry`
    antes del `AirPath` (opción 1).
  - `air_direct_gain` (0.0–1.0, default 0): fracción del aire HP @2500 añadida directo a la salida
    del comb, sin los taps (opción 2).
  - `tone.rs`: `Biquad::high_shelf` (RBJ) y `highpass` expuestos a nivel crate.
  - Defaults en 0 → render bit-idéntico al baseline.
- Transporte: `scene_gains.air_tilt_db` y `scene_gains.air_direct` en `vehicle-audio-engine` y
  `v10_render`; tests: shelf (tone.rs) + ruteo de ambas claves (scene.rs).

## Medidas (steady 11000, engine_air stem)

| Candidato | 2-5k | 5-10k | Pico mix |
|---|---|---|---|
| base | −14.0 | −20.4 | −5.65 |
| tilt 3 dB | −12.3 (+1.7) | −17.7 (+2.7) | −5.16 |
| tilt 6 dB | −10.7 (+3.3) | −15.1 (+5.3) | −4.62 |
| direct 0.15 | −13.8 (+0.2) | −20.8 | −5.62 |
| direct 0.30 | −13.3 (+0.7) | −20.7 | −5.59 |
| tilt 4 + direct 0.20 | −11.4 (+2.6) | −17.2 (+3.2) | −4.88 |

- El **tilt** es el lever efectivo; el bypass directo solo es fase-dependiente (cancela con los
  taps en algunas bandas) y queda en 0.
- Aplicado: `scene_gains.air_tilt_db: 3.0` (poco rasp, +0.5 dB de pico de margen).
- Sweep: LUFS −16.48 → −16.32, pico −3.29 → −3.14 dBFS. Audiciones:
  `reports/audio-v10/cln03/auditions_rasp/`.

## Verificación

- Tests: v10-engine-synth 137 + 5; vehicle-audio-engine 230; formula90-core 29 + 1 + 5 (solo los 3
  `facade_parity` preexistentes).
- `build_windows.ps1` exit 0; `aud_path_bench` exit 0 sin fallback; `run_f1_94 -ValidateRuntimeOnly`
  y `-SmokeAudio` PASS.

## Retrospective

- Funcionó: shelf pre-comb (selectivo, perfil-tunable, apagado por defecto) y HP selectivo para el
  bypass. El bypass directo resultó casi inerte por fase con el comb; si se quiere más densidad
  de rasp, el siguiente paso es rediseñar `AirPath` (tap corto + difusor allpass).
- Pendiente: escucha del usuario; `air_tilt_db` 4–6 dB ya medido si quiere más.

## Anexo — lowpass global del mixer desactivado (2026-09-18)

- Verificación pedida por el usuario: el único LP sobre todo el output de motor es
  `EngineLowPass` en `mix_engine_source` (`vehicle-audio-engine/src/mixer.rs:1479`), aplicado
  también a la fuente GF509.
- Config: `game/sounds/sound_mixer_config.json` → `master.engine_lowpass_hz` 20950.0 →
  **0.0** (desactivado; `EngineLowPass::new` con cutoff ≤ 0 queda transparente). El slope
  `engine_lowpass_db_per_oct: 2.0` se conserva por si se reactiva.
- Validación: JSON ok, `aud_path_bench` exit 0 sin fallback y `run_f1_94 -SmokeAudio` PASS;
  cambio de config sin rebuild.
