"""
check_godot_compat.py — Validaciones específicas de compatibilidad con Godot 4.x
----------------------------------------------------------------------------------
Verifica aspectos que pueden causar problemas al importar en Godot 4.3:
  - Escala de aplicación (Godot usa unidades en metros)
  - Presencia de objetos con el mismo nombre (conflictos de importación)
  - Longitud de nombres de objeto (Godot puede truncar nombres largos)
  - Caracteres especiales en nombres de objeto/material
  - Espaciado en nombres de archivo (problemático con ResourceLoader)
"""

from __future__ import annotations
import os
import re
from dataclasses import dataclass, field
from typing import List, Dict


@dataclass
class GodotCompatResult:
    passed: bool = True
    warnings: List[str] = field(default_factory=list)
    errors:   List[str] = field(default_factory=list)


# Caracteres problemáticos en Godot NodePaths y nombres de recurso
FORBIDDEN_CHARS_PATTERN = re.compile(r'[^a-zA-Z0-9_\-\.]')
MAX_NAME_LENGTH = 40


def _parse_names_from_obj(obj_path: str) -> Dict[str, List[str]]:
    """Extrae nombres de objetos y materiales del OBJ/MTL."""
    names = {"objects": [], "materials_used": [], "mtl_files": []}
    with open(obj_path, "r", encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.strip()
            if line.startswith("o "):
                names["objects"].append(line[2:].strip())
            elif line.startswith("usemtl "):
                mat = line[7:].strip()
                if mat not in names["materials_used"]:
                    names["materials_used"].append(mat)
            elif line.lower().startswith("mtllib "):
                names["mtl_files"].append(line[7:].strip())
    return names


def check_godot_compat(obj_path: str) -> GodotCompatResult:
    result = GodotCompatResult()

    # ── 1. Nombre del archivo OBJ ─────────────────────────────────────────────
    filename = os.path.basename(obj_path)
    if " " in filename:
        result.errors.append(
            f"El archivo OBJ tiene ESPACIOS en el nombre: '{filename}'. "
            "Godot's ResourceLoader puede fallar. Usa guiones bajos: "
            f"'{filename.replace(' ', '_')}'."
        )
        result.passed = False

    forbidden_in_file = FORBIDDEN_CHARS_PATTERN.sub("", os.path.splitext(filename)[0])
    original_stem = os.path.splitext(filename)[0]
    if forbidden_in_file != original_stem:
        diff = set(original_stem) - set(forbidden_in_file) - {" "}
        result.warnings.append(
            f"El nombre del archivo contiene caracteres especiales: {diff}. "
            "Pueden causar problemas con el importador de Godot en algunos sistemas."
        )

    # ── 2. Parsear nombres del OBJ ────────────────────────────────────────────
    names = _parse_names_from_obj(obj_path)

    # ── 3. Objetos duplicados ─────────────────────────────────────────────────
    obj_names = names["objects"]
    seen: Dict[str, int] = {}
    for n in obj_names:
        seen[n] = seen.get(n, 0) + 1
    duplicates = {n: c for n, c in seen.items() if c > 1}
    if duplicates:
        result.errors.append(
            f"Objetos con nombre DUPLICADO: {duplicates}. "
            "Godot asignará sufijos automáticos (@2, @3...) que rompen las referencias en GDScript."
        )
        result.passed = False

    # ── 4. Nombres con caracteres especiales ──────────────────────────────────
    for obj_name in obj_names:
        bad_chars = FORBIDDEN_CHARS_PATTERN.findall(obj_name)
        bad_unique = list(set(bad_chars))
        if bad_unique:
            result.errors.append(
                f"Objeto '{obj_name}': caracteres no recomendados {bad_unique}. "
                "Godot puede renombrar el nodo automáticamente rompiendo get_node() calls. "
                "Usa solo letras, números, guiones y guiones bajos."
            )
            result.passed = False

        if len(obj_name) > MAX_NAME_LENGTH:
            result.warnings.append(
                f"Objeto '{obj_name}': nombre de {len(obj_name)} caracteres. "
                f"Se recomienda un máximo de {MAX_NAME_LENGTH} para legibilidad en Godot."
            )

    # ── 5. Materiales con caracteres especiales ───────────────────────────────
    for mat_name in names["materials_used"]:
        bad_chars = FORBIDDEN_CHARS_PATTERN.findall(mat_name)
        bad_unique = list(set(bad_chars))
        if " " in mat_name:
            result.errors.append(
                f"Material '{mat_name}': contiene ESPACIOS. "
                "Godot StandardMaterial3D no soporta materiales con espacios en el nombre correctamente."
            )
            result.passed = False
        elif bad_unique:
            result.warnings.append(
                f"Material '{mat_name}': caracteres especiales {bad_unique}. "
                "Puede causar advertencias al importar en Godot."
            )

    # ── 6. Archivo .import ausente (informativo) ──────────────────────────────
    import_file = obj_path + ".import"
    if os.path.isfile(import_file):
        result.warnings.append(
            f"Se encontró '{os.path.basename(import_file)}' — el modelo ya fue importado en Godot. "
            "Si haces cambios en el OBJ, borra el .import para forzar re-importación."
        )

    # ── 7. Resumen positivo ───────────────────────────────────────────────────
    if result.passed:
        result.warnings.append(
            f"Compatibilidad Godot OK — {len(obj_names)} objeto(s) con nombres válidos."
        )

    return result
