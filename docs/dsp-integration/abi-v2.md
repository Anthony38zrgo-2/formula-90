Estado: **implementado técnicamente; pendiente de revisión y gate humano de escucha**.

# ABI v2 — Post-combustion DSP (f90_audio_dsp)

Versión: `F90_AUDIO_DSP_ABI_VERSION = 2` (cambio de semántica respecto a v1: Rust es
autoridad temporal; la DLL ya NO reconstruye el banco B).

## Principio rector (Fase 2.1 / 3.2)

- **Rust entrega los eventos finales de ambos bancos** en cada `f90_dsp_event_block`.
- **C++ NO calcula un segundo banco a partir de RPM.** El offset mecánico de 72°
  (medio paso de banco) se resuelve en el dominio de eventos, en Rust, produciendo
  un `f90_dsp_event` explícito con `bank = 1` y `sample_offset` exacto.
- Eventos que cruzan el límite del bloque: Rust los conserva en una cola de
  pendientes basada en tiempo absoluto y los entrega en el bloque siguiente. C++
  nunca los trunca ni los reconstruye.

## Layout (estable, verificado por dsp-abi-check)

| Struct | Campos | Tamaño | Alineación |
|---|---|---|---|
| `f90_dsp_config`      | struct_size, sample_rate, channels, simulated_cylinders, bank_count, flags, half_block_offset_deg, idle_rpm, max_rpm | 24 | 4 |
| `f90_dsp_event`       | sample_offset(u32), cylinder(u8), bank(u8), reserved(u16), crank_phase_deg(f32), pressure(f32), pressure_derivative(f32), energy(f32), cycle_variation(f32), event_pad(f32) | 32 | 4 |
| `f90_dsp_event_block` | stream_block_id(u64), event_count(u32), block_samples(u32), events[512] | 16400 | 8 |
| `f90_dsp_controls`    | rpm, throttle, load, tc_cut, master_gain, lod(u8), bypass(u8), reserved0(u8), reserved1(u8) | 24 | 4 |
| `f90_dsp_diagnostics` | struct_size, events_received, events_dropped, nonfinite_inputs, nonfinite_outputs, rt_violations, last_error_code, blocks_processed, samples_processed | 40 | 4 |

Restricciones de `create`:
- `config.struct_size` en `[24, sizeof(f90_dsp_config)]`.
- `sample_rate == 44100`; `simulated_cylinders` en `[1,5]`; `bank_count` en `[1,2]`;
  `channels` en `{1,2}`; `flags == 0`.

## Ordenación (Fase 2.3)

- Los eventos deben ordenarse por `(sample_offset, bank, cylinder)`.
- Se permiten múltiples eventos en el mismo `sample_offset`.
- Se rechaza cualquier retroceso del tuple completo; offsets iguales son válidos
  si `(bank, cylinder)` no decrece.
- `sample_offset` siempre `< block_samples`.
- `stream_block_id` debe ser estrictamente creciente entre llamadas aceptadas.
  Después de `reset`, el primer valor vuelve a establecer el baseline y puede ser cualquiera.

## Códigos de error

`F90_DSP_OK=0`, `F90_DSP_ERR_NULL_ARGUMENT`, `_INVALID_ABI`, `_UNSUPPORTED_SAMPLE_RATE`,
`_NOT_SUPPORTED`, `_INVALID_STATE` (evento no monotónico → silencio), `_EVENT_OVERFLOW`
(>512 eventos), `_BLOCK_TOO_LARGE` (>4096 frames), `_NONFINITE_INPUT` (controls/inputs no
finites saneados a 0).

## Determinismo y seguridad de tiempo real

- Mismo `(estado post-create | post-reset, bloque, controles)` → misma salida bit a bit.
- `process` no asigna, no bloquea, no hace I/O, no lanza. Cualquier alloc durante
  `process` incrementa `rt_violations` (contador global atómico en `rt_alloc_guard.cpp`).
- Bypass: salida silenciosa exacta, pero sigue consumiendo eventos y avanzando envolventes.

## Conversión física → acústica

`f90_dsp_event.pressure_derivative` conserva unidades físicas de presión/segundo en ABI v2. La ABI **no** redefine ese campo como amplitud `[-1,1]`.

La única conversión autorizada está en `game/native/vehicle-audio-dsp/src/dsp_instance.cpp`:

- referencia nominal: `30_000 pressure-units/s -> 1.0` de excitación acústica;
- clamp por evento: `±1.5`;
- clamp de suma de eventos simultáneos: `±2.5`.

Esto evita que productores Rust tengan que conocer escalas de audio y evita volver a introducir el fallo de saturación permanente.

`f90_dsp_controls.load` es un control acústico secundario. La amplitud del evento ya está contenida en la derivada física; por tanto `load` no debe ser una segunda multiplicación directa por throttle.
