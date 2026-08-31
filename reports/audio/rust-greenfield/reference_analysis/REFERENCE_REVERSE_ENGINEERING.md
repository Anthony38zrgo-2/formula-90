# Reverse engineering de referencias V10 interiores (1998)

Fecha: 2026-08-31

## Alcance y reproducibilidad

Se analizaron directamente los tres WAV originales, sin normalizarlos ni procesarlos:

- `98_int_low.wav` — SHA-256 `2EAC0EF746C818ECAC50FE77CA0099894F4762D55428A3CDDFEB3007A6092CC5`
- `98_int_med.wav` — SHA-256 `D1A527D5DDF3EC6D4F38127D1C6A3B00D7B7771D5473F68315E4352C98DB4007`
- `98_int_max_5.wav` — SHA-256 `F6B6CD4389FEE32CB5F4B41A550E7AA146D4A5D0912FDA86551DABFD598AA50B`

El análisis usa NumPy, SciPy y Matplotlib y puede repetirse con `scripts/audio/analyze_v10_references.py`. Los resultados completos están en `reference_metrics.json`, `order_spectra.csv`, `reference_overview.png` y `reference_order_spectra.png`.

Como control se incluyó el candidato Rust `gf310d_5000rpm_weight_balanced.wav`.

## Contexto de reproducción descubierto

Los WAV no representan estados aislados completos. El mod los usa como cinco capas solapadas y pitch-shifted, tanto en power como en coast. En `Form98_Extra00_Upgrades.ini`:

| Capa | WAV | Rango activo | RPM de afinación |
|---:|---|---:|---:|
| 2 | `98_int_low.wav` | 3600.5–7000.5 | 7499.31 |
| 3 | `98_int_med.wav` | 6001.3–9500 | 8313.25 |
| 5 | `98_int_max_5.wav` | 13550–50000 | 15537.94 |

Esto es crucial: el objetivo audible del juego es una mezcla interpolada, no cualquiera de los tres archivos por separado. También explica por qué el estimador ciego recupera aproximadamente 7425 rpm para `low` y 8202 rpm para `med`. En `max_5`, la firma de 720 grados es tan fuerte que un estimador sin contexto se engancha al subarmónico de media orden (127.725 Hz); la configuración resuelve la ambigüedad y fija el eje cerca de 258.97 Hz / 15538 rpm.

## Medidas directas

| Métrica | low | med | max_5 | Rust actual (5000 rpm) |
|---|---:|---:|---:|---:|
| Duración (s) | 2.500 | 2.500 | 5.054 | 10.000 |
| RMS (dBFS) | -12.18 | -10.75 | -5.04 | -23.22 |
| Crest factor (dB) | 11.46 | 10.46 | 4.99 | 13.10 |
| Centroide (Hz) | 589 | 580 | 761 | 832 |
| Rolloff 85 % (Hz) | 927 | 751 | 1277 | 1667 |
| Energía 20–80 Hz | 1.04 % | 6.17 % | 0.42 % | 15.74 % |
| Energía 80–250 Hz | 31.06 % | 18.01 % | 26.30 % | 17.53 % |
| Energía 250–600 Hz | 24.04 % | 25.17 % | 25.16 % | 45.11 % |
| Energía 600–2500 Hz | 42.12 % | 49.22 % | 44.34 % | 10.42 % |
| Energía 2500–7000 Hz | 1.74 % | 1.41 % | 3.47 % | 10.62 % |
| Correlación a 720° | 0.854 | 0.873 | 0.929 | 0.790 |
| Variación entre ciclos 720° (desv.) | 0.059 | 0.062 | 0.024 | 0.104 |
| Modulación lenta dominante | 0.80 Hz | 1.20 Hz | 0.67 Hz | **41.75 Hz** |

Las bandas absolutas deben interpretarse con cautela porque los WAV de referencia están afinados a RPM distintas. Las relaciones de órdenes y las estadísticas por ciclo son comparaciones más robustas.

## Qué hace que suenen a motor y no a oscilador

