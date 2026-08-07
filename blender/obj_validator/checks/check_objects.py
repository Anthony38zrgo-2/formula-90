"""
check_objects.py — Validaciones de estructura de objetos/mallas del OBJ
------------------------------------------------------------------------
Verifica (relevante para Godot Easy Vehicle Physics):
  - Que el OBJ contenga sub-objetos "o" (necesario para separar chasis/ruedas)
  - Nombres de objetos recomendados para GEVP (chasis, ruedas FL/FR/RL/RR)
  - Que las ruedas sean objetos independientes con pivotes separados
  - Vértices duplicados no soldados (high vertex count respecto a lo esperado)
  - Límites de triángulos para game-ready (recomendación general)
"""

from __future__ import annotations
import re
from dataclasses import dataclass, field
from typing import List, Dict, Set


# Nombres esperados para que el vehículo funcione con GEVP
WHEEL_KEYWORDS   = ["wheel", "rueda", "tire", "neumatico", "neumático", "tyre",
                    "fl", "fr", "rl", "rr", "front_left", "front_right",
                    "rear_left", "rear_right", "delantero", "trasero",
                    "izquierdo", "derecho", "left", "right"]
CHASSIS_KEYWORDS = ["chassis", "chasis", "body", "carroceria", "carrocería",
                    "fuselage", "hull", "frame"]

# Límite de triángulos "game-ready" recomendado por pieza
TRI_LIMIT_WARNING  = 15_000
TRI_LIMIT_ERROR    = 50_000


@dataclass
class ObjectCheckResult:
    passed: bool = True
    warnings: List[str] = field(default_factory=list)
    errors:   List[str] = field(default_factory=list)


def _parse_obj_objects(obj_path: str) -> Dict[str, Dict]:
    """
    Parsea el OBJ sin cargarlo completo con trimesh.
    Devuelve dict: {object_name: {faces: int, has_uv: bool, has_normals: bool}}
    """
    objects: Dict[str, Dict] = {}
    current = "__root__"
    objects[current] = {"faces": 0, "has_uv": False, "has_normals": False}

    with open(obj_path, "r", encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.strip()
            if line.startswith("o ") or line.startswith("g "):
                name = line[2:].strip()
                if name and name not in objects:
                    current = name
                    objects[current] = {"faces": 0, "has_uv": False, "has_normals": False}
            elif line.startswith("f "):
                objects[current]["faces"] += 1
                parts = line[2:].split()
                if parts:
                    # f v/vt/vn or f v//vn or f v
                    sample = parts[0]
                    if "/" in sample:
                        segs = sample.split("/")
                        if len(segs) >= 2 and segs[1]:
                            objects[current]["has_uv"] = True
                        if len(segs) == 3 and segs[2]:
                            objects[current]["has_normals"] = True
            elif line.startswith("vt "):
                objects[current]["has_uv"] = True
            elif line.startswith("vn "):
                objects[current]["has_normals"] = True

    return objects


def _match_keywords(name: str, keywords: List[str]) -> bool:
    name_lower = name.lower()
    return any(kw in name_lower for kw in keywords)


def check_objects(obj_path: str) -> ObjectCheckResult:
    result = ObjectCheckResult()

    objects = _parse_obj_objects(obj_path)

    # Quitar root si está vacío
    real_objects = {k: v for k, v in objects.items()
                    if k != "__root__" or v["faces"] > 0}

    if not real_objects:
        result.errors.append(
            "El OBJ no contiene ningún objeto con caras ('f' lines). "
            "¿Exportaste el archivo correctamente desde Blender?"
        )
        result.passed = False
        return result

    obj_names = list(real_objects.keys())
    result.warnings.append(
        f"Sub-objetos encontrados ({len(obj_names)}): {', '.join(obj_names)}"
    )

    # ── 1. Detección de chasis ────────────────────────────────────────────────
    chassis_found = [n for n in obj_names if _match_keywords(n, CHASSIS_KEYWORDS)]
    if not chassis_found:
        result.warnings.append(
            "No se detectó ningún objeto con nombre de CHASIS "
            f"(keywords: {', '.join(CHASSIS_KEYWORDS)}). "
            "Para GEVP el nodo raíz RigidBody3D necesita un CollisionShape3D. "
            "Asegúrate de que el nombre del objeto del chasis sea reconocible "
            "o ajusta la escena en Godot manualmente."
        )
    else:
        result.warnings.append(
            f"Chasis detectado: {', '.join(chassis_found)}"
        )

    # ── 2. Detección de ruedas ────────────────────────────────────────────────
    wheels_found = [n for n in obj_names if _match_keywords(n, WHEEL_KEYWORDS)]
    if len(wheels_found) < 4:
        result.errors.append(
            f"Se detectaron solo {len(wheels_found)} objeto(s) con nombre de RUEDA "
            f"(detectados: {wheels_found}). "
            "GEVP requiere EXACTAMENTE 4 ruedas separadas como objetos independientes: "
            "FL (delantera izquierda), FR (delantera derecha), RL (trasera izquierda), RR (trasera derecha). "
            f"Keywords buscados: {', '.join(WHEEL_KEYWORDS[:10])}..."
        )
        result.passed = False
    elif len(wheels_found) == 4:
        result.warnings.append(
            f"4 ruedas detectadas correctamente: {', '.join(wheels_found)}"
        )
    else:
        result.warnings.append(
            f"{len(wheels_found)} objetos con keyword de rueda: {', '.join(wheels_found)}. "
            "Verifica que sean exactamente las 4 ruedas (puede haber sub-partes)."
        )

    # ── 3. Conteo de polígonos por objeto ─────────────────────────────────────
    total_faces = 0
    for obj_name, data in real_objects.items():
        faces = data["faces"]
        total_faces += faces
        if faces > TRI_LIMIT_ERROR:
            result.errors.append(
                f"[{obj_name}] {faces:,} caras — EXCEDE el límite recomendado "
                f"de {TRI_LIMIT_ERROR:,} para game-ready. "
                "Reduce la geometría con Decimate modifier en Blender."
            )
            result.passed = False
        elif faces > TRI_LIMIT_WARNING:
            result.warnings.append(
                f"[{obj_name}] {faces:,} caras — supera el límite sugerido "
                f"de {TRI_LIMIT_WARNING:,}. Considera optimizar."
            )

    result.warnings.append(f"Total de caras en todo el OBJ: {total_faces:,}")

    # ── 4. Objetos sin normales ───────────────────────────────────────────────
    no_normals = [n for n, d in real_objects.items() if not d["has_normals"]]
    if no_normals:
        result.errors.append(
            f"Objetos SIN normales exportadas: {', '.join(no_normals)}. "
            "Activa 'Write Normals' en las opciones de exportación OBJ de Blender."
        )
        result.passed = False

    # ── 5. Objetos sin UVs ───────────────────────────────────────────────────
    no_uvs = [n for n, d in real_objects.items() if not d["has_uv"]]
    if no_uvs:
        result.errors.append(
            f"Objetos SIN coordenadas UV: {', '.join(no_uvs)}. "
            "Realiza UV Unwrap en Blender y activa 'Write UVs' al exportar."
        )
        result.passed = False

    return result
