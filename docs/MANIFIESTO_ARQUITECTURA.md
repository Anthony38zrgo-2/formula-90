# Manifiesto de Arquitectura Técnica — Formula-90

Guía técnica de arquitectura, antipatrones detectados, estándares de código e instrucciones directas de refactor continuo al desarrollar features.

---

## 1. Pipeline de Ejecución Canónico (`run_jordan_handling.ps1`)

Todo cambio en el código debe mantener compatible y funcional el pipeline de validación:

```
run_jordan_handling.ps1
 ├── 1. Resuelve ejecutable Godot (v4.7.1)
 ├── 2. Valida SHA-256 de 3 GLBs contra vehicle_manifest.json (Chassis, Wheel Front, Wheel Rear)
 ├── 3. Valida assets de pista (la_chutana.glb, la_chutana_vegetation.glb)
 ├── 4. Verifica compilación de DLL GDExtension (libformula90s)
 ├── 5. Importa recursos (godot --headless --import) si los hashes .import difieren
 └── 6. Lanza la escena canónica: res://scenes/tracks/test_field/jordan_197_handling_test.tscn
```

### Escenas críticas que componen el runtime:
- `jordan_197_handling_test.tscn`: Escena raíz de pruebas.
- `la_chutana_generated.tscn`: Pista de prueba, mallas de colisión, cielo y vegetación.
- `jordan_197.tscn`: Vehículo físico y visual canónico (3 GLBs, 4 instancias de rueda).
- `debug_hud.tscn` / `world_hud_compositor.tscn`: Compositor de render 640x360 y capa de telemetría/UI.

---

## 2. Inventario de Clases y Antipatrones a Atacar

### 2.1 Módulo GEVP (Vendor — `game/addons/gevp/`)
> **Regla de oro:** Código de terceros congelado. **No modificar directamente**. Se encapsula y adapta desde la capa Formula90s.

| Archivo | Problema / Antipatrón | Acción Requerida |
|---|---|---|
| `vehicle.gd` (1094 líneas) | **God Class:** Mezcla dinámica de cuerpos rígidos, curvas de motor, transmisión, frenos ABS, tracción, aerodinámica y estabilizadores en un solo script monolítico. Requiere 4 ruedas independientes vinculadas para cálculo correcto de tracción y slip. | No editar. Aislar accesos mediante recursos de datos (`VehicleSpec`) y contratos tipados (`VehicleTunableContract`). Asignar referencias explícitas a las 4 ruedas en el inspector. |
| `wheel.gd` (348 líneas) | **Cálculo fuera de ciclo físico:** Procesa amortiguación, slip y fuerzas de neumático en `_process()` en vez de `_physics_process()`. | Monitorear variaciones de tasa de refresco; asegurar que el tick rate del proyecto se mantenga en 120 Hz. |
| `vehicle_controllergd.gd` | Controlador upstream básico con inputs directos. | Extender mediante `FormulaVehicleController`, no tocar base. |

---

### 2.2 Módulo Propio (`game/addons/formula90s/`)

| Archivo | Problema / Antipatrón | Acción Requerida |
|---|---|---|
| `driving_aids.gd` | **Mutación por reflexión:** Modifica ~12 variables internas del vehículo mediante `vehicle_node.set("propiedad", valor)` con cadenas de texto sin validar tipos ni rangos. | Reemplazar las llamadas `.set()` por métodos del contrato tipado `VehicleTunableContract.set_value()`. |
| `handling_tuning_panel.gd` | **UI en código y acoplamiento:** 250 líneas que construyen la UI con código procedural (`Button.new()`), manipula estructuras internas (`front_axle.brake_bias`) y contiene nombres hardcodeados ("JORDAN 191"). | Separar la UI a un `.tscn` dedicado. Leer y escribir ajustes a través del contrato tipado. |
| `telemetry_manager.gd` | **Búsqueda ciega y formato mixto:** Busca vehículos en el árbol cada segundo con `find_children("*", "Vehicle")`. Escribe archivos a `res://telemetry/` (debe ser `user://`). Incrusta un blob JSON completo dentro de cada línea del CSV. | Cambiar ruta a `user://telemetry/`. Separar el snapshot inicial a un archivo `.json` individual y dejar el `.csv` únicamente para series temporales. Registrar el vehículo mediante eventos en vez de polling. |
| `world_hud_compositor.gd` | **Acoplamiento por nombres:** Concatena rutas de nodos con cadenas (`"../../WorldViewport/WorldContent/" + suffix`) y usa comparaciones como `control.name == &"DebugHud"`. | Utilizar grupos (`add_to_group("hud_layer")`) o tipos de clase para el descubrimiento y reparenting de elementos UI. |
| `forest.gd` | **Complejidad $O(n^2)$ y rutas fijas:** Itera sobre todos los árboles previos para validar distancia (`for existing in _placed`). Mallas y materiales hardcodeados en strings dentro del código. | Reemplazar búsqueda lineal por Spatial Hashing / Grid 2D. Exponer mallas y materiales como `@export var` de tipo `Resource`. |
| `vehicle_path_resolver.gd` | **Rutas hardcodeadas:** Lista explícita de rutas candidatas (`Jordan197/VehicleRigidBody`, `Jordan191/...`). | Resolver buscando el nodo hijo estándar `VehicleRigidBody` o por tipo `Vehicle`. |

