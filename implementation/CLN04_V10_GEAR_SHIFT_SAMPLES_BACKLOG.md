# CLN04 — Blip de cambio con samples (gearup/geardn) en lugar del blip procedural

Date: 2026-09-18 · Branch: `main-clean` · Base: CLN03 (working tree)
Origen: pedido del usuario — dos WAV colocados en `implementation/` (`gearup.wav`, `geardn.wav`)
deben reubicarse en la carpeta de samples del V10 (`game/audio/v10_f2002_experimental`),
normalizados al formato del resto, y reemplazan el blip sintetizado del gesto de cambio
(`ShiftGesture` blip, `engine.rs`).

## Alcance

- Normalizar a mono PCM16 44.1 kHz, DC removido, pico −1.0 dBFS (convención
  `prepare_v10_sample_layer.py`, `--target-peak-dbfs -1.0`).
- Nuevo `gear_shift.rs`: carga opcional `gearup.wav`/`geardn.wav` del directorio de la capa
  sample y un player one-shot (retrigger por flanco de `Cut`: upshift=1, downshift=3).
- `EngineConfig.blip_enabled` (default true): cuando los samples están presentes el blip
  procedural de downshift (fase 4) se desactiva; el wobble de recuperación se mantiene.
- `EngineFrame.gear_shift` sumado al `master` en `AcousticScene` (misma ruta estructural).
- Transporte `gear_shift_gain` (default 1.0) en `V10LayerTuning` y `audio.gf509`; `v10_render`
  con `--gear-shift-gain` y carga desde `--sample-layer-dir` (paridad offline).
- Los one-shots del mixer (CLN03) no se tocan; `Trigger::ShiftUp/Down` siguen no-op.

## Constraints

- Sin resample ni repitch en el runtime: los WAV deben llegar ya en 44.1 kHz mono PCM16.
- Determinista, sin allocations en render, acotado.
- El clack/whine de transmisión (CLN03) se mantiene; el sample solo sustituye el blip.

## Acceptance

- Samples presentes en `game/audio/v10_f2002_experimental/gearup.wav`/`geardn.wav` con formato
  verificado (44.1k mono PCM16, pico −1.0 dBFS, DC < 1e-4).
- Con samples cargados: un disparo por flanco de Cut, retrigger si hay otro cambio, silencio con
  `gear_shift_gain 0`, sin cambios cuando los archivos no existen (bit-idéntico al baseline).
- Blip procedural desactivado cuando los samples están activos; tests verdes de las 3 crates;
  paridad BUILD/HEAD tras rebuild; validaciones `aud_path_bench`/`run_f1_94` verdes.

## Evidence log

### CLN04-1 assets (2026-09-18)

- Origen (borrados tras mover): `implementation/gearup.wav` (sha256 `f656d22c…`),
  `implementation/geardn.wav` (`421a7136…`).
- Normalizados con la convención de `prepare_v10_sample_layer.py`: mono, 44.1 kHz, PCM16,
  DC removido, pico −1.0 dBFS.
- Destino: `game/audio/v10_f2002_experimental/gearup.wav` (`12221667…`) y `geardn.wav`
  (`994f9a64…`); 13057 y 8545 frames; DC < 1e-5.

### CLN04-2 implementación (2026-09-18)

- Nuevo `gear_shift.rs`: `GearShiftSamples::load_directory` (opcional; requiere 44.1 kHz y
  ≥128 frames) y `GearShiftPlayer` (retrigger por flanco de Cut: upshift=1, downshift=3,
  one-shot por voz, gain 0–2). 5 tests.
- `EngineConfig.blip_enabled` (default true): el runtime lo apaga cuando los samples existen;
  test `blip_is_suppressed_when_disabled`.
- `EngineFrame.gear_shift` sumado al `master` en `AcousticScene`; `Gf509Runtime` lo alimenta por
  muestra; transporte `gear_shift_gain` en `V10LayerTuning` + parse en `formula90-core`;
  `v10_render` con `--gear-shift-gain`, stem `gear_shift` y metadata.
- Comportamiento con archivos ausentes: `load_directory` → `None` → blip procedural intacto
  (bit-idéntico al baseline).

### CLN04-3 medidas y validación (2026-09-18)

- Matriz offline (sweep `u@2.2,u@4.6,d@8.2`, gain 1.0/0.5/0.0): pico −3.9 dBFS en todos los
  casos (sin clipping); el transitorio del sample aparece a ~20 ms del evento con contenido
  100–1000 Hz; LUFS invariante.
