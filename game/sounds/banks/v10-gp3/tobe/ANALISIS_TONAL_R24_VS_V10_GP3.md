# Análisis tonal: referencia R24 frente al banco V10 GP3

Fecha: 2026-09-21  
Alcance: análisis no destructivo. No se modificó ningún sample fuente.

> **Estado actual:** la implementación sintética documentada al final fue descartada por revisión del usuario. El 21 de septiembre de 2026 se restauraron byte por byte los `gearup.wav` y `geardn.wav` originales, junto con sus copias del banco runtime. Esa sección se conserva sólo como registro de la prueba rechazada y no describe los WAV activos.

## Objetivo

Determinar qué contenido tonal y armónico le falta al banco actual `v10-gp3` para aproximar la referencia `r24-sound.mp3` mediante una capa de síntesis aditiva en Python.

En este documento, **tonalidad** significa color espectral, distribución de energía y armónicos. No significa clave musical.

## Fuentes analizadas

- Referencia R24: `tobe/r24-sound.mp3`, 130.612 s, 44.1 kHz, estéreo, MP3 a 192 kb/s.
- Loops sostenidos del banco: `98_int_idle.wav`, `98_int_low.wav`, `98_int_med.wav`, `98_int_high_1.wav` y `98_int_max_5.wav`.
- Los eventos de cambio, limitador y backfire quedaron fuera de la comparación tonal sostenida.
- Anclas actuales del sampler: 4543, 8643, 9563, 13033 y 17831 RPM.

SHA-256 de la referencia:

`2642678264de5481fb872fd165eed2df704885bd609a09e34cb018b172a0b537`

## Método

1. La referencia se decodificó temporalmente a PCM16/44.1 kHz sin alterar el MP3.
2. Se analizó en ventanas de 1 s con salto de 0.5 s.
3. Se excluyeron ventanas por debajo de -35 dBFS y el 30 % con mayor flujo espectral para reducir la influencia de transitorios.
4. Como no hay telemetría de RPM, el grupo de alta velocidad tonal se definió como el 30 % de las ventanas estables con mayor centroide espectral. Son 53 ventanas distribuidas a lo largo de la grabación, no un único fragmento.
5. Para cada loop se usó el espectro mediano de hasta 12 ventanas Hann largas. Las comparaciones por banda están normalizadas por energía audible total; por ello describen **balance tonal**, no volumen absoluto.

## Resultado principal

El banco no necesita más subgrave. Le falta una columna armónica continua desde aproximadamente **1.25 kHz hasta 12 kHz**, con prioridad en **1.25–2.5 kHz** y **2.5–5 kHz**. El déficit aumenta hacia la zona de 8–12 kHz. Al mismo tiempo, `98_int_max_5.wav` tiene demasiado peso relativo en 80–160 Hz, dominado por un pico cercano a 128 Hz.

La referencia R24 de alta velocidad tiene un centroide mediano de **1508 Hz** y roll-off de 85 % en **1888 Hz**. `98_int_max_5.wav` queda en **712 Hz / 1277 Hz**; `98_int_high_1.wav`, en **688 Hz / 935 Hz**. La referencia también es algo menos periódicamente limpia: flatness mediana `0.00517`, frente a `0.00482` en max y `0.00181` en high. Esto apunta a una corona armónica más rica y ligeramente más ancha, no a ruido blanco de banda completa.

## Balance espectral de alta velocidad

Valores en porcentaje de energía audible. La columna Δ es `10·log10(R24 / loop max)` después de normalizar ambos espectros.

