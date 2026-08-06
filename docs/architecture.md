# Arquitectura

`formula-90s` separa simulación, presentación y contenido declarativo. Toda lógica de runtime permanece en C++20 mediante GDExtension; no hay GDScript.

## Flujo principal

`GameBootstrap` mantiene la raíz y carga `MainMenu` o `TestField`. `ArcadeCarController` ejecuta movimiento y transmisión en `_physics_process`; conserva los transforms físicos anterior y actual y expone `get_visual_transform()` interpolado. Cámara, visual, HUD, reset y audio consumen ese estado sin decidir física.

## Vehículo jugable

La composición de `player_car.tscn` es:

```text
PlayerCar (ArcadeCarController)
├─ CollisionShape3D
├─ VehicleVisualRoot (VehicleVisual3DController)
│  └─ V10Model
├─ SurfaceProbes
│  ├─ FrontProbe
│  └─ RearProbe
├─ CameraRig
├─ AudioRoot
└─ ResetMarker
```

La colisión continúa siendo un `BoxShape3D` simple de `1.7 × 0.8 × 3.8 m`. La malla importada nunca participa en la colisión dinámica. `VehicleVisualRoot` se vuelve top-level durante el runtime y sigue la pose interpolada; escala y movimientos secundarios se aplican solo a la presentación.

`VehicleVisual3DConfig` declara escala, rotación, offset, rutas opcionales de ruedas, radio, dirección visual, límites de roll/pitch, suavizado y vibración. `VehicleVisual3DController` tolera modelo, ruedas o sondas opcionales ausentes y registra el resultado al iniciar.

Las dos `RayCast3D` consultan grupos `kerb`/`gravel` o metadata `surface_type`. En superficies no etiquetadas la vibración es cero. El GLB actual no separa ruedas, de modo que las rutas están vacías y la animación individual no se ejecuta.

## Importación del GLB

`game/assets/models/vehicles/v10/v10.glb` es una copia byte por byte de la fuente conservada en `references`. Godot genera su escena importada; el proyecto no la edita. `v10_visual.tscn` la envuelve y `player_car.tscn` instancia esa escena bajo `VehicleVisualRoot`.

## Cámara y presentación

`ArcadeChaseCamera` consume la misma pose interpolada. Altura mundial y pitch quedan bloqueados; `look_height` permite ajustar el centro vertical al modelo sin diving. La cámara mantiene el impulso Z sutil y el desplazamiento X por giro establecidos previamente.

`DirectionalVehicleSprite` ya no representa al jugador. Se conserva para el placeholder estático, validación y futura decoración 2.5D. El pipeline de sprites y sus pruebas permanecen activos.

## Otros sistemas

`EngineAudioController` sigue siendo hijo directo del coche bajo el nombre `AudioRoot`; así conserva acceso a RPM, marcha y throttle. `StaticMinimapController` dibuja una proyección fija y continúa usando la raíz física del jugador, no la malla visual.

La decisión se documenta en `docs/decisions/0002-3d-vehicle-visuals.md`.
