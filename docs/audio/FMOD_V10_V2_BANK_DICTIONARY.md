# Diccionario del banco V10 v2

Estado: diccionario permanente; vertical slice implementado y pendiente de gate auditivo. Banco: `vrc_2005_renault_r25.bank`.

## Resumen verificable

- Tamaño: 19,075,456 bytes.
- SHA-256: `6CC0E286E382BB71F1D8169CC8C7F7EA86D2A126AA790B4B52B9E92203843D03`.
- `GUIDs.txt` SHA-256: `E8017455427813E9648C8434D146839449C404F2EBD81A1FC8627DF0709F8B03`.
- FMOD file-version 80; audio FSB5 mixto Vorbis/PCM16.
- 19 eventos, 42 samples, 32 parámetros/layouts, 19 timelines, 55 instrumentos, 3 moduladores, 120 curvas/controladores, 8 transiciones.
- Sample rates mixtos: 44.1 y 48 kHz; canales mono y estéreo.
- 10 eventos resuelven samples locales. Nueve no resuelven samples mediante el grafo estático disponible: pueden ser eventos de control, eventos anidados, timelines vacíos o referencias no incluidas. No se debe inventar su implementación.
- 2 moduladores ADSR y 1 Random. Casi todos los instrumentos de motor son loops (`LoopCount=-1`) controlados directamente por `rpms`; el diseño es un mezclador por regiones RPM, no el crossfade triangular normalizado del banco anterior.

## Parámetros recuperados

| parámetro | rango | tipo | posible input Formula-90 |
|---|---:|---|---|
| `rpms` | 0..20000 | game-controlled | RPM real, sin normalizar |
| `throttle` | 0..1 | game-controlled | pedal/torque request; conviene añadir estado on/off con hysteresis |
| `drivetrain_speed` | 0..350 | game-controlled | velocidad del eje/caja o wheel speed motriz, km/h equivalentes |
| `speed` | 0..500 | game-controlled | vehicle/wheel speed km/h |
| `susp_travel_speed` | 0..1 | game-controlled | velocidad de suspensión normalizada por rueda |
| `suspension_damage` | 0..1 | game-controlled | daño mecánico persistente; actualmente no existe como contrato de audio |
| `inflation` | 0..1 | game-controlled | presión efectiva/objetivo normalizada |
| `brake` | 0..1 | game-controlled | pedal y/o brake torque normalizado |
| `boost` | 0..1 | game-controlled | presión de admisión normalizada |
| `bov` | 0..1 | game-controlled | evento/estado de descarga al levantar gas |
| `bov_decay` | 0..10 | game-controlled | envelope interno del adaptador |
| `state` | 0..1 | game-controlled | estado binario/continuo específico de puerta/cambio |
| `decay` | 0..1 | game-controlled | envelope interno del adaptador |
| `air_pressure` | 0.8..1.5 | game-controlled | presión/air density relativa; clima futuro |
| `Distance` | 0..15/20/25/50/500 | automático | distancia emisor-listener; renderer Godot |
| `Event Cone Angle` | 0..180 | automático | orientación emisor-listener; renderer Godot |

## Diccionario de eventos

