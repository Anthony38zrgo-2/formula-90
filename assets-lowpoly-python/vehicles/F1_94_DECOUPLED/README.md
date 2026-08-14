# F1_94 — paquete desacoplado para Godot / GEVP

Este paquete separa la geometría del color visible.

## Regla

- Los `.glb` contienen geometría, normales, UV0, jerarquía, pivotes `JNT_*` y referencias `DATUM_*`.
- Los `.glb` NO contienen imágenes ni texturas albedo embebidas.
- Cada `GEO_*` tiene un PNG externo en `textures/albedo/<GEO_NAME>.png`.
- Los UV NO fueron repaquetados ni modificados. Cada PNG conserva exactamente el layout 64×64 del atlas original. Esto prioriza seguridad y evita introducir nuevos errores UV.
- Los materiales placeholder del GLB son blancos, simples, sin textura y se llaman `MAT_<GEO_NAME>`.

## Asset principal

`geometry/F1_94_geometry.glb`

## Assets GEVP

- `geometry/F1_94_chassis_geometry.glb`
- `geometry/F1_94_wheel_front_geometry.glb`
- `geometry/F1_94_wheel_rear_geometry.glb`

Las ruedas canónicas mantienen su origen en el centro de rotación.

## Godot

`godot/f1_94_external_albedo_binder.gd` asigna automáticamente el PNG externo cuyo nombre coincide con cada nodo `GEO_*` y fuerza `Nearest`, roughness 1.0, metallic 0 y alpha scissor.

Ajusta `albedo_root` a la carpeta `textures/albedo` dentro de tu proyecto.

## Importante

No se hizo UV painting nuevo. No se horneó color en vértices. No se repaquetaron UVs. La asociación es estrictamente:

`GEO_* -> UV0 del GLB -> PNG externo del mismo nombre`.
