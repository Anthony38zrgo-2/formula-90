# V10 audio refactor — backlog and handoff

Fecha de corte: 2026-09-05
Worktree: `D:\Formula90s-aud-01-07`  
Rama: `codex/aud-01-07`
HEAD de origen: `e5c8a6d9825742745c553e56d78f78077c399039`

## Regla de autoridad

Este documento conserva el alcance de AUD-01 a AUD-07 que fue definido por el
modelo GPT-6 Astra en la conversación de trabajo. Las instrucciones técnicas
de este handoff deben limitarse a ese alcance y a los criterios que Astra dejó
establecidos.

Los documentos producidos después por el agente GPT-5.6 Sol son evidencia de
avance, no una nueva fuente de requisitos. En particular,
`docs/audio-v10/AUD_01_07_EXECUTION_PLAN.md`,
`docs/audio-v10/R25_REFERENCE_CORPUS.md` y
`docs/audio-v10/AUD_02_BASELINE.md` no deben ampliar ni reinterpretar el
backlog de Astra.

Los documentos existentes en `implementation/` tienen procedencia de Git, pero
no contienen una marca que permita demostrar que fueron escritos por Astra.
Bajo el criterio del usuario, pueden consultarse como contexto técnico, pero
no deben sustituir las instrucciones de Astra.

## Resultado que se busca

Mejorar la semejanza sonora con un V10 Renault R25 y reducir el coste de la
capa de audio Rust sin romper su frontera de tiempo real. El trabajo debe
preservar:

- síntesis de diez cilindros, escena acústica y capa híbrida de samples;
- autoridad de RPM y estados mecánicos en la física del vehículo;
- renderizado sample-accurate;
- ausencia de I/O, locks, strings o asignaciones no acotadas en el callback;
- cambios pequeños, medibles y reversibles.

No forman parte de este lote: rediseñar todo el banco R25, rehacer la
termodinámica del vehículo, cambiar relaciones de transmisión, rediseñar la
espacialización, crear una reverb nueva o hacer un mastering global sin una
medición que lo justifique.

## Estado actual

### AUD-01 — referencia R25

**Estado: preparado técnicamente; gate humano pendiente.**

Se inventariaron seis capturas en `docs/audio-design/V10-curated`:

| Fuente | Estado registrado |
|---|---|
| `01_23_r25_int_on_mid.wav` | on-throttle, mid; RPM estimada 5122.5 |
| `01_01_r25_int_on_midhigh.wav` | on-throttle, mid-high; RPM estimada 5965.5 |
| `01_22_r25_int_on_high.wav` | on-throttle, high; RPM estimada 8731.5 |
| `01_08_r25_int_off_mid.wav` | off-throttle, mid; RPM estimada 8172.0 |
| `01_06_r25_int_off_downshift.wav` | off-throttle/downshift; RPM desconocida |
| `01_29_r25_int_on_upshift.wav` | on-throttle/upshift; RPM desconocida |

Las posiciones de micrófono, marcha y velocidad no están calibradas. Las RPM
son estimaciones espectrales salvo que aparezca telemetría sincronizada. No se
debe declarar una preferencia auditiva antes de la escucha humana nivelada.

**Gate de aceptación:** comparar aceleración, RPM sostenida, lift/coast, un
upshift y un downshift contra GF509 con igual sonoridad. Registrar preferencia,
tono, brillo, pulso de combustión, balance admisión/escape, raspado/aliasing,
contenido mecánico, timing y artefactos de loop.

**Evidencia:** [R25_REFERENCE_CORPUS.md](D:/Formula90s-aud-01-07/docs/audio-v10/R25_REFERENCE_CORPUS.md).

### AUD-02 — baseline reproducible

**Estado: terminado técnicamente; aún no es un gate sonoro.**

Se creó `aud_runtime_baseline`, que activa y comprueba explícitamente
`ContinuousSourceKind::V10Gf509`, captura ambos canales y rechaza muestras no
finitas o fuera de rango antes de escribir PCM. También se conservaron stems
del renderizador `v10_render` como diagnóstico, sin afirmar paridad con el
runtime.

