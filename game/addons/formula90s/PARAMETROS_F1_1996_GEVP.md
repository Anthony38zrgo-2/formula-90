# Parámetros GEVP — Fórmula 1 de 1996 (Referencia Realista)

Set de parámetros para **Godot Easy Vehicle Physics (GEVP)** ajustado para replicar el comportamiento de un monoplaza de F1 de la temporada 1996 (ej. Williams FW18 / Ferrari F310), basado en la estructura del documento GEVP original.

> ⚠️ Nota histórica clave: en **1994 se prohibieron el Control de Tracción (TCS), el ABS y la suspensión activa** tras el accidente de Senna en Imola. Esa prohibición seguía vigente en 1996 (el TC regresó recién en 2001). Por eso, en este set, TCS y ABS aparecen **desactivados**, y no hay ayudas electrónicas de estabilidad (ESP), tal como en la realidad.

---

## 📑 Tabla de Contenidos
1. [Dirección](#1-dirección-steering)
2. [Acelerador y Frenos](#2-acelerador-y-frenos-throttle--braking)
3. [Motor, Embrague y Transmisión](#3-motor-embrague-y-transmisión)
4. [Tren Motriz y Diferenciales](#4-tren-motriz-y-diferenciales)
5. [Masa y Estabilidad](#5-masa-y-estabilidad-chassis)
6. [Suspensión y Geometría](#6-suspensión-y-geometría)
7. [Neumáticos](#7-neumáticos)
8. [Aerodinámica](#8-aerodinámica)
9. [Variantes de Configuración por Circuito](#9-variantes-de-configuración-por-circuito)

---

## 1. Dirección (Steering)

Dirección de piñón-cremallera muy directa, sin asistencia electrónica de corrección (no existía countersteer assist en un F1 real, pero se deja en 0 para no "ayudar" artificialmente).

| Parámetro | Valor | Justificación |
|---|---|---|
| `steering_speed` | `14.0` | Dirección extremadamente directa y rápida; el piloto mueve el volante muy poco para girar mucho. |
| `countersteer_speed` | `10.0` | Retorno rápido al centro, propio de una cremallera de relación muy corta. |
| `steering_speed_decay` | `0.85` | A alta velocidad (250+ km/h) el input debe reducirse fuertemente; un F1 no tolera giros bruscos a esas velocidades. |
| `steering_slip_assist` | `0.0` | Sin asistencia: el piloto (jugador) es responsable total del control del deslizamiento. |
| `countersteer_assist` | `0.0` | Sin ayuda de contravolante automática (no existía en la era). |
| `steering_exponent` | `1.3` | Mayor precisión cerca del centro para correcciones finas a alta velocidad. |
| `max_steering_angle` | `deg_to_rad(22.0)` | Relación de dirección muy corta; un F1 solo necesita ~20-25° de giro de rueda para las curvas más cerradas (ej. horquilla de Mónaco). |
| `front_steering_ratio` | `1.0` | Dirección 100% delantera. |
| `rear_steering_ratio` | `0.0` | Sin dirección trasera. |

---

## 2. Acelerador y Frenos (Throttle & Braking)

Frenos de disco de carbono-carbono (introducidos en los 80, estándar en 1996), con muchísima potencia de frenado pero que requieren temperatura de trabajo. **TCS y ABS deshabilitados** por reglamento de la época.

| Parámetro | Valor | Justificación |
|---|---|---|
| `throttle_speed` | `9.0` | Respuesta de acelerador muy rápida (mariposas electrónicas / mecánicas de carrera). |
| `throttle_steering_adjust` | `0.15` | Ligera reducción de potencia en curva para evitar patinar el eje motriz (trasero) en salida. |
| `braking_speed` | `12.0` | Pedal de freno rígido, de recorrido corto, respuesta casi instantánea. |
| `brake_force_multiplier` | `1.6` | Frenos de carbono-carbono generan deceleraciones de ~4-5g, muy por encima de un auto de calle. |
| `front_brake_bias` | `0.60` | Los F1 de la época repartían el frenado ~58-64% al eje delantero (ajustable por el piloto en carrera con una perilla). |
| `traction_control_max_slip` | `-1.0` | **TCS desactivado**: prohibido por reglamento desde 1994. |
| `front_abs_pulse_time` / `rear_abs_pulse_time` | `999.0` | **ABS desactivado** (prohibido desde 1994); valor alto para anular su efecto práctico. |
| `front_abs_spin_difference_threshold` / `rear_abs_spin_difference_threshold` | `999.0` | Idem, para asegurar que el sistema de ABS nunca se active. |

---

## 3. Motor, Embrague y Transmisión

Motor atmosférico V10 de 3.0L (ej. Renault RS8), ~730-760 CV a ~15.500-16.000 RPM, límite de corte cercano a 17.000 RPM. Caja secuencial semiautomática de 6 velocidades accionada por paletas.

| Parámetro | Valor | Justificación |
|---|---|---|
| `max_torque` | `280.0` (N·m) | V10 de altas revoluciones: mucha potencia pero torque relativamente bajo comparado a motores turbo o V8 modernos (potencia = torque × RPM). |
| `max_rpm` | `17000.0` | RPM de corte típico de los V10 atmosféricos de 1996. |
| `idle_rpm` | `4500.0` | Los motores de F1 de la época tenían un ralentí muy alto (4.000-5.000 RPM) para evitar calarse. |
| `torque_curve` | Curva ascendente suave desde `idle_rpm`, pico entre el 85-95% de `max_rpm`, caída abrupta tras el pico | Curva de "power band" muy alta y estrecha, típica de motores atmosféricos de carrera de alta revolución. |
| `motor_drag` | `0.015` | Freno motor proporcional moderado (fricción interna de un motor muy ligero de partes). |
| `motor_brake` | `15.0` | Freno motor constante bajo; el motor gira libre con facilidad al soltar el acelerador. |
| `motor_moment` | `0.12` (kg·m²) | Cigüeñal y componentes internos muy livianos (titanio/aleaciones especiales) → sube de RPM casi instantáneamente. |
| `clutch_out_rpm` | `6000.0` | RPM de embrague en la salida (launch), por encima del ralentí para evitar calarse en la largada. |
| `max_clutch_torque_ratio` | `1.4` | Embrague multidisco de carbono, sobredimensionado respecto al torque del motor. |
| `gear_ratios` | `[2.85, 2.29, 1.89, 1.60, 1.38, 1.20]` | 6 marchas muy cerradas entre sí, típicas de una caja secuencial de F1 optimizada para mantener el motor en su power band. |
| `final_drive` | `3.60` | Ajustable por circuito (ver sección 9); valor medio de referencia. |
| `reverse_ratio` | `3.00` | Marcha atrás obligatoria por reglamento, poco usada. |
| `shift_time` | `0.04` | Cambios semiautomáticos por paletas: interrupción de potencia casi imperceptible (40 ms). |
| `automatic_transmission` | `false` | Caja secuencial manual (el piloto decide cuándo cambiar); no es automática, aunque el embrague sea electrohidráulico. |
| `automatic_time_between_shifts` | `0.0` | No aplica (transmisión manual). |
| `gear_inertia` | `0.03` | Engranajes livianos de titanio/aleación, inercia interna mínima. |

---

## 4. Tren Motriz y Diferenciales

Los F1 de 1996 eran **100% propulsión trasera (RWD)**, con diferencial autoblocante mecánico (LSD) sin vectorización electrónica.

| Parámetro | Valor | Justificación |
|---|---|---|
| `front_torque_split` | `0.0` | Tracción 100% trasera (RWD), como todos los F1. |
| `variable_torque_split` | `false` | No aplica en un RWD puro. |
| `front_variable_split` | `0.0` | No aplica. |
| `variable_split_speed` | `0.0` | No aplica. |
| `front_locking_differential_engage_torque` | `0.0` | No hay diferencial motriz delantero. |
| `rear_locking_differential_engage_torque` | `120.0` | LSD trasero mecánico de rampa, típico para controlar el "wheelspin" en salida de curva. |
| `front_torque_vectoring` | `0.0` | No aplica (sin tracción delantera). |
| `rear_torque_vectoring` | `0.0` | No existía vectorización electrónica de torque en 1996; el reparto es puramente mecánico (LSD). |

---

## 5. Masa y Estabilidad (Chassis)

Peso mínimo reglamentario en 1996: **595 kg** (con piloto, sin combustible). Centro de gravedad extremadamente bajo por el uso de motor plano/V10 tumbado y monocasco de fibra de carbono.

| Parámetro | Valor | Justificación |
|---|---|---|
| `vehicle_mass` | `595.0` (kg) | Peso mínimo FIA 1996 (chasis + piloto, sin combustible). |
| `front_weight_distribution` | `0.46` | Reparto típico ~46% delante / 54% atrás, por el motor y caja al fondo. |
| `center_of_gravity_height_offset` | `-0.45` | CdG extremadamente bajo (fibra de carbono, motor lo más bajo posible); factor crítico para el grip en curva. |
| `inertia_multiplier` | `0.9` | Masa muy concentrada y compacta (alta densidad, poca dispersión), tensor de inercia reducido respecto a un auto de calle. |
| `enable_stability` | `false` | Sin ESP: no existía ningún sistema de estabilidad electrónico en la F1 de 1996. |
| `stability_yaw_engage_angle` | `0.0` | No aplica (desactivado). |
| `stability_yaw_strength` | `0.0` | No aplica (desactivado). |
| `stability_yaw_ground_multiplier` | `0.0` | No aplica (desactivado). |
| `stability_upright_spring` / `stability_upright_damping` | `0.0` | Sin autonivelación en el aire; un F1 real que despega gira libremente (ver saltos en Eau Rouge histórico). |

---

## 6. Suspensión y Geometría

Suspensión de doble horquilla (double wishbone) con push-rod/pull-rod, muy rígida, recorrido corto. Sin suspensión activa (prohibida desde 1994).

| Parámetro | Valor | Justificación |
|---|---|---|
| `front_spring_length` | `0.05` (m) | Recorrido de resorte muy corto (~5 cm), coherente con el bajo CdG y el fondo plano aerodinámico. |
| `rear_spring_length` | `0.06` (m) | Ligeramente mayor que el delantero, común en la puesta a punto de la época. |
| `front_resting_ratio` | `0.45` | Suspensión pre-cargada, poco recorrido libre en reposo. |
| `rear_resting_ratio` | `0.45` | Idem trasero. |
| `front_damping_ratio` | `0.75` | Amortiguamiento firme, propio de un auto de carrera con neumáticos slick de altísimo grip. |
| `rear_damping_ratio` | `0.78` | Ligeramente más firme para controlar la transferencia de peso al acelerar. |
| `front_bump_damp_multiplier` | `1.3` | Compresión dura: minimiza el cabeceo (pitch) al frenar, clave para mantener el fondo plano estable. |
| `rear_bump_damp_multiplier` | `1.3` | Idem trasero. |
| `front_rebound_damp_multiplier` | `1.1` | Rebote controlado para no perder contacto tras un bache/piano. |
| `rear_rebound_damp_multiplier` | `1.15` | Levemente mayor para estabilizar la salida de curva. |
| `front_arb_ratio` | `0.65` | Barra estabilizadora delantera rígida para reducir el roll (el auto depende del downforce, no puede "rolear"). |
| `rear_arb_ratio` | `0.55` | Barra trasera algo menos rígida, favorece tracción en salida (evita levantar la rueda interior trasera). |
| `front_camber` | `deg_to_rad(-3.5)` | Camber negativo pronunciado para maximizar el contacto del neumático en curvas de alta carga lateral. |
| `rear_camber` | `deg_to_rad(-2.0)` | Camber negativo trasero, algo menor para no perder tracción en aceleración. |
| `front_toe` | `deg_to_rad(0.1)` | Toe-in mínimo delantero, para estabilidad en frenada. |
| `rear_toe` | `deg_to_rad(0.2)` | Toe-in trasero leve, mejora estabilidad direccional a alta velocidad. |
| `front_bump_stop_multiplier` | `3.0` | Tope de suspensión muy rígido: el fondo plano no puede tocar el piso sin generar "porpoising"/pérdida de downforce. |
| `rear_bump_stop_multiplier` | `3.0` | Idem trasero. |
| `front_beam_axle` / `rear_beam_axle` | `false` | Ambos ejes son de suspensión independiente (double wishbone), no eje rígido. |

---

## 7. Neumáticos

Neumáticos **Goodyear slick** (sin ranuras; las ranuras "grooved" recién se introdujeron en 1998), de compuestos blandos y anchos, con un nivel de agarre mecánico extremadamente alto.

| Parámetro | Valor | Justificación |
|---|---|---|
| `contact_patch` | `0.22` (m) | Área de contacto amplia por el gran ancho de los slicks. |
| `braking_grip_multiplier` | `1.15` | Los slicks generan tracción longitudinal adicional en frenada por el aumento de temperatura y carga aerodinámica. |
| `wheel_to_body_torque_multiplier` | `1.0` | Transferencia longitudinal estándar. |
| `tire_stiffnesses` | `{"Road": 220000, "Curb": 90000}` (N/rad) | Rigidez lateral muy alta, propia de un slick de competición sobre asfalto. |
| `coefficient_of_friction` | `{"Road": 2.4, "Curb": 1.2, "Grass": 0.5, "Gravel": 0.6}` | Los slicks de F1 alcanzan un Mu efectivo de ~2.0-2.5 con downforce y temperatura óptimas (muy por encima de un neumático de calle, Mu ~1.0). |
| `rolling_resistance` | `{"Road": 45.0}` (N) | Baja resistencia a la rodadura por la construcción rígida del neumático de competición. |
| `lateral_grip_assist` | `{"Road": 0.0}` | Sin asistencia adicional: el grip depende 100% del modelo físico + downforce. |
| `longitudinal_grip_ratio` | `{"Road": 1.0}` | Relación neutra longitudinal/lateral (sin sesgo artificial). |
| `front_tire_radius` | `0.305` (m) | ~13" de llanta + neumático, medida real aproximada de un F1 de mediados de los 90. |
| `rear_tire_radius` | `0.330` (m) | Neumático trasero de mayor diámetro que el delantero. |
| `front_tire_width` | `245.0` (mm) | Ancho real aproximado del neumático delantero de 1996. |
| `rear_tire_width` | `365.0` (mm) | Ancho real aproximado del neumático trasero (mucho más ancho para maximizar tracción). |
| `front_wheel_mass` | `6.0` (kg) | Conjunto llanta+neumático muy liviano (llantas de magnesio). |
| `rear_wheel_mass` | `7.5` (kg) | Ligeramente más pesado por el mayor tamaño. |

---

## 8. Aerodinámica

La aerodinámica es el factor más determinante de un F1 de 1996: alerones grandes, fondo plano con difusor y efecto suelo parcial (el "ground effect" total estaba prohibido desde 1983, pero el fondo plano seguía generando carga aerodinámica significativa).

| Parámetro | Valor | Justificación |
|---|---|---|
| `coefficient_of_drag` (Cd) | `0.95` | Configuración de downforce medio-alto (ej. circuito tipo Silverstone/Hungaroring); los F1 de la época rondaban Cd 0.7 (baja carga, Monza) a 1.1+ (alta carga, Mónaco). |
| `air_density` | `1.225` (kg/m³) | Densidad estándar a nivel del mar; ajustar según altitud del circuito (ej. más baja en México/Interlagos). |
| `frontal_area` | `1.50` (m²) | Área frontal real aproximada de un monoplaza de F1 de los 90 (chasis angosto y bajo). |

> 💡 **Nota:** en GEVP, la carga aerodinámica (downforce) suele modelarse como una fuerza adicional aplicada hacia abajo proporcional al cuadrado de la velocidad, separada del cálculo de `coefficient_of_drag`. Si tu implementación de GEVP no incluye downforce nativo, se recomienda añadir un script adicional que aplique una fuerza `apply_force()` vertical descendente en el centro de presión, usando un coeficiente de sustentación negativo (Cl ≈ -3.0 a -3.5 a alta configuración) para reproducir el brutal agarre en curvas rápidas característico de estos autos.

---

## 9. Variantes de Configuración por Circuito

Los equipos de F1 cambiaban el `final_drive`, los `gear_ratios` y el nivel de downforce (`coefficient_of_drag` / carga aerodinámica) según el trazado. Dos ejemplos de referencia:

### Baja Downforce / Alta Velocidad (ej. Monza, Hockenheim)
- `coefficient_of_drag`: `0.70`
- `final_drive`: `3.30` (marchas más largas, mayor velocidad punta ~350 km/h)
- `front_arb_ratio` / `rear_arb_ratio`: reducir ~15% (menos apoyo, menos roll esperado por menor carga aero)

### Alta Downforce / Circuito Lento y Sinuoso (ej. Mónaco, Hungaroring)
- `coefficient_of_drag`: `1.10`
- `final_drive`: `4.10` (marchas más cortas, prioriza tracción y aceleración sobre velocidad punta)
- `front_brake_bias`: subir a `0.62` (más peso relativo en frenadas de baja velocidad y muchas curvas de horquilla)

---

### Resumen de referencia rápida

| Magnitud | Valor real 1996 aprox. |
|---|---|
| Peso mínimo | 595 kg |
| Potencia | ~730-760 CV |
| 0-100 km/h | ~2.5-2.8 s |
| Velocidad máxima | ~330-355 km/h (según downforce) |
| Aceleración lateral en curva | hasta ~4.5-5.0 g |
| Deceleración en frenada | hasta ~4.5-5.0 g |

Estos valores son aproximaciones documentadas de la época (fuentes técnicas públicas de la F1 de mediados de los 90) adaptadas a las unidades y estructura de parámetros que usa GEVP; se recomienda ajustarlos empíricamente en el editor de Godot hasta lograr la sensación deseada, ya que el modelo de neumáticos de GEVP (basado en RayCast3D) no reproduce exactamente la física de un neumático real de F1.
