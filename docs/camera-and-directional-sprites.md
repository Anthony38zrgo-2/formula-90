# Cámara y presentación de vehículos

> **Referencia mixta.** Los detalles del V10/`ArcadeCarController` corresponden
> a un corte histórico. La cámara y el sprite direccional siguen registrados,
> pero consumen el vehículo GEVP mediante `VehicleAdapter`; la autoridad actual
> está en `docs/architecture.md`.

`ArcadeCarController` mantiene la simulación en `_physics_process` y conserva los transforms físicos anterior y actual. Cámara y presentación consumen `get_visual_transform()`, interpolado con la fracción física de Godot, sin alterar la simulación.

## Cámara

`ArcadeChaseCamera` se actualiza en `_process` con prioridad `-10`. La altura mundial Y y el pitch se bloquean al inicializarse, por lo que no existe diving. `look_height` selecciona el punto vertical fijo de enfoque y permite adaptar el encuadre al modelo sin variar el pitch durante la carrera.

Z recibe únicamente un impulso longitudinal breve y limitado; X depende del lado e intensidad del giro. El punto de mirada hereda primero el mismo offset X de la cámara y añade después una anticipación pequeña hacia el interior de la curva. Esto garantiza que el eje óptico apunte hacia el giro sin acumular el coche fuera del encuadre. La aceleración lateral o vertical no desplaza la cámara. En reversa se corrige el signo de giro según la dirección real de viaje. Resets y teletransportes reinician los acumuladores.

Valores del V10 3D en `player_car.tscn`:

- `distance = 7.2` y `height = 3.4`.
- `look_ahead = 2.8` y `look_height = 0.65`.
- `horizontal_dead_zone = 0.12`.
- `inertia_strength = 0.003` y `maximum_camera_offset = 0.05`.
- `lateral_swing = 0.65` y `turn_look_offset = 0.35`.
- `horizontal_smoothing = 7`, `heading_smoothing = 8` y `turn_offset_smoothing = 6`.
- `maximum_follow_lag = 0.55`.
- `base_fov = 60` y `speed_fov_gain = 3`.

## Visual 3D del jugador

`VehicleVisual3DController` se actualiza en `_process` con prioridad `5`, después de que la física haya producido su estado. Se vuelve top-level y aplica explícitamente la pose interpolada para evitar copiar saltos del tick físico.

El roll usa aceleración lateral local y el pitch aceleración longitudinal local. Ambos tienen ganancia, límite angular y suavizado independiente del framerate. La vibración solo aparece cuando `SurfaceProbes` detecta nodos en grupos `kerb`/`piano`/`gravel` o metadata `surface_type`.

Cuando existen nodos de rueda configurados, las delanteras reciben dirección visual suavizada y las cuatro acumulan giro según velocidad longitudinal y radio. Rutas vacías o nodos ausentes no impiden mostrar el modelo. El GLB actual tiene una sola malla y resuelve cero ruedas opcionales.

## Sprite direccional

`DirectionalVehicleSprite` ya no representa al jugador. Permanece para el placeholder estático, escenas de validación y futura decoración 2.5D.

El selector usa la orientación real del nodo respecto de la cámara, ángulos declarados por JSON, histéresis angular y filtro nearest. No usa la velocidad para orientar el sprite y no espeja el atlas V10 de 16 vistas. Cambios de cámara, reset o teletransporte reinician la histéresis.

`static_directional_car.tscn` reutiliza la clase, atlas y metadata sin física de movimiento. Las herramientas y pruebas del pipeline se conservan.
