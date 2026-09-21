# SFX-01 — Integración de `implementation/sfx` en el V10 del f1_2030

Date: 2026-09-19 · Branch: `main-clean` · HEAD: `70f10c166b1db4626b54ae430afc5cb1b2778bfa`
BUILD_SOURCE: `70f10c166b1db4626b54ae430afc5cb1b2778bfa` (rebuild completo release + debug)
Origen: pedido del usuario — revisar `implementation/sfx`, comparar con la capa de samples del
motor `f1_2030_v10` e integrar. Alcance aprobado por gate: **idle + backfires + limiter/TC**;
tomas de micrófono (rear/front/near/far) **excluidas**; eventos por el **mixer legacy**.

Resumen: 3 fases implementadas. Fase 1 añade la zona de ralentí real (3504 rpm) al banco
schema-2; Fase 2 añade 5 variantes de backfire al banco `commons`; Fase 3 añade disparo por
flanco de limitador y TC con dos acentos nuevos. Todo con tests verdes, `aud_path_bench` sin
fallback, BUILD/HEAD válido y smoke de audio PASS. Gate humano de escucha pendiente.

## Fase 0 — Procedencia y auditoría

- Inventario SHA-256 de los 33 WAV de `implementation/sfx` (incluye `GUIDs.txt`):
  `reports/audio-v10/sfx-integration/source_inventory.json` (untracked; `reports/` está
  ignorado). La carpeta queda como fuente inmutable.
- Auditoría de los 3 candidatos de ralentí (`idle.wav`, `idle3.wav`, `idle_rear.wav`):
  - `reports/audio-v10/sfx-integration/idle-audit/extended_audit.json` (rejilla 30–170 Hz,
    estabilidad por tercios, tabla de órdenes, picos bajos).
  - `reports/audio-v10/sfx-integration/idle-audit/periodicity.json` (autocorrelación 720°/360°).
  - `reports/audio-v10/sfx-integration/idle-audit/cycle_sweep.json` (barrido 1500–10500 rpm).
  - `reports/audio-v10/sfx-integration/idle-audit/loop_anchor_check.json` (verificación sobre
    el loop preparado).
- Resultado:
  - **`idle.wav` incluido**: 3504.0 rpm (58.4 Hz de eje). El barrido de autocorrelación 720°
    da un pico único en 3502.5 rpm (r=0.471) estable en los tres tercios; el loop preparado
    (36 ciclos de 720°) confirma r=0.509 y 36.008 ciclos. Órdenes dominantes 2.5/2/5 de 58.4 Hz.
  - **`idle3.wav` rechazado**: periodicidad 720° débil (máx r=0.28); no hay ciclo estable en
    5.6 s.
  - **`idle_rear.wav` rechazado**: ambigüedad de octava (2266.5 vs 4533 rpm) y es una toma
    trasera, fuera del alcance aprobado.
- Contexto de la capa actual (sin cambios): 6 zonas ON (low_on 4579.5 … max_on 8952) + grupo
  variante de agudos; 4 zonas OFF (low_off 3954 … max_off 8268); la zona más baja era
  `low_off` 3954 → no había muestra real por debajo de ~4000 rpm y el ralentí del perfil es
  4500.

## Fase 1 — Zona de ralentí (implementado)

Assets:
- `implementation/sfx/idle.wav` (sha256 `ba8da994…`, 192.9 s, estéreo 44.1 kHz) preparado con
  `scripts/audio/prepare_v10_sample_layer.py` (`--rpm idle.wav=3504.0 --loop-cycles 36
  --target-peak-dbfs -1 --sample-rate 44100`): loop de 54372 frames, fase de encendido
  45.95°, seam normalizado 0.1656 (el más alto del banco; ver Riesgos).
- Ancla añadida a `reports/audio-v10/f2002-experimental/calibration/anchors.json` con la nota
  de verificación y bloque `idle_audit` (rechazos documentados).
- `scripts/audio/build_v10_f2002_experimental_bank.py`: `idle` primero en `ON_SOURCES`,
  `RPM_SOURCE_OVERRIDES` (`explicit_reviewed`), limpieza selectiva (ya no borra el directorio
  completo: preserva `gearup.wav`/`geardn.wav` y los `.import`) y falla si faltan los
  one-shots estáticos.
