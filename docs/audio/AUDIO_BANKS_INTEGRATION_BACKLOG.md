# Backlog de integración `common` + `v10_v2`

Estado de ejecución (2026-08-21): corrección técnica del vertical slice
implementada y pendiente de gate auditivo humano. El juego principal conserva
`LegacyV10Pcm` como valor seguro; `vehicle_test_session` activa explícitamente
`CommonV10Commands` para la validación A/B. Las casillas `[x]` indican código o
activos disponibles, no autorización para borrar el fallback.

## Arquitectura objetivo

```text
Simulation tick
  -> AudioTelemetryFrame
  -> Formula90AudioDirector (Rust)
       -> V10V2Adapter      [engine/drivetrain/wheel/wind]
       -> CommonBankAdapter [surfaces/collisions/dirt/pit/ambience]
       -> VoiceArbiter      [ownership, budgets, dedupe]
  -> AudioCommandFrame
  -> GodotAudioRenderer
```

`Formula90AudioDirector` es una composición especializada de dos catálogos, no un motor FMOD genérico. El `VoiceArbiter` garantiza que una familia tenga un único dueño: por ejemplo, `v10_v2/wheel.tyre_rolling` reemplaza cualquier rodadura duplicada, mientras `common/gravel` añade textura de superficie sin volver a generar skid.

## Matriz de propiedad final

| familia | fuente final | reemplaza/depreca |
|---|---|---|
| engine interior | `v10_v2 engine_int` | cinco bandas `engine_idle/low/mid/high/redline` |
| engine exterior | `v10_v2 engine_ext` | inexistente en mixer actual; evita duplicar el engine interior |
| shifts/on-off | capas v10_v2 dentro de engine + eventos confirmados | `shift_up.wav`, `shift_down.wav`, `shift_3.wav` y detector genérico si quedan redundantes |
| backfire | v10_v2 cuando se resuelva; fallback temporal antiguo | heurística fija `throttle 0.80→0.15 && rpm>12000` |
| transmission | `v10_v2 transmission` | ningún reemplazo directo; retirar reglas genéricas futuras |
| TC/limiter | v10_v2 solo tras resolver assets | `engine_tc.wav`, `engine_limiter.wav` cuando haya paridad |
| tyre rolling/damage/brake | `v10_v2 wheel` | hueco `common tyre_rolling`; cualquier bed genérico de rodadura |
| skid | `v10_v2 skid_int/ext` | uso del único `aggregate slip` como bed de superficie |
| wind | `v10_v2 wind` | cualquier ambience de viento vehicular |
| surface/kerb/gravel/grass/sand | `common` | `surf_rumble/grass/sand` |
| dirt/debris | `common` | sand colapsado + ausencia de dirtiness persistente |
| collisions/scrapes | `common` | `impact_hit_1..4`, `impact_barrier`, `impact_cone`, `impact_scrape` |
| cockpit/bodywork | v10_v2 bodywork + capas common en colisión | scrape procedural y rattles duplicados |
| ambience/pit | `common` | ninguno actual |

## Elementos Rust a conservar

- `formula90-core` como propietario del tick y punto de publicación atómico.
- Separación pura Rust/Godot y tests de replay determinista.
- Estado de underfloor (`intensity`, tangential speed, onset), pero como input del evento common track scrape.
- Suavizado temporal reutilizable, RNG determinista, limitación de voces y telemetría de diagnóstico.
- Loader WAV/manifest solo como fallback/tooling durante la migración; no como parser de `.bank` runtime.

## Elementos Rust a reemplazar o deprecar