Baseline de runtime verificado en dos ejecuciones:

- 44 100 Hz, bloques de 256 frames, 1 034 bloques, 264 704 frames.
- SHA-256 `Gf509Runtime`: `84c36644bca5b6106340cf54fb1c1180d950c15b610a18b0ebef7141297fbd70`.
- SHA-256 `VehicleAudioEngine`: `22d36a126b300de6f01af307883af86f8cf07b0257a645e2b8203a6f6533d559`.
- SHA-256 de la traza: `a29b723fe31eeaa2e3bec5e458b166d92264e6142c09aa58f4ac2fe13429739c`.
- Se documentó que la ruta directa recibe `dt_seconds`, mientras que la ruta
  `VehicleAudioEngine::set_telemetry` aún reenvía `dt_seconds = 0.0`.

Captura histórica anterior a AUD-04 (no representa la última entrega):

- `D:\Formula90s-aud-01-07\reports\audio-v10\listening\current\gf509_runtime.wav`
- SHA-256 `84c36644bca5b6106340cf54fb1c1180d950c15b610a18b0ebef7141297fbd70`.

**Evidencia:** [AUD_02_BASELINE.md](D:/Formula90s-aud-01-07/docs/audio-v10/AUD_02_BASELINE.md) y
`reports/audio-v10/aud02/runtime-v2/`.

### AUD-03 — benchmark de la ruta real

**Estado: instrumentado y ejecutado; revisión y cierre pendientes.**

El benchmark mide por separado `Gf509Runtime`, `VehicleAudioEngine`,
`CoreFacade` y un control `CoreFacadeNoAudio`. Confirma activación GF509,
tiempo de CPU por bloque, percentiles de tiempo de pared y contadores de
asignación. Resultado de la última corrida registrada:

| Ruta | CPU de un núcleo | P50 | P95 | Asignaciones durante medida |
|---|---:|---:|---:|---:|
| `Gf509Runtime` | 17.15 % | 950.2 µs | 1251.9 µs | 0 |
| `VehicleAudioEngine` | 17.35 % | 978.6 µs | 1190.4 µs | 7 / 62 bytes |
| `CoreFacade` | 18.66 % | 1056.4 µs | 1852.9 µs | 45 056 / 7.84 MB |
| `CoreFacadeNoAudio` | 0.13 % | 5.4 µs | 5.6 µs | 45 056 / 7.84 MB |

Las asignaciones iguales de `CoreFacade` y `CoreFacadeNoAudio` impiden atribuir
esas 45 056 asignaciones exclusivamente al audio. `VehicleAudioEngine` aún
debe investigarse porque registra siete asignaciones en la medición. El
benchmark de `CoreFacade` usa telemetría producida por la física y las rutas
directas usan una traza sintética; no deben compararse como si fueran la misma
entrada.

**Gate técnico:** reproducir en release, con la misma máquina, traza, assets,
configuración y tamaño de bloque; publicar la variante exacta y separar coste
de física, mixer, GF509 y puente.

**Evidencia:** `reports/audio-v10/aud03/path_benchmark.json` y sus tres corridas;
`game/crates/formula90-core/src/bin/aud_path_bench.rs`.

### AUD-04 — telemetría física autoritativa

**Estado: implementación técnica parcial; pruebas focalizadas pasan; revisión y
gate humano pendientes.**

La instrucción de Astra es conectar a GF509 la carga, torque firmado, embrague,
fase de cambio, TC y limitador desde los estados autoritativos de la física,
eliminando actualizaciones provisionales redundantes. La implementación actual
usa un adaptador de control-rate desde `PowertrainState`, conserva la ruta C ABI
V3 y entrega la telemetría al runtime con `dt_seconds`; no mueve la física al
audio ni vuelve a aplicar el corte de TC en la carga.

La ruta provisional descrita en versiones anteriores fue sustituida por el adaptador físico. El estado y las limitaciones verificadas de la última entrega se detallan en el anexo de revisión al final de este documento. No considerar cerrada AUD-04 por el nombre de las pruebas ni por los resultados reportados por el implementador.

**Trabajo requerido por Astra:**

