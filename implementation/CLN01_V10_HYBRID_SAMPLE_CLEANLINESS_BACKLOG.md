# CLN01 — V10 híbrido: limpieza de la capa sample y realce de medios/medios graves

Date: 2026-09-18 · Branch: `main-clean` · HEAD: `2ad3adc5072efe9e7c1d5aa94fe0488a8db5ce1f`
Source of scope: análisis de cadena solicitado por el usuario (respuestas: ruta híbrida in-game,
prioridad "menos ruido/hiss de samples", solo perfil primero sin rebuild). Vehículo lanzado:
`f1_2030_v10` → `game/data/vehicles/f1_2030/f1_2030_v10_geometric.json` (`audio.gf509`) →
`game/audio/v10_f2002_experimental/manifest.json` (schema 2), 44100 Hz.

## Objetivo

Reducir el hiss audible de la capa sample (residuo broadband y rasp 1.8–6.5 kHz) y, en el mismo
movimiento, realzar los medios y medios graves (~150–800 Hz) del híbrido sin perder transitorios,
identidad V10 ni cuerpo. Todo con claves ya transportadas por perfil, sin recompilar.

## Constraints

- Solo claves de perfil ya soportadas (`formula90-core/audio.rs:60-134`, `mixer.rs:1895-1912`):
  `residual_gain_scale`, `disable_sample_rasp`, `sample_blend_weight`, `physical_blend_weight`,
  `sample_zone_trim_db`, `scene_gains` (dry_low/mid/high, engine_air, engine_cover,
  mount_monocoque, output), `cover_radiation_lowpass_hz`, `header_length_scale`,
  `collector_geometry`.
- No tocar la física (pistones, bore, stroke, orden de encendido, RPM, par). Sin EQ global, sin
  pitch shift, sin subarmónicos, sin osciladores, sin bajar temperatura.
- La ruta de escena (`scene.rs`) y los samples no se modifican en código en esta iteración.
- Baseline reversible: sin claves nuevas, el render debe ser bit-idéntico al actual.
- La activación en el perfil lanzado requiere gate humano y refresco de `physics_sha256`.
- Aislar de los cambios sucios existentes (assets f1-2030, `instrucciones.md`): commits atómicos
  y rutas explícitas solo si el usuario los autoriza.

## Diagnóstico de partida (medido sobre stems `exh08/fifty50_v2`, pesos 50/50)

- `sample_residual` es el stem más fuerte de la capa sample (RMS −15 a −19 dB frente a
  `sample_tonal` −23 a −27 dB): el contenido de 2–5 kHz del híbrido es mayormente ruido de loop,
  no tono.
- En `hybrid_mix` la energía de medios se reparte: 150–350 Hz lo manda `mount_monocoque`
  (modos 148–548 Hz, saturado con tanh 13/15); 350–800 Hz `master`/`engine_air`; 800–2k y 2–5k
  la capa sample.
- El perfil activo ya trae `sample_blend_weight 0.8165`, trims `[4.5, -4.63, 3.36, 1.84, 2.44,
  3.88]`, escena `mount 0.713 / cover 0.9156 / air 0.841`, `cover_radiation_lowpass_hz 1e6`,
  `disable_sample_rasp false`, sin `residual_gain_scale` (=1.0).
- `docs/V10_GF509_HYBRID_AUDIO_ARCHITECTURE.md` describe un duck 350–2000 Hz y rutas
  metal/airbox/under_seat que ya no existen en `scene.rs`: doc desactualizada, se corregirá como
  parte de la evidencia.

## Plan

- CLN-A Baseline: compilar `v10_render` desde este HEAD; capturar perfil activo (4 RPM fijos
  3000/6000/9000/12000 + sweep 5000→18000 lift/coast) con stems; analizador de bandas
  40-150/150-350/350-800/800-2k/2k-5k/5k-10k sobre `hybrid_mix`, `sample_layer`, `scene_mix`,
  `sample_tonal`, `sample_residual`, `sample_max_rasp`; metadata + SHA-256.
