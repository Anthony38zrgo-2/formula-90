# OBJ Validator — Godot Easy Vehicle Physics

Validador de modelos `.obj` para asegurar compatibilidad con Godot 4.x y el addon GEVP.

## Uso

```bash
# Validar un .obj específico
python validate_obj.py D:\Proyectos\FormulaSourceSDK\blender\current\modelo.obj

# Validar todos los .obj de una carpeta (recursivo)
python validate_obj.py D:\Proyectos\FormulaSourceSDK\blender\current

# Carpeta de logs personalizada
python validate_obj.py modelo.obj --logs-dir D:\MisLogs
```

En Windows también puedes **arrastrar un `.obj` sobre `validate.bat`**.

## Dependencias

```bash
pip install trimesh numpy
```

## Estructura

```
obj_validator/
├── validate_obj.py          ← Script principal (ejecutar esto)
├── validate.bat             ← Atajo Windows
├── README.md
├── logs/                    ← Logs generados automáticamente
└── checks/
    ├── check_geometry.py    ← Triángulos, degenerados, normales, escala, pivote
    ├── check_uvs.py         ← Coordenadas UV, tiling, cobertura
    ├── check_materials.py   ← MTL, rutas absolutas, texturas faltantes
    ├── check_objects.py     ← Sub-mallas, chasis/ruedas GEVP, polígonos
    └── check_godot_compat.py← Nombres, caracteres especiales, duplicados
```

## Qué verifica cada check

### `check_objects.py` — Estructura de sub-mallas
| Validación | Severidad |
|---|---|
| Al menos 4 objetos con nombre de rueda | ERROR |
| Objeto de chasis detectado | WARNING |
| Objetos sin normales exportadas | ERROR |
| Objetos sin coordenadas UV | ERROR |
| Más de 50,000 caras por objeto | ERROR |
| Más de 15,000 caras por objeto | WARNING |

### `check_materials.py` — Materiales y texturas
| Validación | Severidad |
|---|---|
| Archivo .mtl existe | ERROR |
| Rutas de textura ABSOLUTAS | ERROR |
| Archivos de textura existen en disco | ERROR |
| Materiales sin textura difusa | WARNING |

### `check_geometry.py` — Geometría 3D
| Validación | Severidad |
|---|---|
| Caras degeneradas (área≈0) | ERROR |
| Mesh no watertight (huecos) | WARNING |
| Normales inconsistentes | WARNING |
| Dimensiones fuera de rango vehicular | ERROR |
| Centroide lejos del origen | WARNING |

### `check_uvs.py` — Coordenadas UV
| Validación | Severidad |
|---|---|
| Sin coordenadas UV | ERROR |
| >5% vértices en (0,0) | ERROR |
| UVs fuera de [0,1] (tiling) | WARNING |
| Cobertura UV muy baja | ERROR |

### `check_godot_compat.py` — Compatibilidad Godot
| Validación | Severidad |
|---|---|
| Espacios en nombre de archivo | ERROR |
| Nombres de objetos duplicados | ERROR |
| Caracteres especiales en nombres | ERROR |
| Espacios en nombres de material | ERROR |
| Nombres muy largos (>40 chars) | WARNING |

## Requisitos del modelo para GEVP

Para poder usar el modelo con Godot Easy Vehicle Physics, necesitas:

1. **4 ruedas separadas** como sub-objetos (`o wheel_fl`, `o wheel_fr`, etc.)
2. **1 chasis** como sub-objeto (`o chassis` o `o body`)
3. **Escala en metros** (Ctrl+A > Apply Scale en Blender)
4. **Pivote en el centro geométrico** de cada pieza
5. **UVs correctos** en todos los objetos
6. **Normales exportadas** (Write Normals activo en Blender)
7. **Texturas con rutas relativas** (no absolutas) o empaquetadas junto al .obj
8. **Nombres sin espacios ni caracteres especiales**

## Interpretar el log

```
✔  — Pasó el check
⚠  — Aviso (no bloquea, pero debería revisarse)
✗  — Error crítico (debe corregirse antes de importar)
```

El log final dice:
- **APROBADO** → El modelo puede importarse en Godot (revisa los avisos)
- **NO APROBADO** → Lista los errores críticos que debes corregir en Blender