- Perfil: `audio.gf509.gear_shift_gain: 1.0`.
- Tests: v10-engine-synth 132 + 5; vehicle-audio-engine 239; formula90-core 29 + 1 + 5 (solo los
  3 `facade_parity` preexistentes).
- `build_windows.ps1` exit 0; `aud_path_bench` con perfil real exit 0 sin fallback;
  `run_f1_94 -ValidateRuntimeOnly` y `-SmokeAudio` PASS (un reintento por lock transitorio del
  GLB del chasis que el usuario estaba regenerando).
- Audiciones: `reports/audio-v10/cln03/auditions_gearshift/` (gain 1.0 / 0.5 / 0.0).

### CLN04-4 corrección de ruteo y nivel (2026-09-18)

- Síntoma reportado por el usuario: el sample se oía como un "tick" artificial.
- Diagnóstico: los samples son casi 100 % banda baja (gearup 40-360 Hz a −25.7 dB vs 360-2650 a
  −32.2; geardn 40-360 −23.7 vs 360-2650 −46.6). El ruteo original los sumaba al `master`, y la
  escena los pasaba por el split dry con `dry_low = 0.0` (CLN01) → el cuerpo se eliminaba y solo
  sobrevivía un residuo fino 360-2650 = "tick". Además, a gain 1.0 con ruteo completo habría
  clipeado (pico −1 dBFS × output_gain 2.9 × headroom 0.61 ≈ 1.6).
- Corrección:
  - `AcousticScene`: `gear_shift` sale del `master` (split dry) y se suma **full-band** a la
    salida de la escena antes de `output_gain` (mismo estéreo/onboard que el resto).
  - `GearShiftPlayer`: fades anti-click (2 ms ataque, 8 ms liberación).
  - Perfil: `gear_shift_gain` 1.0 → 0.3 (pico de contribución ≈ 0.47 en el mix).
- Medidas tras la corrección (sweep con `u@2.2,u@4.6,d@8.2`):
  - fb0.2: 40-150 +2.0 dB share, pico −3.9 dBFS.
  - fb0.3 (elegido): 40-150 +3.6 dB, pico −3.6 dBFS, blip a ~18 ms del evento en 100-300 Hz
    (delta positivo en la ventana: +0.24/−0.33/+1.70 dB).
  - fb0.4: +5.0 dB pero pico −2.2 dBFS (descarta).
- Audiciones: `reports/audio-v10/cln03/auditions_gearshift_fb/` (control, 0.2, 0.3, 0.4).
- Validación: tests v10-engine-synth 132 + 5; `build_windows.ps1` exit 0; `aud_path_bench` exit 0
  sin fallback; `run_f1_94 -ValidateRuntimeOnly` y `-SmokeAudio` PASS.

### CLN04-5 pitch tracking y estabilización del clack (2026-09-18)

- Pregunta del usuario: "¿el gear shift está de acuerdo a las rpm? cambian de pitch y están muy
  elevados".
- Diagnóstico: (a) el player reproducía el sample a pitch nativo fijo (1 muestra/frame) sin
  seguir RPM; (b) los samples son de banda baja (gearup f0 ≈129 Hz, geardn ≈43–65 Hz); (c) el
  artefacto "elevado y de pitch variable" era el **clack de transmisión** (modos 1.5–4.3 kHz con
  escala aleatoria ±6–12 % por cambio) disparado en el mismo flanco.
- Correcciones:
  - `GearShiftPlayer`: reproducción con **rate = rpm/reference_rpm** (0.5–2.0, suavizado 20 ms,
    cursor fraccional con interpolación lineal); nueva clave `gear_shift_reference_rpm`
    (default 10000, perfil 10000).
  - Clack: variación de pitch por cambio reducida (0.985 ± 0.03), click ×0.8 → ×0.35, y
    `transmission.clack` 1.0 → 0.7 en el perfil.
  - `V10LayerTuning`: `gear_shift_gain`/`gear_shift_reference_rpm` pasan a `Option` (None =
    default del runtime) para no romper el init con tuning default.
- Evidencia de pitch-tracking (stem `gear_shift`, ventana 2.0–2.35 s del blip de upshift):
  - 7000 rpm → 86 Hz; 11000 rpm → 129 Hz; ratio 1.50 (esperado ≈1.57, desviación por rampa).
- Sweep final (perfil exacto: transmission 0.35/clack 0.7, gear_shift 0.3/ref 10000): pico
  −2.6 dBFS, LUFS −16.4. Audiciones: `reports/audio-v10/cln03/auditions_rpm_track/`
  (control, sweep final, steady 7000, steady 11000).
