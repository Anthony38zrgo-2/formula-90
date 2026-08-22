# Diccionario y propuesta de adaptación de `common.bank`

Estado: diccionario permanente; vertical slice implementado y pendiente de gate auditivo. Fecha del análisis: 2026-08-21.

Complementos: [diccionario V10 v2](FMOD_V10_V2_BANK_DICTIONARY.md) y [backlog conjunto](AUDIO_BANKS_INTEGRATION_BACKLOG.md).

## Decisión de arquitectura

El banco no debe cargarse con FMOD durante el juego ni debe convertirse en un motor FMOD genérico. Debe tratarse como una especificación de diseño congelada y especializada:

```text
Simulation
  -> AudioTelemetryFrame (Rust, una vez por tick fijo)
  -> CommonBankAdapter (Rust, reglas específicas de estos 16 eventos)
  -> AudioCommandFrame v2 (Rust, estado + journal de one-shots)
  -> GodotAudioRenderer (Godot, voces, streams, buses y espacialización)
```

Rust es la única autoridad que decide qué evento existe, cuándo comienza/termina y con qué parámetros. Godot no vuelve a inferir superficies, impactos, suciedad o selección aleatoria; solo aplica comandos idempotentes a reproductores. El adaptador no parsea `.bank` en runtime. Sus datos se extraen offline a un manifiesto versionado y sus reglas son código/configuración Rust revisable.

La integración mínima se hace dentro de la fachada existente `formula90-core`, inmediatamente después de producir la física del tick y antes de publicar `CoreFrame`. Se reutilizan el reloj fijo, el estado de audio y el transporte FFI existentes. `vehicle-audio-engine` conserva el motor V10 actual como fallback; el nuevo adaptador produce comandos, no PCM mezclado.

## Evidencia del escaneo

Archivos analizados:

- `common.bank`: 44,099,840 bytes; SHA-256 `25E1BB7044152C900F839FC508F56B7840299AF05A55A557B3B9F780DA4E82E1`.
- `common.strings.bank`: 3,584 bytes; SHA-256 `51747AA0D02340A0715F0C7A002A0E10DDF07DDE4FB1A2A63A163B07BC9786EB`.
- Formato del banco: file-version 80 (`FILEVERSION_AUTOPITCH_AT_MINIMUM`), FSB5 PCM16.
- Estructura: 16 eventos, 31 WAV, 30 nodos de parámetro, 30 layouts de parámetro, 16 timelines, 76 instrumentos, 44 moduladores, 161 controladores y 161 curvas.
- Los 31 samples son estéreo, 44.1 kHz. El evento `event:/surfaces/tyre_rolling` no resolvió un sample; los otros 15 eventos sí.
- Parámetros: `speed` 0..500; `impact_speed` 0..1000; `impact_angle` 0..180; `dirtiness` 0..1; `decay` 0..1 o 0..10 según evento; `Distance` automático con máximo 20, 25 o 300.
- Hay 42 moduladores ADSR y 2 moduladores Random. Existen dos playlists `SmartRandom`: tres variantes de wheel-gun-off y dos de wheel-gun-on, todas con peso 1.
- Los valores base de instrumento contienen gain/volume en dB, pitch en semitonos, probabilidad y loop count. Las 161 curvas se recuperaron como puntos `(x,y,shape,type)`, pero el parser disponible no resuelve de forma confiable toda la cadena controller/owner/parameter hasta una propiedad nombrada. No se deben copiar curvas huérfanas al runtime; se validarán auditivamente por evento en el backlog.

Limitación importante: un `.bank` compilado no conserva la intención editorial completa del proyecto FMOD. El diccionario marca como “confirmado” lo recuperado estructuralmente y como “propuesto” la interpretación Formula-90.

## Contratos propuestos

`AudioTelemetryFrame` debe ser un valor puro por tick, no una colección de lecturas tardías desde Godot:

