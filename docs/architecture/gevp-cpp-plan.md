# Plan de Arquitectura Híbrida: GEVP (GDScript) + Módulos C++20

## 1. Resumen de la Redefinición
Se establece una división clara de responsabilidades entre el complemento **GEVP (GDScript)** para la simulación física del vehículo y la extensión **GDExtension (C++20)** para los módulos de presentación de alto rendimiento, cámara arcade, DSP de audio y telemetría/UI.

Se **elimina definitivamente** la dependencia y desarrollo del pipeline de **Sprites 2.5D** (`DirectionalVehicleSprite`).

---

## 2. Reparto de Responsabilidades

### A. Físicas y Control de Vehículo (GDScript / Addon GEVP)
- **`RigidBody3D` & `vehicle.gd`**: Simulación dinámica de suspensión por RayCast3D, masa, inercia, aceleración y frenado.
- **`wheel.gd`**: Cálculo de adherencia longitudinal/lateral y fuerzas por rueda.
- **`vehicle_controller.gd`**: Recepción de eventos de entrada y control de tracción/diferenciales.

### B. Módulos Exclusivos en C++20 (GDExtension)
- **Cámara Arcade 3D (`ArcadeChaseCamera`)**:
  - Seguimiento dinámico de cámara arcade con anticipación de velocidad, `look_ahead`, elevación Z e inercia suave en curvas.
- **Síntesis y Procesamiento de Audio (`EngineAudioController` / `EngineAudioConfig`)**:
  - Reproducción DSP nativa del motor V10 con crossfade dinámico de capas mono PCM16 a 44.1 kHz según RPM y carga, libre de latencia.
- **Interpolación de Presentación 3D (`VehicleVisual3DController` / `VehicleVisual3DConfig`)**:
  - Desacoplamiento de FPS de física y renderizado usando `get_physics_interpolation_fraction()`.
  - Animación de rotación de ruedas, respuesta visual de inclinación (*roll* y *pitch*) por aceleración/fuerza G y vibraciones por tipo de terreno (*kerb* / *gravel*).
- **Proyección de Minimapa y UI de Rendimiento (`StaticMinimapController` / `DebugHudController`)**:
  - Cálculo nativo de proyecciones 3D → 2D para el minimapa en tiempo real e interfaz de telemetría y rendimiento.

---

## 3. Estado de los Componentes y Cambios Planeados

### Tabla de Ajustes por Componente

| Componente | Lenguaje | Acción | Estado |
|---|---|---|---|
| **Física ArcadeCarController** | C++ | **Migrar a GEVP (GDScript)** | Desactivar / Retirar como física activa |
| **Física de Neumáticos y Chasis** | GDScript | **Asignar a GEVP** | Activo vía `res://addons/gevp/` |
| **Cámara Chase Arcade** | C++ | **Conservar C++ (`ArcadeChaseCamera`)** | Activo y vinculado a `ArcadeChaseCamera` |
| **Audio DSP Motor V10** | C++ | **Conservar C++ (`EngineAudioController`)** | Activo y vinculado a `AudioRoot` |
| **Interpolación Visual 3D** | C++ | **Conservar C++ (`VehicleVisual3DController`)** | Activo y vinculado a `VehicleVisualRoot` |
| **Minimapa y Telemetría UI** | C++ | **Conservar C++ (`StaticMinimapController`)** | Activo en el HUD |
| **Sprites 2.5D (`DirectionalVehicleSprite`)** | C++ | **ELIMINAR / DESACTIVAR** | Retirado de la arquitectura |

---

## 4. Estructura de la Escena Principal (`f1_2026_car.tscn`)

```text
F1_2026_Vehicle (RigidBody3D - script GEVP vehicle.gd)
├─ CollisionShape3D (BoxShape3D)
├─ WheelFrontLeft (RayCast3D - script GEVP wheel.gd)
├─ WheelFrontRight (RayCast3D - script GEVP wheel.gd)
├─ WheelRearLeft (RayCast3D - script GEVP wheel.gd)
├─ WheelRearRight (RayCast3D - script GEVP wheel.gd)
├─ VehicleVisualRoot (VehicleVisual3DController C++)
│  └─ F1_2026Model (Instancia f1_2026_visual.tscn)
├─ SurfaceProbes (FrontProbe / RearProbe)
├─ CameraRig (ArcadeChaseCamera C++)
│  └─ Camera3D
├─ AudioRoot (EngineAudioController C++)
└─ ResetMarker
```

---

## 5. Pasos de Ejecución

1. **Actualizar documentación de arquitectura**: Reflejar el diseño híbrido en `docs/architecture/runtime-map.md`.
2. **Reconfigurar escena del vehículo**: Adaptar `f1_2026_car.tscn` para integrar las ruedas raycast de GEVP con los sub-nodos C++ (`VehicleVisualRoot`, `CameraRig`, `AudioRoot`).
3. **Mapeo de Entradas GEVP**: Mapear el `InputMap` de Godot para alimentar los nombres de acciones requeridos por GEVP sin romper la cámara o el audio C++.
4. **Verificación de Compilación y Prueba**: Asegurar que SCons compila la extensión C++ limpia sin dependencias de `DirectionalVehicleSprite`.
