# Proceso de vegetación con hojas lanceoladas

## Objetivo visual

La biblioteca busca conservar el lenguaje low-poly del modelo de referencia: tronco y ramas legibles, hojas pequeñas identificables de manera individual, huecos dentro de la copa y variación de color sin texturas externas.

## Cómo se llegó al resultado

1. Se comparó el primer `tree_3d_01` con el modelo de referencia. El diagnóstico mostró que el color no era el problema principal: el generador representaba cada grupo de follaje mediante tres octágonos grandes cruzados.
2. Se sustituyó esa geometría por hojas lanceoladas individuales. Cada hoja tiene seis puntos de contorno, extremos agudos y un centro ligeramente elevado que forma un pliegue o nervio.
3. Se aumentó la densidad sin rellenar por completo la copa. Los árboles usan entre 9 y 10 hojas por grupo; los arbustos, entre 7 y 8. La dispersión local mantiene espacios negativos y permite ver las ramas.
4. Se hicieron parametrizables el ancho, el largo y la separación de las hojas. Así cada familia conserva su escala y silueta, en vez de aplicar una medida única a árboles y arbustos.
5. Se mantuvieron caras por ambos lados para que las hojas sean visibles desde cualquier dirección en Godot y Blender.
6. Se aplicó pintura procedural en Blender: marrones por altura, iluminación y grano para la madera; verdes profundos, oliva, verdes iluminados y acentos dorados para las hojas.

## Parámetros de receta

- `leaf_style: "lanceolate"`: activa la hoja individual puntiaguda.
- `cards_per_cluster`: número de hojas de cada grupo.
- `card_width_m`: ancho nominal de una hoja en metros.
- `card_height_m`: largo nominal de una hoja en metros.
- `leaf_cluster_spread_m`: separación alrededor del centro del grupo.
- `cluster_count`: cantidad de grupos distribuidos entre terminaciones de ramas y volumen de copa.

Las semillas existentes siguen controlando toda la variación, por lo que una misma receta produce bytes reproducibles.

## Regeneración

Desde la raíz del repositorio:

```powershell
python game/resources/environment/tools/build_vegetation_3d_library.py `
  --recipes game/resources/environment/recipes/vegetation_library.json `
  --repo .
```

El comando reconstruye seis árboles, seis arbustos, sus previews de auditoría y `assets/manifest.json` con hashes SHA-256.

## Validación requerida

```powershell
python -m unittest discover -s game/resources/environment/tests -v
python game/resources/environment/tools/audit_vegetation_3d_library.py --repo .
```

La auditoría comprueba dimensiones, anclaje al terreno, colores de vértice, follaje de doble cara, hashes del manifiesto y el límite de 10.000 triángulos por asset. Como segunda validación visual, los GLB deben poder importarse y renderizarse en Blender sin errores.

## Pintura y preview aislado

`tools/preview_vertex_paint_blender.py` importa un GLB, pinta sus colores de vértice dentro de Blender y genera un PNG. No sobrescribe el GLB fuente. Esto permite revisar el acabado antes de integrar materiales equivalentes en el runtime.
