# Cámara y sprites direccionales

`ArcadeCarController` mantiene la simulación en `_physics_process` y conserva los transforms físicos anterior y actual. Cámara y sprite consumen `get_visual_transform()`, interpolado con la fracción de física de Godot, sin alterar la simulación. `ArcadeChaseCamera` se actualiza en `_process` con prioridad `-10`; `DirectionalVehicleSprite` actualiza después con prioridad `10` y usa la posición de cámara del mismo frame.

## Cámara

La posición base utiliza un heading suavizado independiente del framerate. El seguimiento y el impulso de inercia tienen límites separados: el primero permite amortiguación real sin perder al coche y el segundo conserva un efecto inicial breve. La anticipación por velocidad solo modifica el punto de mirada.

La aceleración se convierte en un impulso de corta duración: se aplica el cambio inicial de fuerza y luego decae incluso si el acelerador continúa presionado. Los resets y teletransportes reinician los acumuladores para evitar interpolar desde una pose antigua.

Parámetros iniciales:

- `horizontal_smoothing = 5.0`: seguimiento horizontal. Bajar a 4 aumenta peso; subir a 6 lo hace más firme.
- `vertical_smoothing = 7.5`: respuesta vertical, normalmente más firme que la horizontal.
- `horizontal_dead_zone = 0.12` y `vertical_dead_zone = 0.08`: filtran movimientos pequeños.
- `velocity_anticipation = 0.025`: adelanto limitado del punto de mirada.
- `inertia_strength = 0.018`: intensidad del impulso inicial; rango recomendado 0.01–0.025.
- `maximum_camera_offset = 0.55`: limita exclusivamente el efecto dinámico de inercia.
- `maximum_follow_lag = 1.5`: limita por separado cuánto puede retrasarse la posición base.
- `heading_smoothing = 5.0`: evita que la posición trasera salte en giros y trompos.
- `offset_smoothing = 3.2`: decaimiento del impulso; un valor mayor lo hace más corto.
- `speed_fov_gain = 4.0`: variación reducida de FOV para conservar tamaño y nitidez.
- `distance`, `height` y `look_ahead`: encuadre base; ajustar al final.

## Sprite direccional

El ángulo parte exclusivamente del eje trasero de la pose interpolada del chasis respecto de la cámara. El vector de velocidad no modifica la orientación; por ello carrocería y trayectoria pueden divergir durante un derrape. El Sprite3D se vuelve `top_level` y recibe la posición interpolada explícitamente para no heredar los saltos del tick físico.

Los ángulos vienen del JSON del atlas y el número efectivo de direcciones es el número real de regiones cargadas.

- `orientation_count`: fallback cuando no hay JSON; admite 1, 8, 12 o 16.
- `first_frame_angular_offset = 0`: corrige la orientación base sin reordenar el atlas.
- `angular_hysteresis = 3°`: evita alternancia en fronteras; probar entre 2° y 5°.
- `minimum_visual_speed = 0.5 m/s`: aumenta la histéresis un 50 % a baja velocidad, pero permite actualizar la vista si la cámara se mueve.
- `velocity_direction_influence = 0`: propiedad conservada por compatibilidad; ya no interviene en la selección.
- `allow_mirroring = false`: el atlas V10 contiene ambos lados.

Al cambiar la cámara activa, reiniciar o teletransportar el coche, el selector adopta inmediatamente el sector correcto y reinicia la histéresis. Se exponen `get_current_relative_angle()`, `get_selected_frame()` e `is_hysteresis_held()` para telemetría sin imprimir cada frame.

El Sprite3D fuerza `TEXTURE_FILTER_NEAREST`; los PNG no usan mipmaps ni compresión con pérdida. Esto evita filtrado lineal perceptible durante desplazamientos subpíxel.