| Banda | R24 alta | GP3 max | Δ R24 vs max | Lectura |
|---:|---:|---:|---:|---|
| 20–80 Hz | 0.146 % | 0.456 % | -4.95 dB | No añadir |
| 80–160 Hz | 0.820 % | 32.114 % | -15.93 dB | Exceso severo en max |
| 160–315 Hz | 0.333 % | 7.879 % | -13.74 dB | Exceso en max |
| 315–630 Hz | 3.060 % | 11.758 % | -5.85 dB | Exceso moderado |
| 630–1250 Hz | 42.734 % | 26.924 % | +2.01 dB | Falta cuerpo tonal superior |
| 1250–2500 Hz | 38.847 % | 17.665 % | +3.42 dB | Déficit principal audible |
| 2500–5000 Hz | 5.456 % | 2.437 % | +3.50 dB | Falta mordida/rasp armónico |
| 5000–8000 Hz | 1.547 % | 0.594 % | +4.16 dB | Falta presencia fina |
| 8000–12000 Hz | 0.892 % | 0.159 % | +7.49 dB | Falta corona armónica |
| 12000–18000 Hz | 0.261 % | 0.015 % | +12.52 dB | Diferencia grande, pero de baja energía absoluta y sensible al MP3 |

No conviene interpretar los últimos dos deltas como ganancias de ecualizador literales: parten de cantidades pequeñas y la referencia MP3 puede incluir ambiente, transmisión y artefactos de codificación.

## Perfil del banco actual

| Loop | Ancla | RMS | Centroide | Roll-off 85 % | Rasgo dominante |
|---|---:|---:|---:|---:|---|
| `98_int_idle.wav` | 4543 RPM | -12.27 dBFS | 417 Hz | 492 Hz | 79.1 % en 315–630 Hz |
| `98_int_low.wav` | 8643 RPM | -12.21 dBFS | 560 Hz | 809 Hz | Concentrado debajo de 630 Hz |
| `98_int_med.wav` | 9563 RPM | -10.75 dBFS | 582 Hz | 692 Hz | 44.2 % en 630–1250 Hz |
| `98_int_high_1.wav` | 13033 RPM | -4.95 dBFS | 688 Hz | 935 Hz | Muy poco contenido sobre 2.5 kHz |
| `98_int_max_5.wav` | 17831 RPM | -5.04 dBFS | 712 Hz | 1277 Hz | Pico ~128 Hz; 32.1 % en 80–160 Hz |

El salto de nivel entre med y high también es grande. No debe corregirse haciendo la capa aditiva igual de fuerte: primero debe calibrarse el loudness de los loops y reservar headroom.

## Qué debe aportar la síntesis aditiva

Para un V10 de cuatro tiempos, la frecuencia global de disparo es:

`frecuencia_disparo_hz = rpm / 12`

Por tanto, entre 12 000 y 18 000 RPM la base física recorre aproximadamente 1000–1500 Hz. Esa trayectoria coincide con la región donde la referencia concentra su identidad y el banco pierde energía.

La capa recomendada es:

1. **Orden principal dinámico en `rpm / 12`**: debe aportar continuidad en 630–2500 Hz y seguir RPM sin escalones.
2. **Parciales 2–4 del orden de disparo**: forman la zona 2.5–5 kHz responsable de mordida y rasp. Ganancia descendente, dependiente de carga.
3. **Parciales 4–8, limitados por Nyquist**: aportan 5–12 kHz. Deben ser débiles, ensanchados y modulados; no una hilera de senos estáticos.
4. **Dos bancos de fase a 72°**: usar grupos A/B con pequeñas diferencias de amplitud y fase para evitar un tono monofónico estéril. La suma debe conservar coherencia con los diez eventos de combustión.
5. **Microvariación controlada**: variación lenta de fase/amplitud y un residual estrecho asociado a carga. No añadir ruido blanco general.
6. **High-pass en la capa aditiva alrededor de 600–700 Hz**: la síntesis no debe reforzar el exceso existente debajo de 630 Hz.
7. **Tilt dependiente de RPM**: abrir progresivamente 2.5–12 kHz desde aproximadamente 10 000 RPM y reducirlo en lift/coast.

## Objetivo inicial de mezcla

Los deltas medidos son descriptores del balance final, no ganancias directas de oscilador. Como punto de partida conservador para una primera prueba offline:

| Región sintetizada | Aporte inicial sugerido al mix final |
|---|---:|
| 630–1250 Hz | +1 a +2 dB relativos |
| 1250–2500 Hz | +2.5 a +3.5 dB relativos |
| 2500–5000 Hz | +2.5 a +3.5 dB relativos |
| 5000–8000 Hz | +2 a +3 dB relativos |
| 8000–12000 Hz | +2 a +4 dB relativos, con techo estricto |
| Menos de 630 Hz | 0 dB de aporte; considerar atenuar el loop max |

La meta no es copiar exactamente los porcentajes del MP3, sino mover el centroide de la mezcla de alta RPM desde ~712 Hz hacia una primera ventana de **1100–1400 Hz**, manteniendo margen antes de intentar los ~1508 Hz de la referencia.

## Implementación Python propuesta

Bibliotecas suficientes: `numpy`, `scipy.signal` y `wave` o `soundfile` si se instala. La primera versión debe ser un render offline reproducible, no una modificación directa de los WAV actuales.

Arquitectura mínima:

```text
RPM envelope
  -> firing frequency = RPM / 12
  -> phase accumulator
  -> partial bank [1, 2, 3, 4, 5, 6, 7, 8]
  -> A/B phase and amplitude variation
  -> load/RPM spectral envelope
  -> high-pass 650 Hz
  -> soft band-limited saturation
  -> loudness calibration
  -> mix with immutable GP3 loop
```

Controles recomendados por parcial:

- Frecuencia: `partial_index * rpm / 12`.
- Amplitud: tabla interpolada por RPM y carga, no constante.
- Fase: acumulador continuo; no reiniciar por bloque.
- Modulación: amplitud ±0.5–2 % y fase muy lenta, con semilla fija para renders comparables.
- Anti-aliasing: omitir cualquier parcial por encima de 0.45 × sample rate.
- Headroom: normalizar sólo al final y conservar al menos 1 dBTP de margen en la prueba.

## Orden de experimentación

1. Render A: banco actual sin cambios.
2. Render B: sólo orden principal y parciales 2–4, high-pass 650 Hz.
3. Render C: B más parciales 5–8 y microvariación.
4. Igualar loudness A/B/C antes de escuchar.
5. Medir centroide, roll-off, energía por bandas, clipping y diferencia RMS.
6. Escuchar a RPM fijas de 9000, 12 000, 15 000 y 18 000, además de un sweep idéntico.
7. Sólo después de la aprobación humana, decidir si se atenúa el componente de 128 Hz de `98_int_max_5.wav` o si se reemplaza ese loop.

## Criterios de aceptación

- La capa no aumenta energía por debajo de 630 Hz.
- La mezcla gana energía continua en 1.25–5 kHz sin silbido sinusoidal identificable.
- 8–12 kHz aparece como textura ligada a RPM/carga, no como hiss constante.
- No hay discontinuidades de fase entre bloques ni aliasing al acercarse a 18 000 RPM.
- Pico verdadero menor o igual a -1 dBTP en los renders de evaluación.
- Comparación perceptual con loudness igualado; las métricas no sustituyen la escucha.

## Análisis de `gearup.wav` y `geardn.wav`

### Método específico

Los dos samples se midieron como eventos independientes. En la referencia R24 se buscaron discontinuidades mediante flujo espectral, cambio de RMS y variación del centroide antes/después del evento. Una caída de centroide se trató como candidata a subida de marcha y una elevación como candidata a reducción.

Este método localizó múltiples transiciones compatibles, pero no puede aislar la caja del motor ni confirmar la marcha sin telemetría. Por ello, las ventanas R24 describen el **evento completo de conducción** —motor, corte o blip, transmisión y ambiente— mientras `gearup.wav` y `geardn.wav` son one-shots mono. Los porcentajes no deben convertirse directamente en una curva de ecualización.

### Mediciones de los samples actuales

