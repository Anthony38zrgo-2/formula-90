# Diseño Data-Driven (Próxima Evolución)

Actualmente, muchas configuraciones de físicas están hardcodeadas en escenas específicas (`f1_2026_car.tscn`).

## Deuda Técnica y Evolución
Para facilitar la adición de futuros vehículos o motores, el proyecto debe evolucionar hacia un patrón donde el comportamiento provenga de recursos (`.tres`) o archivos JSON en la carpeta `data/`:

- `data/engines/v10_engine.tres` (Max RPM, Torque, Inercia)
- `data/aero/2026_aero.tres`
- `data/tyres/slicks.tres`

El sistema físico cargará estas configuraciones al iniciar, permitiendo combinaciones tipo Lego: `Chasis 2026` + `Motor V10`.