---

### 2.3 Pistas y Circuitos Procedurales (`game/scenes/tracks/`)

| Archivo | Problema / Antipatrón | Acción Requerida |
|---|---|---|
| `formula90s_test_track.gd` (304 líneas)<br>`la_chutana_track.gd` (387 líneas) | **Duplicación masiva de código CSG:** Ambos scripts implementan de forma independiente la creación de materiales, mallas CSG, curvas de borde, peraltes y cuerpos estáticos de colisión con un 80% de lógica idéntica. | Extraer la lógica común a una clase base `TrackBuilderBase`. Los scripts de pista solo deben declarar el trazado de puntos y peraltes. |

---

### 2.4 Escenas de Vehículos (`game/scenes/vehicles/`)

| Archivo | Problema / Antipatrón | Acción Requerida |
|---|---|---|
| `jordan_197.tscn`<br>`jordan_191.tscn`<br>`f1_1996_car.tscn` | **Parámetros de física hardcodeados en la escena:** Más de 90 propiedades numéricas (masas, relaciones de cambio, fricción por superficie, suspensión, aerodinámica) definidas dentro del archivo de escena `.tscn`. | Extraer todas las propiedades numéricas a archivos de recursos `.tres` (`VehicleSpec`, `EngineConfig`, etc.). La escena solo debe contener estructura de nodos. |
| `f1_1996_car.tscn` | **Jerarquía inconsistente:** El nodo raíz es directamente el `RigidBody3D`, carece del nodo padre `FormulaVehicleController`. | Estandarizar a la jerarquía canónica: Raíz `FormulaVehicleController` -> Hijo `VehicleRigidBody`. |
| `jordan_197.tscn`<br>`jordan_191.tscn` | **Contrato de 3 GLBs:** Estándar de 1 chasis + 1 rueda delantera compartida + 1 rueda trasera compartida. | Mantener cuatro nodos `Wheel` (FL, FR, RL, RR) y cuatro instancias visuales independientes. Compartir el recurso GLB no comparte el estado físico de GEVP. |

---

## 3. Arquitectura y Estándar Modular Objetivo

### 3.1 Separación Estricta: Escena vs Datos vs Lógica

```
┌─────────────────────────────────────────────────────────────┐
│                 ESCENA (.tscn)                              │
│  - Define exclusivamente la jerarquía de nodos.             │
│  - Instancia modelos 3D (3 GLBs) y colisionadores.          │
│  - No contiene números de tuning de física hardcodeados.   │
└──────────────────────────────┬──────────────────────────────┘
                               │ Referencia
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                 RECURSOS DE DATOS (.tres)                   │
│  - Archivos independientes versionables en Git.             │
│  - VehicleSpec (Masa, centro de gravedad, distribución)     │
│  - EngineConfig, GearboxConfig, TireCompound                │
│  - SuspensionPreset, AeroConfig, DifferentialConfig         │
└──────────────────────────────┬──────────────────────────────┘
                               │ Aplicado por
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                 ENSAMBLADOR / ADAPTADOR                     │
│  - VehicleAssembler: Lee el .tres e inicializa el Vehicle.  │
│  - VehicleTunableContract: Valida y aplica cambios en vivo. │
└─────────────────────────────────────────────────────────────┘
```

---

### 3.2 Jerarquía Canónica de un Vehículo (Contrato 3-GLB)

Todo vehículo en el proyecto debe respetar esta estructura de nodos con 4 ruedas independientes:

```text
NombreVehiculo (FormulaVehicleController)              <-- Raíz: Manejo de inputs
├── VehicleAssembler (Node)                           <-- Aplica VehicleSpec (.tres)
├── VehicleRigidBody (Vehicle - RigidBody3D)          <-- Física GEVP (Vendor)
│   ├── ChassisVisual (Node3D -> Instancia GLB)       <-- 1. Malla del chasis (chassis.glb)
│   ├── CollisionShape3D (CollisionTub, Nose, etc.)   <-- Colisiones del cuerpo
│   ├── WheelFrontLeft (RayCast3D / Wheel)            <-- Instancia delantera izquierda
│   │   └── Pivot -> Visual (wheel_front.glb)
│   ├── WheelFrontRight (RayCast3D / Wheel)           <-- Instancia delantera derecha
│   │   └── Pivot -> Orientation -> Visual (wheel_front.glb)
│   ├── WheelRearLeft (RayCast3D / Wheel)             <-- Instancia trasera izquierda
│   │   └── Pivot -> Visual (wheel_rear.glb)
│   ├── WheelRearRight (RayCast3D / Wheel)            <-- Instancia trasera derecha
│   │   └── Pivot -> Orientation -> Visual (wheel_rear.glb)
│   └── VehicleVisualContractGuard (Node)             <-- Validador de alineación
```

---

## 4. Instrucciones Directas de Refactor al Crear Features

