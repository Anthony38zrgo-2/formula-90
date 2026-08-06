# Vehículo V10

V10 es el primer coche jugable de `formula-90s`. Su física y transmisión no cambiaron durante la migración visual 3D.

## Fuente 3D

- Fuente intacta: `references/vehicles/v10_3d/source/formula-v10.glb`.
- Copia importada: `game/assets/models/vehicles/v10/v10.glb`.
- SHA-256 de ambas copias: `4cb73cdc216224cd869d8e43e74450cd54ee1d64bfc06f42d7da720673de1a72`.
- Formato: glTF Binary 2.0, generador declarado Tripo.

Auditoría detectada:

- una escena, un nodo, una malla y una primitiva;
- 970 vértices y 1.806 triángulos;
- límites `(-0.221680, 0, -0.499023)` a `(0.221680, 0.263672, 0.499023)`;
- dimensiones `0.443359 × 0.263672 × 0.998047` unidades;
- `+Y` arriba, `+Z` frente de la fuente y `+X` derecha;
- origen centrado en X/Z, con la base en Y=0;
- posiciones y normales presentes;
- sin UV, texturas, imágenes, skins, animaciones ni transparencias;
- un material opaco blanco, metallic 0 y roughness 0.5;
- ruedas, suspensión y carrocería integradas en la misma malla, sin pivotes independientes.

## Integración

Godot importa el GLB sin editarlo. `v10_visual.tscn` actúa como contenedor y `v10_visual_3d.tres` aplica escala uniforme `4.0`, rotación Y `180°` para convertir `+Z` de fuente en `-Z` runtime y offset Y `0.08 m` para compensar el margen de contacto del cuerpo físico. El resultado mide aproximadamente `1.77 × 1.05 × 3.99 m`.

`VehicleVisual3DController` sigue la pose interpolada del `ArcadeCarController`, gira ruedas opcionales según velocidad longitudinal, dirige las delanteras mediante el input real, limita roll/pitch derivados de aceleración y añade vibración cuando las sondas detectan pianos o grava. Todas esas operaciones son visuales.

El GLB actual produce `0` nodos de rueda resueltos; esta es una limitación verificada, no un error. Para habilitar giro y rotación se necesita otro GLB con cuatro nodos de rueda, pivotes centrados y rutas configuradas en `v10_visual_3d.tres`.

## Compatibilidad

El `BoxShape3D`, `CarPhysicsConfig`, `VehicleDefinition`, transmisión, HUD, audio, minimapa y reset se mantienen. El `DirectionalVehicleSprite` del jugador fue eliminado después de validar el GLB con iluminación y sombra; su clase, atlas, placeholder estático y pruebas continúan en el proyecto.

La referencia y pipeline históricos de sprites permanecen bajo `references/vehicles/v10/` y `game/assets/sprites/vehicles/v10/` para decoración y validación, no para renderizar el jugador.
