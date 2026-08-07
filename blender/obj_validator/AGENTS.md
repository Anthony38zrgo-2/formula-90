# Guía para Agentes AI — Módulo `obj_validator`

Instrucciones y reglas operativas para agentes de IA que trabajen analizando, validando, reparando o exportando modelos 3D en esta carpeta (`D:\Formula90s\blender\obj_validator`).

---

## 🎯 Alcance y Propósito

Este módulo contiene la suite de **diagnóstico y auto-reparación (auto-fixer)** de modelos 3D formato `.obj` para asegurar compatibilidad total con **Godot 4.x** y el motor de físicas **Godot Easy Vehicle Physics (GEVP)**.

---

## 🛠️ Herramientas y Scripts

### 1. Diagnóstico (`validate_obj.py`)
Inspecciona modelos `.obj` sin realizar modificaciones y genera reportes detallados en `logs/`.

```bash
# Validar un modelo
python validate_obj.py ruta/al/modelo.obj

# Validar carpeta completa
python validate_obj.py ruta/a/carpeta/
```

Módulos de check en `checks/`:
- `check_objects.py`: Estructura de mallas, recuento de caras, detección de chasis y ruedas.
- `check_materials.py`: Archivo `.mtl`, detección de rutas absolutas, asignación de texturas.
- `check_geometry.py`: Caras degeneradas, normalizado, escala vehicular (rango ~5m), centroide.
- `check_uvs.py`: Cobertura de coordenadas UV y mapeo.
- `check_godot_compat.py`: Sanidad de nombres de objetos/materiales (sin espacios ni caracteres especiales).

---

### 2. Auto-Reparación (`fix_obj.py`)
Aplica correcciones geométricas, de nombres, escala, centrado de pivotes y texturas.

```bash
# Analizar reporte de cambios sin modificar
python fix_obj.py modelo.obj --analyze-only

# Aplicar todos los fixes automáticos
python fix_obj.py modelo.obj --all

# Fixes específicos
python fix_obj.py modelo.obj --fix-scale auto
python fix_obj.py modelo.obj --fix-geometry
python fix_obj.py modelo.obj --fix-wheels
python fix_obj.py modelo.obj --fix-symmetry nombre_objeto
python fix_obj.py modelo.obj --ai-textures
```

Módulos de reparación en `fixes/`:
- `obj_io.py`: Parser/Writer personalizado que preserva la jerarquía de sub-objetos (`o`, `usemtl`), evitando que `trimesh` aplane el modelo.
- `fix_scale.py`: Escala automática a dimensiones reales de vehículo (~5.0m en Z).
- `fix_geometry.py`: Eliminación de caras degeneradas, fusión de vértices duplicados y recalculado de normales.
- `fix_wheels.py`: Identificación de ruedas, clasificación FL/FR/RL/RR, centrado de pivotes en centroide y renombrado estándar GEVP.
- `fix_symmetry.py`: Análisis de simetría axial X y espejado de geometría.
- `fix_names.py`: Limpieza de nombres incompatibles con Godot.
- `ai_texture.py`: Pipeline de generación de prompts para texturas PBR y actualización del `.mtl`.

---

## ⚠️ Reglas Obligatorias para Agentes AI

1. **Inspección Previa Siempre:**
   Antes de ejecutar reparaciones o modificar un `.obj`, siempre correr `python fix_obj.py <modelo.obj> --analyze-only` o `validate_obj.py` para inspeccionar la estructura real sin alterar archivos.

2. **Reglas Estrictas de Identificación de Ruedas:**
   - Una rueda se identifica **únicamente** si el nombre contiene palabras clave primarias: `wheel`, `rueda`, `tire`, `tyre`, `neumatico`, `neumático`.
   - NUNCA clasificar piezas de aerodinámica (`aleron`, `wing`, `spoiler`) ni de carrocería (`chasis`, `body`) como ruedas.
   - Clasificación por coordenadas 3D:
     - `X < 0` = Izquierda (Left) | `X > 0` = Derecha (Right)
     - `Z > global_center` = Delantero (Front) | `Z < global_center` = Trasero (Rear)

3. **Preservación de Pivotes y Geometría:**
   - Cada rueda procesada por `fix_wheels.py` debe tener su origen/pivote local exactamente centrado en su centroide geométrico `(0,0,0)`.

4. **Exportación e Integración a Godot:**
   - En Godot 4.x, los archivos `.obj` cargan como `ArrayMesh`. Para integrar el modelo en la escena `.tscn` como `PackedScene`, exportar el modelo procesado en assets divididos formato `.glb` (`f1_body.glb`, `wheel_fl.glb`, `wheel_fr.glb`, `wheel_rl.glb`, `wheel_rr.glb`).

5. **Verificación Post-Procesado:**
   Al finalizar cualquier corrección, ejecutar `validate_obj.py` sobre el archivo de salida para garantizar que **Errores Críticos = 0**.