1. Añadir primero una prueba integrada que muestre el paquete que sale de la
   física y el estado que recibe GF509.
2. Conservar torque negativo como estado de retención y mantener separados
   throttle, carga y torque.
3. Conectar embrague, fase de cambio, TC y limitador desde sus estados
   autoritativos; si el contrato actual no los contiene, extenderlo con un POD
   versionado y validado, sin romper la ABI.
4. Evitar dos aplicaciones de TC, dos disparos de cambio o una reducción ciega
   de ganancia por el mismo estado.
5. Verificar aceleración, retención, free-rev/embrague abierto, TC, upshift,
   downshift y limitador a RPM comparable.

**Gate:** la carga ya no es alias de throttle; el torque negativo llega al
audio; las fases son deterministas; la traza no tiene picos o resets; la
fachada y GF509 reciben el mismo significado de cada campo.

**Evidencia de esta entrega (2026-09-05):** `RuntimeTelemetry` interno incluye
clutch, TC y limiter con validación de límites. Clutch y TC se interpolan por
muestra; shift y limiter son estados discretos. El sintetizador consume torque
firmado (retención), clutch, fase de corte/recuperación, limiter y TC como
textura sin reducir dos veces la carga. Pasan `cargo test -p formula90_core
aud04_integration` (2/2) y `cargo test -p v10-engine-synth --lib` (76/76, 1
ignorado). La matriz integrada cubre potencia, coast, clutch abierto, cambios,
TC, render finito y reset; además hay una prueba temporal a RPM fija para cut y
recovery. La suite de `vehicle_audio_engine` pasa 207/214; 7 fallos son los
tests C++ preexistentes que requieren `f90_audio_dsp.dll` ausente en este
worktree (`DllNotFound`). Sigue pendiente comparar el WAV candidato post-AUD04
con el baseline, reproducir la fachada→GF509 en release y completar la escucha
humana nivelada.

### AUD-05 — precálculo y actualización a frecuencia de control

**Estado: implementación parcial; evidencia técnica disponible, gate de ahorro
y escucha pendientes.**

La instrucción de Astra es medir antes de editar y sacar del bucle por muestra
los cálculos invariantes: coeficientes, `exp`, trigonometría y configuración de
filtros. Los controles que cambien más despacio pueden actualizarse a una tasa
menor e interpolarse, solamente después de demostrar que no se introducen
escalones audibles.

Requisitos:

- comparar CPU antes/después con AUD-03;
- conservar validación en los límites de entrada;
- no añadir I/O, locks, allocations, strings ni accesos Godot al callback;
- probar estabilidad de filtros, transitorios, zíper, pumping y discontinuidad;
- no precalcular algo que dependa de RPM, fase o carga por muestra sin una
  estrategia explícita de actualización/interpolación.

**Gate:** ahorro medido, salida finita y estable, sin regresión audible frente
al baseline.

### AUD-06 — suspensión de zonas de samples

**Estado: implementación parcial; continuidad y fase probadas, gate de ahorro
y escucha pendientes.**

La instrucción de Astra es evitar procesar zonas de samples con peso cero o
inaudible y conservar el avance de sus cursores/fase. Las zonas cercanas a un
crossfade deben seguir disponibles para no romper la transición.

Requisitos:

- no reiniciar el cursor al suspender;
- mantener la fase mecánica aunque la zona no se mezcle;
- reactivar con estado de filtros preparado o con una prueba que demuestre que
  la discontinuidad queda bajo la tolerancia acordada;
- cubrir crossfades ascendentes y descendentes, cambios de throttle y
  retención;
- medir trabajo ahorrado y verificar que la salida no adquiere clics, saltos o
  pérdida de continuidad.

**Gate:** menos CPU fuera de las zonas audibles, continuidad de fase y cero
artefactos audibles en las transiciones probadas.

### AUD-07 — resampling y aliasing

**Estado: implementación técnica acotada, revisión pendiente.**

La instrucción de Astra separa dos cambios que no deben confundirse:

1. kernels sinc tabulados, polifásicos o precalculados para reducir coste de
   CPU;
