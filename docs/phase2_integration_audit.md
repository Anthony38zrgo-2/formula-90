# Auditoría de integración de la Fase 2

Fecha de corte: 2026-08-05. Checkpoint: `c5e0697`. Rama de evaluación: `refactor/vitavehicle-racing-camera-directional-sprite`.

## Línea base verificada

`scripts/test_windows.ps1` compiló la GDExtension debug, ejecutó 27 pruebas nativas, las pruebas de herramientas de sprites y WAV, validó assets sin errores ni avisos, importó el proyecto con Godot 4.7.1 y abrió en smoke `main_menu.tscn`, `test_field.tscn`, `player_car.tscn` y `debug_hud.tscn`. `scripts/test_audio_tools_windows.ps1` ejecutó una prueba Pytest y Ruff correctamente. Godot emitió cinco veces `Failed to read the root certificate store`; es un defecto preexistente del entorno Windows y cada proceso terminó con código 0. PowerShell requirió `-ExecutionPolicy Bypass` y el `.venv` requirió ejecución fuera del usuario aislado.

## Implementación actual

### Simulación del vehículo

`PlayerCar` es un `ArcadeCarController`, derivado de `CharacterBody3D`. Captura input y actualiza toda la simulación en `_physics_process`. Descompone `velocity` sobre los ejes forward/right del chasis, aplica motor, freno, rodadura y drag al componente longitudinal, rota el cuerpo con dirección dependiente de velocidad y recompone la velocidad antes de `move_and_slide()`.

El agarre lateral usa decaimiento exponencial independiente del framerate (`lateral_speed * exp(-grip * delta)`). El drift reduce gradualmente el agarre mediante `drift_factor`; una corrección de yaw proporcional a velocidad lateral aporta sobreviraje recuperable. No existe un modelo explícito de neumático, slip por rueda, subviraje, contravolante ni contacto de rueda. El contravolante emerge únicamente de la entrada de dirección y la velocidad lateral restante.

No hay `SurfaceDetector`, etiquetas de superficie ni multiplicadores de asfalto/grava/arena en runtime. `test_field.tscn` contiene un único plano con material de asfalto. Tampoco hay ruedas, raycasts, muelles, amortiguadores ni compresión de suspensión. Por tanto, el circuito actual no es una implementación verificable de La Chutana y las afirmaciones sobre grava, arena o suspensión pertenecen al objetivo, no al estado actual.

La transmisión automática/manual está contenida en `AutomaticTransmission`. La reversa requiere una parada estable; `ArcadeCarController` publica velocidad, RPM, marcha, input de dirección, aceleración mundial, velocidad lateral y estado. `CarPhysicsConfig` y `VehicleDefinition` agrupan los valores y recursos actuales.

### Cámara

`CameraRig` es `ArcadeChaseCamera`, hijo de `PlayerCar` con `top_level=true`. Se actualiza en `_process` con prioridad `-10`, después de que la física haya publicado velocidad y aceleración. Forma un objetivo con posición del coche, anticipación limitada por velocidad y un impulso transitorio limitado basado en aceleración real; aplica zonas muertas y suavizado exponencial independiente del framerate por eje. El offset total está limitado y la cámara interpola FOV de forma conservadora. No hay vibración deliberada.

Acoplamientos: la cámara convierte directamente a `ArcadeCarController`, espera un `Camera3D` llamado `Camera3D` y debe ser hija directa del coche. Cambiar el tipo de cuerpo o las rutas rompe el seguimiento.

### Sprite direccional

`DirectionalVehicleSprite` deriva de `Sprite3D`, carga `v10_sheet.json`, adopta el número y los ángulos reales de sus frames y usa filtro nearest sin espejado. En `_process`, prioridad `10`, calcula el ángulo horario entre el eje trasero físico del chasis y el vector horizontal hacia la cámara. La orientación física es primaria; la velocidad solo influye de forma secundaria y limitada para permitir derrape. Normaliza a `[0, 360)`, selecciona el frame angular más cercano, aplica histéresis y conserva la vista a muy baja velocidad.

Acoplamientos: convierte directamente al padre en `ArcadeCarController`, usa la cámara activa del viewport y espera el JSON y atlas actuales. La escena configura 16 direcciones, offset cero, histéresis de 3 grados y velocidad mínima de 0.5 m/s.

### Orden y consumidores

1. `ArcadeCarController::_physics_process`: input, transmisión, física y telemetría.
2. `ArcadeChaseCamera::_process` con prioridad `-10`: presentación de cámara.
3. `DirectionalVehicleSprite::_process` con prioridad `10`: selección visual.
4. HUD, minimapa y audio leen directamente el controlador en `_process`.

Las rutas sensibles son `PlayerCar`, `PlayerCar/CameraRig/Camera3D`, `PlayerCar/DirectionalVehicleSprite`, `../PlayerCar` desde el HUD y los recursos `default_car_physics.tres`, `v10_vehicle.tres`, `v10_engine_audio.tres`.

