# Cámara y sprites direccionales

`ArcadeCarController` mantiene la simulación en `_physics_process` y publica velocidad, aceleración mundial, dirección y deslizamiento. `ArcadeChaseCamera` se interpola en `_process` con prioridad `-10`; `DirectionalVehicleSprite` actualiza después con prioridad `10`. Así, la presentación consume el último estado físico sin modificarlo y el sprite utiliza la posición de cámara del mismo frame.

## Cámara

El objetivo combina la posición del coche con una anticipación visual muy limitada. La aceleración se convierte en un impulso de corta duración: se aplica el cambio inicial de fuerza y luego decae incluso si el acelerador continúa presionado. Así comunica arranque, frenada o entrada en curva sin trasladar continuamente toda la inercia. Todos los suavizados usan `1-exp(-rate*delta)`, por lo que son independientes del framerate.

Parámetros iniciales y ajustes recomendados:

- `horizontal_smoothing = 5.0`: seguimiento horizontal. Bajar a 4 aumenta peso; subir a 6 lo hace más firme.
- `vertical_smoothing = 7.5`: respuesta vertical, normalmente más firme que la horizontal.
- `horizontal_dead_zone = 0.12` y `vertical_dead_zone = 0.08`: filtran movimientos pequeños. Aumentarlos solo si persiste ruido.
- `velocity_anticipation = 0.025`: adelanto limitado del punto de mirada; ya no desplaza la posición base de la cámara.
- `inertia_strength = 0.018`: intensidad del impulso inicial. Mantener entre 0.01 y 0.025.
- `maximum_camera_offset = 0.55`: límite conservador del efecto dinámico y del retraso de seguimiento.
- `offset_smoothing = 3.2`: decaimiento del impulso; un valor mayor lo hace más corto.
- `speed_fov_gain = 4.0`: variación de FOV reducida para conservar tamaño y nitidez del coche.
- `distance`, `height` y `look_ahead`: encuadre base; ajustar solo después de validar los parámetros dinámicos.

## Sprite direccional

El ángulo parte del eje trasero real del coche respecto de la cámara. La dirección de la velocidad influye como máximo un 10 % y solo a velocidad suficiente, permitiendo que carrocería y trayectoria diverjan durante un derrape. Los ángulos vienen del JSON de la hoja; el número efectivo de direcciones es el número real de regiones cargadas.

- `orientation_count`: fallback cuando no hay JSON; admite 1, 8, 12 o 16.
- `first_frame_angular_offset = 0`: corrige la orientación base del atlas sin reordenarlo.
- `angular_hysteresis = 3°`: evita alternancia en fronteras; probar entre 2° y 5°.
- `minimum_visual_speed = 0.5 m/s`: congela el frame al detenerse.
- `velocity_direction_influence = 0.10`: influencia secundaria del movimiento; no superar 0.20 salvo una dirección artística deliberada.
- `allow_mirroring = false`: el atlas V10 contiene ambos lados y no se espeja automáticamente.

El Sprite3D fuerza `TEXTURE_FILTER_NEAREST`; los PNG no usan mipmaps ni compresión con pérdida. Esto evita el suavizado lineal perceptible durante desplazamientos subpíxel.