2. reducción de ancho de banda/filtrado antialias cuando la relación de pitch
   supera uno.

Requisitos:

- comparar cada variante contra el sinc actual con la misma traza;
- medir CPU y contenido de aliasing por separado;
- probar pitch bajo, cercano a uno y alto, además de subir y bajar RPM;
- no llamar “solución antialias” a una simple tabla sinc;
- verificar que la mejora no introduzca modulación, pérdida de brillo, clicks o
   cambios de fase perceptibles.

**Gate:** ahorro reproducible y aliasing bajo el umbral definido, sin regresión
audible respecto al resampler de referencia.

Evidencia AUD-07 (2026-09-05): `sample_layer.rs` conserva el sinc evaluado y la
tabla estrecha con la bandera de referencia. La ruta habilitada precomputa al
crear la capa una tabla de 64 niveles de `cutoff=1/ratio`, 1025 fases y 32 taps;
la fila de ratio 1 es la tabla estrecha embebida y las filas siguientes se
interpolan continuamente en cutoff y fase dentro del resampling. Las lecturas
son circulares para loops periódicos. El ratio máximo se deriva por asset como
`25000/rpm_anchor*source_sample_rate/output_sample_rate`; para el asset 44.1
kHz más bajo es aproximadamente 26.95x a 8 kHz, sin suponer un límite 4x.

Tests directos separados de CPU cubren passband y alias plegado a 1.3x y 2x a
44.1 kHz, además de 10x y 26.9x a 8 kHz, y un barrido fraccionario de subida y
bajada alrededor de ratio 1 contra el mismo cursor. El caso 26.9x usa un tono
claramente fuera de banda para medir rechazo; no demuestra transparencia de
32 taps en todo el intervalo hasta el nuevo Nyquist. La tabla ocupa 8,528,000
bytes (8.528 MB decimales; 8.13 MiB) por capa y se reserva sólo durante la
inicialización;
la métrica de coste AA queda separada del sinc de referencia. El filtro
Butterworth posterior de 14 kHz y sus tests fueron retirados por no prevenir
folding antes del resampling. Sigue pendiente el benchmark release comparable,
la revisión del launcher/BUILD y el gate humano de timbre; no se afirma escucha.

## Instrucciones operativas para el siguiente modelo

1. Comprobar antes de editar rama, HEAD y `git status`. El checkout original
   `D:\Formula90s` contiene cambios ajenos en el submódulo
   `third_party/godot-cpp`; no limpiarlo, resetearlo ni incorporarlo.
2. Continuar en `D:\Formula90s-aud-01-07`, revisar los cambios no comprometidos
   y no usar `git reset --hard`, `git checkout --` ni comandos destructivos.
3. Leer `AGENTS.md` y este handoff. Tomar como autoridad de requisitos solo el
   backlog de Astra conservado aquí. Tratar los documentos de Sol como
   evidencia, no como instrucciones nuevas.
4. No regenerar ni sobrescribir el baseline de AUD-02 después de cambios de
   DSP. Cada candidato debe tener su propio directorio, traza, hashes de
   fuente, configuración y resultado.
5. Cerrar primero AUD-03 documentalmente si hace falta, y luego ejecutar AUD-04
   con una prueba integrada antes de cambiar el sonido. No saltar a AUD-05–07
   mientras el cambio de telemetría no tenga una comparación reproducible.
6. Para cada optimización, modificar un subsistema por vez, ejecutar pruebas
   focalizadas, `git diff --check` y el benchmark de release. No reclamar
   porcentaje de ahorro a partir de corridas con distinta ruta, traza o
   configuración.
7. Mantener los gates humanos de AUD-01 abiertos: el modelo puede preparar
   WAVs, métricas y hojas de escucha, pero no inventar una preferencia auditiva.
8. Antes de declarar un ítem terminado, registrar: archivos modificados,
   comandos exactos, resultado, hashes, riesgos y gate pendiente. Si una
   instrucción no aparece aquí o en el backlog de Astra, detenerse y pedir
   aclaración en vez de inventarla.
9. Mantener los cambios atómicos. No hacer commit ni merge automáticamente;
   dejar los diffs listos para revisión humana.
