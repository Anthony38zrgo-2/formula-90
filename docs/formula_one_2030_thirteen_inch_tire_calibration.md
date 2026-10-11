# F1 2030: candidata de calibración de neumáticos de 13 pulgadas

Fecha: 11 de octubre de 2026. Rama: `codex/thirteen-inch-tire-calibration`.
Fuente inicial: `67dc72fff7966cd246c88f93079779f10d6552bb`.
Referencia de 18 pulgadas: perfil geométrico del padre de `c60bed52`.
Trabajo aislado del checkout principal, que contiene cambios previos en binarios, modelos y otros archivos.
Promoción a `main-clean` autorizada por el usuario el 11 de octubre de 2026. La corrección térmica está en `70ae42af` y la calibración con sus manifiestos en `b7ba2a0e`. La reconstrucción y los resultados del runtime se registran por separado bajo `scratch/thirteen_inch_calibration_main_clean_build_20261011/`.

## Alcance y fundamento

La candidata aumenta la flexibilidad y hace más progresiva la respuesta de carcasa sin reducir el coeficiente máximo de fricción. Los coeficientes son estimaciones para el coche ficticio del juego, no mediciones de un neumático Pirelli.

Pirelli describe menos flexión y más precisión al reducir el flanco para 18 pulgadas:
https://press.pirelli.com/a-look-into-the-future-of-formula-1-with-new-18-inch-tyres-for-2022-cars/
El cambio inverso respalda la dirección del ajuste, pero no publica coeficientes para esta calibración.
La referencia histórica del juego tiene diámetro exterior de 660 mm; no reproduce los 720 mm del Pirelli de 2022.

## Parámetros

| Parámetro | Referencia | Candidata delantera | Candidata trasera |
| --- | ---: | ---: | ---: |
| Rigidez vertical, N/m | 180000 | 144000 | 153000 |
| Amortiguamiento radial, N·s/m | 1200 | 1250 | 1300 |
| Ángulo del pico lateral, rad | 0.100 | 0.115 | 0.115 |
| Relajación mínima, s | 0.035 | 0.045 | 0.045 |

Se conserva la recuperación relativa de 1.6, el modelo de presión y el coeficiente máximo de fricción. El desplazamiento del pico lateral reduce la pendiente de fuerza para ángulos pequeños; no pretende simular una carcasa lateral con grados de libertad independientes.
La estabilidad permanece disponible en el perfil de conducción. Se elimina de los ensayos mediante una máscara de ayudas igual a cero; el banco aislado no contiene controlador de estabilidad.

## Comparación con carga idéntica

Productores de evidencia: `compare_tire_construction.rs` y `comparison.json` bajo `scratch/thirteen_inch_tire_calibration/`.
Condiciones: 145 kPa manométricos en ambos ejes; banda a 95 °C, carcasa y gas a 80 °C; velocidad de 30 m/s; carga de 2500 N delante y 3000 N detrás; referencia de carga de 698 kg común para la huella. No se permite evolución térmica en este banco.
Escalón de ángulo de rueda de 0.02 rad durante 0.5 s, después de 0.5 s de estabilización, seguido de 0.5 s de liberación. Frecuencia de evaluación: 960 Hz.
Irregularidad del banco de contacto: seno de 2 mm a 8 Hz durante un ciclo, con buje fijado. El evaluador de contacto del modelo acoplado calcula las fuerzas; no se utiliza un modelo alternativo de fuerza radial.

| Medida | Referencia delantera | Candidata delantera | Referencia trasera | Candidata trasera |
| --- | ---: | ---: | ---: | ---: |
| Deformación estática, mm | 13.889 | 17.361 | 16.667 | 19.608 |
| Rigidez lateral efectiva, N/rad | 165444 | 145017 | 199723 | 174844 |
| Tiempo al 90% de fuerza, ms | 75.000 | 97.917 | 75.000 | 97.917 |
| Tiempo de liberación al 10%, ms | 137.500 | 176.042 | 137.500 | 176.042 |
| Energía disipada por ciclo, J | 0.75799 | 0.78957 | 0.75799 | 0.82115 |

Criterios definidos en el productor antes de evaluar: deformación mayor en al menos 10%; pendiente lateral entre 75% y 99% de referencia; tiempo de respuesta mayor en al menos 15%; pico de fuerza dentro de ±5%; disipación positiva y mayor que la referencia; cierre de la energía del ciclo dentro de 1e-8 relativo.
Ambos ejes pasan. El pico de fuerza se conserva con diferencia numérica despreciable. La mayor disipación del ciclo se verifica para igual amplitud y frecuencia; no implica que toda irregularidad produzca más calor ni garantiza menor aceleración de carrocería.