| Métrica | `gearup.wav` | `geardn.wav` |
|---|---:|---:|
| Duración total | 313.1 ms | 314.0 ms |
| Duración activa a -30 dB de la envolvente | 286.7 ms | 216.7 ms |
| Duración que contiene 90 % de la energía | 205.4 ms | 126.8 ms |
| Inicio al máximo de envolvente | 23.5 ms | 97.3 ms |
| RMS | -16.34 dBFS | -15.86 dBFS |
| Pico | -4.49 dBFS | -3.00 dBFS |
| Centroide | 242 Hz | 174 Hz |
| Roll-off de 85 % | 388 Hz | 205 Hz |
| Energía bajo 315 Hz | 81.41 % | 92.86 % |
| Energía sobre 1.25 kHz | 1.28 % | 1.82 % |

Ambos samples tienen margen de pico, pero son extremadamente oscuros. La bajada es todavía más grave y tiene un máximo tardío: funciona como un golpe blando de baja frecuencia, no como un engrane rápido acompañado por rev-match.

### Referencia R24: comportamiento del cambio completo

Después de excluir el inicio y final de la grabación, las candidatas principales muestran:

| Propiedad | Subida R24 | Reducción R24 |
|---|---:|---:|
| Cambio mediano del centroide después/antes | 0.82× | 1.19× |
| Centroide de la ventana completa | ~1014 Hz | ~1488 Hz |
| Roll-off de 85 % | ~1125 Hz | ~1841 Hz |
| Energía 630–1250 Hz | ~83.5 % | ~47.4 % |
| Energía 1250–5000 Hz | ~10.2 % | ~24.3 % |
| Energía 5–12 kHz | ~1.24 % | ~2.51 % |

La diferencia más importante no es el volumen del golpe. En la subida, la referencia presenta una caída clara del régimen y reconstrucción del motor en una marcha superior. En la reducción, aparece una subida tonal equivalente a un blip o rev-match y bastante más energía medio-aguda. Los samples actuales no contienen ni pueden controlar esas trayectorias.

### Qué le falta a `gearup.wav`

1. **Ataque mecánico medio/agudo**: falta un clic corto entre aproximadamente 1.25–5 kHz y una cola débil hasta 8 kHz. El sample actual sólo tiene 1.04 % en 1.25–2.5 kHz y 0.21 % en 2.5–5 kHz.
2. **Corte de par/encendido sincronizado**: el realismo R24 depende de una interrupción breve del motor, no de sumar otro golpe encima de un loop continuo.
3. **Caída de RPM posterior**: las candidatas R24 caen a aproximadamente 0.82× en centroide. Esa transición debe ejecutarla el motor/sampler con fase continua; no debe hornearse completa dentro del one-shot.
4. **Dos etapas temporales**: un ataque de caja de pocos milisegundos y una segunda respuesta de acople/carga. El sample actual tiene cuerpo largo, pero poca definición espectral.
5. **Variación por carga y marcha**: una única muestra idéntica en cada cambio produce repetición audible. Se necesitan pequeñas variaciones de ganancia, duración y brillo ligadas a torque transferido.

Tratamiento recomendado: conservar el cuerpo grave actual a nivel reducido, añadir una capa sintética band-limited de contacto metálico de 10–35 ms, y controlar aparte un corte/reingreso del motor de aproximadamente 50–140 ms. El tiempo exacto debe calibrarse con telemetría y escucha; no se puede recuperar de forma fiable del MP3 mezclado.

### Qué le falta a `geardn.wav`