- Banco regenerado: 8 ON + 5 OFF; `manifest.json` con la zona `idle` (3504 rpm) al frente;
  `idle.loop.tonal.wav` / `idle.loop.residual.wav` + `.import` generados con Godot headless.

Código/perfil:
- `sample_layer.rs` (test `experimental_schema2_bank_covers_sweep_with_all_sources`):
  13 fuentes, trayectoria 3000→18000→5000.
- `f1_2030_v10_geometric.json`: `sample_zone_trim_db` pasa a 7 valores con la zona nueva al
  frente: `[0.0, 4.5, -4.63, 3.36, 1.84, 2.44, 3.88]`.
- Sin cambios de Rust en producción.

## Medidas

Estabilidad y rango (render offline con el perfil exacto, 5 s medidos, control = banco sin idle):

| Escenario | 40–150 | 150–350 | 350–800 | 800–2k | 2k–5k | 5k–10k | RMS mix |
|---|---|---|---|---|---|---|---|
| steady 3000 (trim0) | −0.4 | −0.1 | −3.3 | −1.8 | +1.6 | +4.4 | — |
| steady 3500 (trim0) | +0.2 | 0.0 | −1.9 | −4.1 | +1.2 | +3.6 | −0.16 dB |
| steady 4000 (trim0) | −0.2 | −0.1 | −0.9 | −1.2 | +0.6 | +1.8 | — |
| steady 4500 | +0.2 | +0.2 | +0.1 | +0.5 | +0.3 | +0.3 | — |
| sweep 3000→18000 (inicio) | +2.2 | +0.1 | −3.0 | −0.3 | +4.0 | +7.3 | −0.16 dB |

- El error de octava del estimador no afecta el render: el pitch es `rpm/ancla`, y la capa
  impone la fase por `crank_phase_deg`, así que un ancla de 3504 rpm con fase medida a 8754
  produciría batido. La verificación sobre el loop preparado (36.008 ciclos a 3504 rpm,
  r=0.509) confirma que la etiqueta 3504 es la que el runtime necesita.
- Trim de la zona idle elegido **0.0 dB** (no +4.5): reduce el realce 2–10k a ~+1.6/+4.4 dB a
  3000 rpm y +1.2/+3.6 a 3500, con −0.16 dB de RMS; con +4.5 el exceso llegaba a +4.6/+8.0 y
  +3.9/+7.2. El control de 4500 rpm confirma que el error de zona desaparece al cruzar el
  midpoint (4262 rpm).
- Carácter medido de `idle.wav`: banda 2–10k con flatness 0.52 (Welch 8192) y correlación de
  envolvente ~0 con el encendido (292 Hz) → es siseo de grabación, no textura de combustión;
  el trim 0.0 es la contención razonable.
- `aud_path_bench` con el perfil real: exit 0 sin fallback; GF509 13.9 % CPU / p95 919 µs
  (2 corridas: 13.87/13.80). El coste del banco no cambió.
- Tests: `v10-engine-synth` 137 + 1 ignorado; `vehicle-audio-engine` 230; `formula90_core`
  29 (los 3 `facade_parity` preexistentes no se reproducen aquí).

## Evidencia de escucha (gate humano pendiente)

- `reports/audio-v10/sfx-integration/idle/sweep/auditions/`:
  `sweep_sweep_control_final.wav` vs `sweep_sweep_idle_trim0.wav` (LUFS −14.88 ambas, sin
  ganancia aplicada, pico −2.09 dBFS).
- `reports/audio-v10/sfx-integration/idle/sweep/` (renders crudos, 12 s 3000→18000→3000).
- `reports/audio-v10/sfx-integration/idle/steady_3500/` y `control_3500/` (stems completos).

## Fase 2 — Backfires (implementado)

Assets:
- `500_backfire3..7.wav` normalizados (mono PCM16 44.1 kHz, DC removido, fades 2/12 ms, pico
  0.89) y promovidos como `backfire_3..7.wav` con rol `engine_backfire` en
  `game/sounds/banks/commons`.
- El mixer ya rota todas las muestras con rol `engine_backfire` por trigger determinista
  (`mixer.rs:700`, RNG con estado fijo). El banco pasa de 2 a 7 variantes: las dos de F1-2008
  más las cinco nuevas.
- Herramienta: `tools/audio/promote_f1_2030_backfire_sounds.py` (determinista, actualiza
  hashes/`synthesis`/`provenance` del manifiesto). Fuente staging: `scratch/audio/limiter-sources/`.