| elemento actual | acción | razón |
|---|---|---|
| `ENGINE_BAND_KEYS` y `ENGINE_BAND_NATIVE_RPM` | reemplazar por catálogo v10_v2 y regiones RPM extraídas | samples y curvas nuevas no forman una escalera de cinco bandas |
| `BAND_CENTERS`, `BAND_WIDTH`, `engine_weights()` | deprecar tras paridad | el banco nuevo usa ventanas RPM reales y capas on/off independientes |
| `engine_pitch_scale(rpm/native_rpm)` global | reemplazar por pitch/automation por instrumento | varios samples ya están vinculados a `rpms`; pitch base y curvas son distintos |
| `engine_gain(0.45 + 0.55*throttle)` | reemplazar por mezcla on/off-throttle | el nuevo banco separa carga y coast en muestras distintas |
| `surface_key()` y `surface_gain()` | mover a `CommonBankAdapter`; retirar del mixer V10 | ownership de superficies corresponde a common |
| `VehicleAudioState.surface: &'static str` | reemplazar por superficies/carga/slip por rueda | el token dominante pierde mezcla por rueda y gravel/dirt separados |
| `slip` agregado único | reemplazar por slip ratio/angle/load `[4]` | skid exterior debe posicionarse y gated por contacto |
| enum `Trigger::Hit1..Hit4/Barrier/Cone/Scrape` | reemplazar por eventos common tipados con parámetros físicos | variantes y curvas pertenecen al adaptador common |
| one-shots `shift_up/down` | fallback temporal; deprecar si capas v10 cubren transición | evitar doble shift |
| detector de backfire fijo | feature-gate y luego retirar | banco nuevo modela throttle/on-off; falta resolver backfire events |
| `bed_cursor`, `bed_key`, `smoothed_bed_gain` | retirar al activar CommonCommands | Godot mantendrá voces common por command ID |
| `scrape_cursor` y `read_region_looped(impact_scrape)` | retirar tras track/scrape common | workaround específico del asset antiguo |
| mixer PCM `render()` y limiter Rust | deprecar como backend principal | Godot debe ser renderer; conservar `LegacyV10Pcm` hasta gate final |
| `VehicleSoundBank` mono PCM16/44.1 obligatorio | sustituir por catálogo de assets renderizables | v10_v2 es mono/estéreo, 44.1/48 kHz y Vorbis/PCM16 |
| ABI `vehicle_audio_render` y DLL separada | deprecar después de command renderer | `formula90-core` ya es la fachada consolidada |
| `GevpTelemetry` adapter y relectura desde Godot | retirar del camino principal | telemetría debe venir del mismo tick Rust |
| `AudioReadouts.weights[5]/pitches[5]` | versionar/reemplazar por voces activas/event IDs | contrato ligado al banco anterior |

No eliminar ninguno hasta que `AudioBackend::CommonV10Commands` pase el gate humano. Las funciones puras de interpolación/envelope pueden moverse a utilidades si siguen siendo necesarias para tests/offline.

## Contrato de telemetría ampliado

Añadir a `AudioTelemetryFrame`:

- `rpm`, `rpm_rate`, `throttle`, `throttle_rate`, gear y shift phase.
- torque request/delivered, drivetrain shaft speed y torque sign.
- boost/BOV state si el modelo físico realmente lo soporta; de lo contrario turbo queda desactivado.
- wheel speed, slip ratio, slip angle, normal load, surface y contact por rueda.
- brake input/torque/temperature por rueda.
- tire inflation/pressure y estados `normal/deflating/flat/blowout`.
- suspension velocity y damage state.
- airspeed y air-density ratio.
- hechos de colisión y underfloor ya descritos para common.
- listener mode (`cockpit/exterior/replay`) y transform solo para política; Godot calcula atenuación/cone final.

## Backlog ordenado

### Sprint 0 — evidencia y legalidad (gate obligatorio)

- [x] Exportar offline los 42 samples manteniendo codec, sample rate, canales y hashes (loop policy se aplica desde catálogo/adaptador).
- [ ] Resolver las 120 curvas hasta event/instrument/property, priorizando `engine_int`, `engine_ext`, `transmission`, `wheel` y `wind`.
- [ ] Auditar los samples con nombre `f2000`, `z4gt3`, `gt3r` y la licencia/procedencia de ambos soundpacks.
- [ ] Auditar los nueve eventos sin samples resueltos y decidir: nested/control, dependencia faltante o evento vacío.
- [ ] Validar auditivamente loop seams y semántica de `missgear` marcado loop.
- [ ] Gate humano sobre propiedad final de familias y derecho de distribución.

### Sprint 1 — contratos y catálogo offline

- [x] Definir `AudioTelemetryFrame` inicial y eventos físicos discretos (ampliación por rueda queda pendiente).
- [x] Definir un catálogo versionado de assets extraídos; `.bank` y FMOD quedan fuera del runtime.
- [x] Definir IDs de evento/voz, emitter, loop, gain, pitch, política de retrigger y regiones de loop en `AudioCommandFrame` v2.
- [x] Añadir salida FFI JSON versionada sin alterar/reutilizar `F90CoreFrameOut`.
- [ ] Golden tests de hashes, GUID mapping y catálogo.