10. En la integración final, usar `run_f1_94.ps1` y comprobar paridad BUILD/HEAD
    según `AGENTS.md`; una build de otra rama, DLL, objeto o cache no es válida.

## Definición de terminado de este lote

AUD-01–AUD-07 solo pueden declararse cerrados cuando:

- el gate humano de AUD-01 tiene notas de escucha completas;
- AUD-02 tiene baseline reproducible separado de cada candidato;
- AUD-03 compara la ruta real y atribuye costes con datos comparables;
- AUD-04 conserva los estados físicos sin alias throttle/carga ni doble TC;
- AUD-05 demuestra ahorro sin artefactos;
- AUD-06 demuestra suspensión/reactivación sin pérdida de fase ni clicks;
- AUD-07 demuestra ahorro de resampling y control de aliasing por separado;
- los tests, asignaciones del callback, launcher y paridad BUILD/HEAD pasan;
- el gate humano revisa aceleración, sostenido, retención y cambios.

Si solo queda una escucha humana, dejar el estado como “técnicamente listo,
gate humano pendiente”; no convertirlo en “done”.


## Anexo autoritativo de revisión y handoff externo — 2026-09-05

Este anexo sustituye cualquier estado contradictorio en los apartados anteriores.
Conserva los requisitos AUD-01–07 ya registrados; las observaciones siguientes son
hallazgos de revisión del código actual, no citas literales del backlog original.
No se reconstruyen instrucciones AUD-08–16 ni se atribuyen requisitos nuevos a Astra.

### Suspensión y procedencia

- Usuario ordenó detener al agente. `/root/aud_luna_high` fue interrumpido cuando
  ya había terminado su última entrega. No reanudarlo automáticamente.
- Automatización `supervisar-audio-r25-aud-01-a-aud-07`: PAUSED, confirmado.
- Worktree exclusivo: `D:\Formula90s-aud-01-07`; rama `codex/aud-01-07`;
  HEAD `e5c8a6d9825742745c553e56d78f78077c399039`. Implementación SIN COMMIT.
  HEAD identifica la base, NO identifica por sí solo el DSP modificado.
- Backup documental y patch tracked de esta revisión:
  `D:\Formula90s-aud-01-07-backups\handoff-review-20260905`.
  El patch NO incluye archivos untracked ni WAV ignorados. Backup previo de
  fuentes/documentación: `D:\Formula90s-aud-01-07-backups\resume-20260905`.
- Antes de modificar, el agente externo debe inventariar y preservar también
  untracked, assets y resultados ignorados. No asumir que clonar HEAD recupera
  esta entrega. No tocar ni limpiar `D:\Formula90s` ni sus submódulos.

### Estado consolidado

| Ítem | Estado verificable | Trabajo necesario para cierre |
|---|---|---|
| AUD-01 | Corpus de seis capturas documentado; RPM estimadas | Escucha humana nivelada y notas por escenario; no hay aprobación sonora |
| AUD-02 | Baseline stereo determinista preservado | Mantener inmutable; trasladar WAV/traza/metadata al entorno externo |
| AUD-03 | Harness release implementado y ejecutado | Repeticiones comparables, atribución de 7 allocations del mixer y revisión final |
| AUD-04 | Adaptador/consumo y 3 pruebas de integración verificadas; revisión parcial | Resolver límites restantes de R1–R4, matriz release completa y gate sonoro |
| AUD-05 | Implementación parcial; tests de invariantes/control pasan | Comparación CPU reproducible, revisar filtros/transitorios y gate humano |
| AUD-06 | Implementación parcial; suspensión, cursores y reactivación probados | Medir ahorro comparable y completar gate humano de transiciones |
| AUD-07 | Implementación técnica acotada, revisión pendiente | Kernel AA pretabulado dentro del resampling; medir CPU/alias por separado y pasar gate humano |

No existe evidencia para afirmar que el sonido ya se parece al R25 ni que
AUD-05–07 produjeron ahorro. Las diferencias de CPU entre corridas de AUD-04
no demuestran una optimización: hubo cambios de entrada/control y ruido temporal.

