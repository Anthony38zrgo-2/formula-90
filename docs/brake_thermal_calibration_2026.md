# Calibración física de frenos del F1 2030 con referencias Brembo 2026

Fecha: 2026-10-02. Rama: `main-clean`. HEAD anterior a los cambios: `e80f5537ac6bd6b27d76615c6e110ced1dfde02a`.

## Alcance y referencias

El perfil activo es `game/data/vehicles/f1_2030/f1_2030_v10_geometric.json`, cargado por `f1_2030_v10_rust.tscn` en la sesión por defecto de `scripts/run_f1_94.ps1`. Las escenas que comparten ese perfil reciben la misma calibración. El perfil legado `f1_2030_v10_physics.json`, las escenas, los GLB y los materiales no se modifican.

Brembo publica para 2026 un diámetro delantero de 330 mm, masa de 2 kg por disco, 1440 canales de ventilación de 2,5 mm y espesores de 32 o 34 mm. También explica que puede alcanzarse un diámetro delantero de 345 mm: [Miami 2026, variación de los discos](https://www.brembo.com/en/motorsport/formula1/2026/facts-miami-2026).

Para el eje trasero, Brembo declara diámetros de 260–280 mm: [Reglas 2026](https://www.brembo.com/en/motorsport/formula1/f1-rules-2026). Se seleccionan 280 mm, sin introducir en este V10 una recuperación eléctrica que el modelo no contempla.

La ventana óptima de referencia es 350–550 °C; los picos de frenada pueden superar esa ventana. Brembo señala que cerrar parcialmente las tomas evita el enfriamiento excesivo y que temperaturas inferiores a 350 °C reducen el coeficiente de fricción: [Ventilación y ventana térmica](https://www.brembo.com/en/motorsport/formula1/ventilation-holes).

El modelo térmico usa masa, material y diámetros anulares. No representa individualmente el espesor ni los microcanales publicados por Brembo. Los diámetros interiores conservados, la masa trasera, la emisividad y las conductancias siguientes son supuestos de calibración; no son especificaciones medidas o publicadas por Brembo.

## Configuración aplicada

| Parámetro por rueda | Anterior delantero / trasero | Actual delantero / trasero |
|---|---:|---:|
| Diámetro exterior | 278 / 250 mm | 330 / 280 mm |
| Diámetro interior | 105 / 100 mm | 105 / 100 mm |
| Masa térmica del rotor | 1,35 / 1,80 kg | 2,00 / 1,40 kg |
| Capacidad térmica resuelta | 1500 / 2000 J/K | 2222 / 1556 J/K |
| Transferencia efectiva rotor–rin | 40,06 / 33,63 W/K | 10 / 8 W/K |
| Emisividad de radiación hacia el ambiente | Desactivada | 0,8 / 0,8 |
| Apertura del conducto | 2 / 4 % | 6 / 6 % |

La masa trasera se estima con la proporción de áreas anulares y la misma construcción de carbono: `2 × (0,280² − 0,100²) / (0,330² − 0,105²) = 1,398 kg`, redondeada a 1,4 kg. Su valor requiere validación adicional si se dispone de una especificación concreta del fabricante.

La transferencia rotor–rin anterior, modelada como una conductancia constante elevada, extraía demasiado calor durante el calentamiento. Se reduce esa vía efectiva y se incorpora radiación hacia el ambiente según `emisividad × área de ambas caras × constante de Stefan–Boltzmann × (T_rotor⁴ − T_ambiente⁴)`, usando temperaturas absolutas. El área radiativa no multiplica los canales internos de ventilación. Este modelo supone ambas caras radiando hacia un ambiente efectivo; no resuelve las temperaturas de las cubiertas ni sus factores de vista.

La emisividad es opcional y vale cero cuando no se declara. Los otros perfiles mantienen el comportamiento térmico anterior. El cargador valida el intervalo [0,1] y conserva el valor al serializar.

La ventana óptima pasa de 400–800 a 350–550 °C. El inicio de pérdida de eficiencia por exceso de temperatura permanece en 900 °C y el umbral crítico en 1100 °C. Estar momentáneamente por encima de 550 °C no implica activar inmediatamente esa pérdida. La curva fría alcanza eficiencia completa al llegar a 350 °C.

El par total máximo sigue siendo 3967,5 N·m y el reparto 56/44: 1110,9 N·m por rueda delantera y 872,85 N·m por rueda trasera con eficiencia completa. El modelo configura directamente par en la rueda; no calcula presión hidráulica ni fuerza de pinza a partir del radio del disco. El cambio de diámetro modifica las superficies de enfriamiento y radiación, y la masa modifica la capacidad térmica. Sus efectos sobre temperatura y eficiencia sí repercuten en el par aplicado. No se extrapola la potencia de frenado de un F1 híbrido 2026 a este V10.

## Simulaciones exclusivamente en Rust

El ejemplo `game/crates/vehicle-physics-engine/examples/brake_thermal_calibration.rs` carga el perfil real y usa `BrakeThermalSystem`. Integra a 120 Hz con torque y velocidades de rueda procedentes de tres sesiones canónicas. Recupera el par solicitado dividiendo el par registrado entre su eficiencia térmica y aplica la eficiencia del estado simulado. Así, la generación térmica usa el par realmente aplicado, no un calentamiento artificial.

Se interpolan las muestras registradas y se repite cada sesión cinco veces conservando el estado térmico. Los porcentajes de tiempo y las medias se calculan en la última repetición; los máximos abarcan toda la simulación. La repetición aproxima un régimen periódico, sin demostrar una convergencia matemática exacta. Las velocidades de rueda y las temperaturas de carcasa y gas se mantienen como condiciones registradas. Esto es una simulación del subsistema térmico, no una vuelta nueva con bloqueo de ruedas, neumáticos y trayectoria recalculados.

Cada informe incluye 13 escenarios por sesión: una ejecución de la duración original, cinco repeticiones con la configuración aplicada, ocho aperturas entre 0 y 100 %, y tres escalas de demanda de frenado (0,5, 1,5 y 2). Las escalas superiores a uno son pruebas de carga térmica, no una modificación del par máximo del vehículo.

### Validación de la reproducción de la configuración anterior

| Sesión | Temperaturas finales registradas, FL / FR / RL / RR | Temperaturas finales reproducidas |
|---|---:|---:|
| `telemetry_20261002_093624_236.csv` | 552,4 / 535,0 / 402,6 / 402,5 °C | 551,4 / 533,8 / 402,1 / 401,9 °C |
| `telemetry_20261001_214417_746.csv` | 376,6 / 370,4 / 284,3 / 283,1 °C | 377,4 / 371,1 / 284,9 / 283,6 °C |

La diferencia máxima final es inferior a 1,2 °C y el error de energía de frenado acumulada es inferior al 0,1 % por rueda. Los informes cuentan y descartan únicamente el último registro si está truncado; un registro incompleto intermedio produce un error.

### Configuración final, apertura 6 % en ambos ejes

| Sesión y duración de cinco repeticiones | Media de la última repetición FL / FR / RL / RR | Máximo FL / FR / RL / RR | Primer cruce de 350 °C FL / FR / RL / RR |
|---|---:|---:|---:|
| `20261002_093624_236`, 663,77 s | 539,2 / 530,2 / 554,0 / 553,1 °C | 662,1 / 657,6 / 693,0 / 692,1 °C | 54,5 / 54,5 / 53,5 / 53,6 s |
| `20261001_214417_746`, 1095,95 s | 363,0 / 357,0 / 360,6 / 358,2 °C | 488,9 / 494,4 / 510,1 / 508,6 °C | 100,2 / 100,6 / 99,8 / 99,9 s |
| `20261001_213150_765`, 1068,78 s | 387,9 / 385,9 / 386,5 / 384,5 °C | 556,6 / 556,9 / 578,8 / 576,8 °C | 99,9 / 100,0 / 99,6 / 99,7 s |

En la sesión más intensa, los traseros permanecen dentro de 350–550 °C aproximadamente el 39 % de la última repetición; en las otras sesiones, entre el 56 y el 62 %. Hay fases frías y picos por encima de la ventana óptima. Ninguna de las tres sesiones con la configuración final alcanza los 900 °C de inicio de pérdida de eficiencia.

### Necesidad del reajuste trasero

Una simulación adicional conserva las dimensiones nuevas y la radiación, pero restaura detrás la masa de 1,8 kg, la transferencia de 33,63 W/K y la apertura del 4 % anteriores. En la sesión `20261001_214417_746`, los traseros no alcanzan 350 °C durante los primeros 219,19 s y su media en la quinta repetición es 293,2 / 291,1 °C. Con la configuración final llegan a 350 °C a los 99,8 / 99,9 s y su media sube a 360,6 / 358,2 °C. Basta un reajuste térmico; las simulaciones no justifican desplazar el reparto mecánico hacia atrás para calentarlos.

Cambiar solamente los diámetros y la masa delantera, conservando las pérdidas anteriores, tampoco resuelve el problema: los traseros terminan la sesión menos intensa en 273,9 / 272,8 °C y no alcanzan 350 °C durante ella.

### Apertura coherente con la carga de frenado

La apertura del 6 % es un compromiso inicial entre las tres sesiones, no una garantía para cualquier circuito o piloto. La apertura modifica área efectiva, caudal de aire, disipación y resistencia aerodinámica. Cerrar por completo elimina el caudal del conducto; permanecen convección exterior, radiación y transferencia al rin.

| Apertura en ambos ejes | Sesión intensa: medias delanteras / traseras | Sesión menos intensa: medias delanteras / traseras |
|---|---:|---:|
| 0 % | 607–616 / 637–638 °C | 447–453 / 454–457 °C |
| 2 % | 584–593 / 612–613 °C | 419–425 / 424–426 °C |
| 4 % | 557–566 / 582–583 °C | 387–393 / 390–393 °C |
| 6 % | 530–539 / 553–554 °C | 357–363 / 358–361 °C |
| 12 % | 455–463 / 472–473 °C | 279–284 / 277–279 °C |
| 100 % | 148–151 / 148 °C | 84–85 / 81 °C |

Para la sesión intensa, 12 % sitúa mejor ambos ejes en la ventana óptima. Para las sesiones con menos frenadas, 2–4 % conserva mejor su temperatura. Las pruebas con menor demanda permiten que los discos vuelvan a estar fríos; el modelo no añade calor para mantener una temperatura objetivo. Con mayor demanda, la temperatura aumenta y puede activar la pérdida de eficiencia si la apertura resulta insuficiente.

## Reproducción y archivos

Desde la raíz del repositorio:

```powershell
cargo run --manifest-path game/crates/Cargo.toml -p vehicle_physics_engine --example brake_thermal_calibration -- game/data/vehicles/f1_2030/f1_2030_v10_geometric.json game/telemetry/telemetry_20261002_093624_236.csv game/telemetry/telemetry_20261001_214417_746.csv game/telemetry/telemetry_20261001_213150_765.csv
cargo test --manifest-path game/crates/Cargo.toml -p vehicle_physics_engine --lib --test brake_thermal_test --example brake_thermal_calibration
```

Los resultados completos y las configuraciones de comparación están en `scratch/brake_thermal_calibration/`: `original_thermal_results.json`, `geometry_only_thermal_results.json`, `unbalanced_rear_thermal_results.json` y `final_thermal_results.json`. `provenance.json` registra HEAD y SHA-256 de fuentes y sesiones. Son archivos locales de diagnóstico ignorados por Git. El ejemplo y este informe permanecen en el repositorio.

Validación: 147 pruebas de biblioteca, 12 pruebas de integración térmica y una prueba del lector del ejemplo. Las pruebas nuevas verifican radiación, equilibrio a temperatura ambiente, compatibilidad de perfiles sin emisividad, validación y serialización, respuesta a apertura/demanda, y el acoplamiento del simulador completo entre eficiencia, par, potencia y energía. La integración mecánica se verifica en frío, en la ventana óptima y con pérdida de eficiencia por exceso de temperatura.

No se ejecutó Godot ni se reconstruyeron o instalaron las DLL del juego. Los binarios y `BUILD_SOURCE` que ya estaban modificados se conservan. La prueba de conducción y el efecto sobre la evolución libre de los neumáticos quedan pendientes de una ejecución del juego con binarios reconstruidos desde estas fuentes.
