# Propuesta: Remapeo de Controles, Control de Ayudas y HUD

## 1. Resumen Ejecutivo
El objetivo es adaptar el esquema de controles del vehículo a un layout puramente arcade/teclado, introducir un sistema dinámico de **Ayudas a la Conducción** alternables en tiempo real mediante las teclas numéricas (`1` a `5`), y reflejar claramente el estado de estas ayudas y la telemetría en el **HUD (UI)**.

---

## 2. Remapeo de Controles (Keybinds)

El nuevo mapa de teclas simplifica la conducción eliminando entradas legacy/duplicadas y reservando las teclas numéricas exclusivamente para la gestión de asistencias:

| Acción | Tecla Principal | Alternativa | Descripción |
|---|---|---|---|
| **Acelerar** | `Flecha Arriba (↑)` | Gamepad Trigger Der. | Aceleración longitudinal |
| **Frenar / Reversa** | `Flecha Abajo (↓)` | Gamepad Trigger Izq. | Frenado principal |
| **Girar Izquierda** | `Flecha Izquierda (←)` | Gamepad Stick Izq. | Dirección izquierda |
| **Girar Derecha** | `Flecha Derecha (→)` | Gamepad Stick Der. | Dirección derecha |
| **Subir Marcha** | `A` | Gamepad Botón 2 | Cambio ascendente (manual) |
| **Bajar Marcha** | `Z` | Gamepad Botón 3 | Cambio descendente (manual) |
| **Freno Secundario** | `Espacio` | Gamepad Botón 0 | Freno de mano / derrape |
| **Reset Vehículo** | `R` | Gamepad Botón 3 | Reinicia posición en pista |

---

## 3. Sistema de Control de Ayudas (Driving Aids)

El vehículo dispondrá de **5 asistencias independientes** activables/desactivables al vuelo durante la partida mediante el teclado numérico:

| Tecla | Ayuda | Parámetro Físico Afectado | Comportamiento al Activar |
|---|---|---|---|
| **`1`** | **Transmisión Automática** | `automatic_enabled` | Alterna entre transmisión automática y cambios manuales. |
| **`2`** | **Control de Estabilidad** | `stability_recovery` | Incrementa la fuerza de auto-recuperación ante sobrevirajes bruscos. |
| **`3`** | **Asistencia de Dirección** | `high_speed_steering` | Suaviza el ángulo de giro a altas velocidades para evitar giros involuntarios. |
| **`4`** | **Asistencia de Frenado** | `strong_brake_force` | Multiplica la capacidad de frenado para desaceleraciones más cortas. |
| **`5`** | **Asistencia de Agarre (Grip)** | `lateral_grip` / `drift_factor` | Aumenta el agarre lateral de los neumáticos y minimiza el derrape de cola. |

*Nota de arquitectura: Los valores originales configurados en el recurso `.tres` del coche se conservan como base estática (baseline) para que al desactivar una ayuda el vehículo retorne exactamente a su física pura.*

---

## 4. Diseño del HUD (Interfaz de Usuario)

El HUD se dividirá en dos bloques principales en pantalla:

### A. Panel de Telemetría (Esquina Superior Izquierda - Color Cyan)
- **Velocidad**: `km/h` actual.
- **Marcha**: Estado de la caja (`1-6`, `N`, `R`).
- **Modo Transmisión**: `AUTO` / `MANUAL`.
- **Revoluciones**: `RPM` en tiempo real.
- **Estado C++**: Estado dinámico (`READY`, `DRIVING`, `AIRBORNE`, etc.).

### B. Panel de Ayudas en Tiempo Real (Debajo del Panel de Telemetría - Color Ámbar)
Muestra un indicador directo del estado de cada tecla de asistencia:

```text
AYUDAS ACTIVAS
1 AUTO    ON / OFF
2 ESTAB   ON / OFF
3 DIRECC  ON / OFF
4 FRENOS  ON / OFF
5 GRIP    ON / OFF
```

---

## 5. Integración en la Escena de Juego (`test_field.tscn`)

Para asegurar que los controles y el HUD funcionen de forma integrada con el C++ runtime:
1. La escena `test_field.tscn` instancia directamente `PlayerCar` (`ArcadeCarController` C++) y `DebugHud` (`DebugHudController` C++).
2. El `DebugHud` lee dinámicamente el estado del `PlayerCar` hermano en el árbol de nodos sin acoplamiento rígido.
