# Eventos por bloque — contrato v1

Estado: **histórico; no constituye aprobación humana del timbre actual**.
Fecha: 2026-08-30. Relacionado: `contract.md`, `abi-v1.md`.

## 1. Objeto

Define qué es un evento mecánico, cómo llega del crate Rust (única autoridad
mecánica) a la DLL DSP vía `f90_dsp_event_block`, y las reglas de orden,
capacidad y overflow que garantizan una operación determinista y sin bloqueos
en el hilo de audio.

## 2. Fuente y responsabilidad

El crate Rust `vehicle-audio-engine` genera, por cada bloque de audio, un
`f90_dsp_event_block` con:

- `crank_phase_deg`: ángulo de cigüeñal del evento, derivado de la RPM y del
  bloque de secuencia del sonido del motor.
- `pressure` (presión pico), `pressure_derivative` (derivada en el evento),
  `energy` y `cycle_variation`: valores mecánicos del evento (jitter por
  evento, variación de ciclo; ver `event_gen.rs`/`EventJitter`).
- `sample_offset`: muestra exacta dentro del bloque donde el evento ocurre.

La DLL no recalcula la mecánica, ni el orden de encendido, ni el jitter: los
toma como marca por evento y los consume como *instrucciones de disparo*.
El render de la envolvente (ataque/decay de impulso, filtros, saturación,
capa de cuerpo, limitador) es responsabilidad exclusiva de la DLL.

**Evento vs sistema:** un evento representa UN disparo de un cilindro
físico; no es un control ni un paso de simulación. No caen fuera del bloque
que los generó; no se almacenan entre bloques.

## 3. Cinemática de v1

- Motor: 5 cilindros físicos, orden de encendido
  `DEFAULT_FIRING_PHASES_DEG = [0, 144, 288, 432, 576]` (primeras fases en
  grados, ciclo de 720°), configurado por `DEFAULT_FIRING_PHASES_DEG` del
  crate `src/synth/mod.rs`. Sin parámetros de fase aplicados por la DLL: llega
  en `crank_phase_deg` de cada evento.
- Bank B: v1 genera solo eventos `bank=0` (físicos). El banco B es una
  reconstrucción de banco desplazado `half_block_offset_deg` (por defecto 72°;
  ver `half_block_reconstruct.rs`), y es responsabilidad del DSP C++ durante
  `process`. La DLL puede generarlo usando el mismo conejo de eventos: con el
  mismo `crank_phase_deg`/presión del evento original con desplazamiento de
  `half_block_offset_deg` (si `bank_count >= 2`).
- Con `simulated_cylinders < 5`, solo los primeros N cilindros del orden de
  encendido emiten eventos (los bancos B correspondientes quedan desplazados
  y **no** se generan eventos para cilindros no simulados).

## 4. Capacidad y garantía de no solapamiento

- Máximo evento por bloque: `F90_DSP_MAX_EVENTS_PER_BLOCK = 512`.
- Garantía de separación: la mínima separación entre eventos del mismo bloque
  es el desplazamiento entre disparos del banco físico (con cifra de
  invariancia: cualquier par del mismo evento entra en muestras distintas,
  separados mucho más de 1 muestra incluso a rpm máxima con bloque 4096 y
  llamada 44.1 kHz; en el peor caso del modelo actual (0.15 s de ventana)
  el evento máximo viable por bloque es menor a 512 → margen muy holgado).
- La DLL valida `event_count <= 512` y, en caso de truncamiento por exceso,
  procesa los primeros 512 y devuelve `F90_DSP_ERR_EVENT_OVERFLOW` contando
  en `events_dropped`. El emisor Rust corta en origen (mismo límite) y el
  runtime nunca reencauza eventos.

## 5. Orden y reglas de índice

- `sample_offset < block_samples` (0-index dentro de `f90_dsp_event_block`).
  Si `sample_offset >= block_samples`, el evento se descarta + cuenta
  `events_dropped`; si `block_samples != frames` de `process`, error
  `F90_DSP_ERR_BLOCK_TOO_LARGE`/`ERR_INVALID_STATE` según la desviación.
- Un bloque llega ordenado por `sample_offset` ascendente (el emisor es el
  que garantiza orden estricto; con una misma cue, se rompe por `cylinder`,
  pero en v1 no hay dos eventos en la misma muestra porque provienen de
  disparos distintos → no se admite la misma `sample_offset` para dos eventos;
  la DLL no reordena (sin B) — si detecta `sample_offset` no monótono, lo
  cuenta como error `F90_DSP_ERR_INVALID_STATE` y salida de silencio en ese
  bloque).
- No hay prioridad: en un bloque con `event_count == 0` -> salida pura de
  controles (revs/decay/limbo) sin eventos.

## 6. Controles por bloque

El «dato» de control `f90_dsp_controls` es la salida por bloque del motor
Rust (solo rutinas de barrido): `rpm`, `throttle`, `load`, `tc_cut`,
`master_gain`, `lod`, `bypass`. La DLL lo toma como instrucción (saneado en
la frontera, ver `abi-v1.md` §5). El filtro de controles entre bloques
(ramps para evitar clicks) es de responsabilidad de la DLL; los ramps se
definen en F90-DSP-012.

## 7. Determinismo y sesiones

- `stream_block_id` es un contador monotónico del emisor que se usará en el
  futuro para depurar/verificar; v1 la DLL solo lo refleja (sigue en 0 o
  cuenta los bloques válidos; no se usa cálculo en el proceso).
- Para blob de eventos válido → mismo estado en reset → misma salida bit a
  bit.
- Los eventos nunca se emiten fuera de `process`; el bloque N-1 completo se
  resuelve antes de empezar el N (sin buffer de adelanto de eventos).

## 8. Fuera de alcance (sprint posterior)

- Eventos con `bank=1` emitidos por Rust (reconstrucción en DLL).
- Control de revs con reglas de inercia/limbo desde la DLL.
- Cambio del orden de encendido o de `half_block_offset_deg` en caliente.
