"""
fix_names.py — Saneado de nombres de objetos y materiales
----------------------------------------------------------
Problemas que corrige:
  - Espacios en nombres de objeto → reemplaza por _
  - Caracteres especiales → elimina o reemplaza
  - Nombres duplicados → añade sufijo _2, _3...
  - Nombres demasiado largos → trunca a MAX_NAME_LENGTH
  - Actualiza referencias cruzadas entre .obj y .mtl de forma consistente
"""

from __future__ import annotations

import os
import re
from typing import Dict, Any, List, Tuple

from .obj_io import ObjFile

MAX_NAME_LENGTH = 40
SAFE_CHAR_RE = re.compile(r'[^a-zA-Z0-9_\-\.]')


def _sanitize(name: str) -> str:
    """Convierte un nombre en uno seguro para Godot."""
    # Reemplazar espacios por _
    name = name.replace(" ", "_")
    # Eliminar caracteres no permitidos
    name = SAFE_CHAR_RE.sub("", name)
    # Truncar si es necesario
    if len(name) > MAX_NAME_LENGTH:
        name = name[:MAX_NAME_LENGTH]
    # Si quedó vacío, poner nombre genérico
    if not name:
        name = "object"
    return name


def fix_names(obj_file: ObjFile) -> Dict[str, Any]:
    """
    Sanea todos los nombres de objetos en el ObjFile.
    Devuelve un mapa de renombrados: {old_name: new_name}
    """
    renames: Dict[str, str] = {}
    used_names: Dict[str, int] = {}

    for obj in obj_file.objects:
        original = obj.name
        sanitized = _sanitize(original)

        # Resolver duplicados
        if sanitized in used_names:
            used_names[sanitized] += 1
            sanitized = f"{sanitized}_{used_names[sanitized]}"
        else:
            used_names[sanitized] = 1

        if sanitized != original:
            renames[original] = sanitized
            obj.name = sanitized

    # Sanear nombres de materiales en las caras
    mat_renames: Dict[str, str] = {}
    for obj in obj_file.objects:
        for face in obj.faces:
            if face.material and face.material not in mat_renames:
                sanitized_mat = _sanitize(face.material)
                if sanitized_mat != face.material:
                    mat_renames[face.material] = sanitized_mat
            if face.material in mat_renames:
                face.material = mat_renames[face.material]

    return {
        "object_renames": renames,
        "material_renames": mat_renames,
        "total_renamed": len(renames) + len(mat_renames),
    }


def fix_mtl_names(mtl_path: str, mat_renames: Dict[str, str]) -> Dict[str, Any]:
    """
    Aplica los renombrados de materiales al archivo .mtl.
    Opera directamente sobre el archivo en disco.
    """
    if not os.path.isfile(mtl_path) or not mat_renames:
        return {"updated": 0}

    with open(mtl_path, "r", encoding="utf-8", errors="replace") as f:
        lines = f.readlines()

    updated = 0
    new_lines = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("newmtl "):
            mat_name = stripped[7:].strip()
            if mat_name in mat_renames:
                new_name = mat_renames[mat_name]
                line = line.replace(mat_name, new_name, 1)
                updated += 1
        new_lines.append(line)

    if updated > 0:
        with open(mtl_path, "w", encoding="utf-8") as f:
            f.writelines(new_lines)

    return {"mtl_path": mtl_path, "updated": updated}