### Sprint 2 — vertical slice interior

- [x] Implementar adaptación `engine_int` con RPM real y capas on/off.
- [x] Corregir la capa alta interior: usa `r25 int on high`, nunca el sample de transmisión `z4gt3`.
- [x] Recuperar la pendiente de pitch RPM del motor interior y los gains base por instrumento.
- [x] Implementar renderer Godot con voces persistentes, smoothing, loops/one-shots explícitos y resampling nativo.
- [x] Añadir journal temporal de one-shots con IDs deduplicables para no perder impactos entre 120 Hz de simulación y el render de Godot.
- [x] Añadir selector exclusivo `CommonV10Commands | LegacyV10Pcm | Disabled`.
- [ ] A/B contra cinco bandas antiguas: continuidad RPM, lift-off, shifts, latencia y clipping.
- [x] Mantener motor antiguo como fallback, nunca simultáneo.

### Sprint 3 — common prioritario + deduplicación

- [x] Implementar common: kerb, gravel, grass, sand/dirt, track hit y track scrape.
- [x] Aplicar ownership por IDs estables: textura de superficie y skid son voces distintas.
- [x] Mover selección de evento/sample de colisión a Rust; C++ solo clasifica y entrega magnitudes.
- [x] Desactivar mixer `surf_*` e impactos antiguos bajo backend nuevo.

### Sprint 4 — exterior, transmisión y neumáticos

- [x] Implementar primer LOD `engine_ext` con mezcla interior/exterior según distancia del listener.
- [x] Implementar transmission y wind (primera calibración).
- [ ] Implementar skid interior/exterior por rueda.
- [x] Implementar tyre rolling, brake loop, flat/flutter y blowout desde carga, freno y presión reales.
- [x] Establecer presupuesto inicial de voces continuas/one-shots y limiter en el bus `Vehicle`.

### Sprint 5 — eventos bloqueados y cobertura secundaria

- [ ] Resolver/implementar backfire, limiter, TC y gear int/ext; retirar fallback antiguo solo con paridad.
- [ ] Evaluar turbo; desactivarlo si no corresponde al vehículo/modelo físico.
- [ ] Implementar bodywork y luego pit/ambience common.
- [ ] Mantener door/horn fuera de alcance salvo requisito de gameplay.

### Sprint 6 — deprecación controlada

- [ ] Hacer backend de comandos default en el nodo Godot `F90Core` solo después del gate auditivo; actualmente el default seguro es legacy y la escena de prueba hace opt-in.
- [ ] Deprecar `VehicleAudioEngine::render`, `VehicleSoundBank`, ABI independiente y `v10_vehicle` manifest/samples.
- [ ] Deprecar `VehicleAudioControllerNative` y reglas espejo GDScript.
- [ ] Migrar HUD/telemetry de cinco weights/pitches a voces/eventos activos.
- [ ] Dos sprints estables sin duplicados, gaps, regresiones de CPU o replay antes de borrar fallback.
- [ ] Gate humano y retrospectiva antes de eliminar assets/código.

## Criterios de aceptación

- Un solo `AudioCommandFrame` por tick y un único dueño Rust de decisiones.
- Ninguna carga de FMOD ni parsing `.bank` en runtime.
- Engine interior/exterior nunca se duplican accidentalmente; su mezcla depende de listener mode.
- Surface, rolling y skid tienen ownership distinto y métricas separadas.
- Misma telemetría+seed produce mismos comandos/variantes.
- Godot realiza reproducción, resampling, spatialization, buses y efectos; no infiere física.
- El backend anterior puede habilitarse como fallback exclusivo hasta completar el gate.

## Registro de ejecución (2026-08-21, Fase 2b — short-region scan y test de reset)

- Barrido de regiones cortas (4..40 periodos) por capa en
  `fix_loop_regions.py`: las regiones cortas mejoran algunos casos (brakes
  4 ciclos -10.4 dB, ext_idle 5 ciclos -4.0) pero **ninguna capa alcanza
  seams <= -30 dB**: las grabaciones no son loops estacionarios (drift/
  sweep). `loop_regions.json` registra por sample las tres políticas
  medibles (whole / región larga k*P / región corta) para la decisión
  auditiva; el horneado de crossfades quedó descartado como no suficiente
  por medición y la decisión final es del gate (enmascaramiento en mezcla).
  Regiones aplicadas en `commands.rs` se mantienen (mejores medidas para
  on_mid/on_high/off_mid; on_midhigh conserva la previa, mejor seam).
