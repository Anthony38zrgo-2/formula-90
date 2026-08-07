# Guía Detallada de Parámetros de Física del Vehículo (GEVP)

Este documento explica en detalle cada uno de los parámetros que configuran el comportamiento dinámico, la física, el motor, la transmisión, la suspensión y la aerodinámica del sistema **Godot Easy Vehicle Physics (GEVP)** en Godot 4.

---

## 📑 Tabla de Contenidos
1. [Principios de Arquitectura Física](#1-principios-de-arquitectura-física)
2. [Dirección y Asistencia (Steering)](#2-dirección-y-asistencia-steering)
3. [Acelerador, Frenos y ABS/TCS (Throttle & Braking)](#3-acelerador-frenos-y-abstcs-throttle--braking)
4. [Motor, Embrague y Transmisión (Motor & Gearbox)](#4-motor-embrague-y-transmisión-motor--gearbox)
5. [Tren Motriz y Diferenciales (Drivetrain)](#5-tren-motriz-y-diferenciales-drivetrain)
6. [Masa, Masa Inercial y Estabilidad (Chassis & Stability)](#6-masa-masa-inercial-y-estabilidad-chassis--stability)
7. [Suspensión y Geometría de Ejes (Suspension & Axles)](#7-suspensión-y-geometría-de-ejes-suspension--axles)
8. [Neumáticos y Superficies (Tires & Surfaces)](#8-neumáticos-y-superficies-tires--surfaces)
9. [Aerodinámica (Aerodynamics)](#9-aerodinámica-aerodynamics)
10. [Recomendaciones de Tuning (Arcade vs Simulación)](#10-recomendaciones-de-tuning-arcade-vs-simulación)

---

## 1. Principios de Arquitectura Física

GEVP utiliza un modelo de suspensión basado en **RayCast3D**:
- La raíz del vehículo es un `RigidBody3D`.
- Cada rueda es un nodo de tipo `Wheel` que hereda de `RayCast3D`.
- El RayCast3D proyecta verticalmente hacia abajo la longitud máxima de la suspensión más el radio del neumático.
- Las fuerzas de resorte, amortiguamiento, fricción lateral y fricción longitudinal son calculadas por las ruedas y aplicadas directamente sobre la masa del `RigidBody3D` utilizando `apply_force()`.

---

## 2. Dirección y Asistencia (Steering)

Controla cómo responde la dirección a la entrada del jugador y aplica asistencias para prevenir sobreviraje incontrolable.

| Parámetro | Tipo | Unidad | Descripción |
|---|---|---|---|
| `steering_speed` | `float` | rad/s | Velocidad con la que la rueda gira hacia la dirección solicitada. Valores altos dan respuesta rápida/arcade; valores bajos suavizan la dirección. |
| `countersteer_speed` | `float` | rad/s | Velocidad a la que el volante retorna al centro al soltar la dirección. |
| `steering_speed_decay` | `float` | - | Reduce la sensibilidad de la dirección según la velocidad del coche. Evita que a 200 km/h un girada brusca vuelva inestable al auto. |
| `steering_slip_assist` | `float` | ratio | Si el deslice lateral de las ruedas supera este umbral, restringe mayor entrada de dirección para prevenir que el neumático colapse en fricción. |
| `countersteer_assist` | `float` | factor | Corrección automática de contravolante (drifting assist). Ajusta el ángulo hacia la dirección real de avance cuando el vehículo derrapa de lado. |
| `steering_exponent` | `float` | - | Exponente no lineal aplicado al input. Un valor > 1.0 da alta precisión cerca del centro y giros más rápidos al llegar al tope. |
| `max_steering_angle` | `float` | rad (deg inspector) | Ángulo máximo de giro de las ruedas delanteras (típicamente entre `30°` y `45°`). |
| `front_steering_ratio` | `float` | ratio (0 a 1) | Proporción del ángulo máximo aplicado al eje delantero. |
| `rear_steering_ratio` | `float` | ratio (0 a 1) | Dirección en las ruedas traseras (four-wheel steering). Si es > 0, permite dirección trasera (útil para montacargas o giros cerrados). |

---

## 3. Acelerador, Frenos y ABS/TCS (Throttle & Braking)

Gestión del reparto de aceleración, frenado y sistemas electrónicos de asistencia.

| Parámetro | Tipo | Unidad | Descripción |
|---|---|---|---|
| `throttle_speed` | `float` | /s | Suavizado del pedal de acelerador. Evita picos bruscos de torque al acelerar de 0 a 1. |
| `throttle_steering_adjust` | `float` | - | Reduce la respuesta del acelerador cuando se gira el volante, ayudando a traccionar mejor en curva. |
| `braking_speed` | `float` | /s | Suavizado de la entrada de freno. |
| `brake_force_multiplier` | `float` | multiplicador | Multiplicador general sobre la fuerza total de frenado calculada. |
| `front_brake_bias` | `float` | ratio (0.0 - 1.0) | Reparto de frenada hacia el eje delantero (bias). Si se establece en `< 0.0`, el sistema calcula automáticamente el sesgo óptimo según la transferencia de peso. |
| `traction_control_max_slip` | `float` | rad/s | **Control de Tracción (TCS)**: Deslice máximo permitido antes de cortar torque del motor. Un valor `< 0` desactiva el TCS. |
| `front_abs_pulse_time` / `rear_abs_pulse_time` | `float` | s | Tiempo que el ABS libera la presión del freno cuando detecta bloqueo. |
| `front_abs_spin_difference_threshold` / `rear_abs_spin_difference_threshold` | `float` | rad/s | Diferencia de velocidad entre el neumático y el suelo requerida para activar el **ABS**. |

---

## 4. Motor, Embrague y Transmisión (Motor & Gearbox)

Define la entrega de potencia, curvas de torque y la caja de velocidades.

| Parámetro | Tipo | Unidad | Descripción |
|---|---|---|---|
| `max_torque` | `float` | N·m | Torque pico del motor. |
| `max_rpm` | `float` | RPM | RPM máximas antes del corte/limitador. |
| `idle_rpm` | `float` | RPM | RPM en marcha mínima/ralentí. |
| `torque_curve` | `Curve` | norma 0..1 | Curva de torque en función de las RPM. El eje X representa RPM de `idle_rpm` a `max_rpm` y el eje Y representa la fracción de `max_torque`. |
| `motor_drag` | `float` | N·m / RPM | Freno motor proporcional a las RPM. |
| `motor_brake` | `float` | N·m | Freno motor constante al soltar el acelerador. |
| `motor_moment` | `float` | kg·m² | Momento de inercia del cigüeñal/volante de inercia. Determina qué tan rápido suben o bajan las RPM sin carga. |
| `clutch_out_rpm` | `float` | RPM | RPM a las que el embrague acopla completamente al arrancar desde cero. |
| `max_clutch_torque_ratio` | `float` | ratio | Límite de torque que puede soportar el embrague antes de patinar. |
| `gear_ratios` | `Array[float]` | rel. | Relaciones de marcha hacia adelante (1ª, 2ª, 3ª...). Ej: `[3.8, 2.3, 1.7, 1.3, 1.0, 0.8]`. |
| `final_drive` | `float` | ratio | Relación del diferencial de reducción final (diferencial de corona/piñón). |
| `reverse_ratio` | `float` | ratio | Relación de la marcha atrás (Reversa). |
| `shift_time` | `float` | s | Duración de la interrupción de potencia durante el cambio de marcha. |
| `automatic_transmission` | `bool` | - | `true` para transmisión automática, `false` para manual. |
| `automatic_time_between_shifts` | `float` | ms | Intervalo mínimo entre cambios automáticos para evitar brincos repetitivos de marcha. |
| `gear_inertia` | `float` | kg·m² | Inercia interna de los engranajes de la caja de cambios. |

---

## 5. Tren Motriz y Diferenciales (Drivetrain)

Distribución de tracción entre ruedas y ejes.

| Parámetro | Tipo | Unidad | Descripción |
|---|---|---|---|
| `front_torque_split` | `float` | ratio (0.0 - 1.0) | Reparto estático de tracción: `0.0` = Tracción Trasera (RWD), `1.0` = Tracción Delantera (FWD), `0.5` = Tracción Total (AWD 50/50). |
| `variable_torque_split` | `bool` | - | Activa reparto variable de tracción dinámico según la velocidad del vehículo. |
| `front_variable_split` | `float` | ratio | Reparto inicial a baja velocidad cuando el split variable está activo. |
| `variable_split_speed` | `float` | m/s | Velocidad a la cual se completa la transición del reparto variable. |
| `front_locking_differential_engage_torque` / `rear_locking_differential_engage_torque` | `float` | N·m | Torque umbral para bloquear el diferencial autoblocante (LSD) en cada eje. |
| `front_torque_vectoring` / `rear_torque_vectoring` | `float` | factor | Vectorización de torque: redistribuye más potencia a la rueda exterior en giro para reducir subviraje. |

---

## 6. Masa, Masa Inercial y Estabilidad (Chassis & Stability)

Parámetros del cuerpo rígido del auto y control de estabilidad aerodinámica/vuelco.

| Parámetro | Tipo | Unidad | Descripción |
|---|---|---|---|
| `vehicle_mass` | `float` | kg | Masa total del chasis. |
| `front_weight_distribution` | `float` | ratio (0.0 - 1.0) | Distribución estática de peso (`0.5` = 50% adelante / 50% atrás, `0.6` = 60% adelante). |
| `center_of_gravity_height_offset` | `float` | m | Desplazamiento vertical del centro de gravedad (CdG). Valores negativos bajan el CdG aumentando la estabilidad en curva. |
| `inertia_multiplier` | `float` | factor | Escala el tensor de inercia del `RigidBody3D` para simular la distribución de componentes pesados (motor, transmisión). |
| `enable_stability` | `bool` | - | Activa el control electrónico de estabilidad (ESP) por software. |
| `stability_yaw_engage_angle` | `float` | ratio (dot) | Ángulo de guiñada a partir del cual el ESP aplica torque de corrección. |
| `stability_yaw_strength` | `float` | factor | Fuerza de corrección de guiñada cuando el vehículo derrapa excesivamente. |
| `stability_yaw_ground_multiplier` | `float` | multiplicador | Incrementador de corrección de guiñada en contacto con el suelo para superar la fricción del neumático. |
| `stability_upright_spring` / `stability_upright_damping` | `float` | - | Torques de auto-nivelación en el aire cuando el vehículo está en un salto. |

---

## 7. Suspensión y Geometría de Ejes (Suspension & Axles)

Configuración separada por eje (`front_*` y `rear_*`).

| Parámetro | Tipo | Unidad | Descripción |
|---|---|---|---|
| `front_spring_length` / `rear_spring_length` | `float` | m | Recorrido libre total del resorte de suspensión. |
| `front_resting_ratio` / `rear_resting_ratio` | `float` | ratio (0..1) | Altura de reposo del resorte bajo el peso estático. `0.5` significa que en reposo la suspensión está comprimida al 50%. |
| `front_damping_ratio` / `rear_damping_ratio` | `float` | ratio | Ratio de amortiguamiento (`1.0` = amortiguamiento crítico; autos de carrera usan `0.6` a `0.9`). |
| `front_bump_damp_multiplier` / `rear_bump_damp_multiplier` | `float` | factor | Multiplicador de compresión (Bump). Afecta la dureza al baches directos. |
| `front_rebound_damp_multiplier` / `rear_rebound_damp_multiplier` | `float` | factor | Multiplicador de rebote (Rebound). Controla qué tan rápido se extiende la suspensión tras comprimirse. |
| `front_arb_ratio` / `rear_arb_ratio` | `float` | ratio | Rigidez de la barra estabilizadora (Anti-Roll Bar). Reduce la inclinación lateral (roll) del chasis en curvas. |
| `front_camber` / `rear_camber` | `float` | rad | Caída (camber) de las ruedas para estabilidad del RayCast3D. |
| `front_toe` / `rear_toe` | `float` | rad | Convergencia/Divergencia (toe-in / toe-out) de las ruedas. |
| `front_bump_stop_multiplier` / `rear_bump_stop_multiplier` | `float` | multiplicador | Rigidez extra aplicada cuando la suspensión llega al tope de compresión (bump stop). |
| `front_beam_axle` / `rear_beam_axle` | `bool` | - | Simula la inclinación geométrica visual de un eje rígido continuo. |

---

## 8. Neumáticos y Superficies (Tires & Surfaces)

Control de adherencia y fricción por terreno.

| Parámetro | Tipo | Unidad | Descripción |
|---|---|---|---|
| `contact_patch` | `float` | m | Tamaño estimado del área de contacto del neumático con el suelo. |
| `braking_grip_multiplier` | `float` | multiplicador | Incrementador de tracción longitudinal disponible exclusivamente durante frenadas. |
| `wheel_to_body_torque_multiplier` | `float` | multiplicador | Transferencia de torque del frenado hacia la carrocería (inclinación longitudinal al frenar). |
| `tire_stiffnesses` | `Dictionary` | N/rad | Rigidez lateral del neumático según superficie (`"Road"`, `"Dirt"`, `"Grass"`). |
| `coefficient_of_friction` | `Dictionary` | CoF | Coeficiente de agarre (Mu) por terreno (`"Road": 2.0`, `"Grass": 1.0`). |
| `rolling_resistance` | `Dictionary` | N | Resistencia a la rodadura por terreno. |
| `lateral_grip_assist` | `Dictionary` | factor | Asistencia adicional de adherencia lateral por superficie. |
| `longitudinal_grip_ratio` | `Dictionary` | ratio | Relación de tracción longitudinal vs lateral por terreno. |
| `front_tire_radius` / `rear_tire_radius` | `float` | m | Radio físico del neumático en metros. |
| `front_tire_width` / `rear_tire_width` | `float` | mm | Ancho del neumático. |
| `front_wheel_mass` / `rear_wheel_mass` | `float` | kg | Masa individual de cada conjunto rueda/llanta. |

---

## 9. Aerodinámica (Aerodynamics)

Calcula la resistencia del aire y carga aerodinámica pasiva según la velocidad.

| Parámetro | Tipo | Unidad | Descripción |
|---|---|---|---|
| `coefficient_of_drag` (Cd) | `float` | - | Coeficiente de arrastre aerodinámico. Un F1 ronda `0.7` a `1.1` (por alerones), un deportivo de calle `0.3`. |
| `air_density` | `float` | kg/m³ | Densidad del aire (estándar a nivel del mar: `1.225`). |
| `frontal_area` | `float` | m² | Área frontal expuesta del chasis en metros cuadrados. |

---

## 10. Recomendaciones de Tuning (Arcade vs Simulación)

### Configuración F1 / Arcade Racing (Grip Alto, Respuesta Directa)
- `steering_speed`: `8.0` a `12.0`
- `max_steering_angle`: `deg_to_rad(30.0)`
- `front_weight_distribution`: `0.45` (45% adelante, 55% atrás)
- `center_of_gravity_height_offset`: `-0.35`
- `front_spring_length`: `0.10`, `rear_spring_length`: `0.12`
- `front_damping_ratio`: `0.8`, `rear_damping_ratio`: `0.8`
- `coefficient_of_friction`: `{"Road": 3.5, ...}`
- `enable_stability`: `true`, `stability_yaw_strength`: `8.0`

### Configuración Drift / Simcade (Derrape Controlable)
- `front_torque_split`: `0.0` (Tracción trasera RWD)
- `steering_speed`: `6.0`, `countersteer_speed`: `14.0`
- `countersteer_assist`: `1.2`
- `traction_control_max_slip`: `-1.0` (Desactivado)
- `rear_arb_ratio`: `0.5` (Barra estabilizadora trasera rígida)
- `coefficient_of_friction`: `{"Road": 1.8, ...}`
