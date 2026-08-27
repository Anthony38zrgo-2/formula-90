# Audio bank manifest v1

Este formato es el límite compartido entre el tooling Python, el runtime Rust y
el adaptador Godot. Describe un banco ya construido; no describe cómo
sintetizarlo ni cómo mezclarlo durante gameplay.

Contrato ejecutable: `manifest.schema.json`.

## Núcleo obligatorio

El manifiesto declara:

- `schema_version`: actualmente `1`;
- `bank_name`: identidad estable del banco;
- `sample_rate`: `44100`;
- `channels`: `1`;
- `pcm_bits`: `16`;
- `files`: una o más entradas WAV.

Cada entrada exige solamente:

- `file`: basename WAV relativo, nunca un path;
- `role`: función semántica, por ejemplo `engine_idle`;
- `loop`: si el playback puede repetir la muestra;
- `duration_s`: evidencia básica para inspección;
- `sha256`: integridad del archivo runtime.

Los nombres de archivo deben ser únicos dentro del banco.

## Playback por muestra

Las muestras cuyo pitch depende de RPM declaran `playback.native_rpm`. Una capa
de motor añade `playback.engine_band` con `index`, `center` y `width`. Estos datos
viajan con el WAV y dejan de inferirse por orden o por nombre.

Límites de pitch, ganancias, headroom y smoothing permanecen en el mixer: son
política de reproducción y no propiedades de una muestra.

## Extensiones toleradas

`generator`, `seed`, métricas, `provenance`, `synthesis` y límites de loop son
metadatos conocidos pero no forman parte del núcleo que todos los consumidores
deben entender. Un consumidor ignora campos desconocidos.

`synthesis.source_file` es procedencia de authoring; nunca se resuelve durante
runtime. `sound_mixer_config.json` tampoco pertenece a este formato: es política
de mezcla, no estructura del banco.

## Evolución ligera

- Agregar un campo opcional no cambia `schema_version`.
- Convertir un campo opcional en obligatorio o cambiar su significado sí exige
  una nueva versión y un cutover coordinado.
- No se mantiene compatibilidad histórica hasta que exista un consumidor real
  que la necesite.
- El schema detecta estructura barata antes del human gate; escuchar el A/B
  sigue siendo la validación principal del producto.
