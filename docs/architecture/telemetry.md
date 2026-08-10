# Arquitectura de Telemetría

Estado: **infraestructura pendiente de ampliación**. El HUD arcade actual es
GDScript y muestra velocidad, marcha, ayudas y minimapa; no se debe interpretar
esta lista como una promesa de que todas las métricas ya estén capturadas o
expuestas en runtime.

La telemetría es crucial para que los agentes puedan diagnosticar y testear el vehículo objetivamente.

## Métricas Clave
Se debe buscar u obligar la exposición de las siguientes variables (vía GDScript a HUD en C++):
- `speed` (Velocidad en km/h)
- `rpm` (Revoluciones actuales del motor)
- `gear` (Marcha actual)
- `throttle`, `brake`, `steering` (Inputs del jugador o IA)
- `front_slip`, `rear_slip` (Niveles de deslizamiento para diagnosticar understeer/oversteer)
- `lateral_g` y `longitudinal_g`

El agente que modifique físicas debe comprobar en telemetría si las métricas responden como se espera.