1. **Ataque más temprano**: el máximo llega a 97.3 ms, demasiado tarde para comunicar el instante de engrane. Debe existir un componente inicial definido en los primeros 10–30 ms.
2. **Rev-match o blip real**: las candidatas R24 elevan el centroide a aproximadamente 1.19×. Esto debe provenir de la trayectoria de RPM/carga del motor, no de subir el pitch del golpe grave.
3. **Contenido de 630 Hz a 5 kHz**: `geardn.wav` concentra 92.86 % bajo 315 Hz y sólo 3.22 % entre 315–630 Hz. Le faltan engrane, dientes, dog-ring y respuesta estructural audible.
4. **Corona transitoria controlada de 5–12 kHz**: debe ser corta y dependiente de carga; la muestra actual aporta sólo 0.12 % en esa región.
5. **Separación tímbrica respecto a la subida**: actualmente ambos eventos comparten el carácter de golpe oscuro. La reducción debe distinguirse por ataque más seco, blip ascendente y una cola mecánica breve, no sólo por otro low-end thump.

Tratamiento recomendado: adelantar o sintetizar un ataque de engrane, conservar sólo parte del cuerpo grave, añadir parciales/resonancias amortiguadas entre 800 Hz y 5 kHz, y hacer que el motor genere el blip ascendente. Evitar una subida de pitch global del sample porque también desplazaría artificialmente el golpe y el ruido.

### Arquitectura de síntesis recomendada para los cambios

La réplica debe dividir cada evento en componentes controlables:

```text
evento de cambio
  -> envolvente de torque y motor
  -> trayectoria de RPM antes/después
  -> ataque de selector/dog-ring
  -> cuerpo estructural grave amortiguado
  -> resonancias metálicas band-limited
  -> cola dependiente de carga
```

Para Python, `numpy` y `scipy.signal` bastan para prototipar:

- Ataque: impulso corto filtrado, con energía principal entre 1.25–5 kHz.
- Cuerpo: resonadores amortiguados, no seno sostenido, alrededor de 180–450 Hz para subida y 250–700 Hz para reducción.
- Contacto metálico: 2–4 resonadores no armónicos entre 1–6 kHz con decaimientos diferentes.
- Variación: semilla reproducible, desviaciones pequeñas de frecuencia, amplitud y decay.
- Motor: automatización independiente del gain y RPM; nunca sustituirla por el one-shot.
- Mezcla: loudness igualado, pico verdadero máximo de -1 dBTP y verificación de que el cambio no enmascare el motor.

### Prueba A/B específica

1. A: eventos actuales con el motor continuo.
2. B: eventos actuales más corte/blip y trayectoria correcta de RPM.
3. C: B más ataque mecánico y resonancias medio-agudas sintetizadas.
4. D: C con tres variaciones deterministas por subida y tres por reducción.

Si B aporta la mayor mejora, el problema principal está en la integración motor/transmisión. Si C mejora claramente sobre B, entonces la carencia tímbrica del sample también es determinante. No reemplazar los WAV originales hasta completar esta escucha.

## Limitaciones

- `r24-sound.mp3` es una referencia estéreo comprimida y probablemente contiene ambiente, transmisión y procesamiento final. No es un stem de motor aislado.
- No hay RPM ni carga sincronizadas con la grabación. La clasificación alta/baja se basa en brillo espectral y no prueba un régimen exacto.
- El análisis establece una dirección de síntesis y bandas objetivo; no demuestra todavía que el timbre resultante replique perceptualmente al R24.
- Para una calibración física más precisa hace falta una referencia R24 aislada con RPM conocida o telemetría sincronizada.
- Las detecciones de gearshift son candidatas derivadas del cambio espectral; sin video marcado o telemetría no constituyen etiquetas perfectas de subida/reducción.

## Conclusión

La carencia del motor no es “más grave”: es **más orden de disparo y más escalera armónica por encima de 1.25 kHz**, especialmente entre 1.25–5 kHz, más una corona controlada hasta 12 kHz. La síntesis aditiva debe construir esa estructura desde `rpm / 12`, conservar continuidad de fase y mantenerse fuera del grave ya sobrerrepresentado. El pico de ~128 Hz del loop máximo es el principal elemento que impide acercar el balance al R24 únicamente añadiendo parciales.