```rust
struct AudioTelemetryFrame {
    tick: u64,
    dt_s: f32,
    vehicle_speed_m_s: f32,
    wheel_speed_m_s: [f32; 4],
    slip_ratio: [f32; 4],
    slip_angle_rad: [f32; 4],
    wheel_load_n: [f32; 4],
    suspension_velocity_m_s: [f32; 4],
    surface: [SurfaceType; 4],
    contact_mask: u8,
    dirtiness: f32,
    collision: Option<CollisionAudioInput>,
    pit: PitAudioState,
    listener_distance_m: f32,
}
```

`AudioCommandFrame` separa estados continuos de deltas. Cada voz lleva un ID estable para que Godot no reinicie loops en cada tick:

```rust
struct AudioCommandFrame {
    tick: u64,
    continuous: Vec<SetVoice>, // ensure/start/update/stop con gain, pitch y 3D
    one_shots: Vec<PlayOneShot>, // event id, variant id/seed, gain, pitch, 3D
    stops: Vec<VoiceId>,
}
```

La selección de variantes debe ser determinista en Rust (`race_seed + entity_id + event_counter`). Godot recibe el sample/variant ya elegido. Los comandos deben incluir `generation` o `tick` para deduplicación y permitir replay exacto.

## Mapeo de parámetros

| Parámetro FMOD | Fuente Formula-90 | Transformación inicial propuesta | Observaciones |
|---|---|---|---|
| `speed` 0..500 | velocidad tangencial por rueda; fallback `vehicle_speed_m_s * 3.6` | km/h, clamp 0..500; para superficies usar RMS/max de ruedas en contacto de esa superficie | No usar solo velocidad del chasis cuando hay wheelspin o lock-up. |
| `impact_speed` 0..1000 | velocidad relativa normal y/o impulso del contacto | `max(abs(v_rel·n), impulse/mass)` en m/s; adaptar a dominio del banco con curva calibrada | El C++ actual usa `max(normal_speed, impulse*0.1)`; mover la decisión a Rust. |
| `impact_angle` 0..180 | normal de contacto y vector relativo | grados entre `-v_rel` y normal; 0 frontal, 90 tangencial | Sirve para crossfade hit/scrape y paneo/posición. |
| `dirtiness` 0..1 | estado persistente off-track por rueda | ataque por tiempo/carga sobre dirt/gravel/sand; recuperación gradual sobre asfalto | No equivale a surface enum. Debe persistir y modular debris incluso al reingresar. |
| `decay` 0..1/10 | estado derivado del evento | envelope/release normalizado, no input de Godot | Mantener interno al adaptador; no añadirlo a física. |
| `Distance` | distancia emisor-listener | metros, calculada/presentada por Godot o incluida en telemetría de cámara | Preferible que Godot aplique atenuación 3D; Rust solo fija política/rango. |

Variables complementarias:

- `slip_ratio` y `slip_angle`: intensidad de tyre rolling/gravel y transición rodadura↔skid.
- `wheel_load_n`: gate de contacto y ponderación por rueda; evita audio de rueda en el aire.
- `suspension_velocity_m_s`: golpes de kerb y cockpit rattle, con filtro high-pass/onset.
- `surface[4]`: mezcla por rueda, no una única superficie dominante.
- máscara/contact duration: hysteresis de entrada/salida para evitar chatter.
- energía tangencial de contacto: scrape continuo; energía normal: hit one-shot.
- estado de pit: comandos semánticos `WheelGunOn/Off`, no heurística acústica.

## Diccionario de eventos

Convenciones: `loop` indica que al menos un instrumento asociado tiene `LoopCount=-1`; “mixto” combina beds continuos y elementos disparados. Gain/pitch son valores base recuperados de instrumentos y pueden estar automatizados por curvas. Estado “pendiente” significa identificado pero aún no conectado al contrato Rust.

### Surfaces

