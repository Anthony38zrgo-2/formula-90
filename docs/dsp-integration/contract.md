# Contrato de integración DSP (Rust ↔ C++20)

Estado: **pendiente de gate humano; ninguna aprobación acústica está registrada**.
Fecha: 2026-08-30. Relacionado: `abi-v1.md`, `events-v1.md`. Items de backlog
cubiertos: F90-DSP-001..003, 010..012.

## 1. Objeto y alcance

Este documento define el contrato entre el motor mecánico de audio (Rust) y el
motor DSP de render (C++20/Faust). El alcance de v1 es el **front-end seguro y
determinista**: una DLL C++ sin asignaciones ni bloqueos en el callback,
acoplada por ABI C estable, compilada en el build canónico.

Fuera de alcance en v1:

- Compilación/detección de archivos `.dsp` de Faust (los hooks se preparan, los
  gráficos Faust se adoptan en un sprint posterior, siempre dentro de la DLL).
- LOD > 0, sample rates distintos de 44 100 Hz, más de 2 bancos.
- Cambios de timbre sin aprobación humana por escucha.

## 2. Componentes y autoridades

| Componente | Ruta | Autoridad |
|---|---|---|
| Motor mecánico de audio | `game/crates/vehicle-audio-engine/` | **Único autor mecánico**: RPM, fase de cigüeñal, orden de encendido, jitter por evento, variación de ciclo, energía/presión pico. Es la única parte que decide *qué* suena. |
| DSP de render C++ | `game/native/vehicle-audio-dsp/` | Procesamiento source–filter posterior. Consume ambos bancos explícitos; no reconstruye mecánica. |
| Faust | dentro de la DLL | Implementa el filtro post-combustión generado offline desde `post_combustion.dsp`; no decide eventos. |
| Godot | `game/` | Reproducción (mandos de vehículo → controles) y creación del buffer de audio. Nunca genera eventos mecánicos. |
| Scripts de build | `scripts/` | Garantizan paridad `BUILD/HEAD` (ver §6). |

Regla de oro: **Rust decide qué se oye; la DLL decide cómo se suena.** Cualquier
contradicción entre capas se resuelve a favor del contrato de eventos (b). La
DLL no lee configuración del juego: recibe `f90_dsp_config`, un bloque de
eventos y un bloque de controles.

## 3. Flujo de señal y frontera de propiedad

```
Godot (estado del coche, mandos)
   │  llamada de bloque (≤ F90_DSP_MAX_BLOCK_SAMPLES muestras a 44.1 kHz)
   ▼
Rust vehicle-audio-engine (mecánica: rpm, fase, firing order, jitter, energía)
   │  produce por bloque: f90_dsp_event_block + f90_dsp_controls
   ✂──────────────────────────────────────────────────────────────
   │  ABI C estable (f90_audio_dsp.h) — única frontera
   ▼
DLL f90_audio_dsp (C++20/Faust, RT-safe)
   │  DSP post-combustión: consume offsets/bancos exactos; filtros source–filter
   │  salida dual-mono exacta: out_left == out_right (bit a bit)
   ▼
AudioStreamGeneratorPlayback (Godot) → mezcla maestra
```

Propiedad de datos:

- El bloque de eventos es propiedad del llamador; la DLL lo copia a
  almacenamiento propio fijo antes de resolver el bloque. No retiene punteros.
- Los buffers de salida son propiedad del llamador.
- La DLL no invoca callbacks de vuelta a Rust ni a Godot.
- La resolución de un bloque es **todo o nada**: con los mismos
  `event_block` + `controls` + estado interno, la salida es determinista (ver
  §5) tras `reset`.

## 4. Modelo de hilos y tiempo real

- `f90_dsp_process` se ejecuta en el hilo de audio de Godot (un único hilo RT).
- `f90_dsp_create`, `f90_dsp_reset` y `f90_dsp_destroy` se ejecutan en el hilo
  de control lógico, sin solaparse con ningún `process` del mismo handle
  (sincronización por el llamador según la política de F90-DSP-012).
- Un handle no puede entrar en `process` de forma reentrante desde dos hilos a
  la vez. Si ocurre, la DLL puede detectarlo (vía contador protegido de
  forma lock-free) y reportar `F90_DSP_ERR_INVALID_STATE`, sin bloquear.
- La DLL no crea hilos, ni mutex, ni allegados. Biblioteca está sin locks.
- Toda operación I-O (logs, disco, red) está prohibida dentro de
  `f90_dsp_process`.

## 5. Invariantes real-time de la DLL (obligatorios)

1. **Cero asignaciones dinámicas después de `create`.** La instancia es de
   tamaño fijo y reserva todo su estado en `create`. `process` no llama a
   `new`/`delete`/`malloc`/`aligned_alloc` (verificado con contador global de
   operadores en tests; ver F90-DSP-012).
2. **Cero locks, sin syscalls, sin log, sin excepciones** cruzando la frontera
   ABI (`extern "C"`, `noexcept`).