- CLN-B Matriz de candidatos (una variable por render, sin rebuild):
  - B1 `residual_gain_scale` 0.7 y 0.5;
  - B2 `disable_sample_rasp` true;
  - B3 `sample_blend_weight` 0.70;
  - B4 cuerpo procedural: `dry_mid=0.60`, `dry_low=0.28`, `mount_monocoque=0.85` por separado;
  - B5 combinación de ganadores (y sweep de finalistas).
- CLN-C Validación runtime: `aud_path_bench` con perfil candidato (transporte real) y
  `run_f1_94.ps1 -ValidateRuntimeOnly` con hash refrescado.
- CLN-D Gate humano: audiciones normalizadas en loudness (BS.1770) y tabla de bandas; activar
  solo lo aprobado en `audio.gf509`, refrescar `physics_sha256`, revalidar.
- Si el hiss no baja sin adelgazar, se abre fase 2 separada (linearizar saturaciones de
  `scene.rs`, exponer gains físicos) con rebuild y paridad BUILD/HEAD: **no autorizada aquí**.

## Acceptance

- `sample_residual` y banda 2–5 kHz de `sample_layer`: −3 dB o más.
- `hybrid_mix` 150–800 Hz no baja más de 1 dB; ratio 150–800 vs 800–5k mejora ≥2 dB a
  3000/6000/9000.
- Pico ≤0.999 en PCM16; sin clipping ni artefactos en lift/coast; sin droning en sweep.
- Tests de `v10-engine-synth` / `vehicle-audio-engine` / `formula90-core` verdes;
  `aud_path_bench` exit 0; `run_f1_94.ps1 -ValidateRuntimeOnly` verde.
- Reversibilidad: baseline sin claves nuevas bit-idéntico (SHA-256).

## Evidence log

### CLN-A baseline (2026-09-18)

- `v10_render` compilado en release desde este HEAD (target workspace `game/crates/target/release`,
  28.5 s). Captura: `reports/audio-v10/cln01/capture_cln01.ps1` (ignorada), perfil replicado con
  claves exactas (`scene_gains` del perfil, `sample_blend_weight 0.8165`, trims `[4.5,-4.63,3.36,
  1.84,2.44,3.88]`), 4 RPM fijos + sweep, stems completos.
- Herramientas nuevas: `scripts/audio/analyze_hybrid_bands.py` (bandas 40-150/150-350/350-800/
  800-2k/2k-5k/5k-10k, RMS/pico/LUFS), `scripts/audio/make_loudness_auditions.py` (BS.1770).
- Baseline: LUFS −13.5/ −19.5/ −17.8/ −17.8 y pico −4.5/ −9.8/ −7.0/ −7.4 dBFS a
  3000/6000/9000/12000. Nota: el salto de loudness a 3000 RPM es preexistente (EXH-01 ya reportó
  pico 1.105 en `steady_3000_proc`); se neutraliza en las audiciones.
- Sweep base: pico −3.6 dBFS, LUFS −17.58; sin clipping.

### CLN-B matriz de candidatos (2026-09-18)

Deltas vs baseline (dB de potencia por banda, `hybrid_mix`; MID = 150-2000 Hz, HISS = 2k-10k):

| Candidato | 3000 RPM | 6000 RPM | 9000 RPM | 12000 RPM |
|---|---|---|---|---|
| `res070` (`residual_gain_scale 0.7`) | MID −0.30 / HISS −2.63 | −0.25 / −2.03 | −0.31 / −2.25 | −0.30 / −2.01 |
| `res050` | MID −0.46 / HISS −4.82 | −0.38 / −3.48 | −0.47 / −3.99 | −0.45 / −3.49 |
| `norasp` (`disable_sample_rasp`) | 0.00 / 0.00 | 0.00 / 0.00 | −0.10 / −1.06 | −0.09 / −1.10 |
| `sw070` (`sample_blend_weight 0.7`) | −0.13 / −1.20 | −0.18 / −1.01 | −0.28 / −1.22 | −0.18 / −1.27 |
| `mount085` | +1.36 / 0.00 | +1.18 / 0.00 | +0.82 / 0.00 | +0.82 / 0.00 |
| `drymid060` | +0.02 / +0.21 | +0.20 / +0.47 | +0.46 / +0.16 | +0.66 / +0.18 |
| `combo_c` (res070+norasp+dry_mid .60+mount .85) | **+1.15 / −2.26** | **+1.15 / −1.30** | **+0.91 / −2.90** | **+1.11 / −2.72** |
| `combo_d` (res050+norasp+dry_mid .60+mount .85) | **+1.04 / −4.22** | **+1.06 / −2.50** | **+0.80 / −4.41** | **+1.01 / −4.04** |
| `combo_e` (res070+norasp+sw .70+mount .85) | +1.12 / −3.25 | +1.06 / −1.99 | +0.76 / −3.89 | +1.05 / −3.79 |

