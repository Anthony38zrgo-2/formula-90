Estado: **implementado técnicamente; pendiente de revisión y gate humano de escucha**.

# Eventos v2 — Contrato Rust → C++

Versión del contrato: alineada con `abi-v2.md` (semántica de bancos explícitos).

## Quién es la autoridad

Rust (el sintetizador / la capa de eventos del `vehicle-audio-engine`) es la única
autoridad mecánica. Produce, para cada bloque, la secuencia final de eventos de
combustión de **ambos bancos** y la entrega a la DLL en un `f90_dsp_event_block`.

## Campos por evento (`f90_dsp_event`)

| Campo | Significado |
|---|---|
| `sample_offset` | Muestra exacta dentro del bloque en que suena el evento (resolución de muestra). |
| `bank` | 0 = banco A (físico), 1 = banco B (derivado a 72°). |
| `cylinder` | Cilindro físico 0–4. |
| `crank_phase_deg` | Fase de cigüeñal en el instante del disparo (0–720°). |
| `pressure` | Pico de presión producido por el cilindro para ese ciclo. |
| `pressure_derivative` | `(presión_siguiente − presión_actual) × sample_rate`, en unidades de presión por segundo. |
| `energy` | Energía suavizada que alimentó el evento físico. |
| `cycle_variation` | Ganancia ciclo-a-ciclo efectivamente aplicada por `CylState`; el transporte no genera azar. |

**Contrato de frontera:** este campo sigue siendo físico en ABI v2. No debe conectarse directamente a una entrada de audio normalizada. `game/native/vehicle-audio-dsp/src/dsp_instance.cpp` realiza una única conversión a dominio acústico usando una referencia de 30 000 unidades de presión/s y límites explícitos antes de Faust.

## Geometría del V10

- V10 de cuatro tiempos → 10 combustiones por ciclo de 720° y 5 combustiones
  globales por revolución (no cinco por banco por vuelta).
- Paso de encendido dentro de un banco: `720° / 5 = 144°`.
- Banco B desfasado `72°` (medio paso) respecto al A.
- Frecuencia de encendido global: `RPM / 12` eventos/segundo.
- Orden dominante mecánico: 5 (armónico fundamental de la firma V10).

## Derivación del banco B (en Rust, no en C++)

```
evento_A = CylState::step(...)                 // única autoridad física
evento_B.sample_absoluto = evento_A.sample_absoluto
                          + round(72° / incremento_real_del_cigüeñal)
```
Si `sample_offset >= block_samples`, el evento se encola en la cola de pendientes
(absoluta en tiempo) y se entrega en el bloque siguiente con su `sample_offset`
recomputado. C++ solo dispara donde se lo indican.

## Reglas de integridad

1. Sin pérdida ni duplicación de eventos en límites de bloque (cola de pendientes).
2. Banco B nunca se infiere desde RPM dentro de la DLL.
3. El sink no contiene seed, RNG, fase ni firing schedule; copia los valores de `CylState`.
4. L y R salen idénticos bit a bit (salida dual-mono del DSP).

## Verificación (gate de salida Fase 2)

- Test Rust (`dsp_contract.rs`): secuencia conocida, banco B a +72°, ordenada por
  `(sample_offset, bank, cylinder)`, determinista por seed, sin pérdida en bloques
  continuos.
- Test C++ (`test_main.cpp`): consume eventos con `bank = 1` explícito y produce
  señal; rechaza offsets decrecientes; no reconstruye bancos.
- `dsp-abi-check` (Rust) confirma que el layout C y Rust coinciden al compilar.