3. **Salida siempre finita.** Entradas no finitas se detectan, se cuentan en
   `f90_dsp_diagnostics::nonfinite_inputs` y se sanean (clamp + producción de
   valores finitos). Los denormales se enfrentan con flush (FTZ/DAZ al
   compilar y/o política de piso tras cada operación); outputs no finitos
   cuentan `nonfinite_outputs` y se reemplazan por 0.
4. **Determinismo.** Para un mismo estado inicial (post-create o post-reset) y
   la misma secuencia de entradas, dos ejecuciones en la misma máquina y
   configuración producen salidas bit a bit idénticas.
5. **Bypass.** Con `bypass=1`, la salida es silencio exacto (0.0) sin importar
   el estado de los controles (excepto controles no finitos, que se contabilizan).
6. **Sin auto-gain.** La DLL nunca aplica ganancia automática ni medición
   adaptativa de nivel.

## 6. Paridad BUILD/HEAD

- `generated/build_source.h` (generado, gitignore) contiene el SHA-1 de 40
  hex de `HEAD` en el momento del build, en `F90_DSP_BUILD_SOURCE`. El mismo
  valor se escribe en `game/BUILD_SOURCE` y en el valor embebido de cada crate.
- Toda DLL desplegada debe anunciar su build mediante `f90_dsp_build_source()`
  y debe coincidir con `HEAD`:
  - `scripts/build_windows.ps1` compila y despliega; falla si no puede
    verificar el origen del valor.
  - `scripts/run_f1_94.ps1` **valida paridad** `BUILD/HEAD` (`game/BUILD_SOURCE`
    y todos los binarios verificables) **antes** de iniciar Godot y rechaza la
    ejecución si no coincide (F90-DSP-011).
  - `f90_dsp_check_build_source(expected)` es la comprobación embebida por si
    el runtime desea verificar dentro del juego.
- Se prohíbe reutilizar DLLs, objetos o cachés de otra rama o de otra compilación.

## 7. Catálogo de parámetros (v1)

| Parámetro | Unidad | Rango válido v1 | Política en la DLL |
|---|---|---|---|
| `sample_rate` | Hz | `44100` | Otro valor → `ERR_UNSUPPORTED_SAMPLE_RATE` en `create` |
| `channels` | — | `1`, `2` | Con `2`, salida dual-mono exacta; otro valor → `ERR_NOT_SUPPORTED` |
| `simulated_cylinders` | — | `1..5` | Fuera de rango → `ERR_NOT_SUPPORTED` |
| `bank_count` | — | `1..2` | La DLL valida el banco recibido; nunca crea eventos ni bancos. |
| `half_block_offset_deg` | ° | `0..360` | Metadato de configuración; la temporización de 72° ya llega resuelta por Rust. |
| `idle_rpm` / `max_rpm` | rpm | `>0` / `> idle_rpm` | Solo informativos, la DLL no limita RPM de entrada |
| `rpm` | rpm | clamp a `[0, 100000]` | Saneado antes de filtrar |
| `throttle`, `load`, `tc_cut` | 0..1 | clamp `[0, 1]` | No finitos → cuentan `nonfinite_inputs` y se fijan a 0 |
| `master_gain` | ganancia lineal | clamp `[0, 4]` | Sin boosting automático (ver §5.6) |
| `lod` | — | `0` en v1 | Otros valores → `ERR_NOT_SUPPORTED` |
| `flags` (config) | — | `0` en v1 | Sin flags definidos; valores ≠ 0 → `ERR_NOT_SUPPORTED` |

Persistencia de estado: `process` es puro con respecto a los parámetros por
bloque (introduce límites/RAMPs con contadores para evitar click en cambios
bruscos; el diseño de los ramps se detalla en F90-DSP-012).

## 8. Calidad y rendimiento (v1)

- **Capa de cuerpo:** la capa tonal de la DLL debe mantenerse dentro de
  ±0.25 dB del objetivo de la capa de cuerpo del crate (ver
  `mixer.rs`/capas), comprobada por tests de tono/banda.
- **CPU:** procesamiento por canal a 44.1 kHz con bloque de 1024 muestras:
  Near ≤ 5 %, Far ≤ 2 % (objetivo de diseño ≤ 1.9 %). Método de medición y
  máquina de referencia se definen en la fase de tuning (F90-DSP-012).
- **Silencio digital:** con RPM=0 y sin eventos, salida exactly 0.0 para todas
  las muestras (incluye cola de envolventes ya agotada).

## 9. Verificación y gate

Los criterios de aceptación de F90-DSP-001 son:

1. Aprobación humana de este documento, `abi-v1.md` y `events-v1.md`
   **antes** de implementar la ABI en C++ (este gate).
2. `static_assert` de layout en C++ con los tamaños/offsets de `abi-v1.md` y
   crate espejo `dsp-abi-check` en Rust (mismo cálculo, dos implementaciones).
3. `ctest` cubriendo: create/abad, procesamiento nominal, determinismo,
   overflow de eventos, no finitos, paridad BUILD/HEAD, bloqueo de evento fuera
   de rango y bypass.
4. Balance de timbre: pendiente de escucha y aprobación humana explícita.

Cualquier cambio en §2..§7 de este documento exige re-negociación con el
responsable humano (el contratador del backlog `instrucciones.md`).
