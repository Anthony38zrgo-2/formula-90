# ABI f90_audio_dsp v1

Estado: **histórico; no constituye aprobación humana del timbre actual**.
Fecha: 2026-08-30. Relacionado: `contract.md`, `events-v1.md`.
Fuente de verdad en C++: `game/native/vehicle-audio-dsp/include/f90_audio_dsp.h`.

## 1. Objeto

Interfaz C estable (`extern "C"`) entre el crate Rust `vehicle-audio-engine` y
la DLL C++20 `f90_audio_dsp`. Objetivos: ABI estable y ampliable, layouts
verificados por `static_assert` (C++) y por el crate espejo `dsp-abi-check`
(Rust), sin cambio de ruptura en v1.

Reglas de versionado:

- La ABI solo crece: añadir campos al **final** de un struct (respetando
  `struct_size`), nunca reutilizar `reserved`.
- `F90_AUDIO_DSP_ABI_VERSION = 1` se incrementa solo en ruptura incompatible.
- El consumidor declara el tamaño de cada struct que rellena (`struct_size`);
  la DLL no leerá más allá del tamaño declarado (por eso cada struct tiene
  `struct_size` o `reserved` de respaldo).

Convenciones: entero y floats centrados en little-endian x86-64 (Win64);
alineación: 4 para structs con floats/u32, 8 para el bloque de eventos
(miembro `u64`). Sin `#pragma pack`; se usa `alignas` cuando sea necesario.

## 2. Constantes

| Nombre | Valor |
|---|---|
| `F90_AUDIO_DSP_ABI_VERSION` | 1 |
| `F90_DSP_MAX_BLOCK_SAMPLES` | 4096 |
| `F90_DSP_MAX_EVENTS_PER_BLOCK` | 512 |
| `F90_DSP_BUILD_SOURCE_LEN` | 40 (sha1 hex + `\0`) |
| `F90_DSP_MAX_CYLINDERS` | 5 |
| `F90_DSP_MAX_BANKS` | 2 |
| `F90_DSP_MAX_CHANNELS` | 2 |

`F90_DSP_MAX_BLOCK_SAMPLES` es el tope por llamada a `process`; el runtime de
Godot usa bloques de 1024 o menos.

## 3. Códigos de error (`int32_t`)

| Código | Nombre | Descripción |
|---|---|---|
| 0 | `F90_DSP_OK` | Éxito |
| 1 | `F90_DSP_ERR_NULL_ARGUMENT` | Puntero requerido NULL |
| 2 | `F90_DSP_ERR_INVALID_ABI` | `abi_version`/`struct_size` incompatibles |
| 3 | `F90_DSP_ERR_BUILD_MISMATCH` | `expected` de build no coincide |
| 4 | `F90_DSP_ERR_UNSUPPORTED_SAMPLE_RATE` | sample rate inválido en `create` |
| 5 | `F90_DSP_ERR_BLOCK_TOO_LARGE` | `block_samples` > `F90_DSP_MAX_BLOCK_SAMPLES` |
| 6 | `F90_DSP_ERR_EVENT_OVERFLOW` | Eventos truncados en el bloque (truncado, sin reencauzar) |
| 7 | `F90_DSP_ERR_NONFINITE_INPUT` | Entrada no finita detectada y saneada |
| 8 | `F90_DSP_ERR_INVALID_STATE` | Handle en estado inválido (o reentrada detectada) |
| 9 | `F90_DSP_ERR_NOT_SUPPORTED` | Combinación no soportada en v1 |

| Función | Prototipo |
|---|---|
| abort | `uint32_t f90_dsp_abi_version(void);` |
| build | `const char* f90_dsp_build_source(void);` (40 hex + `'\0'`, estático) |
| check | `int32_t f90_dsp_check_build_source(const char* expected);` (3 = mismatch) |
| create | `int32_t f90_dsp_create(const f90_dsp_config* cfg, f90_dsp_handle* out);` |
| process | `int32_t f90_dsp_process(f90_dsp_handle h, const f90_dsp_event_block* ev, const f90_dsp_controls* ctl, float* out_left, float* out_right, uint32_t frames);` |
| reset | `int32_t f90_dsp_reset(f90_dsp_handle h);` |
| diag | `int32_t f90_dsp_get_diagnostics(f90_dsp_handle h, f90_dsp_diagnostics* out);` |
| destroy | `void f90_dsp_destroy(f90_dsp_handle h);` |

Todas son `extern "C"`, `noexcept`, no lanzan excepciones. Handle opaco:
`struct f90_dsp_instance;` + `typedef struct f90_dsp_instance* f90_dsp_handle;`.

## 4. Tipos

### `f90_dsp_config` (24 B, align 4)

| Off | Tipo | Campo | Nota |
|---|---|---|---|
| 0 | `u32` | `struct_size` | rellenar con `sizeof`; la DLL no lee más allá |
| 4 | `u32` | `sample_rate` | 44100 en v1 |
| 8 | `u8` | `channels` | 1 o 2 |
| 9 | `u8` | `simulated_cylinders` | 1..5 |
| 10 | `u8` | `bank_count` | 1..2 |
| 11 | `u8` | `flags` | 0 en v1 |
| 12 | `f32` | `half_block_offset_deg` | 72.0 (float) |
| 16 | `f32` | `idle_rpm` | referencia, informativo |
| 20 | `f32` | `max_rpm` | referencia, informativo |