| event | samples | tipo | parámetros | loop/oneshot | categoría | posible input Formula-90 | estado propuesto | notas |
|---|---|---|---|---|---|---|---|---|
| `engine_int` | 9 capas `r25 int ...`: idle, on mid/midhigh/high/upshift, off low/mid/midhigh/downshift | motor interior multicapa | RPM, throttle, Distance 0..50 | loops | engine | rpm real, throttle, gear phase, camera interior | corregido en comandos v2 (ventanas y gains exactos del banco, ver `catalog.v2.json`); gate auditivo pendiente | Ventanas recuperadas a nivel de instrumento: idle 0→3800-4600; off_low 3800-4600→6600-8200; off_mid 6600-8200→9400-11000 y 9400-11000→11800-14200; off_midhigh 11800-14200→15200-15600; off_downshift 15200-15600→; on_mid 6600-8200→9200-10600; on_midhigh 9200-10600→13400-15600; on_high 16800-17400→; on_upshift 13400-15600→16800-17400. `auto_pitch_ref` por instrumento (idle 3320, off_low 4600, on/off mid 7800, on_midhigh 11400, off_midhigh/downshift 15000, on_upshift 15600, on_high 16700) registrado en el catálogo, pendiente de calibración auditiva. |
| `engine_ext` | 18 capas front/rear, close/far, on/off y shifts | motor exterior espacial | RPM, throttle, Distance 0..500, cone angle | loops | engine | rpm, throttle, posición/orientación, listener | LOD inicial implementado; espacialización completa pendiente | Mezcla por distancia con emisor 3D; faltan front/rear, close/far y cone exactos. Incluye dos samples `f2000`, anomalía de procedencia a revisar. |
| `transmission` | `z4gt3 int on tr midhigh`, `z4gt3 int on tr high`, `gt3r 19 trans main mid 2` | whine transmisión | throttle, drivetrain speed, Distance 0..15 | loops | drivetrain | shaft speed, gear ratio, torque sign/load | MVP implementado con speed/gear; shaft speed pendiente | Samples no-R25 por nombre; validar licencia/procedencia y timbre. |
| `wind` | `single seater highspeed wind` | bed aerodinámico | speed, air_pressure, Distance 0..25 | loop | aero | airspeed, air density, cockpit/exterior mix | MVP implementado con speed; densidad pendiente | Sustituye cualquier futura aproximación genérica de viento. |
| `wheel` | `lb brakes`, `tyre_explosion`, `flat_tyre_mono`, `tyre_rolling`, `flutter_4` | rueda/daño/frenos compuesto | suspension_damage, speed, inflation, brake | mixto | wheel | wheel speed/load, brake torque/temp, pressure, puncture state | MVP físico implementado; por-rueda pendiente | Carga, freno y presión reales controlan las capas; explosion usa one-shot deduplicable. Rolling/flat/brakes son loops. |
| `bodywork` | `single seater bodywork` | rattle mecánico | susp_travel_speed | loop | mechanical | suspension velocity/load, chassis acceleration | P2 | Base recuperada -80 dB: la curva lo hace audible; no usar gain base aislado. |
| `skid_int` | `Skid` | skid cockpit | ninguno recuperado | loop | tires | slip ratio/angle, wheel load, camera interior | MVP implementado con slip agregado; por-rueda pendiente | Base -2 dB, -3 semitonos. La intensidad viene de Rust. |
| `skid_ext` | `skid_ext_mono` | skid exterior 3D | ninguno recuperado | loop | tires | slip energy per wheel, load, emitter position | P1 | Mono, adecuado para fuente 3D por rueda. |
| `gear_grind` | `missgear` | fallo de cambio | ninguno | marcado loop en instrumento, semánticamente transient | drivetrain | rejected shift/missed gear | P2 | Tratar como one-shot/segmento pese al loop flag; requiere confirmación auditiva. |
| `door` | `door_open`, `lh door close` | puerta | state | one-shots | other | state transition | fuera de alcance inicial | Poco relevante para monoplaza; mantener en catálogo, no activar. |
| `backfire_int` | no resuelto | backfire interior | throttle (seek 2.5) | indeterminado | engine | throttle lift, rpm, exhaust state | bloqueado | Probablemente controla/nest eventos o samples incorporados a engine; inspección auditiva requerida. |
| `backfire_ext` | no resuelto | backfire exterior | throttle, cone angle | indeterminado | engine | throttle lift, rpm, emitter orientation | bloqueado | No conservar automáticamente el detector >12k del mixer anterior. |
| `limiter` | no resuelto | limitador | decay, cone angle | indeterminado | engine | rpm vs limiter, ignition cut state | bloqueado | Debe recibir estado real de limiter, no inferirse solo de RPM. |
| `tractioncontrol_int` | no resuelto | TC interior | decay | indeterminado | drivetrain | `tc_active`, cut ratio | bloqueado | Formula-90 ya expone telemetría TC adecuada. |
| `tractioncontrol_ext` | no resuelto | TC exterior | decay | indeterminado | drivetrain | `tc_active`, cut ratio, 3D emitter | bloqueado | No hay sample local resuelto. |
| `gear_int` | no resuelto | cambio interior | state, Distance 0..20 | indeterminado | drivetrain | gear transition phase | bloqueado | Puede ser timeline de control. |
| `gear_ext` | no resuelto | cambio exterior | state | indeterminado | drivetrain | gear transition + exterior emitter | bloqueado | Puede ser timeline de control. |
| `turbo` | no resuelto | turbo/BOV | bov, bov_decay, boost | indeterminado | induction | boost pressure, throttle lift | bloqueado/no prioritario | R25 V10 atmosférico: revisar si debe excluirse por identidad del coche. |
| `horn` | no resuelto | horn | ninguno | indeterminado | other | horn input | fuera de alcance | Sin sample local. |

Rutas completas: `event:/cars/vrc_2005_renault_r25/<event>`.

## Samples por familia

### Motor interior

`r25 int idle`, `r25 int on mid`, `r25 int on midhigh`, `r25 int on high`, `r25 int on upshift`, `r25 int off low`, `r25 int off mid`, `r25 int off midhigh`, `r25 int off downshift`.

### Motor exterior

`r25 ext idle`; capas on/off mid, midhigh y high; variantes front/rear, close/far; cuatro regiones upshift; dos downshift; además `f2000 ext on high rear close` y `f2000 ext on upshift rear close`.

### Rueda y chasis

`tyre_rolling` (mono, 10.067 s), `flat_tyre_mono` (8.935 s), `tyre_explosion` (0.358 s one-shot), `lb brakes` (38.384 s), `flutter_4` (17.276 s), `Skid`, `skid_ext_mono`, `single seater bodywork` y `single seater highspeed wind`.

## Curvas relevantes recuperadas

Las 120 curvas contienen regiones RPM repetidas y consistentes: 3,800–4,600; 5,200–6,800; 6,600–8,200; 9,000–10,600; 9,600–12,800/13,800; 11,200/11,400–13,800/14,200; 12,800–14,200; 13,400–15,600; 15,200–15,600; 16,800/17,000/17,200–17,400 rpm. También hay curvas por speed aproximadamente 15–300 y por drivetrain speed hasta ~281.

Estas regiones deben convertirse a tablas especializadas del adaptador después de resolver controller→propiedad. No deben reducirse a las constantes antiguas `BAND_CENTERS`, `BAND_WIDTH` y `ENGINE_BAND_NATIVE_RPM`.

## Conclusión

`v10_v2` debe ser la fuente del motor, transmisión, viento, skid y rueda. `common` debe ser la fuente de superficies, dirt/debris, kerbs, colisiones, pit y ambience. Comparten telemetría y comandos, pero nunca deben correr como dos motores de audio independientes.