## Clasificación

| Componente | Clasificación | Motivo |
|---|---|---|
| `AutomaticTransmission`, audio V10, minimapa, reset, input y assets propios | Conservar sin cambios | No duplicados por las dependencias evaluadas. |
| Curvas de handling, parada estable para reversa, metadatos del atlas, histéresis y cámara contenida | Conservar como comportamiento Formula-90s | Definen el comportamiento ya validado. |
| `CarPhysicsConfig` y `VehicleDefinition` | Adaptar como configuración | Son el límite natural para futuras implementaciones. |
| Acceso directo de cámara/sprite/HUD al controlador | Adaptar detrás de un contrato neutral | Es el acoplamiento principal actual. |
| `ArcadeCarController`, `ArcadeChaseCamera`, `DirectionalVehicleSprite` | Mantener como backend Legacy | Son los únicos backends compatibles y probados en este corte. |
| Física de ruedas, suspensión y superficies | Ausente; no eliminar ni fingir sustitución | Debe implementarse o integrarse tras elegir una dependencia compatible. |

No se clasifica ningún sistema para eliminación: no existe reemplazo ejecutable y comparado.

## Evaluación de dependencias

### VitaVehicle

Se inspeccionó `jreo03/g-rcp2`, rama `beta`, commit `d60c8d90061b14a55e395132109bb16248e191c4`. Es un proyecto completo Godot 3 (`config_version=4`, GLES2, sintaxis `tool`, `Spatial`, `translation`, `PoolStringArray`) y no un addon autocontenido. Su simulación depende de autoload, scripts `car.gd`, `wheel.gd`, escenas, UI de tuning, input propio y una escala no métrica de 0.30592. El autor advierte inestabilidad con Bullet. Integrarlo en Godot 4.7.1 exige un port sustancial, reemplazar el cuerpo actual y validar miles de líneas GDScript. Esto contradice tanto la prohibición de migrar/portar como la regla local que exige runtime C++20. Estado: rechazado para esta rama; no se copian código, demos ni assets.

### Racing Cameras

Se inspeccionó tag `v0.3`, commit `eb2af2d3a7a07e8e09169bc093aba8942fbb5c4c`; `plugin.cfg` aún declara versión 0.2. Es Godot 4.2 y MIT. El addon registra el autoload `cameraman` e incluye cinco cámaras GDScript. `RacingChaseCamera` espera un `PhysicsBody3D` con `linear_velocity`, hace que la cámara mire principalmente hacia la dirección de velocidad y usa pesos `lerp` fijos por frame (0.1, 0.05, 0.03, etc.). No consume aceleración real ni ofrece las zonas muertas y límites exigidos. Activarlo incumpliría la política C++ del runtime; corregirlo requeriría un fork material del addon y cambiar su comportamiento central. Estado: técnicamente compatible con Godot 4.x, pero inadecuado para esta arquitectura y estos criterios; no se instala.

### DirectionalSprite3D

La fuente enlazada públicamente era `MeagherGames/addons/tree/main/addons/DirectionalSprite3D/addons/directional_sprite_3d`. El repositorio devuelve actualmente `Repository not found`; no hay release oficial, commit o licencia verificables disponibles. Una página de foro cacheada no constituye una fuente redistribuible. Estado: bloqueado por disponibilidad y licencia; no se copia ni se recrea su API.

## Arquitectura propuesta y decisión

La dirección válida sigue siendo `simulación física -> estado neutral -> pose visual/cámara/HUD`. Un futuro `VehicleDynamicsAdapter` C++ deberá publicar longitudinal/lateral velocity y acceleration, yaw rate, slip, contactos, compresión, superficie e inputs sin exponer tipos de terceros. Los controladores de cámara y sprite consumirán ese contrato; un enum por dominio habilitará exactamente un backend. Los backends nuevos solo deben añadirse cuando una dependencia sea compatible y se pueda ejecutar una escena aislada antes de tocar `player_car.tscn`.

En este corte no se añaden enums nominales ni adaptadores vacíos: aparentarían backends que no existen. Legacy permanece activo y no se modifica la escena. La alternativa segura es buscar una implementación Godot 4.7 con licencia verificable y API desacoplable, o autorizar explícitamente una implementación C++ propia para ruedas/superficies; ambas decisiones quedan fuera de esta auditoría.

## Plan de validación cuando exista un reemplazo

La escena aislada deberá verificar 0/90/180/270 grados, diagonales, ambos sentidos, 359→0, cámara rotatoria, baja velocidad, histéresis y sprites asimétricos. El circuito deberá adquirir primero superficies reales y pruebas de detección; después se compararán aceleración, frenada, curvas, drift, contravolante, salida/retorno de grava y arena, reinicio y variación de timestep. No se hará el commit de limpieza hasta que exista comparación instrumentada de los tres dominios.