En los cambios, `gearup.wav` y `geardn.wav` ya aportan cuerpo grave, pero les faltan ataque mecánico definido, energía media/alta y, sobre todo, coordinación con el corte o blip y la trayectoria de RPM. La mejora realista requiere separar el one-shot de caja de la respuesta dinámica del motor.

## Implementación aprobada: síntesis R24 para subida y reducción

Se sustituyeron los dos one-shots del banco por una generación determinista en Python (`numpy` y `scipy.signal`) conservando copias inmutables de los originales. La receta se encuentra en `scripts/audio/synthesize_r24_gear_events.py` y usa la semilla fija `241994`.

| Resultado | `gearup.wav` | `geardn.wav` |
|---|---:|---:|
| Duración | 320 ms | 320 ms |
| RMS | -21.30 dBFS | -20.25 dBFS |
| Pico verdadero máximo | -4.00 dBFS | -4.00 dBFS |
| Centroide | 615 Hz | 796 Hz |
| Roll-off de 85 % | 1351 Hz | 923 Hz |
| Energía 20–315 Hz | 55.27 % | 12.21 % |
| Energía 315–1250 Hz | 18.04 % | 74.68 % |
| Energía 1.25–5 kHz | 26.62 % | 12.95 % |

La subida combina el cuerpo grave original reducido con ataque de selector, resonadores amortiguados y contacto metálico limitado en banda. La reducción adelanta el ataque, desplaza la respuesta estructural hacia medios y añade resonancias de engrane más claras. No se aplicó pitch global.

Los hashes SHA-256 generados son:

- `gearup.wav`: `362462903a76d404d5a9ce5dc71ee837c78069dcc43a10e4691a205979ab1ccd`
- `geardn.wav`: `e158076cfd1e406a364bb8f4d14d1651354218bdd5f87aee60dd358195cfce5d`

La generación se ejecutó dos veces y produjo hashes idénticos. El informe medido completo quedó en `R24_GEAR_SYNTHESIS_GENERATION.json`.

### Integración dinámica del motor

El sampler Grand Prix ahora atenúa suavemente el motor continuo durante la fase de cambio para que el evento mecánico no quede oculto:

- subida: ganancia objetivo del motor `0.18`;
- reducción: ganancia objetivo del motor `0.42`, preservando más motor para el blip;
- ataque de la atenuación: `6 ms`;
- recuperación: `45 ms`.

La envolvente se calcula por muestra y vuelve a ganancia unitaria al terminar el cambio. El test `shift_phases_duck_and_restore_the_continuous_engine` verifica la atenuación y recuperación. La trayectoria real de RPM sigue perteneciendo a la física y al sampler continuo, no al WAV.

### Prueba desde el launcher canónico

El banco preparado se regeneró en `game/audio/formula_one_2030_grand_prix_sampler` y el proyecto se recompiló desde cachés limpias. `game/BUILD_SOURCE` coincide con el HEAD `d0729b8af2cb9ccc111aa2dfc1f85f1e95084154`.

Para escuchar los cambios en contexto se usa directamente:

```powershell
.\run_f1_94.ps1
```

La comprobación automatizada `./run_f1_94.ps1 -SmokeAudio` pasó la validación de paquete, paridad BUILD/HEAD, importación y telemetría. En modo smoke Godot usa audio headless/Dummy, por lo que `audio_initialized=false` es el comportamiento esperado de la salida física; no significa que el banco haya fallado. La ruta del perfil F1 2030 hacia Grand Prix Sampler fue verificada además con un test dedicado, y los 17 tests filtrados del sampler pasaron.

El sample preexistente `commons/impact_scrape.wav` impedía cargar el banco por estar a 48 kHz, estéreo y 24 bits. Se conservó su contenido mediante downmix aritmético y resampleo polifásico a 44.1 kHz mono PCM16, y el original exacto quedó en el respaldo aprobado.

La validación técnica confirma formato, hashes, enrutamiento y comportamiento de la envolvente. La semejanza perceptual final con el R24 requiere una escucha humana dentro de una sesión real de cambios ascendentes y reducciones.