### Mapa de cambios sin commit

Rutas relativas al worktree:

| Archivo | Función actual |
|---|---|
| `.gitignore` | Permite versionar los harness bajo directorios bin |
| `game/crates/Cargo.lock` y `formula90-core/Cargo.toml` | Dependencias/registro del benchmark |
| `game/crates/formula90-core/src/audio_telemetry.rs` (nuevo) | MechanicalAudioState y AudioTelemetryAdapter |
| `game/crates/formula90-core/src/lib.rs` | Snapshot físico en ambas rutas de step, integración y tests |
| `game/crates/formula90-core/src/audio.rs` | Adaptador, telemetría timed y reset |
| `game/crates/formula90-core/src/bin/aud_path_bench.rs` (nuevo) | Benchmark por rutas y asignaciones |
| `game/crates/vehicle-audio-engine/src/mixer.rs` | set_telemetry_timed y reenvío de campos |
| `game/crates/vehicle-audio-engine/src/bin/aud_runtime_baseline.rs` (nuevo) | WAV stereo, traza y hashes |
| `game/crates/v10-engine-synth/src/runtime.rs` | Contrato interno, interpolación y consumo mecánico |
| `game/crates/v10-engine-synth/src/engine.rs` | Energía según torque/shift/limiter y textura TC |
| `game/crates/v10-engine-synth/src/sample_layer.rs` | Torque y clutch en peso off-throttle |
| `game/crates/v10-engine-synth/src/bin/v10_render.rs` | Adaptación al contrato de SampleLayerInput |
| `docs/audio-v10/` y este archivo | Evidencias y handoff; sin commit |

### Contrato implementado en AUD-04

`MechanicalAudioState::from_physics` calcula:

- `torque = clamp(engine_torque / max(max_torque,1), -1,1)`.
- `load = clamp(abs(clutch_torque)/max(max_torque,1),0,1)` si engagement>1e-4;
  de lo contrario load=0. Cuando engine_torque>0, multiplica load una sola vez
  por `1-clamp(tc_cut_ratio_smoothed,0,1)`; torque conserva su significado.
- Clutch/TC pertenecen a [0,1]; limiter procede de `is_rev_limited`.
- Cut procede de `shift_timer>0`; dirección de `target_gear<current_gear`.
  Recuperación acústica de 40 ms al terminar cut. No se sintetiza downshift blip.
- Derivadas EMA con constante 20 ms; dt válido finito en (0,0.1]; primera
  observación/dt inválido reinicia derivadas. Signo de torque usa banda ±0.025.
- `VehicleAudioTelemetryV3` conserva ABI existente. Extensiones son internas
  de Rust; no interpretar que todo consumidor externo fue probado.
- Runtime interpola clutch/TC; shift/limiter son discretos. Engine consume
  estados mecánicos, SampleLayerInput recibe torque/clutch. Continúa release
  energético de 413 ms. No hay prueba suficiente de timing perceptivo.

### Observaciones bloqueantes para revisión de AUD-04

**R1 — Parcialmente superada; queda revisión semántica del stem.**
En `sample_layer.rs::off_throttle_weight` la fórmula exacta es:

```text
overrun = clamp((0.30-throttle)/0.30,0,1)
low_load = clamp((0.40-load)/0.40,0,1)
low_load_engaged = low_load*clutch
retention = max(-torque,0)*clutch
weight = overrun*max(low_load_engaged,retention)*gain
```

Con throttle=0, load=0, torque=0, clutch=0 y gain=0.20 resulta weight=0.0; la
prueba coherente de free-rev ya pasa. La retención con carga alta conserva el
término de torque firmado y la prueba de TC confirma que no se aplica una
segunda reducción. Sigue pendiente una revisión perceptual de la semántica del
stem; el caso clutch=0/load alto permanece cubierto como entrada incoherente.

