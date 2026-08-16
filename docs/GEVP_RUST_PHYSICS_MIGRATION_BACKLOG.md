# Backlog: Migración de Físicas GEVP a Rust (`vehicle_physics_engine`)

**Estado:** `PLANNED`  
**Owner:** `vehicle-physics` / `core-engine`  
**Patrón Arquitectónico:** Módulo puro en Rust sin dependencias de Godot en el core (idéntico a `vehicle_audio_engine` y `skybox_engine`), desacoplado, determinista y testeable offline.

---

## 1. Motivación y Objetivos

1. **Determinismo y Rendimiento**:
   - Reemplazar la ejecución de físicas en GDScript de GEVP (`vehicle.gd` y `wheel.gd`) por un núcleo en Rust puro de alto rendimiento y ejecución matemática determinista.
2. **Modelo de Suspensión Tri-Raycast (3 Raycasts por Rueda)**:
   - Superar la limitación de 1 solo raycast central (ceguera de bordillos / *raycast curb blindness* y picos de fuerza anómalos).
   - Cada rueda utiliza 3 raycasts transversales/longitudinales para capturar la inclinación real del asfalto, bordillos, pianos y superficies mixtas con transiciones suaves de carga.
3. **Telemetría Idéntica y Verificación de Paridad**:
   - Generación directa en Rust del mismo esquema CSV de 25 columnas utilizado por `TelemetryManager.gd` para contrastar punto por punto la dinámica de Godot vs Rust.
4. **Pruebas Deterministas Offline (`cargo test`)**:
   - Poder simular y verificar aceleración en recta, frenada, paso por curva (skidpad), balance aerodinámico y absorción de bordillos sin necesidad de abrir ni compilar la interfaz de Godot.

---

## 2. Flujo Matemático de las Físicas (GEVP Rust Core)

```text
Inputs (Steer, Throttle, Brake, Gear)
  │
  ▼
[1. Input Filtering & Driver Aids] (Steering speed decay, countersteer assist)
  │
  ▼
[2. Powertrain & Transmission] (Torque curve, RPM inertia, Clutch, Gearbox, Differential)
  │
  ▼
[3. Aerodynamics] (Front/Rear Downforce ~ v², Drag ~ v², Aero balance)
  │
  ▼
[4. 3-Raycast Suspension & Geometry] (3 raycasts/wheel -> Camber, Compression, Dampers, ARB, Bumpstops)
  │
  ▼
[5. Tire Friction & Dynamics] (Slip angle α, Slip ratio κ, Brush model, Friction ellipse, Multi-surface)
  │
  ▼
[6. 6-DOF Rigid Body Integration] (Linear/Angular Accel, Velocity, Quaternions, Weight transfer)
  │
  ▼
[7. Telemetry Snapshot Generator] (25-column CSV output, state export, Godot visual sync)
```

---

## 3. Backlog de Tareas y Criterios de Aceptación