| event | samples | tipo | parámetros | loop/oneshot | categoría | posible input Formula-90 | estado de implementación | notas |
|---|---|---|---|---|---|---|---|---|
| `event:/surfaces/tyre_rolling` | ninguno resuelto | continuo esperado | ninguno recuperado | indeterminado | surfaces | wheel speed, load, slip, asphalt contact | pendiente/bloqueado por asset | Evento válido sin WAV resoluble en este banco; no inventar sample. |
| `event:/surfaces/gravel` | `tyres_gravel` | bed granular | `speed` 0..500, `decay` 0..10 | instrumento base no-loop; evento continuo por timeline | surfaces | wheel speed, slip, load, gravel mask | MVP implementado; calibración pendiente | Base: +4.5 dB, +2.4 semitonos; sample 13.003 s. |
| `event:/surfaces/kerb` | `kerb3_p6`, `kerb p`, `kerb_rumble_digital` | bed/impulsos de kerb | `Distance` 0..300, `decay` 0..1, `speed` 0..500 | mixto | surfaces | curb contact por rueda, wheel speed, suspension velocity/load | MVP implementado; variantes pendientes | Variantes incluyen `kerb p` con +19.5 st y modulador Random; `kerb_rumble_digital` 15.209 s. |
| `event:/surfaces/extraturf` | `kerb p` | superficie especial | `decay` 0..10, `speed` 0..500 | one-shot/segmentado | surfaces | surface subtype extraturf, speed, load | pendiente; requiere subtype | No colapsar automáticamente a kerb hasta confirmar semántica de pista. |
| `event:/surfaces/grass` | `grass_5_p`, `grass_4_p`, `stone hits unp`, `stone hits p short` | bed + debris | `decay` 0..10, `Distance` 0..300, `dirtiness` 0..1, `speed` 0..500 | mixto; stone layers loop en algunas instancias | surfaces | grass contact, speed, slip, load, dirtiness | MVP implementado; por-rueda pendiente | `grass_4_p` tiene instancias loop y no-loop; debris entra por dirtiness/velocidad. |
| `event:/surfaces/sand` | `stone hits 2unp`, `stone hits unp`, `sand2_p` | bed + debris | `speed` 0..500, `Distance` 0..20, `decay` 0..10 | mixto; stone layers loop | surfaces | sand contact, wheel speed, slip/load | MVP implementado; por-rueda pendiente | Distance corto (20 m), apropiado para fuente vehículo. |
| `event:/surfaces/old` | `grass_4_p`, `kerb p` | legado/híbrido | `Distance` 0..300, `decay` 0..10, `speed` 0..500 | one-shot/segmentado | surfaces | solo fallback de superficies antiguas | deprecar/no activar por defecto | Nombre explícitamente legacy; conservar en manifiesto para paridad. |

### Collisions

| event | samples | tipo | parámetros | loop/oneshot | categoría | posible input Formula-90 | estado de implementación | notas |
|---|---|---|---|---|---|---|---|---|
| `event:/collisions/car/hit` | `crash_wall_p2`, `impact_2` | impacto | `impact_speed` 0..1000, `impact_angle` 0..180 | one-shot | collisions | car-car relative normal velocity, impulse, angle | pendiente | Curvas visibles con umbrales de impacto; calibrar, no copiar por GUID. |
| `event:/collisions/object/hit` | `cone_hit`, `impact_2` | impacto objeto | `impact_speed` 0..1000, `impact_angle` 0..180 | one-shot | collisions | collision class + relative velocity/impulse | MVP implementado | `cone_hit` 0.441 s; existen bases +7.5 dB/+0.7 st y +5.5 dB/-1.4 st. |
| `event:/collisions/track/hit` | `crash_wall_p2`, `rattle_cockpit_p`, `crash_hit_nascar`, `crash_wall_p`, `crash_squeak_unp`, `rattle_cockpit_p2`, `impact_2`, `crash_wall_2unp` | impacto compuesto | `impact_angle` 0..180, `impact_speed` 0..1000 | one-shot multicapa | collisions | barrier/track class, normal speed, impulse, angle, chassis location | MVP implementado; multicapa pendiente | C++ entrega hechos; Rust selecciona evento/sample bajo el backend nuevo. |
| `event:/collisions/track/scrape` | `floorboard_indy` | roce sostenido | `decay` 0..1, `speed` 0..500, `Distance` 0..25 | loop | collisions | tangential contact speed, scrape force/intensity, underfloor state | implementado con telemetría underfloor | `floorboard_indy`: 5.039 s, +10 dB, +0.6 st, loop. |
| `event:/collisions/car/scrape` | `grass_4_p`, `rattle_cockpit_p2` | roce sostenido multicapa | `decay` 0..1, `speed` 0..500 | mixto; rattle loop | collisions | car-car tangential speed, contact force, chassis vibration | pendiente | No reutilizar el one-shot `impact_scrape` como loop reiniciado. |

