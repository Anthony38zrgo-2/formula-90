# vehicles — Composición de escenas de vehículos

Ámbito: todas las escenas de vehículos de esta carpeta y sus subcarpetas.
Consumidores: las definiciones de vehículos, las sesiones de conducción y los lanzadores canónicos.

## Amortiguamiento lineal obligatorio

Todo cuerpo `RigidBody3D` de vehículo integrado por Godot cuyas fuerzas y resistencias calcula el modelo físico de Rust debe declarar explícitamente:

```ini
linear_damp_mode = 1
linear_damp = 0.0
```

`linear_damp_mode = 1` corresponde a `RigidBody3D.DAMP_MODE_REPLACE`. Es obligatorio sustituir el amortiguamiento heredado: establecer únicamente `linear_damp = 0.0` en modo Combine conserva el amortiguamiento global de Godot.

Esta configuración evita una resistencia adicional ajena al modelo de Rust, que consume empuje y potencia y reduce la aceleración y la velocidad máxima. No compensar ese efecto aumentando el par motor ni reduciendo el drag aerodinámico del perfil.

Al crear o modificar una escena de vehículo, verificar que el cuerpo conserve ambos valores. Esta regla se refiere al amortiguamiento lineal; no prescribe cambios en el amortiguamiento angular.