Stems de la capa sample (delta vs baseline):

- `sample_residual`: `combo_c` −3.1 a −4.3 dB; `combo_d` −6.0 a −7.2 dB.
- `sample_layer` 2–5 kHz: `combo_c` −2.8 (6000) / −3.6 (9000) / −3.1 (12000); `combo_d`
  −5.3 / −5.5 / −4.5.
- `sample_tonal` intacto (±0.2 dB bajo 800 Hz; −1.3 dB máximo en 2k-5k por el rasp de la zona max).
- `150-800 Hz` combinado del híbrido sube +1.1 a +1.4 dB en todos los candidatos con `mount085`.
- Picos de todos los combos ≤ −4.7 dBFS (sin riesgo de clipping); sweep `combo_d` pico −4.6 dBFS,
  LUFS dentro de 0.4 dB del baseline.

### CLN-C validación (2026-09-18)

- Perfiles candidato en `reports/audio-v10/cln01/profile_combo_{c,d}.json` (copias; el perfil
  lanzado NO fue modificado).
- `aud_path_bench` (bank `game/sounds/banks/v10_vehicle`, assets `game/audio/v10_f2002_experimental`):
  baseline exit 0; `combo_c` exit 0; `combo_d` exit 0 sin avisos de tuning inválido ni fallback.
- Tests: v10-engine-synth 115+5 passed (1 ignored); vehicle-audio-engine 241 passed;
  formula90-core 28+1+5+3... con los 3 fallos preexistentes de `facade_parity` ya reproducidos en
  EXH-04 sobre HEAD limpio (no relacionados).
- `run_f1_94.ps1 -ValidateRuntimeOnly` exit 0, paridad BUILD/HEAD `2ad3adc5` verde, Fuji y runtime OK.

### CLN-D audiciones y gate (2026-09-18)

- `reports/audio-v10/cln01/auditions/` con `base`, `combo_c`, `combo_d`, `combo_e` para
  sweep + 4 RPM, normalizadas BS.1770 a −19.50 LUFS (objetivo = mínimo del set) y tope de pico
  0.999; ganancias en `audition_gains.csv`. Picos finales ≤ −5.5 dBFS.
- Gate humano: usuario selecciona **combo_c** (`residual_gain_scale 0.7`, `disable_sample_rasp
  true`, `scene_gains.dry_mid 0.6`, `scene_gains.mount_monocoque 0.85`).
- Activado solo en `game/data/vehicles/f1_2030/f1_2030_v10_geometric.json` (`audio.gf509`);
  diff verificado: exactamente las 4 claves, nada más.
- `physics_sha256` del manifest cubre `f1_2030_v10_physics.json` (sin cambios, `ad1489…`), no el
  perfil geométrico: no requiere refresco. El manifest no se tocó.
- Verificación post-activación: JSON activado == candidato validado; `aud_path_bench` con el
  perfil real exit 0 sin fallbacks; `run_f1_94.ps1 -ValidateRuntimeOnly` exit 0 con paridad
  BUILD/HEAD `2ad3adc5` verde. El runtime instalado (`55cb8323`) es descendiente del commit de
  transporte `616d98e4`, así que las claves se aplican sin rebuild.
