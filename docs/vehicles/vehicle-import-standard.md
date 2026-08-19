# Manifiesto estándar para modelos 3D de vehículos

Todo modelo 3D destinado a Formula-90 debe cumplir este estándar antes de integrarse en Godot.

## Convenciones espaciales

- `+X` = derecha del vehículo.
- `-X` = izquierda.
- `+Y` = arriba.
- `-Z` = parte delantera.
- `+Z` = parte trasera.
- El origen principal debe estar centrado longitudinalmente entre ambos ejes.
- La escala debe estar en metros.
- Las transformaciones deben estar aplicadas antes de exportar.

## Geometría

- Todas las caras deben tener winding consistente.
- Las normales deben apuntar hacia el exterior.
- No debe dependerse de `Cull Disabled` para visualizar correctamente el modelo.
- No debe haber caras degeneradas, duplicadas o geometría accidental.
- El modelo debe mantener una topología suficientemente simple para tiempo real.
- Los elementos físicos importantes deben poder identificarse y separarse.

## UV y materiales

- Los UV deben mantenerse dentro de `[0,1]`, salvo casos explícitamente justificados.
- No debe haber UV rotos o desplazados al exportar.
- Las texturas deben usar rutas relativas.
- Para estética pixel-art, usar filtrado `Nearest`.
- Evitar shaders complejos innecesarios.
- Preferir materiales simples y compatibles con Godot.

## Nombres obligatorios

La geometría debe utilizar nombres explícitos y sin ambigüedad:

```text
GEO_CHASSIS
GEO_AERO_FRONT_WING
GEO_AERO_REAR_WING
GEO_SUSPENSION_FL
GEO_SUSPENSION_FR
GEO_SUSPENSION_RL
GEO_SUSPENSION_RR
GEO_WHEEL_FRONT_*
GEO_WHEEL_REAR_*
GEO_DRIVER_*
```

`FRONT` y `REAR` identifican los recursos visuales compartidos por eje. `FL`,
`FR`, `RL` y `RR` se reservan para datums, puntos de unión e instancias de
escena: `Front Left`, `Front Right`, `Rear Left` y `Rear Right`. `GEO_`
identifica geometría visible, `JNT_` un punto de unión/pivot y `DATUM_` una
referencia espacial. Un paquete asimétrico puede usar nombres `GEO_WHEEL_FL_*`
por esquina si declara la excepción.

## Puntos de unión

Todo vehículo debe incluir como mínimo:

```text
DATUM_VEHICLE_ORIGIN
DATUM_FRONT_AXLE_CENTER
DATUM_REAR_AXLE_CENTER
JNT_WHEEL_FL
JNT_WHEEL_FR
JNT_WHEEL_RL
JNT_WHEEL_RR
JNT_FRONT_WING_MOUNT
JNT_REAR_WING_MOUNT
```

Para suspensión se requieren los pares `JNT_SUSP_FL_CHASSIS` /
`JNT_SUSP_FL_HUB`, `FR`, `RL` y `RR`. Cuando sea posible, cada brazo debe
identificar también sus extremos `INNER` y `OUTER`.

## Ruedas y GEVP

- El paquete runtime estándar contiene una rueda delantera y una rueda trasera compartibles.
- Las cuatro esquinas deben ser instancias de escena independientes, aunque reutilicen esos dos GLB.
- El origen de cada GLB de rueda debe coincidir exactamente con su centro de rotación.
- La geometría visual no debe contener offsets arbitrarios.
- Los pivotes deben permitir que GEVP controle directamente rotación, dirección y desplazamiento vertical.
- La posición física se define mediante `RayCast3D`, nunca horneada en la geometría.
- La orientación visual de cada lado puede resolverse con un nodo de presentación bajo el `wheel_node`; no debe mutar ni compartir el estado físico de GEVP.
- Cuatro GLB de rueda por esquina son una excepción permitida solamente cuando exista asimetría geométrica o visual real y esté documentada en el manifiesto.

Jerarquía esperada:

```text
WheelFrontLeft
└── FrontLeftWheel
    └── Visual
```

La misma forma se aplica a las otras tres ruedas.

## Exportación

El formato principal es `GLB/glTF`. OBJ puede conservarse como edición o
respaldo, pero no es el formato principal de integración en Godot. El paquete
runtime contiene `vehicle_chassis.glb`, `vehicle_wheel_front.glb`,
`vehicle_wheel_rear.glb`, `textures/` y `vehicle_metadata.json`. Un
`vehicle.glb` completamente ensamblado puede conservarse como referencia de
autoría o validación, pero no forma parte del runtime obligatorio. Los tres GLB
runtime obligatorios son el chasis, la
rueda delantera compartida y la rueda trasera compartida. Godot debe crear
cuatro instancias visuales independientes sobre cuatro `RayCast3D`; compartir
el recurso GLB no comparte el estado físico.

El manifiesto debe declarar como mínimo las claves `chassis`, `wheel_front` y
`wheel_rear`. Sólo una excepción asimétrica documentada puede sustituirlas por
`wheel_fl`, `wheel_fr`, `wheel_rl` y `wheel_rr`.

Un modelo está listo cuando puede importarse en Godot sin correcciones manuales
de escala, orientación, normales, UV, pivotes o nombres.
