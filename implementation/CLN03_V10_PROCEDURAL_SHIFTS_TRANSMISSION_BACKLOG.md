# CLN03 — Shifts y transmisión 100% procedurales (retiro del sample de cambio)

Date: 2026-09-18 · Branch: `main-clean` · Base: `c5abe4fd`
Origen: pedido del usuario: los cambios usan sample (`shift_up.wav`/`shift_down.wav`); se
quiere gesto de cambio enteramente procedural y sumar sonido procedural de transmisión.
Decisiones del gate previo: whine + clack + rattle de carga + clutch slip; WAV de shift se
dejan sin usar (sin borrar); whine de carácter F1 marcado; ratios automáticos desde
`powertrain.gear_ratios`/`final_drive` del perfil (overrides opcionales en `audio.gf509`).

## Estado actual (análisis)

- Física → `AudioTelemetryAdapter` (`formula90-core/src/audio_telemetry.rs:63-89`) emite
  `shift_phase` (Cut de 0.12 de energía, Recovery 40 ms, Downshift Blip 120 ms).
- Mixer: en `gear != last_gear` dispara el WAV (`vehicle-audio-engine/src/mixer.rs:1042-1052`,
  one-shot `shift_up`/`shift_down` con HP 200 Hz, attack 5 ms en down, `shift_gain 0.80`).
  No hay GDScript/ABI que dispare shifts: ese auto-trigger es el único camino.
- Gesto procedural ya existente (`v10-engine-synth/src/engine.rs:64-131`): cut, wobble de
  recuperación (10 Hz, 175 ms, 0.60–1.15) y blip de downshift (+40%, 120 ms); se propaga a la
  capa sample vía `shift_gesture_gain()`.
- No existe whine ni clack de transmisión; `shift_up_delayed`, `shift_down_delayed` y `shift_3`
  están en el manifest pero no registrados.

## Constraints

- El motor continuo sigue siendo híbrido (samples + procedural); solo se retiran los one-shots
  discretos de shift. El whine queda fase-lockeado al eje (no oscilador libre), conforme al canon
  de `implementation/v10_engine_synth_agent_backlog.md` (no duplicar drivetrain, no mover
  autoridad de gearbox al audio, sin I/O ni allocations en render).
- `Trigger` y códigos ABI 0/1 se mantienen como no-op para no romper la ABI; los WAV no se
  disparan y no se borran en esta iteración.
- Defaults apagados: `output_gain` 0.0 reproduce el comportamiento actual bit a bit hasta que el
  perfil active niveles.
- Todo determinista, acotado y sin allocations en el callback.

## Diseño

- Nuevo `v10-engine-synth/src/transmission.rs`: `TransmissionConfig`, `TransmissionInput`,
  `TransmissionSynth::process`.
  - Whine de malla: `f = rpm/60 / ratio[gear] · dientes` (+ armónicos) y segunda malla de
    final drive; slew de velocidad de eje ~120 ms para deslizar el pitch en el cut; silencio en
    neutral/parado; nivel por marcha/carga.
  - Clack: flanco de `Cut`; burst 15–40 ms con modos 1.2–4.5 kHz + click, amplitud por par previo
    y dirección, variación determinista por contador de cambios.
  - Rattle: ruido 1.5–5 kHz gated por reversiones de par y clutch parcial, envolvente ~100 ms.
  - Clutch slip: textura 300–1500 Hz durante re-enganche (clutch en 0.05–0.95) + thump de
    acople.
- Fase 3 (integración, posterior): sumar a `EngineFrame.transmission` antes de `AcousticScene`,
  alimentar desde `Gf509Runtime`, inyectar ratios desde el core, transporte
  `audio.gf509.transmission`, flags y stems en `v10_render`.

## Plan

1. Fase 0: baseline con `v10_render --shift-sequence` (sweep y steady, con control sin shifts).
2. Fase 1: `transmission.rs` + tests unitarios.
3. Fase 2: retiro de one-shots de shift en el mixer + tests actualizados.
4. Fase 3: integración runtime/escena/transporte/render + matriz A/B contra baseline.
5. Fase 4: rebuild `build_windows.ps1`, validaciones, audiciones y gate humano; activación en
   perfil y commit.