### Debris/dirt

| event | samples | tipo | parámetros | loop/oneshot | categoría | posible input Formula-90 | estado de implementación | notas |
|---|---|---|---|---|---|---|---|---|
| `event:/common/dirt` | `stone hits p short`, `stone hits 2unp` | debris persistente/ráfagas | `dirtiness` 0..1, `speed` 0..500 | mixto; ambos tienen instancias loop | debris/dirt | dirtiness persistente, wheel speed, load, recent off-track | MVP implementado | Estado persistente con ataque off-road y recuperación gradual sobre asfalto. |

### Mechanical/cockpit

No hay un evento standalone de cockpit en el banco. `rattle_cockpit_p`, `rattle_cockpit_p2` y `floorboard_indy` son capas dentro de `track/hit`, `track/scrape` y `car/scrape`. Deben emitirse desde esos eventos compuestos, no como un subsistema duplicado. Samples: `rattle_cockpit_p` 1.981 s, `rattle_cockpit_p2` 5.944 s y `floorboard_indy` 5.039 s.

### Pit

| event | samples | tipo | parámetros | loop/oneshot | categoría | posible input Formula-90 | estado de implementación | notas |
|---|---|---|---|---|---|---|---|---|
| `event:/common/screw` | `pitstop_wheel_gun_on1`, `pitstop_wheel_gun_on2_short` | wheel gun on | ninguno | one-shot, SmartRandom | pit | pit task `WheelGunOn`, wheel index, crew position | pendiente | Dos variantes equiprobables. |
| `event:/common/unscrew` | `pitstop_wheel_off3`, `pitstop_wheel_on3`, `pitstop_wheel_off2`, `pitstop_wheel_on2`, `pitstop_wheel_gun_off3_short`, `pitstop_wheel_gun_off2_short`, `pitstop_wheel_gun_off1_short` | secuencia de rueda | ninguno | one-shot/secuencia; SmartRandom de tres gun-off | pit | task begin/end, wheel index, success state | pendiente | `pitstop_wheel_on3` tiene trigger chance 39.5%; conservar probabilidad determinista en Rust. |

### Ambience

| event | samples | tipo | parámetros | loop/oneshot | categoría | posible input Formula-90 | estado de implementación | notas |
|---|---|---|---|---|---|---|---|---|
| `event:/common/ambience` | `ambience_mix`, `ambient_plane_distant_pass_overhead` | bed + evento raro | ninguno | bed + one-shot probabilístico | ambience | session/track active, camera/listener, deterministic ambient clock | bed implementado; avión pendiente | `ambience_mix` 96 s; la agenda determinista del avión queda pendiente. |

### Otros

No hay otros eventos dentro de `common.bank`. `common.strings.bank` contiene rutas de eventos pertenecientes a otros bancos (`tatuusfa1`, `showroom`) y sus buses/VCA; no forman parte de los 16 eventos cargados en `common.bank` y no deben implementarse desde este adaptador.

## Inventario completo de samples