- Pendiente opcional: escucha en juego (`run_f1_94.ps1`) para confirmar la percepción en cabina.

### CLN-D ajuste post-gate: dry_mid 0.6 → 0.75 (2026-09-18)

- Petición del usuario: "un poco más de gain a dry mid". Matriz rápida sobre el perfil activado
  (`active` = combo_c) con `dry_mid` 0.70/0.75/0.80.
- Deltas vs `active` en `hybrid_mix` (mid = 150-2k, hiss = 2k-10k):
  - `dm070`: 350-800 +0.1 a +0.8; 800-2k +0.2 a +1.0; MID +0.1 a +0.5; HISS +0.3 a +0.5.
  - `dm075`: 350-800 +0.1 a +1.2; 800-2k +0.3 a +1.5; MID +0.2 a +0.7; HISS +0.4 a +0.8.
  - `dm080`: 350-800 +0.2 a +1.6; 800-2k +0.4 a +2.0; MID +0.3 a +0.9; HISS +0.6 a +1.1.
- Seleccionado `dm075`: máximo realce de medios manteniendo el hiss ~2.0–2.5 dB por debajo del
  baseline original; picos ≤ −4.6 dBFS en steady y −3.9 dBFS en sweep.
- Activado `scene_gains.dry_mid: 0.75` en el perfil lanzado; `aud_path_bench` exit 0 sin fallbacks
  y `run_f1_94.ps1 -ValidateRuntimeOnly` verde de nuevo.
- Audición A/B normalizada: `reports/audio-v10/cln01/auditions_drymid75/`
  (`combo_c_sweep.wav` = antes, `dm075_sweep.wav` = después).
- Trade-off registrado: el criterio CLN "MID/HISS ≥ +2 dB" queda en ~1.9 dB a 6000 RPM (los demás
  RPM siguen ≥3.0); es el coste aceptado del realce pedido.

### CLN-D ajuste post-gate: dry_low 0.18 → 0.15 (2026-09-18)

- Petición del usuario: reducir ligeramente `dry_low`. Matriz `dl015`/`dl012`/`dl010` sobre el
  perfil con `dry_mid 0.75`.
- Resultado medido: `dry_low` es prácticamente inerte en el mix actual. `dl015` vs `active75`:
  350-800 −0.02 a +0.06 dB; 800-2k −0.02 a −0.09; 40-150 +0.02 a +0.11 (el comb del `AirPath`
  cancela parte del low; bajar su ganancia reduce la cancelación). `dl012`/`dl010` tampoco pasan
  de ±0.2 dB. Causa: bajo 360 Hz domina `mount_monocoque` (modos 148–548 Hz), no el dry.
- Activado `scene_gains.dry_low: 0.15` (reducción ligera pedida) con efecto audible nulo;
  `aud_path_bench` exit 0 sin fallbacks y `run_f1_94.ps1 -ValidateRuntimeOnly` verde.
- Audición A/B: `reports/audio-v10/cln01/auditions_drylow15/` (`dm075_sweep` = antes,
  `dl015_sweep` = después): LUFS −16.94 vs −16.96, picos −3.94 vs −3.93 dBFS.
- Si se busca una reducción audible de graves/medios graves, la palanca real es
  `scene_gains.mount_monocoque` (0.85) o el HP onboard de 75 Hz, no `dry_low`.

### CLN-D ajuste post-gate: dry_low 0.15 → 0.00 (2026-09-18)

- Petición del usuario: `dry_low` a 0.00. Medido `dl000` vs `active75` (0.18): 40-150 +0.2 a +0.7
  (el sube por menor cancelación del comb), 150-350 ±0.1, 350-800 −0.4 a +0.5, 800-2k −0.5 a +0.03,
  RMS −0.26 a +0.05. En sweep: LUFS −17.05 vs −16.94, pico −3.92 vs −3.94 dBFS.
- Confirmado inerte a oído (≤0.5 dB); a 12000 RPM recorta ~0.5 dB de 800-2k, menor que el realce
  de `dry_mid 0.75`.