- Test nuevo `backend_switch_does_not_block_new_one_shot_ids`: tras salir y
  volver al backend commands, los one-shots siguen con IDs monótonos nuevos
  (el renderer deduplica por ID — check del handoff §7 paso 5).
- Borrador de retrospectiva: `docs/audio/RETROSPECTIVA_INTEGRACION_AUDIO.md`
  (pendiente de gates; incluye checklist de deprecación).
- Regresión de la escena canónica `vehicle_test_session.tscn` (handoff §2):
  schema 2, backend commands, 28 voces, sin dobles productores, sin errores.

## Registro de ejecución (2026-08-21, Fase 6 — grabación y A/B de sesión real)

- Grabador de telemetría por tick: `AudioModule::set_record_path`
  (`f90_core_audio_set_record_path` FFI → `F90Core::set_audio_record_path` →
  export del renderer `debug_record_path`; `user://` se globaliza en C++).
  CSV de 29 campos por tick 120 Hz: rpm, throttle, speed, drivetrain, gear,
  slip, slip_ratio/surfaces/brake/cargas/suspensión/presiones por rueda y
  listener distance — los mismos inputs exactos del adaptador.
- Replay offline: `vehicle-audio-engine/examples/live_replay.rs` (misma
  telemetría → ambos backends; el detector de huecos usa `max(interior,
  exterior)`, ya que el crossfade de cámara silencia un lado a propósito).
- Verificación con sesión headless real: `diagnostics/telemetry_live.csv`
  (2761 ticks) → `diagnostics/ab/live_telemetry_live.legacy.wav` (audio PCM
  legacy de referencia) + `.commands.json` (0 huecos de motor, 33 voces pico).
- Escena `vehicle_test_session_record.tscn`: graba la sesión del usuario para
  su A/B auditivo con el mismo comando del handoff §8.

## Registro de ejecución (2026-08-21, Fase 6 — harness A/B offline)

- `game/crates/vehicle-audio-engine/tests/ab_legacy_commands.rs`: los 4
  escenarios de audición (idle→redline con 3 cambios, asfalto→kerb→sand,
  grass skid, impact; los mismos de `tools/audio/scenarios.py`, remuestreados
  a 120 Hz) se renderizan con **la misma telemetría** en ambos backends:
  - commands: `CommonV10BankAdapter` → `diagnostics/ab/<scenario>.commands.json`
    (trace determinista con `seq`, huecos de motor, shots, muestreo de
    encabezados) y aserciones: motor sin huecos en todo el escenario, 3
    one-shots de cambio en los 3 upshifts, eventos de colisión correctos
    (prop/barrier/body + cockpit_rattle en el fuerte).
  - legacy: `VehicleAudioEngine` + render PCM → `diagnostics/ab/<scenario>.legacy.wav`
    (44.1 kHz estéreo), el audio de referencia que el gate humano compara.
- Resultados resumidos en `diagnostics/ab/*.summary.txt`: idle_to_redline 3
  gear shots, impact 4 shots (prop, barrier, rattle, body), 0 huecos de motor
  en los 4 escenarios, pico de 36 voces.

## Registro de ejecución (2026-08-21, Fase 5 — telemetría por rueda y colisiones)

- `slip_ratio` real por rueda: `finish_frame` ahora entrega
  `tires.wheels[i].slip_ratio` (física) en lugar de `tc_slip_ratio` (leía 0 con
  TC desactivado); el skid ponderado por carga usa el deslizamiento real.
  `drivetrain_speed` y slips comparten un único lookup de entidad por tick.
- Colisiones C++ (`f90_core.cpp`): en modo commands se entregan **todos los
  contactos** (hasta 4 por frame, ordenados por impacto normal, sin cooldown ni
  `break`); `kind=3` (Vehicle) cuando el collider es un `RigidBody3D` (antes
  todo coche-coche caía en Generic). La autoridad de umbral/variantes sigue en
  Rust. Test `collisions_process_multiple_contacts_per_tick_sorted_strongest_first`:
  dos contactos del mismo tick → dos one-shots con IDs monotónicos +
  cockpit_rattle en el fuerte.