| sample | duración s | uso |
|---|---:|---|
| `kerb p` | 6.487 | kerb, extraturf, old |
| `crash_wall_2unp` | 1.521 | track hit |
| `pitstop_wheel_gun_off2_short` | 0.642 | unscrew |
| `stone hits p short` | 9.660 | grass, dirt |
| `cone_hit` | 0.441 | object hit |
| `ambience_mix` | 96.000 | ambience |
| `ambient_plane_distant_pass_overhead` | 39.007 | ambience |
| `pitstop_wheel_on2` | 1.057 | unscrew |
| `rattle_cockpit_p` | 1.981 | track hit |
| `pitstop_wheel_gun_off1_short` | 0.774 | unscrew |
| `grass_5_p` | 3.889 | grass |
| `pitstop_wheel_off3` | 0.406 | unscrew |
| `pitstop_wheel_on3` | 0.746 | unscrew |
| `kerb3_p6` | 2.750 | kerb |
| `crash_hit_nascar` | 0.745 | track hit |
| `tyres_gravel` | 13.003 | gravel |
| `impact_2` | 0.760 | car/object/track hit |
| `pitstop_wheel_gun_on2_short` | 0.673 | screw |
| `crash_wall_p` | 1.969 | track hit |
| `pitstop_wheel_gun_off3_short` | 0.542 | unscrew |
| `crash_squeak_unp` | 2.090 | track hit |
| `pitstop_wheel_gun_on1` | 0.856 | screw |
| `stone hits 2unp` | 16.733 | sand, dirt |
| `kerb_rumble_digital` | 15.209 | kerb |
| `crash_wall_p2` | 1.076 | car/track hit |
| `grass_4_p` | 3.119 | grass, old, car scrape |
| `rattle_cockpit_p2` | 5.944 | track hit, car scrape |
| `floorboard_indy` | 5.039 | track scrape |
| `sand2_p` | 3.657 | sand |
| `pitstop_wheel_off2` | 1.579 | unscrew |
| `stone hits unp` | 11.018 | sand, grass |

## Pipeline actual y puntos de inserción

1. `formula90-core::CoreFacade::step*` produce física y `CoreFrame` en el mismo tick.
2. `finish_frame` reduce las cuatro ruedas a una superficie dominante y un único slip, llama `AudioModule::set_state` y guarda `AudioReadouts`.
3. `AudioModule` alimenta `vehicle-audio-engine`, que carga WAV/manifest del banco `v10_vehicle`, decide shifts/backfire/surface beds y mezcla PCM sample-accurate.
4. `F90Core` nativo bombea ese PCM con `AudioStreamGenerator.push_buffer`; aquí Godot es renderer final, pero Rust todavía es también mixer.
5. `F90Core::process_collision_audio` clasifica contactos y decide one-shots en C++, fuera de la autoridad Rust.
6. La ruta legacy `VehicleAudioControllerNative` relee propiedades del vehículo, vuelve a detectar superficie/slip, dispara cambios y mezcla mediante una DLL separada. El GDScript `vehicle_audio_controller.gd` conserva una tercera implementación espejo.

El adaptador se inserta en `finish_frame`, pero necesita recibir datos por rueda y un `CollisionAudioInput` generado en la frontera de simulación. A corto plazo, la captura de contactos puede seguir en el bridge C++ siempre que este solo transporte hechos (normal, velocidad relativa, impulso, clase), nunca decida el sample/evento.

## Plan de deprecación sin audio duplicado

Introducir un selector único de backend, no flags booleanos independientes:

```text
AudioBackend = CommonCommands | LegacyV10Pcm | Disabled
```

- `CommonCommands`: `CommonBankAdapter` activo; Godot command renderer activo; mixer V10 PCM, reglas GDScript y triggers de colisión C++ inactivos.
- `LegacyV10Pcm`: comportamiento actual; adapter common y command renderer inactivos.
- `Disabled`: solo readouts/diagnóstico.

Deprecaciones por sistema:

| sistema actual | dueño actual | acción |
|---|---|---|
| ambience | no existe en audio Rust actual | añadir solo en `CommonCommands`; no mezclar con futuros nodos de pista independientes |
| surface beds | Rust V10 + detección legacy C++/GDScript | reutilizar normalización/hysteresis útil; desactivar `surf_*` cuando `CommonCommands` esté activo |
| gravel/dirt | sand colapsado en Rust V10 | separar gravel/sand/dirtiness en telemetría; conservar colapso como fallback legacy |
| kerbs | `surf_rumble` | sustituir por evento kerb del adapter; nunca reproducir ambos |
| collisions/impacts | decisión en `F90Core::process_collision_audio` | convertir a captura de hechos y mover clasificación/variant a Rust; feature-gate de triggers legacy |
| underfloor scrape | Rust state + `impact_scrape` | reutilizar intensidad/velocidad/onset; en common comandar `track/scrape` y silenciar scrape V10 |
| cockpit rattles | implícitos en samples common | mantener dentro de eventos compuestos common; no crear subsystem paralelo |
| pit | no implementado | comandos del modelo de pit hacia adapter; Godot solo reproduce |

