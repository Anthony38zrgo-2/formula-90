# Cámara de cockpit del F1 2030 V10 base

La feature se limita a la escena original del F1 2030 V10. La tecla física `C` recorre chase, T-cam, cockpit y vuelve a chase. El piloto conserva brazos, manos, torso y piernas visibles en cockpit. La geometría `DriverHeadAndNeck` se oculta mientras esa cámara está activa y se restaura al salir.

El punto `DriverEyePoint` se genera en el centro de la abertura del visor, 35 mm detrás de su superficie frontal, y queda unido al hueso de la cabeza. El modelo lleva casco y no contiene globos oculares; el visor es la referencia geométrica del punto de vista. La cámara copia el marcador después de actualizar el esqueleto y aplica una elevación artificial configurable de 14 cm, siguiendo la petición de subir la vista aproximadamente un 25% para ver la trazada. La elevación sigue la orientación y los movimientos de la cabeza. La orientación neutra mira al horizonte en la dirección del auto.

Las aceleraciones lateral, longitudinal y vertical de la telemetría alimentan un resorte angular amortiguado. El modificador distribuye la rotación entre cuello y cabeza sin modificar las longitudes de los huesos. El piloto se anima también desde las vistas externas. Los límites restringen la inclinación y los saltos de posición borran la inercia residual.

La configuración vive en `game/data/cameras/formula_one_2030_cockpit_camera.json`:

| Parámetro | Uso |
| --- | --- |
| `force_response_strength` | Intensidad global: `0` desactiva el efecto G, `0.5` lo reduce a la mitad, `1` es la respuesta inicial y `2` la duplica antes de aplicar límites. |
| `lateral_response_degrees_per_gravity` | Inclinación lateral por cada G. |
| `longitudinal_response_degrees_per_gravity` | Inclinación al frenar o acelerar. |
| `vertical_response_degrees_per_gravity` | Movimiento de cabeza ante aceleración vertical. |
| `response_frequency_hertz`, `damping_ratio` | Rapidez de recuperación y amortiguación. |
| `maximum_pitch_degrees`, `maximum_roll_degrees` | Límites de inclinación. |
| `neck_rotation_fraction` | Fracción de la rotación aplicada al cuello; el resto se aplica a la cabeza. |
| `horizon_stabilization_strength` | Compensación de inclinación del chasis, de `0` a `1`. |
| `maximum_horizon_correction_degrees` | Límite de compensación para no forzar el cuello en vuelcos. |
| `field_of_view_degrees` | Campo de visión. |
| `viewpoint_elevation_meters` | Elevación artificial del punto de vista sobre los ojos: `0.14` sube 14 cm; `0` vuelve al visor. |

La configuración se carga al crear el piloto. `set_cockpit_force_response_strength()` permite ajustar la intensidad durante la sesión sin crear otra pose para la cámara.

La suite `game/tests/test_cockpit_camera.gd` comprueba el horizonte, la ubicación del ojo, el movimiento de ambos huesos, los signos de aceleración y frenado, la respuesta vertical, sensibilidad cero y media, límites, teletransporte, longitud de huesos y consistencia a 30, 60 y 120 cuadros por segundo. El smoke canónico usa eventos reales de la tecla `C` para comprobar el ciclo de tres cámaras y la restauración de la cabeza.

Proveniencia inicial: rama `main-clean`, HEAD `4df977712f316e21ca633c8f72978ab96ae8f8f4`. El checkout original tenía BUILD_SOURCE, DLLs e importaciones modificadas previamente. La implementación se preparó en `codex/cockpit-driver-camera`, a partir del mismo HEAD, con compilación nativa propia y cachés nuevas.

Validación: la suite de cockpit completó 749 modificaciones de esqueleto sin fallos; el error máximo de longitud fue de 0.00000019 m. También pasaron la regresión de brazos y manos, la suite de T-cam y los smokes de Fuji para el auto base y la variante MP4/6. Tras elevar la cámara, volvieron a pasar la suite de cockpit y el smoke del auto base.

La captura a 1920×1080 midió 0.54981 m entre el ojo original y la pista. La elevación de 0.14 m equivale al 25.46% de esa altura y permite ver el asfalto por encima del volante. Las capturas de la vista elevada están en `scratch/cockpit_driver_camera/captures_raised/`.

La revisión humana final debe comprobar la comodidad del movimiento y el encuadre al conducir, frenar, pasar pianos y girar. La sensibilidad inicial es una configuración ajustable.