- Variantes del evento kerb: `kerb3_p6` (+2.2 st) y `kerb p` (+19.5 st) con
  retrigger, además del bed `kerb_rumble_digital` ya presente (todas con bases
  del banco: 2.0/0.0 dB).
- `floor_scrape` con base del banco exacta (+10 dB, +0.6 st); resto de bases
  common verificadas contra el dump (kerb 6/0, grass -3.5/0.7, gravel 4.5/2.4,
  sand 7.5/-0.9, crash/impact/cone ya coincidían).
- Presupuesto: 48 voces (pico actual 35 con kerb tri-capa). Tests 76 verdes;
  golden `diagnostics/golden_trace_v10_v2.json`; smoke full sin errores.

## Registro de ejecución (2026-08-21, Fase 3+4 — engine_ext y transmission exactos)

- `engine_ext`: las 17 capas con ventanas y gains exactos del banco
  (24 instrumentos recuperados fusionados por sample; `r25 ext on high rear
  close 1` sin automatización recuperable → ventana de entrada 17000-17400
  documentada como aproximación). Beds continuos (idle 0-7000, off_mid
  5200-12800, off_midhigh/midhigh_front 9600-15600, off_downshift* 15200+)
  cubren todo el rango; capas on_* con 3D front/rear y ×throttle. Gains base
  exactos, pitch del banco (-1 st en capas front), emitters rear/front.
- `transmission`: ventanas exactas en dominio `drivetrain_speed` (km/h):
  gt3r 0..152.2, z4gt3 midhigh 143..176.7, z4gt3 high 172.4..350 (sin huecos).
  **`AudioTelemetryFrame.drivetrain_speed_kph` conectado a la velocidad real
  del eje (media de ruedas motrices × radio × 3.6) en `finish_frame`** — se
  eliminó el proxy `speed_kph/350`. Gains base del banco (6.5/4/2 dB) sin
  trims.
- Presupuesto de voces: 32 → 48 en el renderer (el banco reproduce 55
  instrumentos; el set commands emite 33 en idle + superficie).
- Gates golden añadidos: `engine_ext_bed_sweep_has_no_gain_holes` (ext bed a
  10 m listener: on/off min -6.1 dB, sin huecos) y
  `transmission_windows_cover_full_drivetrain_domain` (5..350 km/h sin
  huecos). Todos los gates: `diagnostics/golden_trace_v10_v2.json`
  (sweeps on/off + ext_10m on/off).
- Smoke `vehicle_test_session_full.tscn` (mutes=0): 28 voces en idle (10 int +
  17 ext + ambience), ext idle -3.0 dB a 10 m y cruce interior/exterior por
  distancia verificado en captura.

## Registro de ejecución (2026-08-21, Fase 2 — loop policy, medición)

- Herramientas: `tools/audio/audit_loop_seams.py` y `tools/audio/fix_loop_regions.py`
  (decodifica WAV y OGG con `soundfile`, periodo de disparo por autocorrelación
  FFT, regiones alineadas a múltiplo entero del periodo, métrica de seam
  `RMS(diff)/RMS` en dB).
- Resultado (`diagnostics/loop_seam_audit_final.json`): **ninguna capa continua
  del banco es un loop estacionario perfecto**: seams de +0.4 a +4.7 dB en
  loop de archivo completo (click audible por ciclo: on_mid cada 1.66 s,
  on_high cada 4.94 s, tyre_rolling cada 10 s...). Las regiones originales
  hardcodeadas también eran malas (4.4-4.6 dB). Las regiones realineadas al
  periodo mejoran lo medible (on_mid 4.4→2.7 dB según auditor, 0.3 dB según
  buscador) pero el drift de las grabaciones impide seams ≤ -60 dB sin editar
  el asset.
- Aplicado en `commands.rs`: regiones medidas finales para on_mid/on_high/
  off_mid (on_midhigh conserva la región previa, mejor seam medido) y
  `common.kerb` con `restart_on_finish=true` (antes moría en silencio a los
  15.2 s con el kerb activo).
