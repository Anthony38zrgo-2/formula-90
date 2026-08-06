# Cámara y sprites direccionales

`ArcadeCarController` mantiene la simulación en `_physics_process` y conserva los transforms físicos anterior y actual. Cámara y sprite consumen `get_visual_transform()`, interpolado con la fracción de física de Godot, sin alterar la simulación. `ArcadeChaseCamera` se actualiza en `_process` con prioridad `-10`; `DirectionalVehicleSprite` actualiza después con prioridad `10` y usa la posición de cámara del mismo frame.

## Cámara

La posición base utiliza un heading suavizado independiente del framerate. Sus offsets se calculan en los ejes locales del coche: la altura Y y el pitch quedan bloqueados al inicializarse, Z recibe solamente un impulso longitudinal muy sutil y X depende del lado e intensidad del giro. El seguimiento de la posición mundial continúa para no abandonar al vehículo.

La aceleración longitudinal se convierte en un impulso de corta duración sobre Z: se aplica el cambio inicial y luego decae aunque el acelerador continúe presionado. La aceleración lateral y vertical no desplaza la cámara. El input de dirección mueve cámara y punto de mirada hacia el interior del giro, dejando el coche hacia el borde opuesto. En reversa se corrige el signo según la dirección real de viaje. Los resets y teletransportes reinician los acumuladores.

Parámetros iniciales:

- `horizontal_smoothing = 5.0`: seguimiento horizontal. Bajar a 4 aumenta peso; subir a 6 lo hace más firme.
- `height = 4.5`: fija la altura mundial inicial; Y no vuelve a interpolarse durante la carrera. El pitch inicial también se conserva, evitando diving.
- `horizontal_dead_zone = 0.12`: filtra movimientos pequeños en el plano XZ.
- `velocity_anticipation = 0`: no hay adelanto adicional por velocidad.
- `inertia_strength = 0.003`: intensidad casi imperceptible del impulso longitudinal.
- `maximum_camera_offset = 0.05`: recorrido máximo de 5 cm para la inercia sobre Z.
- `lateral_swing = 2.1`: recorrido máximo horizontal X provocado por el giro.
- `turn_look_offset = 1.5`: cuánto apunta el objetivo hacia el lado del giro.
- `turn_offset_smoothing = 4.5`: entrada y retorno progresivos del desplazamiento lateral.
- `turn_activation_speed = 0.75 m/s`: evita mover la cámara al girar el volante estando detenido.
- `maximum_follow_lag = 1.25`: límite de retraso del seguimiento base; evita perder el coche.
- `heading_smoothing = 5.0`: evita que la posición trasera salte en giros y trompos.
- `offset_smoothing = 4.5`: decaimiento rápido del impulso longitudinal.
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

`DirectionalVehicleSprite` acepta como dueño tanto `ArcadeCarController` como cualquier `Node3D`. En un coche estático usa directamente el transform fijo del padre, pero continúa seleccionando las 16 caras respecto de la cámara activa. `static_directional_car.tscn` reutiliza esta misma clase, atlas y metadata sin incorporar física de movimiento.