**R2 — Parcialmente superada; falta la matriz release completa.**
`physical_matrix_reaches_facade_and_gf509_at_fixed_rpm` ejecuta escenarios
secuenciales sobre la misma entidad, ocho steps por escenario; no fija ni
compara RPM. Lee `last_normalized_engine_torque` del mixer, no el paquete
consumido por GF509. Comprueba muestras finitas y presencia de etiquetas,
pero no diferencias de energía ni que TC/cut/clutch efectivamente se activaron.
El nombre del test no constituye evidencia. Faltan assertions de precondición,
campos recibidos y respuesta de cada estado, incluido limiter y recuperación.
Las pruebas actuales separan transporte real desde física y DSP a controles
deterministas: la verificación `aud04_integration` pasa 3/3, incluyendo campos
que recibe GF509 y respuestas diferenciadas. Sigue pendiente la matriz release
completa a RPM fija para todos los estados y la revisión de paridad del
launcher; no se modifica la física productiva para fabricar escenarios.

**R3 — Parcialmente superada; falta validar la respuesta perceptual.**
`cut_and_recovery_are_temporally_smooth_at_fixed_rpm` y la matriz de runtime
comprueban ahora que cut/recovery producen respuestas finitas y diferenciadas a
RPM fija. No se cambió gratuitamente `ENERGY_RELEASE_SECONDS=.413`; falta medir
su respuesta temporal con el conjunto procedural+samples y registrar la decisión
humana. Los umbrales perceptivos definitivos no están especificados.

**R4 — Sigue abierta, con evidencia de benchmark separada.**
`aud_runtime_baseline` usa una traza sintética de aceleración/coast y upshift,
clutch=1, TC=0, limiter=false. No demuestra todos los estados de la fachada.
La ruta legacy `set_telemetry` conserva dt=0; la fachada usa la ruta timed. El
benchmark release AUD07 es una medición de paths con GF509 activo, no una matriz
física completa ni una escucha. Se conserva el baseline original y el candidato
AUD07 separado en `reports/audio-v10/aud07-20260905/`; falta la matriz completa
y la comparación humana.

### Evidencia reproducible y límites

Verificación actual: `cargo test -p v10-engine-synth` terminó con 87 passed,
1 ignored, 0 failed; `cargo test -p formula90_core aud04_integration` terminó
con 3 passed, 0 failed. La suite mixer reportó 207 passed/7 failed por
`f90_audio_dsp.dll` ausente. Estos resultados no sustituyen la aceptación
humana ni la matriz release completa. No copiar DLL de otra rama para ocultar
el bloqueo.

Candidato WAV post-AUD04, hashes comprobados por supervisor:
`reports/audio-v10/aud04/audio-candidate-20260905/`

| Archivo | SHA-256 |
|---|---|
| gf509_runtime.wav | BC4996C983898D6C5276EECC8D33E2E9B80DE69BB938BB3B7C1FD34BAF327F71 |
| vehicle_audio_engine.wav | 35A50D0E1C5DA17165E4D0A75CADD80345DCFC7AE1FCA04E4FFD99F6860B28E2 |
| trace.csv | A29B723FE31EEAA2E3BEC5E458B166D92264E6142C09AA58F4AC2FE13429739C |
| metadata.json | C2231CAA67DFB9EBFCA4F72AE5D50A6C7FA062C335DD0C79E292BBF72C17D113 |

Baseline inmutable: `reports/audio-v10/aud02/runtime-v2/run1` y `run2`;
hashes en AUD-02 arriba. `reports/audio-v10/listening/current` contiene audio
ANTERIOR a AUD-04. No distribuirlo como el sonido posterior a estos cambios.
Benchmark previo comparable de referencia:
`reports/audio-v10/resume-20260905/before.json`, hash
`322652F5CFAD1D657E126ECA21F8BA9E3BDF332ADBDEE4B10278197B476C2ACA`.
Último benchmark candidato según informe Luna:
`reports/audio-v10/aud04/candidate-20260905.json`, hash
`FE706F032778F9161CC4F4D0E560BE6AF538DFAEBB177C6D6728E2A223B1B451`.
Este candidato fue regenerado; hashes anteriores de ese mismo nombre están
obsoletos. En adelante usar nombres únicos para cada ejecución.

### Orden de ejecución para el agente externo