1. **El cuerpo real está sobre todo entre 600 y 2500 Hz.** Las referencias concentran allí 42–49 % de su energía; Rust sólo 10 %. El sintetizador no carece simplemente de graves: tiene un hueco en el cuerpo/voz del motor.
2. **Rust acumula energía en los lugares equivocados.** Tiene 15.7 % debajo de 80 Hz, 45.1 % entre 250–600 Hz y 10.6 % entre 2.5–7 kHz. El resultado combina retumbo, tono central y borde áspero, pero deja vacío el rango que da masa acústica.
3. **Las referencias no son una sola serie armónica limpia.** Poseen órdenes enteros y medios (0.5, 1.5, 2.5, 3.5, 4.5...) ligados a la firma de 720°, bancos, cilindros y colectores. Rust muestra un peine mucho más escaso, dominado por el orden de encendido y con huecos profundos.
4. **Hay un continuo residual entre las líneas.** En los espectrogramas originales persiste un piso ancho hasta aproximadamente 6–8 kHz. Su energía total es pequeña, pero elimina la sensación de seno puro. En Rust casi sólo existen líneas finas.
5. **La irregularidad original es estructurada y lenta.** Los originales repiten bien el ciclo de 720° y respiran lentamente a 0.7–1.2 Hz. Rust introduce una modulación dominante a 41.75 Hz, exactamente la frecuencia de ciclo a 5000 rpm, y el doble de dispersión entre ciclos. Eso se oye como flutter/granulación artificial.
6. **`max_5` está fuertemente procesado.** Su crest factor de 4.99 dB, pico cercano a 0 dBFS y alta concentración armónica indican compresión/saturación deliberada. No debe tomarse como presión cruda; es una capa final de alta carga.

## Espectro de órdenes

En `low`, el orden de encendido 5 es dominante, pero los órdenes 1.5, 2, 4.5, 1, 2.5 y 3.5 quedan sólo aproximadamente 2–6 dB por debajo. En `med`, 1, 2, 2.5 y los medios órdenes siguen siendo fuertes. `max_5`, evaluado con su RPM de afinación, combina una familia densa de órdenes con compresión.

El candidato Rust tiene orden 5 dominante, pero el orden 1 queda aproximadamente 8 dB abajo y muchas posiciones intermedias están prácticamente vacías. Sus picos fuertes reaparecen de forma demasiado regular alrededor de múltiplos del encendido. Ese peine ordenado es una causa central del carácter sinusoidal.

## Modelo inverso recomendado para Rust

No conviene copiar los WAV ni resolverlo con una colección de ondas libres. La estructura que se puede reconstruir es:

`eventos de combustión por cilindro → firma fija de 720° por cilindro/banco → blowdown y pulsos de header → colectores → resonancias anchas de bloque/cabeza/cockpit → residuo turbulento sincronizado por evento → no linealidad dependiente de carga`

Cambios concretos, en orden fail-fast:

1. Eliminar o reducir drásticamente la aleatorización independiente por ciclo. Sustituirla por una huella determinista de diez cilindros y deriva correlacionada lenta de 0.7–1.2 Hz.
2. Construir explícitamente una firma de 720° que llene medios órdenes y rompa la igualdad entre revoluciones, sin perder repetibilidad.
3. Reequilibrar el cuerpo: reducir sub-80 Hz y la concentración 250–600 Hz; ensanchar las respuestas de bloque, cabeza y colectores para poblar 600–2500 Hz.
4. Añadir residuo turbulento **disparado y envuelto por cada evento**, con mayor peso 600–2500 Hz y una cola secundaria 2500–7000 Hz. No usar ruido continuo independiente.
5. Introducir dispersión física de tiempos/longitudes entre headers y bancos para ensanchar líneas, no osciladores desafinados arbitrarios.
6. Añadir compresión/no linealidad suave en presión/colector dependiente de carga. Objetivo de crest factor: 10–12 dB en baja/media carga y aproximación progresiva a 5–7 dB en máxima carga.
7. Sólo después ajustar ganancia final. La diferencia de RMS actual no es la causa primaria del timbre.

## Gates objetivos para la siguiente iteración

La primera prueba debe ser estacionaria, no un sweep, porque permite distinguir estructura de transición:

- 5000 rpm, throttle estable, 8–10 s.
- Pico de modulación no debe quedar clavado a la frecuencia de ciclo (41.67 Hz a 5000 rpm).
- Correlación a 720°: 0.84–0.93.
- Desviación de correlación entre ciclos: 0.02–0.07.
- Espectro de órdenes poblado en enteros y medios órdenes, sin huecos profundos entre orden 1 y 10.
- El orden 5 no debe concentrar más de aproximadamente 30 % de la energía armónica local.
- Crest factor inicial: 10–12 dB.
- Comparar forma espectral en dominio de órdenes; usar bandas absolutas como control secundario por la diferencia de RPM.

Después de pasar esos gates se justifica repetir el sweep 5000→15000 rpm y lift-and-coast. Un sweep antes de corregir la señal estacionaria sólo oculta el defecto mediante movimiento de pitch.

## Conclusión

Sí, estos samples permiten reverse engineering útil. La conclusión principal no es “usar más tipos de onda”: el sonido objetivo proviene de una familia densa de órdenes y medios órdenes, un continuo turbulento sincronizado, resonancias estructurales anchas, asimetría determinista de 720° y dinámica no lineal. El módulo actual sí genera una periodicidad V10, pero su síntesis aditiva/eventual está demasiado limpia, escasa y aleatorizada a la frecuencia equivocada. Por eso informa las RPM, pero todavía no materializa un motor.