`F90_DSP_MIN_CONFIG_SIZE = 24` (el mínimo que una versión puede declarar).

### `f90_dsp_event` (32 B, align 4)

| Off | Tipo | Campo | Nota |
|---|---|---|---|
| 0 | `u32` | `sample_offset` | muestra dentro del bloque, `< block_samples` |
| 4 | `u8` | `cylinder` | `0 .. simulated_cylinders-1` |
| 5 | `u8` | `bank` | `0` en v1; el banco B lo reconstruye la DLL |
| 6 | `u16` | `reserved` | 0 |
| 8 | `f32` | `crank_phase_deg` | fase del cigüeñal en el evento |
| 12 | `f32` | `pressure` | presión pico instantánea |
| 16 | `f32` | `pressure_derivative` | derivada en el evento |
| 20 | `f32` | `energy` | energía/amplitud del evento |
| 24 | `f32` | `cycle_variation` | variación de ciclo (rarefacción) |
| 28 | `f32` | `event_pad` | 0 |

### `f90_dsp_event_block` (16 400 B, align 8)

| Off | Tipo | Campo | Nota |
|---|---|---|---|
| 0 | `u64` | `stream_block_id` | id monotónico secuenciador |
| 8 | `u32` | `event_count` | ≤ `F90_DSP_MAX_EVENTS_PER_BLOCK` |
| 12 | `u32` | `block_samples` | **debe ser igual a `frames` de `process`** |
| 16 | `f90_dsp_event[512]` | `events` | |

### `f90_dsp_controls` (24 B, align 4)

| Off | Tipo | Campo | Nota |
|---|---|---|---|
| 0 | `f32` | `rpm` | clamp a `[0, 100000]` |
| 4 | `f32` | `throttle` | clamp a `[0, 1]` |
| 8 | `f32` | `load` | clamp a `[0, 1]` |
| 12 | `f32` | `tc_cut` | clamp a `[0, 1]` |
| 16 | `f32` | `master_gain` | clamp a `[0, 4]` |
| 20 | `u8` | `lod` | 0 en v1 |
| 21 | `u8` | `bypass` | 1 = silencio exacto |
| 22 | `u8` | `reserved0` | 0 |
| 23 | `u8` | `reserved1` | 0 |

### `f90_dsp_diagnostics` (40 B, align 4)

| Off | Tipo | Campo |
|---|---|---|
| 0 | `u32` | `struct_size` |
| 4 | `u32` | `blocks_processed` |
| 8 | `u32` | `samples_processed` |
| 12 | `u32` | `events_received` |
| 16 | `u32` | `events_dropped` |
| 20 | `u32` | `nonfinite_outputs` |
| 24 | `u32` | `nonfinite_inputs` |
| 28 | `u32` | `rt_violations` |
| 32 | `u32` | `last_error_code` |
| 36 | `u32` | `reserved` |

`struct_size` de `f90_dsp_diagnostics` lo rellena el consumidor; `reset` pone a
cero todos los contadores (mantiene `struct_size`).

## 5. Semántica de la interfaz

- `f90_dsp_create` valida `struct_size >= F90_DSP_MIN_CONFIG_SIZE`,
  `sample_rate == 44100`, `simulated_cylinders ∈ 1..5`, `bank_count ∈ 1..2`,
  `channels ∈ {1,2}`, `flags == 0`; luego reserva el estado y pasa a listo.
- `f90_dsp_process`: un solo hilo de audio; `frames ≤ F90_DSP_MAX_BLOCK_SAMPLES`
  y `frames == ev->block_samples`, `ev->event_count ≤ 512`; si el bloque viene
  truncado, devuelve `F90_DSP_ERR_EVENT_OVERFLOW` y cuenta `events_dropped`
  después de procesar los eventos válidos (nunca reencauza, no guarda cola).
  Con `bypass=1` la salida es silencio (y no modifica `events_received`).
- `out_left`/`out_right` siempre iguales (dual-mono exacto); con
  `channels=1` solo se escribe `out_left`.
- `f90_dsp_abi_version`/`f90_dsp_build_source` son triviales y constantes
  (el GCC no depende de instancias).
- `f90_dsp_reset` restaura el estado determinista de post-create conservando
  la config y los contadores de diag (son de sesión, no de procesamiento:
  se ponen a 0 los contadores; ver tabla).
- `f90_dsp_destroy` es válido con handle NULL (no-op) y libera el estado; no
  puede coexistir con `process` concurrente para el mismo handle.
- No se exporta C++ API; `F90_AUDIO_DSP_API` (`__declspec(dllexport)` en msvc)
  solo en las funciones `extern "C"` listadas.

## 6. Verificación de layout

En C++ (CMake target de pruebas):

```cpp
static_assert(sizeof(f90_dsp_config) == 24);
static_assert(sizeof(f90_dsp_event) == 32);
static_assert(sizeof(f90_dsp_event_block) == 16 + 512 * 32);
static_assert(sizeof(f90_dsp_controls) == 24);
static_assert(sizeof(f90_dsp_diagnostics) == 40);
static_assert(alignof(f90_dsp_event_block) == 8);
```

En Rust, los mismos valores se verifican en `game/crates/dsp-abi-check`
(mirror `#[repr(C)]` + asserts `size_of`/`offset_of`, `sizeof(F90_DSP_MIN)`).
En `ctest`, se cruzan ambos resultados (firma de `static_asserts` en una
función de diagnóstico exportada, opcional).
