# Arquitectura del Vehículo

## Nodos Físicos vs Visuales
La regla de oro del sistema de vehículos es la separación estricta entre la física y los gráficos:

1. **Jerarquía Física (`VehicleRigidBody`):** Contiene parámetros de GEVP, raycasts de ruedas y colisiones (`BoxShape3D` o simple). No contiene mallas renderizables complejas ni se escala.
2. **Jerarquía Visual (`VehicleVisualRoot` / `F1_Body_Model`):** Las mallas `.glb` importadas se instancian externamente (ej. en `f1_2026_visual.tscn`) y son controladas o emparejadas con el nodo físico mediante un controlador visual (`VehicleVisual3DController` en C++), el cual interpola el roll, pitch y añade vibraciones sin afectar el modelo de colisión.