- Pendiente de decisión auditiva (gate humano): assets con crossfade horneado
  en el punto de loop para las capas con click perceptible (candidatos:
  on_high, off_mid, ext_on_high, tyre_rolling) y OGGs largos (idle, off_low,
  off_midhigh, ext_idle, ext_off_midhigh, wind, brakes, Skid) que Godot solo
  puede loopear completos (AudioStreamOggVorbis no admite regiones); en su
  defecto re-exportar los beds problemáticos a WAV con región (`*.loop.wav`
  ya se generan offline con `fix_loop_regions.py`).

## Registro de ejecución (2026-08-21, Fase 1 — engine_int)

Instrumentación y tablas verificables añadidas:

- `catalog.v2.json` (`game/sounds/runtime/catalog.v2.json`, generado por
  `tools/audio/build_engine_tables.py` desde el dump offline del parser de
  `.bank`): tablas exactas por instrumento (event, instrument, sample, gains
  base, pitch base, `auto_pitch_ref`, ventanas RPM enter/exit con puntos de
  curva) para `engine_int` (10 voces), `engine_ext` (24 instrumentos) y
  `transmission` (3), según los datos recuperados de `vrc_2005_renault_r25.bank`.
- `commands.rs` sustituyó las ventanas trapezoidales aproximadas de
  `engine_int` por las ventanas del banco: beds continuos
  (idle 0..3800→4600; off_low 3800→8200; off_mid 6600→14200 dos ventanas;
  off_midhigh 11800→15600; off_downshift 15200→17400+) y capas de carga
  (on_mid 6600→10600; on_midhigh 9200→15600; on_high 16800→17400+;
  on_upshift 13400→17400) escaladas por throttle. Gains base exactos del
  banco, sin trims.
- Golden trace con gate automático: `engine_int_sweep_has_no_gain_holes_on_or_off_throttle`
  exige 0 huecos >12 dB en 2053..17400 rpm con throttle 1 y 0. Antes: ralentí
  a -71.5 dB y lift-off 15.6k..17.4k muerto (42 huecos). Ahora: on min -5.7 dB,
  off min -6.8 dB, 0 huecos, determinista, ≤32 voces (`diagnostics/golden_trace_v10_v2.json`).
- Escena de diagnóstico `game/scenes/runtime/vehicle_test_session_engine_int.tscn`
  (solo `engine_int`, captura por comando en `user://audio_capture_engine_int.txt`)
  y mute por familia vía `set_audio_mute_mask` (FFI+C+++GDScript).

Estado: engine_int corregido; pitch global del banco ya coincidente
(5281.519→15687.891); `auto_pitch_ref` por instrumento queda registrado en el
catálogo pendiente de la calibración auditiva (prueba con/without refs en el
gate). El gate auditivo humano del sweep interior sigue pendiente; engine_ext,
transmission, wind, wheel, surfaces y collisions siguen siendo LOD/MVP
aproximados (fases 3-5).

## Revisión de la corrección actual

Problemas reproducibles que motivaron esta pasada y ya están corregidos:

- sample de transmisión asignado por error a `engine_int/on_high`;
- pitch fijo y gains alejados de los metadatos recuperados del banco;
- one-shots visibles durante un solo tick de 120 Hz y, por tanto, perdibles por un renderer más lento;
- todos los instrumentos forzados a loop de archivo completo, incluso cuando eran one-shots o tenían regiones de loop;
- recreación de nodos al cruzar umbrales, reinicios de fase y cambios de gain/pitch sin smoothing;
- entradas declaradas (`brake`, carga, suspensión y presión) publicadas siempre a cero;
- ausencia del director/renderer en `world_hud_compositor` y falta de protección de clipping.

Pendientes antes de promover el backend:

- [ ] Resolver hasta propiedad nombrada las curvas restantes del banco y reemplazar las aproximaciones donde difieran perceptualmente.
- [ ] Validar auditivamente las regiones de loop y los OGG largos en cockpit y cámara exterior.
- [ ] Separar skid y rolling por rueda con posiciones 3D reales; la implementación actual aún los agrega.
- [ ] Refinar `engine_ext` front/rear, close/far y cone angle; el primer LOD solo demuestra ownership y distancia.
- [ ] Añadir estado de pit; no inferir acciones de pistola desde movimiento del vehículo.
- [ ] Resolver los nueve eventos V10 sin sample estático antes de retirar limiter/TC/backfire legacy.
- [ ] Ejecutar el gate humano A/B y documentar RPM sweep, lift-off, cambios, impactos, off-track y clipping.