## Acceptance (global)

- Cero voces one-shot de shift (test del mixer).
- Clack alineado ±10 ms al flanco de Cut, visible en 1–4 kHz; un clack por cambio.
- Whine tonal que sigue rpm/marcha, desliza en el cut y desaparece en neutral/parado.
- Sin clicks, sin clipping, determinista; tests verdes; `aud_path_bench` y
  `run_f1_94 -ValidateRuntimeOnly`/`-SmokeAudio` verdes tras el rebuild.

## Evidence log

### CLN03-0 baseline (2026-09-18)

- Captura: `reports/audio-v10/cln03/capture_shifts.ps1` (perfil activo exacto: CLN01 + shelf
  0.35), escenarios `control_sweep`, `shift_sweep` (`u@2.2,u@4.6,d@8.2`),
  `control_steady`, `shift_steady` (`u@2.0,d@4.0`), con stems.
- Métrica nueva: `scripts/audio/analyze_shift_transients.py` (delta vs control por bandas en
  ventana del evento).
- Resultado: el gesto procedural actual **solo resta energía** (cut); no hay transitorio aditivo
  en ninguna banda. Deltas típicos vs control en la ventana (dB):
  - sweep: 100-300 Hz −17.4/−20.3/−19.6; 300-1k −14.2/−18.9/−18.1; 1-4k −14.3/−14.7/−12.9;
    4-9k −3.4/−6.1/−6.5.
  - steady: 100-300 −17.9/−13.3; 300-1k −18.4/−16.0; 1-4k −12.3/−12.6; 4-9k −3.2/−2.7.
- El "peak" detectado a ~270 ms es el retorno del wobble tras el cut, no un transitorio de
  selección. Línea base correcta: sin clack, sin whine, cambio percibido solo como caída de
  energía. El WAV del mixer no aparece en `v10_render` (vive solo en el mixer; ver Fase 2).

### CLN03-1 TransmissionSynth (2026-09-18)

- Nuevo `game/crates/v10-engine-synth/src/transmission.rs`:
  - `TransmissionConfig` (sample_rate, output_gain 0–2, ratios, reverse, final_drive,
    gear_teeth, final_teeth, gains 0–4, shaft slew, seed) con validación; requerir ratios si
    `output_gain > 0`; default apagado (`output_gain 0.0`, ratios vacíos).
  - `TransmissionInput` (rpm, gear, clutch, torque, throttle, shift_phase) y
    `TransmissionSynth::process` por muestra, determinista y sin allocations.
  - Whine: malla de engranaje (`shaft_hz·gear_teeth`) + malla de final drive
    (`shaft_hz/final_drive·final_teeth`), 1–3 armónicos, slew de eje 120 ms, nivel por
    carga/clutch/rpm; silencio en neutral/parado.
  - Clack: flanco de Cut (1/3), burst 45 ms, 3 modos (1480/2860/4310 Hz con escala y fases
    variables), envolvente 6–10 ms + click; amplitud por par previo; downshift 1.15×.
  - Rattle: ruido 1.5–5 kHz gated por inversión de par o |d par/dt| > 6, envolvente 90 ms.
  - Clutch slip: ruido 300–1500 Hz con nivel `4·clutch·(1−clutch)` + thump de acople de 155 Hz
    solo si hubo un cambio reciente (evita thump en el arranque).
- Tests (7 nuevos, todos verdes): silencio con output 0; malla sigue ratio/marcha
  (~1000 Hz en 1ª y ~1500 Hz en 2ª a 9000 RPM con ratios 3.0/2.0 y 20 dientes); silencio
  estacionario/neutral; un clack por flanco (onset >0.02 y cola <50 %); determinismo bit a bit;
  acotado ante extremos; ratios requeridos si suena.
- Suite completa `v10-engine-synth`: 125 passed + 1 ignored, 0 failed.

### CLN03-2 retiro del sample en el mixer (2026-09-18)

- Fuera `ShiftUp`/`ShiftDown` del registro de one-shots; `gear != last_gear` ya solo actualiza
  `last_gear`; `trigger_at_rpm` no reporta `last_trigger` si no hay voz; eliminados HP 200 Hz,
  attack de downshift y sus constantes; la maquinaria de ventanas RPM queda genérica (tests
  migrados a `impact_hit_1`); nuevo test `gear_change_does_not_fire_a_shift_voice`; los tests de
  shift específicos eliminados.