| ID | Pri. | Dependencia | Estado | Entregable y Criterio de Aceptación |
|---|---|---|---|---|
| `PHY-001` | P0 | - | `DONE` | **Setup del Crate Rust `game/physics/engine`**: Estructura `Cargo.toml`, lib determinista sin dependencias de Godot, tipos matemáticos vectoriales/matriciales 3D (`types.rs`). |
| `PHY-002` | P0 | `PHY-001` | `DONE` | **Configuración del Vehículo F1-94 y Serialización JSON**: Definición de `VehicleConfig` con parámetros oficiales de `data/vehicles/f1_94/` (F1 1994 V10: 17,000 RPM, 340 N·m, masa 505 kg, FD 6.30, relaciones 6V), centro de gravedad, curvas de torque y serialización `serde`. |
| `PHY-003` | P0 | `PHY-002` | `DONE` | **Núcleo de Powertrain y Transmisión**: Simulación determinista de RPM, inercia de motor, embrague, marchas (1..6 + R), diferencial abierto/LSD, y corte de inyección / limitador de RPM (`powertrain.rs`). |
| `PHY-004` | P0 | `PHY-002` | `DONE` | **Modelo de Suspensión Tri-Raycast (3 Raycasts por Rueda)**: Implementar evaluación de 3 puntos de contacto por rueda (Interior, Centro, Exterior). Integración ponderada de compresión, velocidad de suspensión, amortiguación lenta/rápida, barra estabilizadora (ARB) y bump-stops progresivos (`suspension.rs`). |
| `PHY-005` | P0 | `PHY-004` | `DONE` | **Dinámica de Neumáticos y Fricción Multi-Superficie**: Cálculo de ángulo de deriva ($\alpha$), ratio de deslizamiento ($\kappa$), modelo de cepillo (*brush tire model*), elipse de adherencia combinada, resistencia a la rodadura y mezcla de coeficientes de fricción ($\mu$) según los 3 impactos de raycast (`tire.rs`). |
| `PHY-006` | P0 | `PHY-003, PHY-005` | `DONE` | **Aerodinámica e Integrador 6-DOF**: Downforce y drag cuadráticos con velocidad ($v^2$), balance aerodinámico delantero/trasero, integración numérica determinista (Semi-Implicit Euler / RK4) de fuerzas y torques sobre el cuerpo rígido del chasis (`aero.rs`, `simulation.rs`). |
| `PHY-007` | P1 | `PHY-006` | `DONE` | **Generador de Telemetría y CLI Comparator**: Emisión de telemetría en Rust con las 25 columnas de `TelemetryManager` (`Time_ms`, `Speed_kmh`, `RPM`, `Gear`, `Lat_G`, `Long_G`, `FL_Comp`, etc.) y herramienta CLI `telemetry_compare` para contraste automático contra capturas de Godot (`telemetry.rs`, `src/bin/telemetry_compare.rs`). |
| `PHY-008` | P1 | `PHY-006` | `DONE` | **Suite de Pruebas Deterministas Offline (`cargo test`)**: Tests unitarios y de integración sin Godot: aceleración en recta ($0 \to 100$, $0 \to 200\text{ km/h}$), frenada con ABS ($200 \to 0\text{ km/h}$), y respuesta ante escalón de bordillo (3-raycast curb strike) (`tests/`). |
| `PHY-009` | P1 | `PHY-006` | `PLANNED` | **Adaptador GDExtension / Godot Binding para F1-94**: Enlazar el crate Rust con Godot 4 mediante GDExtension, actualizando la escena `res://scenes/vehicles/f1_94/f1_94.tscn` para ejecutar la simulación en `_physics_process` y posicionar los nodos visuales de rueda (`F1_94_wheel_front_geometry.glb`, `F1_94_wheel_rear_geometry.glb`). |
| `PHY-010` | P2 | `PHY-008, PHY-009` | `PLANNED` | **Validación de Paridad y Congelación de F1-94**: Pruebas en pista (La Chutana), verificación de tiempos de vuelta, estabilidad sobre pianos/bordillos y certificación con telemetría golden antes de deprecar GEVP GDScript. |

---

## 4. Invariantes y Reglas de Paridad

1. **Pureza del Núcleo Rust**: El crate `vehicle_physics_engine` no debe importar `godot-rust` ni headers del motor en sus módulos base (`powertrain`, `suspension`, `tire`, `simulation`).
2. **Determinismo Numérico**: A igual secuencia de inputs temporales y perfiles de terreno, el resultado de velocidad, RPM, compresión y posición debe ser idéntico en cualquier plataforma (bit-exact o tolerancia $< 10^{-5}$).
3. **Compatibilidad de Telemetría**: El formato de exportación CSV debe respetar exactamente el orden, encabezados y unidades de `TelemetryManager.gd`.
4. **Comportamiento Tri-Raycast en Bordillos**: Al golpear un piano a $150\text{ km/h}$, la rueda no debe sufrir picos instantáneos de $G_{\text{lat}}$ superiores a $3.0\text{ G}$ no explicados por la suspensión.
