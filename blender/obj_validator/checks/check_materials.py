"""
check_materials.py — Validación del archivo .mtl y texturas referenciadas
--------------------------------------------------------------------------
Verifica:
  - Que exista el archivo .mtl junto al .obj
  - Que los materiales tengan map_Kd (diffuse/albedo) asignado
  - Que las rutas de textura sean absolutas vs relativas (portabilidad)
  - Que los archivos de textura existan en disco
  - Nombres de material vacíos o con espacios (problemático en Godot)
"""

from __future__ import annotations
import os
import re
from dataclasses import dataclass, field
from typing import List, Dict


@dataclass
class MaterialCheckResult:
    passed: bool = True
    warnings: List[str] = field(default_factory=list)
    errors:   List[str] = field(default_factory=list)


def check_materials(obj_path: str) -> MaterialCheckResult:
    result = MaterialCheckResult()
    obj_dir = os.path.dirname(os.path.abspath(obj_path))

    # ── 1. ¿Existe el .obj? ──────────────────────────────────────────────────
    if not os.path.isfile(obj_path):
        result.errors.append(f"Archivo OBJ no encontrado: {obj_path}")
        result.passed = False
        return result

    # ── 2. Buscar referencia al .mtl en el .obj ──────────────────────────────
    mtl_files: List[str] = []
    with open(obj_path, "r", encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.strip()
            if line.lower().startswith("mtllib "):
                mtl_name = line[7:].strip()
                mtl_path = os.path.join(obj_dir, mtl_name)
                mtl_files.append(mtl_path)

    if not mtl_files:
        result.warnings.append(
            "El archivo OBJ no referencia ningún .mtl. "
            "Puede funcionar en Godot pero no tendrá materiales aplicados."
        )
        return result

    # ── 3. ¿Existe el .mtl? ──────────────────────────────────────────────────
    for mtl_path in mtl_files:
        if not os.path.isfile(mtl_path):
            result.errors.append(
                f"Archivo MTL referenciado no encontrado: {mtl_path}. "
                "Asegúrate de exportar el .mtl junto al .obj."
            )
            result.passed = False
            continue

        # ── 4. Parsear el .mtl ───────────────────────────────────────────────
        materials: Dict[str, Dict] = {}
        current_mat = None

        with open(mtl_path, "r", encoding="utf-8", errors="replace") as f:
            for line in f:
                line = line.strip()
                if line.startswith("newmtl "):
                    current_mat = line[7:].strip()
                    materials[current_mat] = {"map_Kd": None, "map_Bump": None, "extra_maps": []}
                elif current_mat:
                    if line.lower().startswith("map_kd "):
                        materials[current_mat]["map_Kd"] = line[7:].strip()
                    elif line.lower().startswith("map_bump ") or line.lower().startswith("map_normal "):
                        # Puede tener flags tipo "-bm 1.0"
                        parts = line.split()
                        tex_path = parts[-1]
                        materials[current_mat]["map_Bump"] = tex_path
                    elif re.match(r"map_\w+", line, re.IGNORECASE):
                        materials[current_mat]["extra_maps"].append(line)

        if not materials:
            result.warnings.append(
                f"El archivo MTL '{os.path.basename(mtl_path)}' existe pero no contiene materiales (newmtl)."
            )
            continue

        result.warnings.append(
            f"MTL OK — {len(materials)} material(es) encontrado(s): {', '.join(materials.keys())}"
        )

        # ── 5. Verificar cada material ───────────────────────────────────────
        for mat_name, mat_data in materials.items():

            # 5a. Nombre con espacios → problemático en Godot
            if " " in mat_name:
                result.errors.append(
                    f"Material '{mat_name}' contiene ESPACIOS. "
                    "Godot puede fallar al importar materiales con espacios en el nombre. "
                    "Renómbralo en Blender (sin espacios)."
                )
                result.passed = False

            # 5b. Sin diffuse map
            if mat_data["map_Kd"] is None:
                result.warnings.append(
                    f"Material '{mat_name}' no tiene map_Kd (textura difusa/albedo). "
                    "El material se importará sin textura base en Godot."
                )

            # 5c. Verificar rutas de textura
            for tex_key, tex_val in [("map_Kd", mat_data["map_Kd"]), ("map_Bump", mat_data["map_Bump"])]:
                if tex_val is None:
                    continue

                # Ruta absoluta → no portátil
                if os.path.isabs(tex_val):
                    result.errors.append(
                        f"Material '{mat_name}' > {tex_key}: ruta ABSOLUTA detectada → "
                        f"'{tex_val}'. "
                        "Esto NO funcionará en otras máquinas ni en Godot. "
                        "Coloca las texturas junto al .obj y usa rutas RELATIVAS en Blender."
                    )
                    result.passed = False

                    # Aún así, comprueba si el archivo existe
                    if not os.path.isfile(tex_val):
                        result.errors.append(
                            f"  └─ Además, el archivo de textura NO existe: '{tex_val}'."
                        )
                else:
                    # Ruta relativa → verificar existencia
                    abs_tex = os.path.normpath(os.path.join(obj_dir, tex_val))
                    if not os.path.isfile(abs_tex):
                        result.errors.append(
                            f"Material '{mat_name}' > {tex_key}: textura relativa NO encontrada → "
                            f"'{tex_val}' (buscada en: {abs_tex})."
                        )
                        result.passed = False

    return result