## Conexión entre disipación y temperatura

La ruta acoplada publicaba la deformación sin actualizar su velocidad. Ahora obtiene la velocidad de cierre desde la velocidad relativa del buje y el contacto.
En la integración térmica acoplada, el calor de carcasa es `amortiguamiento radial × velocidad de deformación al cuadrado × fracción térmica`. Se omite allí el término legado basado en fuerza normal por valor absoluto de velocidad, para no contar dos veces ni convertir trabajo elástico reversible en calor.
La prueba `coupled_tire_hysteresis_test` comprueba compresión, reposo y extensión en el mundo de producción. No cambia el modelo térmico legado ni las fracciones térmicas del perfil.

## Validación y límites

Pasan 59 pruebas seleccionadas: mundo acoplado 25, perfil geométrico 8, auditoría de masa 4, transiciones de agarre 10, térmicas 11 e histéresis acoplada 1.
`suspension_multibody_test` presenta 7 fallos y 9 éxitos. Una copia temporal de la misma suite con el perfil anterior a esta calibración reproduce exactamente los 7 fallos de cierre de masa. El fixture mantiene 600 kg de cuerpo más las nuevas ruedas, pero declara 698 kg totales. La copia temporal se eliminó; no se cambió la suite existente para ocultar los fallos.
Los dos manifiestos de livery se actualizan al hash del perfil candidato. `git diff --check` pasa.

La calibración vertical usa las constantes de la ruta acoplada. La ruta legada continúa derivando su rigidez desde una deformación estática de 8 mm; esta candidata no establece equivalencia entre ambos integradores.
El banco aislado no certifica guiñada de carrocería, aceleraciones sobre baches, despeje aerodinámico dinámico, contacto visual ni comportamiento durante un stint.
La revisión en Godot y la reconstrucción completa con paridad de fuente quedan pendientes de promoción y aprobación humana. Los ejecutables diagnósticos se compilan desde cero en el worktree; no se reutilizan DLL del checkout principal ni se publica un BUILD de juego para esta candidata.

## Reproducción

Desde `game/crates`:

```powershell
cargo run -p vehicle_physics_engine --example compare_tire_construction -- ../../scratch/thirteen_inch_tire_calibration/eighteen_inch_reference.json ../data/vehicles/f1_2030/f1_2030_v10_geometric.json ../../scratch/thirteen_inch_tire_calibration/comparison.json
cargo run --release -p vehicle_physics_engine --example compare_vehicle_tire_packages -- ../../scratch/thirteen_inch_tire_calibration/eighteen_inch_reference.json ../data/vehicles/f1_2030/f1_2030_v10_geometric.json ../../scratch/thirteen_inch_tire_calibration/vehicle_comparison.json
```

El segundo productor usa masas reales de cada paquete, por lo que sus cargas normales y velocidades de avance evolucionan libremente. Compara un escalón de entrada de dirección y una irregularidad suave de 5 mm de altura y 1 m de longitud en una carretera sintética, con combustible cero y máscara de ayudas cero. Reinicia presión y temperaturas antes de cada paso físico. Sus resultados complementan el banco de carga idéntica y no sustituyen ese control.

## Resultado del coche completo

El mundo acoplado completó ambos escenarios con cada paquete. No perdió contacto: las fuerzas normales mínimas permanecieron positivas.

| Medida | Referencia de 18 pulgadas | Candidata de 13 pulgadas |
| --- | ---: | ---: |
| Aceleración vertical máxima sobre la irregularidad, m/s² | 2.25196 | 2.17082 |
| Fuerza normal mínima durante la irregularidad, N | 958.62 | 1039.31 |
| Tiempo al 90% del pico de guiñada en la ventana del escalón, ms | 76.04 | 84.38 |
| Pico de velocidad de guiñada, rad/s | 0.08824 | 0.09214 |
| Velocidad final sobre la irregularidad, m/s | 29.401 | 29.378 |

La aceleración máxima sobre esta irregularidad baja aproximadamente 3.6%. El tiempo de guiñada se mide desde la entrada del escalón y usa el pico de cada variante dentro de la ventana de dirección aplicada; la salida se muestrea cada 8 pasos físicos, con resolución de 8.33 ms. El aumento de tiempo es de una muestra, por lo que es evidencia complementaria de menor rapidez, no una estimación precisa para cualquier maniobra.
El pico de guiñada crece aproximadamente 4.4%; la reducción de masa e inercia impide equiparar una carcasa más progresiva a menor capacidad total de giro.
Las temperaturas y presiones se igualan antes de cada paso físico, pero el integrador térmico continúa activo dentro de ese paso. Los ensayos no validan evolución térmica durante un stint.