## Backlog de implementación

### Sprint 0 — gate de evidencia

- [ ] Archivar un dump offline reproducible del banco (GUIDs, event/sample map, parámetros, instrumentos, curvas y hashes) en tooling, sin dependencia runtime.
- [ ] Escuchar/exportar legalmente los 31 PCM y etiquetar loop seam, loudness, pitch y función perceptual.
- [ ] Resolver controller→property→curve para los eventos prioritarios: kerb, gravel, grass, sand/dirt, track hit y track scrape.
- [ ] Acordar unidades de `speed`, `impact_speed` e `impact_angle` mediante sweep offline.
- [ ] Gate humano: aprobar diccionario y alcance antes de modificar runtime.

### Sprint 1 — contratos, sin reproducción

- [x] Añadir `AudioTelemetryFrame` por rueda a `formula90-core`; derivarlo una sola vez del estado ya calculado.
- [x] Añadir `CollisionAudioInput` como hechos de contacto; mantener decisiones legacy solo detrás del fallback.
- [x] Añadir `AudioCommandFrame` serializable y readouts de diagnóstico.
- [x] Tests de determinismo y supervivencia/deduplicación de one-shots entre tick y render.

### Sprint 2 — adaptador vertical mínimo

- [x] Implementar `CommonBankAdapter` para kerb, gravel, grass, sand/dirt, track hit y track scrape.
- [x] Generar catálogo offline de samples extraídos/convertidos y hashes; no leer `.bank` en runtime.
- [x] Mantener curvas/config específicas del banco junto al adapter, sin abstracción de motor FMOD.
- [x] Selección aleatoria determinista y hysteresis por voz/rueda.

### Sprint 3 — renderer Godot

- [x] Crear un renderer fino de `AudioCommandFrame` con `AudioStreamPlayer(3D)`, IDs estables y buses.
- [x] Aplicar gain/pitch/loop/start-stop, smoothing, posición y atenuación; ninguna heurística de física.
- [x] Implementar `AudioBackend` mutuamente excluyente y protección contra doble productor.
- [ ] A/B contra `LegacyV10Pcm`, con captura de comandos y audio.

### Sprint 4 — cobertura restante

- [ ] Pit `screw/unscrew` con secuencia y random determinista.
- [ ] Ambience con reloj determinista y política de cámara/listener.
- [ ] Evaluar `extraturf`; dejar `old` desactivado salvo pista legacy.
- [ ] Resolver o documentar definitivamente `tyre_rolling` sin asset.

### Sprint 5 — deprecación controlada

- [ ] Hacer `CommonCommands` default tras pruebas de paridad/aceptación.
- [ ] Marcar `VehicleAudioControllerNative`, reglas GDScript y collision triggers C++ como deprecated fallback.
- [ ] No eliminar código hasta dos sprints estables y aprobación humana.
- [ ] Retrospectiva: latencia, voces máximas, CPU, duplicados, gaps de loops y cobertura de eventos.

## Criterios de aceptación

- Una sola decisión de audio por tick y entidad, originada en Rust.
- Cero lectura de telemetría física desde el renderer Godot.
- Cero FMOD runtime y cero parser `.bank` dentro del juego.
- Ningún evento duplicado entre common y V10 legacy.
- Replay determinista produce el mismo `AudioCommandFrame` y las mismas variantes.
- Godot puede reiniciar/recrear nodos sin cambiar la lógica de evento.
- La ruta legacy sigue disponible como fallback explícito hasta el gate de retirada.