- Activado `scene_gains.dry_low: 0.0`; `aud_path_bench` exit 0 sin fallbacks y
  `run_f1_94.ps1 -ValidateRuntimeOnly` verde.
- Audición A/B: `reports/audio-v10/cln01/auditions_drylow00/` (`dm075_sweep` = 0.18,
  `dl015_sweep` = 0.15, `dl000_sweep` = 0.00).

### CLN-D empuje de medios-altos: um_c (2026-09-18)

- Pregunta del usuario: cómo empujar medios-altos (2–5 kHz). Palancas medidas sobre `dl000`:
  - `engine_cover` ↑: sube 800-2k (+0.2 a +0.6), no 2k-5k; presencia, no medios-altos.
  - `dry_high` ↑: ataca 2k-5k (+0.24/+0.49 a 0.28/0.40) pero sube más 5k-10k (+0.8 a +1.5) y se
    desvanece a 12000 por el rolloff `(1 - 0.72*high_rpm)`.
  - `sample_blend_weight` 0.8165→0.90: +0.6 dB parejo en 2k-5k, pero sube hiss 5k-10k +0.3-0.6.
  - `um_a` (sample .90 + cover 1.10 + dry_high .28): +0.7 a +0.9 en 2k-5k, +0.95 a +1.54 en 5k-10k.
  - `um_b` (sample .95 + dry_high .40): +1.3 a +1.5 en 2k-5k con +1.6 a +2.7 en 5k-10k (descarta
    por brillo).
- Seleccionado por el usuario: **`um_c`** = solo escena (`engine_cover 0.9156→1.10`,
  `scene_gains.dry_high 0.28`), sin tocar el peso del sample: conservador en hiss.
- Efecto medido de `um_c` vs perfil previo: 2k-5k +0.24/+0.45/+0.25/+0.04; 5k-10k +0.76/+1.28/
  +0.96/+0.47; 800-2k +0.11/+0.36/+0.31/+0.22; picos ≤ −4.55 dBFS steady y −3.5 dBFS sweep.
- Nota técnica: para 2–5 kHz quirúrgico sin sumar 5–10k hace falta código (gain del head/skin de
  tapa o shelf de medios-altos en `scene.rs`) con rebuild.
- Activado en `f1_2030_v10_geometric.json`; `aud_path_bench` exit 0 sin fallbacks y
  `run_f1_94.ps1 -ValidateRuntimeOnly` verde. Audición A/B:
  `reports/audio-v10/cln01/auditions_uppermid_umc/` (`dl000_sweep` = antes, `um_c_sweep` = después).
- Sucesor: CLN02 (`upper_mid_shelf_gain 0.35`) revierte `dry_high` a 0.14 y cubre 2–5 kHz con la
  etapa de shelf (más selectiva). `engine_cover 1.10` se mantiene.





### Nota de seguimiento

- `docs/V10_GF509_HYBRID_AUDIO_ARCHITECTURE.md` describe duck 350–2000 Hz y rutas metal/airbox/
  under_seat eliminadas: actualizar la doc en un ítem aparte para no mezclar alcance.

## Retrospective

- Funcionó: medir stems antes de elegir candidatos; `residual_gain_scale` y `mount_monocoque`
  atacan hiss y cuerpo sin tocar código; las claves ya transportadas y presentes en el runtime
  instalado permitieron activar sin rebuild; aud_path_bench + ValidateRuntimeOnly confirmaron el
  transporte; las copias de perfil mantuvieron el lanzado intacto hasta el gate.
- Coste: la métrica inicial MID/HISS contaba 800–2k como hiss y hubo que reinterpretarla; el
  salto de loudness preexistente a 3000 RPM obligó a normalizar audiciones para el A/B.
- Límite medido: con solo perfil, el corte de hiss máximo razonable es ~3 dB (combo_c) sin
  adelgazar 800–2k; ir más allá (combo_d) ya recorta medios altos. Si se necesita más limpieza,
  la fase 2 debe linearizar las saturaciones de `scene.rs`/`acoustics.rs` y bajar el drive del
  `ZoneMidProcessor` desde código, con rebuild y gate propios.