- Config: 5 entradas nuevas en `sound_mixer_config.json` con la misma receta que `int_backfire`
  (volume 0.42, pan −0.5, EQ y reverb `exhaust_chamber`), verificado `va_validate --check`.
- Evidencia de hashes/formato: `reports/audio-v10/sfx-integration/backfire/bank_evidence.json`.

Tests:
- `mixer::tests::backfire_selects_one_of_the_bank_variants_deterministically`: exige las 7
  variantes, una sola activa por trigger, cobertura de todas en 32 disparos y replay idéntico.
- `tools/audio/tests/test_bank_determinism.py` extendido para validar los overlays.
- Nota: los 2 fallos restantes de pytest (`retired_keys`) ya fallaban en HEAD, no son de este
  cambio.

## Fase 3 — Limitador y TC (implementado)

Assets:
- `500_limiter.wav` → `limiter_hit.wav` (rol `limiter_hit`) y `traction_control.wav` →
  `tc_cut.wav` (rol `tc_cut`), mismo procesado que los backfires.
- Entradas de mezcla `limiter_hit` (volume 0.5) y `tc_cut` (volume 0.4) en
  `sound_mixer_config.json`.

Código:
- `Trigger::Limiter` y `Trigger::TcCut` (`state.rs`), con `bank_key()` `limiter_hit` / `tc_cut`.
  No se reutilizan `engine_limiter`/`engine_tc` (siguen retirados en `bank.rs`).
- Registro de voces en `VehicleAudioEngine::new` para ambos triggers.
- Disparo por flanco en `set_telemetry_timed`:
  - limitador: `rev_limiter_active` 0→1, cooldown 250 ms (un acento por entrada, no por rebote);
  - TC: `tc_cut_ratio` cruza 0.20 desde abajo, cooldown 400 ms.
  La textura procedural de `engine.rs`/`limiter_tc.rs` no se toca: el sample es acento de evento.
- ABI/FFI intactos: los triggers nuevos no se exponen por código 0..10 (`code_from_bank_key`
  devuelve −1, sin efecto en HUD).
- Tests nuevos: `limiter_edge_fires_once_with_cooldown`, `tc_cut_edge_fires_once_with_cooldown`,
  `limiter_and_tc_triggers_are_noops_without_bank_voices`.

## Riesgos y pendientes

1. **Seam 0.1656** de la zona idle: el más alto del banco (siguiente: med_hi_on 0.132). El
   crossfade de 353 frames mitiga el borde, pero la escucha debe confirmar ausencia de clic al
   entrar/salir de la zona. Candidato a re-preparar con otra región si se oye.
2. **Siseo 2–10k** de la fuente de idle: +1.6/+4.4 dB de share a 3000 rpm respecto al control;
   el control de 4500 no se ve afectado. Si en escucha domina, la palanca es un trim negativo de
   la zona (medido: −3 dB deja +0.3/+2.4).
3. **Backfire trigger sin cambios**: se mantiene el disparo actual (lift ≥13500 rpm + cooldown
   1 s). Cambiarlo a ~11000–12000 queda como decisión de escucha.
4. **Nivel de los acentos limiter/TC**: elegidos 0.5/0.4 por receta (aún sin medición en mix);
   la escucha puede pedir ajuste de perfil sin rebuild.
5. Gate humano de escucha de las tres fases pendiente (ver Evidencia).

## Estado de verificación

| Puerta | Resultado |
|---|---|
| `v10-engine-synth` lib | 137 passed, 1 ignored |
| `vehicle-audio-engine` lib | 233 passed |
| `formula90_core` lib | 29 passed (3 `facade_parity` preexistentes ajenos) |
| `tools/audio` pytest | 47 passed, 2 failed preexistentes en HEAD (`retired_keys`) |
| `va_validate --check` | Configuration is valid |
| `aud_path_bench` (perfil real) | exit 0 sin fallback; GF509 13.7 % CPU / p95 914 µs |
| `scripts/build_windows.ps1` release + debug | exit 0; `BUILD_SOURCE == HEAD` |
| `run_f1_94 -ValidateRuntimeOnly` | PASS (paridad BUILD/HEAD verde) |
| `run_f1_94 -SmokeAudio` | PASS |
| `game/BUILD_SOURCE` | `70f10c166b1db4626b54ae430afc5cb1b2778bfa` == HEAD |