- Tests: v10-engine-synth 133 + 5 (nuevo test de rate); vehicle-audio-engine 239;
  formula90-core 29 + 1 + 5 (solo 3 `facade_parity` preexistentes).
- `build_windows.ps1` exit 0; `run_f1_94 -ValidateRuntimeOnly` y `-SmokeAudio` PASS.

### CLN04-6 reducción general de clack y samples (2026-09-18)

- Pedido del usuario: bajar en general el clack de transmisión y los samples.
- Perfil: `transmission.clack` 0.7 → 0.35 y `gear_shift_gain` 0.3 → 0.15.
- Medido (sweep con `u@2.2,u@4.6,d@8.2`): 40-150 Hz share −16.6 → −18.7 dB (blip −2.1 dB);
  pico −2.6 → −3.2 dBFS; transitorio 1-4k en ventana más cercano al control (clack suave).
- `aud_path_bench` exit 0 sin fallback; cambio de perfil sin rebuild.

### CLN04-7 espacio de medios para el motor (2026-09-18)

- Pedido: que la transmisión ceda espacio a los medios del motor, sin bajar el nivel del whine.
- Medición del solapamiento (steady 11000, gear 4; gap transmisión − motor):
  800-2k −4.8 dB, 2-5k −2.1 dB → la transmisión compite 1:1 en medios.
- Candidatos medidos (sweep + steady, stems):
  - `fd30`/`fd26` (final_teeth 40→30/26): mejora 800-2k en solo 1.6 dB pero **empeora
    350-800 de −17.7 a −3.4 dB** (el parcial baja a medios graves): descartado.
  - `drymid085` (dry_mid 0.75→0.85): 800-2k +0.5 dB, 2-5k +0.3.
  - `shelf045` (shelf 0.35→0.45): 2-5k +0.5.
  - `mid085_045` (ambos): 800-2k +0.5, 2-5k +0.8, pico −2.9 dBFS → **aplicado**.
  - `mid095_050`: 800-2k +0.9, 2-5k +1.3, pico −2.7 (siguiente paso disponible).
- Perfil: `scene_gains.dry_mid 0.85`, `upper_mid_shelf_gain 0.45`; `final_teeth` se queda en 40.
- Audiciones: `reports/audio-v10/cln03/auditions_midspace/` (base vs aplicado).
- `aud_path_bench` exit 0 sin fallback; cambio de perfil sin rebuild.
- Techo profile-only: no alcanza los 2-3 dB de apertura; para eso queda CLN05 (código:
  scoop selectivo 800-2500 Hz + rebaja del 2º armónico del final tone), sin tocar el nivel del
  whine.

### CLN04-8 acoplamiento transmisión–motor en 4ª/5ª a altas RPM (2026-09-18)

- Síntoma: batido/rugosidad en agudos entre transmisión y motor en 4ª y 5ª a altas vueltas.
- Causa (geometría de líneas, firing = rpm·5/60):
  - Con `gear_teeth 20`/`final_teeth 40`: en 4ª el whine queda a **5.1%** del 2º orden
    (batido ≈51 Hz a 12000) y el final drive a **3.5%** del 1º (≈35 Hz); en 5ª el whine está
    a 0.381 pero sus armónicos 2º (4.76 vs orden 5) y el final 2º (2.24 vs 2) se acercan.
    La banda 2-5k es justo donde el motor tiene medios fuertes y el shelf los realza.
- Corrección aplicada (solo perfil, sin rebuild):
  - `transmission.gear_teeth` 20 → **21** (4ª pasa a 0.154 del orden; 5ª cae en 2.5 exacto =
    0.5 de separación).
  - `transmission.final_teeth` 40 → **46** (4ª pasa de 0.035 a 0.110; 5ª 0.120 → 0.289).
  - Distancias por marcha con los nuevos dientes (min dist a orden): 1ª 0.217, 2ª 0.473,
    3ª 0.261, 4ª 0.154, 5ª 0.500, 6ª 0.123 (final: 1ª 0.373, 2ª 0.213, 3ª 0.059, 4ª 0.110,
    5ª 0.289, 6ª 0.483); el compromiso prioriza 4ª/5ª, que es el síntoma reportado.
- `aud_path_bench` exit 0 sin fallback; audiciones A/B por marcha en
  `reports/audio-v10/cln03/auditions_coupling/` (4ª y 5ª a 12000 rpm, antes vs después).
- Si no alcanza: CLN05 código — bajar 2º/3º armónico del gear tone y 2º del final tone
  (menos líneas colisionables), scoop 800-2500 Hz y/o roll-off de whine sobre 10000 rpm.

