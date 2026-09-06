# F1 2026–2008: ancho 1,990 m

Implementación local sobre `main-clean`, base `0bfeaa9cd67d6744cf987c7689307a78c2306ede`.
No se cambió de rama. El estado previo de `third_party/godot-cpp` no forma parte de esta modificación.

## Dimensiones y contrato

| Parámetro | Anterior (m) | Nuevo (m) |
|---|---:|---:|
| Trocha delantera entre centros | 1,445 | 1,635 |
| Trocha trasera entre centros | 1,420 | 1,590 |
| Ancho neumático delantero | 0,355 | 0,355 |
| Ancho neumático trasero | 0,380 | 0,400 |
| Ancho exterior delantero | 1,800 | 1,990 |
| Ancho exterior trasero | 1,800 | 1,990 |
| Batalla | 3,230 | 3,230 |

Coordenadas de GLB/Godot: lateral X, vertical Y, longitudinal Z; frente −Z.
Centros delanteros: `(±0.8175, 0, -1.615)`; traseros: `(±0.795, 0, 1.615)`.
Se conservan los orígenes locales de las ruedas, radio 0,330 m, coordenadas Y/Z
de todos los vértices, jerarquía, nombres, índices, UVs y materiales.

## Geometría

`widen_f1_2026_2008.py` importa los GLB originales en Blender y modifica sus
vértices; escribe posiciones y normales en los buffers originales. No reexporta
texturas ni reconstruye nodos. El GLB delantero es idéntico byte a byte al original.
El trasero escala lateralmente neumático, llanta y hub por `0.400/0.380`;
las caras de referencia pasan de ±0,190 a ±0,200 m.

En el chasis se modifican exclusivamente `GEO_CHASSIS_FRONT_SUSPENSION` y
`GEO_CHASSIS_REAR_SUSPENSION`, además de las cuatro traslaciones X `JNT_WHEEL_*`.
Cada componente conexo se detecta agrupando coincidencias de posición a cinco
decimales, exclusivamente para análisis; no se sueldan ni agregan vértices.
Los extremos de los 8 miembros delanteros llegan a |X|=0,655 m y los 6 traseros
a |X|=0,608 m, dentro del espesor del conjunto interior del hub. Se conserva
la zona de anclaje interior hasta |X|=0,170 m delante y 0,310 m detrás; el tramo
restante se estira lateralmente de forma afín hasta el nuevo extremo.
Las normales reciben la inversa transpuesta de esa deformación.

El `.blend` recibe la misma operación en su eje lateral Y, y sus controles
`WHEEL_*_CONTROL` reciben las nuevas semitrochas. Su jerarquía y topología se
comparan con la copia previa. La orientación canónica del GLB es la rueda
derecha (+X exterior); la escena coloca yaw π en los Visual izquierdos y yaw 0
en los nodos Orientation derechos existentes. Esto corrige la orientación
preexistente invertida sin añadir nodos o cambiar los pivotes de animación.

## Física y contacto

Perfil: `game/data/vehicles/f1_2026_2008/f1_2026_2008_physics.json`.
Escena: `game/scenes/vehicles/f1_2026_2008/f1_2026_2008_rust.tscn`.
Sólo cambian las dos trochas y el ancho del neumático trasero en el perfil.
Los 12 raycasts coinciden con `semitrocha ± ancho_neumático * 0.4`:

| Eje | X absoluto interior | X absoluto central | X absoluto exterior |
|---|---:|---:|---:|
| Delantero | 0,6755 | 0,8175 | 0,9595 |
| Trasero | 0,6350 | 0,7950 | 0,9550 |

El runtime ya obtiene sus anclajes y separación de rayos del perfil Rust;
no requiere cambios en C++/Rust. Las tres BoxShape3D son exclusivamente del
chasis (tub/nose/rear): conservan tamaño y posición porque el chasis no cambia.
No hay colliders sólidos de neumáticos en esta escena; su contacto usa raycasts.

## Reproducción y comprobación

Requisito local: copia original en
`D:/Formula90s-backups/f1_2026_2008-width-1990/`, con carpeta `f1-2026-2008/`
(incluido `source/f1_2026_2008_source.blend`), perfil JSON y escena TSCN originales.
Los scripts usan esa ruta explícita. En otro equipo debe restaurarse esa copia
o adaptarse la constante correspondiente; nunca usar como original el asset ya
deformado. La edición siempre parte de la copia, evitando acumular escalados.

Desde la raíz del repositorio, con Blender y Python disponibles:

```powershell
blender --background --factory-startup --python-exit-code 1 --python tools/blender/widen_f1_2026_2008.py
blender --background --factory-startup --python-exit-code 1 --python tools/blender/validate_f1_width.py
python tools/blender/check_f1_width_contract.py --update-reports
python tools/blender/check_f1_width_contract.py
```

La última orden sólo verifica. El validador Blender mide 1,989999771 m delante
y detrás (tolerancia 10 µm respecto de 1,990), verifica intersección de triángulos
de cada uno de los 14 miembros con su hub, comprueba raycasts y compara el fuente.
La distancia vértice-superficie que acompaña el informe NO es la distancia entre
superficies: puede ser positiva aunque los triángulos se intersecten.
El verificador independiente compara buffers contra el original, preservación
de coordenadas longitudinales/verticales, nodos, datums, perfil y escena.
Los hashes ligan el informe Blender a los archivos efectivamente comprobados.

Resultados persistentes: `validation_report.json`, `export_report.json`,
`vehicle_metadata.json` y hashes de `manifest.json` dentro de la carpeta del modelo.
Vistas y diagnóstico local: `reports/vehicle-width-f1_2026_2008/`.
Los informes anteriores se reemplazan por comprobaciones ejecutadas sobre estos
archivos, sin reutilizar afirmaciones de validación antiguas.

Límite de validación: posición estática de referencia, sin giro, rodadura ni
recorrido de suspensión. La suspensión visual sigue perteneciendo al GLB del
chasis como antes; no se implementa cinemática nueva. No se ejecutó conducción,
compilación nativa ni importación en Godot. Para una prueba de juego debe usarse
`run_f1_94.ps1` respetando la comprobación BUILD/HEAD del proyecto.

El fuente `.blend` y `reports/` están excluidos por el `.gitignore` existente.
El fuente actualizado está disponible localmente y debe entregarse por separado
si se transfiere el trabajo a otro equipo. Los GLB y los informes del modelo sí
son archivos versionables. No se ha creado commit ni push de esta modificación.