1. Leer AGENTS.md, verificar branch/HEAD/status y preservar todos los cambios.
2. Reproducir tests actuales y registrar entorno/toolchain, stdout y exit code.
3. Resolver R1–R4 con pruebas que fallen ante comportamiento incorrecto.
4. Generar matriz de control y audio candidato con fuente/config/assets hashes;
   conservar reproducción determinista, validar ambos canales y valores finitos.
5. Cerrar técnicamente AUD-03/04 mediante review antes de modificar AUD-05.
6. AUD-05: perfilar los bucles de runtime/engine/sample_layer, identificar
   invariantes reales, mover sólo los que no dependan del estado por muestra;
   interpolar controles, comprobar filtros y medir release antes/después.
7. AUD-06: conservar referencia siempre activa; separar avance de cursor/fase
   de trabajo DSP de zonas con peso cero; preparar estados de filtro al volver,
   comparar barridos ascendentes/descendentes, throttle y retención. Documentar
   el criterio de inaudibilidad propuesto; no presentarlo como umbral aprobado.
8. AUD-07: conservar sinc actual como referencia; comparar tabla/polifases para
   coste, y filtro dependiente de ratio para aliasing cuando pitch>1. Medir
   alias frente a referencia adecuadamente filtrada, no sólo diferencias WAV.
   Documentar error, espectro, coste y memoria por separado para cada variante.
9. Un subsistema por cambio, sin commits automáticos; revisión de diff explícito.
10. Preparar escucha nivelada R25/baseline/candidato. Registrar decisión humana;
    no cerrar los gates sonoros mediante tests numéricos.
11. Integración final por run_f1_94.ps1, BUILD/HEAD y fuente dirty documentados;
    no asumir que un benchmark headless valida Godot ni los binarios instalados.

Comandos desde `D:\Formula90s-aud-01-07\game\crates` (PowerShell).
Los directorios `external-review-*` deben ser nuevos; si existen, elegir otro
sufijo antes de ejecutar. No sobrescribir resultados anteriores.

```powershell
cargo test -p formula90_core --lib --no-fail-fast
cargo test -p v10-engine-synth --lib --no-fail-fast
cargo test -p vehicle_audio_engine --lib --no-fail-fast
cargo build --release -p formula90_core --bin aud_path_bench
./target/release/aud_path_bench.exe ../sounds/banks/v10_vehicle ../audio/v10_gf509 ../data/vehicles/f1_2026_2008/f1_2026_2008_physics.json ../../reports/audio-v10/external-review-bench.json
cargo run --release -p vehicle_audio_engine --bin aud_runtime_baseline -- ../sounds/banks/v10_vehicle ../audio/v10_gf509 ../../reports/audio-v10/external-review-audio
 git diff --check
```

El perfil f1_2026_2008 activa GF509; el perfil f1_94 usado inicialmente no lo
activaba. Verificar activación explícita, no inferirla del nombre del vehículo.
Benchmark: 44100 Hz, bloque256, 4096 bloques; física debe avanzar DT=256/44100
por bloque para no mezclar tiempo simulado y render. Distinguir render-only
(0 allocations reportadas), update+render mixer (7/62 bytes) y facade con
física (45056/7839744 bytes también presentes en NoAudio). No atribuir estas
últimas al callback. No comparar rutas con trazas diferentes como equivalentes.

El límite previo de cuota sigue vigente si se retoma dentro de esta cuenta:
no exceder el mínimo de 15% restante; detener nuevas fases al20% y dejar handoff.
No canjear resets. La transferencia no autoriza reactivar automatizaciones.

Verificación independiente final del supervisor: `cargo test -p formula90_core
--lib aud04_integration --no-fail-fast` terminó con exit code 0: 3 passed, 0
failed, 9 filtered out. `git diff --check` limpio. El benchmark release
AUD07 reporta GF509 activo, 13.14% CPU en `Gf509Runtime` y 8,528,000 bytes
de tabla AA por capa durante inicialización. R1–R3 están parcialmente
superadas por pruebas focalizadas; R4 y los gates de matriz release, BUILD/HEAD
y escucha humana permanecen abiertos. No se declara ningún ítem totalmente
cerrado.