- `Trigger` y códigos ABI 0/1 intactos como no-op; los WAV quedan en el banco sin uso.
- `vehicle-audio-engine`: 230 + 2 + 7 tests verdes.

### CLN03-3 integración (2026-09-18)

- `EngineFrame.transmission` sumado a `master` en `AcousticScene.process` (aire/cover/mount).
  `Gf509Runtime` alimenta `TransmissionSynth` por muestra con la telemetría interpolada;
  `Gf509RuntimeConfig.transmission`; `V10LayerTuning` transporta gain/parciales/ratios.
- `formula90-core`: parsea `audio.gf509.transmission` y `apply_powertrain_transmission` inyecta
  `gear_ratios`/`final_drive`/`reverse_ratio` del perfil salvo override; nuevo test de inyección.
- `v10_render`: `--gear`, `--transmission-*`, `--gear-ratios`, `--final-drive`, `--reverse-ratio`,
  `--gear-teeth`, `--final-teeth`; stem `transmission` y metadata.
- Transparencia: render con `--transmission-gain 0` bit-idéntico (SHA-256 `0ad43a7c…`) al
  baseline previo a la integración.
- Matriz offline (sweep con `u@2.2,u@4.6,d@8.2`, gear 4, ratios del perfil):
  - gain 0.8/1.0: pico 0.0 dBFS (clipping) → descartado.
  - gain 0.5: 2-5k +2.0 dB, pico −2.8 dBFS.
  - gain 0.35 (elegido): 2-5k +1.1 dB, 5-10k +0.4, 800-2k +0.5, RMS +0.2, pico −3.2 dBFS,
    LUFS +0.3; clack con pico a +2.2/+2.7/+2.9 ms del evento en 1-4k/4-9k.
  - gain 0.25: 2-5k +0.6 dB, pico −3.5 dBFS.
- Tests: v10-engine-synth 126 + 5 (nuevo test de integración runtime→escena); vehicle-audio-engine
  239; formula90-core 29 + 1 + 5 (solo los 3 `facade_parity` preexistentes).

### CLN03-4 activación y rebuild (2026-09-18)

- Perfil: `audio.gf509.transmission` = `{ gain: 0.35, whine: 1.0, clack: 1.0, rattle: 0.5,
  clutch: 0.5 }` (ratios/teeth automáticos del powertrain: 6 marchas, final 4.25, reverse 4.2).
- `scripts/build_windows.ps1` exit 0; `BUILD_SOURCE == c5abe4fd`; DLLs regenerados.
- `aud_path_bench` con perfil real exit 0 sin fallback; `run_f1_94.ps1 -ValidateRuntimeOnly` y
  `-SmokeAudio` PASS.
- Audiciones normalizadas BS.1770 en `reports/audio-v10/cln03/auditions/`
  (`shift_sweep` = sin transmisión, `shift_sweep_t25`, `shift_sweep_t35`).
- Pendiente: escucha del usuario y commit.

## Retrospective

- Funcionó: separar el synth del mixer evitó tocar la ABI; el whine fase-lockeado al eje no es
  oscilador libre; la inyección de ratios desde el powertrain evita duplicar datos; la
  transparencia a gain 0 permitió verificar la integración bit a bit; medir antes de activar
  detectó el clipping a 0.8.
- Hallazgo clave: la transmisión entra al `master` y la escena la amplifica (~×2.9 con output
  gain); por eso los niveles útiles son 0.25–0.35, no 0.8.
- El aislamiento del clack con la métrica de transitorios es parcial: el cut del gesto resta más
  energía que la que el clack suma en la ventana; el pico a +2-3 ms en 1-4k/4-9k es la evidencia
  robusta.
- Límite: el worker path (`enable_synth_from_profile(..., None)`) no inyecta ratios; si un perfil
  activa transmisión audible sin override de ratios, GF509 falla y cae a legacy. El path de juego
  (facade) sí inyecta; el helper de test se actualizó para pasar la config.