### CLN04-9 ruteo estructural del gearbox (2026-09-18)

- Pregunta: ¿el gearbox pasa por la escena acústica del monocoque? No: la transmisión se sumaba a
  `master` (rama dry/aire); `mount_monocoque` se excitaba solo con campos estructurales del motor.
- Implementado (código, con rebuild):
  - `AcousticSceneConfig.transmission_mount_gain` (0.0–1.5, default 0): send paralelo al camino
    mount/monocoque.
  - `AcousticSceneConfig.transmission_cover_gain` (0.0–2.0, default 0): send paralelo al cover.
  - Transporte por `scene_gains.transmission_mount` / `scene_gains.transmission_cover`; tests de
    ruteo por rama en `scene.rs`.
- Resultado medido:
  - **Mount: inerte** — su banco modal (148–548 Hz) + lowpass (760/920 Hz) no deja pasar el
    gearbox (1–5 kHz). Aun con send 1.0 el stem del monocoque 800-2k solo sube 1.8 dB y el mix no
    cambia. Se deja la clave en 0.0 (documentada) como camino para futuros componentes graves.
  - **Cover: efectivo** — es la única rama ancha (modal 684–5071 Hz). Matriz 0.02/0.05/0.10/0.3/0.6/
    1.0: 0.3 ya sube +1.9 dB RMS con pico −3.6; 0.6/1.0 clipean. Elegido **0.10**: steady 800-2k
    +0.7 dB, sweep +0.3 dB, pico −2.5 dBFS, sin desbalance de bandas.
- Perfil: `scene_gains.transmission_cover: 0.1`; `transmission_mount` sin declarar (default 0).
- Tests: v10-engine-synth 135 + 5; vehicle-audio-engine 230; formula90-core 29 + 1 + 5 (solo los 3
  `facade_parity` preexistentes). `build_windows.ps1` exit 0; `aud_path_bench` exit 0 sin fallback;
  `run_f1_94 -ValidateRuntimeOnly` y `-SmokeAudio` PASS.
- Audiciones: `reports/audio-v10/cln03/auditions_coversend/` (sweep y steady 11000, antes vs
  después).
- Incidencia: el disco D: llegó a 0 bytes durante la matriz; se liberaron ~2.2 GB de renders
  intermedios de evidencia (ignorados). Los stems base y audiciones se conservan.

### CLN04-10 ganancia global de transmisión −15% (2026-09-18)

- Pedido del usuario: bajar toda la ganancia de la transmisión un 15%.
- Perfil: `transmission.gain` 0.35 → **0.2975** (−15% exacto); escala whine/clack/rattle/clutch
  y el send al cover de forma uniforme. `aud_path_bench` exit 0 sin fallback; sin rebuild.

### CLN04-11 ganancia global de transmisión −30% adicional (2026-09-18)

- Pedido del usuario: bajar 30% respecto al valor anterior.
- Perfil: `transmission.gain` 0.2975 → **0.20825** (0.2975 × 0.70, −30% exacto respecto al paso
  previo; acumulado −40.5% sobre 0.35). `aud_path_bench` exit 0 sin fallback; sin rebuild.

### CLN04-12 whine a 0.8 (2026-09-18)

- Pedido del usuario: `transmission.whine` 1.0 → **0.8** (−2 dB sobre el whine; el resto de la
  transmisión sin cambios). `aud_path_bench` exit 0 sin fallback; sin rebuild.

### CLN04-13 whine 0.5 y transmission_cover 0.15 (2026-09-18)

- Pedido del usuario: bajar `transmission.whine` a **0.5** y subir `scene_gains.transmission_cover`
  a **0.15**.
- `aud_path_bench` exit 0 sin fallback; cambio de perfil sin rebuild.

### CLN04-14 whine 0.6 y transmission_cover 0.0 (2026-09-18)

- Pedido del usuario: `transmission.whine` 0.5 → **0.6** y `scene_gains.transmission_cover`
  0.15 → **0.0** (la transmisión vuelve a sonar solo por el aire, sin send al cover).
- `aud_path_bench` exit 0 sin fallback; cambio de perfil sin rebuild.

## Retrospective

- Funcionó: reutilizar `read_mono_pcm16` (mismo formato del banco) evitó duplicar parsers; el
  flag `blip_enabled` desactiva solo el flare sintético y conserva el wobble; los samples a −1
  dBFS entran sin clipping porque coinciden con la ventana del cut (energía baja).
- Pendiente: escucha del usuario en juego (`run_f1_94.ps1`); si el sample domina, `gear_shift_gain`
  0.5 ya está medido y es cambio de perfil sin rebuild.