Aplicar estas acciones técnicas cada vez que se trabaje en el área correspondiente:

### Al trabajar en Vehículos o Física:
1. **Crear / usar Resources tipados para parámetros:**
   - Definir recursos que extiendan `Resource`: `VehicleSpec`, `EngineConfig`, `TireCompound`, `SuspensionPreset`, `AeroConfig`, `GearboxConfig`, `DifferentialConfig`, `SteeringConfig`, `BrakeConfig`.
   - Cada recurso debe incluir su método `apply_to(vehicle: Vehicle)`.
2. **Desacoplar la escena `.tscn`:**
   - Asignar los valores numéricos dentro de archivos `.tres` bajo `data/vehicles/<nombre_vehiculo>/`.
   - Agregar un nodo `VehicleAssembler` en la escena que reciba el `VehicleSpec` y lo aplique en `_ready()`.
3. **Respetar el contrato 3-GLB:**
   - Usar 1 GLB para el chasis, 1 para las ruedas delanteras y 1 para las traseras (`chassis.glb`, `wheel_front.glb`, `wheel_rear.glb`).
   - Cada nodo `RayCast3D` debe conservar su propia instancia de escena y su propio `wheel_node`. GEVP calcula de forma independiente velocidad angular, suspensión y slip aunque dos instancias reutilicen el mismo recurso visual.
   - Usar cuatro GLB de rueda solamente cuando exista asimetría geométrica o visual real, documentada en el manifiesto.

---

### Al trabajar en Asistencias a la Conducción o Inputs:
1. **Eliminar el uso directo de `.set()` por reflexión:**
   - Centralizar todas las modificaciones dinámicas en `VehicleTunableContract.set_value(vehicle, property_name, value)`.
   - Registrar en `VehicleTunableContract` los límites máximos y mínimos (`clamp`) de cada propiedad para evitar inestabilidades numéricas en el integrador físico.
2. **Preservar el estado base (`baseline`):**
   - Antes de aplicar asistencias, leer los valores predeterminados directamente del `VehicleSpec` asignado y no de variables sueltas en runtime.

---

### Al trabajar en Pistas o Elementos de Entorno:
1. **Heredar de `TrackBuilderBase`:**
   - Al crear o modificar circuitos generados por código, extender de `TrackBuilderBase`.
   - No duplicar generación de mallas CSG, materiales ni perfiles de bordillos; sobreescribir únicamente las funciones de datos: `_get_centerline()`, `_get_bank_degrees()`, `_get_curb_sections()`.
2. **Optimizar dispersión de objetos en `forest.gd`:**
   - Sustituir la comprobación lineal de distancias por un registro espacial en celdas (Grid 2D).
   - Declarar las mallas y materiales con `@export_file` o `@export var mesh: Mesh` en lugar de rutas fijas con `load("res://...")`.

---

### Al trabajar en Telemetría, Sesión o UI:
1. **Corregir destinos de archivo:**
   - Todo archivo generado en tiempo de ejecución (logs, CSVs, dumps de telemetría) debe escribirse en `user://telemetry/`, nunca dentro del árbol de fuentes `res://`.
2. **Estructurar la salida de telemetría:**
   - Al iniciar sesión, guardar la configuración del vehículo en un archivo estático: `user://telemetry/<session_id>_setup.json`.
   - Guardar las métricas de cuadro (velocidad, RPM, slip, fuerzas G) en `user://telemetry/<session_id>_run.csv` sin incrustar el JSON de configuración en cada fila.
3. **Eliminar búsquedas recursivas en bucle:**
   - Los vehículos deben registrarse ante el `TelemetryManager` al instanciarse (`TelemetryManager.register_vehicle(self)`), eliminando llamadas a `find_children()` en `_physics_process()`.
4. **Desacoplar la UI de nombres de nodos:**
   - Usar grupos de nodos (`"minimap_target"`, `"telemetry_provider"`) en lugar de buscar nodos por nombres literales como `"DebugHud"` o `"Jordan197"`.

---

### Limpieza de Archivos y Escenas Huérfanas:
1. Mover escenas de prueba antiguas que no formen parte del pipeline a `scenes/vehicles/_archive/`.
2. Eliminar archivos comprimidos (`.zip`, `.rar`) ubicados dentro de la carpeta `scenes/`.
3. Mantener los scripts de prueba organizados en `game/tests/` bajo sus carpetas correspondientes (`smoke/`, `integration/`, `visual/`, `probes/`).

---

## 5. Comandos de Validación Inmediata

Ejecutar estas pruebas después de cualquier modificación para verificar que no existan regresiones:

```powershell
# 1. Validación rápida de estructura, assets y GDExtension
.\scripts\run_jordan_handling.cmd -ValidateRuntimeOnly

# 2. Validación de escena de handling y contrato visual
.\.tools\godot\Godot_v4.7.1-stable_win64.exe --headless --path game --script res://tests/smoke_test_jordan_197_handling_scene.gd

# 3. Validación de ejecución completa del vehículo canónico
.\scripts\run_jordan_handling.cmd
```
