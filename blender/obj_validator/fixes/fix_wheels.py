"""
fix_wheels.py — Detección, centrado y renombrado de ruedas para GEVP
---------------------------------------------------------------------
Para Godot Easy Vehicle Physics, las 4 ruedas deben:
  1. Ser sub-objetos independientes
  2. Tener su pivote (origen) en el centro geométrico de la rueda
  3. Nombrarse de forma estándar: wheel_fl, wheel_fr, wheel_rl, wheel_rr

Este módulo:
  - Detecta ruedas por keywords en el nombre del objeto
  - Clasifica FL/FR/RL/RR por posición X (izq/der) y Z (delantero/trasero)
  - Centra el pivote de cada rueda en su centroide
  - Renombra los objetos
"""

from __future__ import annotations

import numpy as np
from typing import Dict, Any, List, Optional, Tuple

from .obj_io import ObjFile, ObjObject


PRIMARY_WHEEL_KEYWORDS = ["wheel", "rueda", "tire", "tyre", "neumatico", "neumático"]
NON_WHEEL_KEYWORDS = ["aleron", "wing", "spoiler", "chasis", "chassis", "body", "carroceria"]

STANDARD_NAMES = {
    "fl": "wheel_fl",  # front left  (X < 0, Z > 0)
    "fr": "wheel_fr",  # front right (X > 0, Z > 0)
    "rl": "wheel_rl",  # rear left   (X < 0, Z < 0)
    "rr": "wheel_rr",  # rear right  (X > 0, Z < 0)
}

def _is_wheel(name: str) -> bool:
    name_lower = name.lower()
    if any(nw in name_lower for nw in NON_WHEEL_KEYWORDS):
        return False
    return any(kw in name_lower for kw in PRIMARY_WHEEL_KEYWORDS)


def _classify_wheel(centroid: np.ndarray, global_center: np.ndarray) -> str:
    """
    Clasifica la rueda en FL/FR/RL/RR basándose en la posición relativa al centro global.
    Convención Blender/Godot:
      X > center → derecha (right)
      X < center → izquierda (left)
      Z < center → delantero (front) en Godot la Z negativa es hacia adelante
      Z > center → trasero  (rear)
    """
    is_right = centroid[0] > global_center[0]
    is_front = centroid[2] > global_center[2]

    if is_front and not is_right:
        return "fl"
    elif is_front and is_right:
        return "fr"
    elif not is_front and not is_right:
        return "rl"
    else:
        return "rr"


def analyze_wheels(obj_file: ObjFile) -> Dict[str, Any]:
    """
    Detecta las ruedas y devuelve su clasificación actual.
    """
    wheel_objs = [o for o in obj_file.objects if _is_wheel(o.name)]

    if not wheel_objs:
        return {
            "wheels_found": 0,
            "error": "No se detectaron objetos con keywords de rueda.",
            "all_objects": [o.name for o in obj_file.objects],
        }

    # Centro global del modelo
    global_center = obj_file.vertices.mean(axis=0) if len(obj_file.vertices) > 0 else np.zeros(3)

    wheels_info = []
    for w in wheel_objs:
        centroid = obj_file.centroid_of(w)
        vmin, vmax = obj_file.bounding_box_of(w)
        size = vmax - vmin
        slot = _classify_wheel(centroid, global_center)
        wheels_info.append({
            "name": w.name,
            "centroid": centroid.round(4).tolist(),
            "bbox_size": size.round(4).tolist(),
            "classified_as": slot,
            "standard_name": STANDARD_NAMES[slot],
            "face_count": w.face_count(),
        })

    return {
        "wheels_found": len(wheel_objs),
        "global_center": global_center.round(4).tolist(),
        "wheels": wheels_info,
    }


def fix_wheels(obj_file: ObjFile, center_pivots: bool = True, rename: bool = True) -> Dict[str, Any]:
    """
    Aplica correcciones a las ruedas:
      - center_pivots: mueve los vértices de cada rueda para que el centroide sea el origen
      - rename: renombra a wheel_fl/fr/rl/rr según posición

    Devuelve un resumen de los cambios.
    """
    wheel_objs = [o for o in obj_file.objects if _is_wheel(o.name)]

    if not wheel_objs:
        return {"error": "No se detectaron ruedas.", "wheels_found": 0}

    global_center = obj_file.vertices.mean(axis=0) if len(obj_file.vertices) > 0 else np.zeros(3)

    results = []
    used_slots: Dict[str, str] = {}  # slot → nombre original

    for w in wheel_objs:
        centroid = obj_file.centroid_of(w)
        slot = _classify_wheel(centroid, global_center)

        # Resolver conflictos: si el slot ya fue asignado, añadir sufijo
        final_slot = slot
        if slot in used_slots:
            # Puede haber sub-partes de la misma rueda; usar sufijo
            final_slot = f"{slot}_part{len([s for s in used_slots if s.startswith(slot)])}"
        used_slots[final_slot] = w.name

        info = {
            "original_name": w.name,
            "slot": final_slot,
            "centroid_before": centroid.round(4).tolist(),
        }

        if center_pivots:
            delta = obj_file.center_object_origin(w)
            info["pivot_centered"] = True
            info["delta_applied"] = delta.round(4).tolist()
            info["centroid_after"] = obj_file.centroid_of(w).round(4).tolist()

        new_name = STANDARD_NAMES.get(final_slot, f"wheel_{final_slot}")
        if rename:
            obj_file.rename_object(w.name, new_name)
            info["renamed_to"] = new_name

        results.append(info)

    return {
        "wheels_processed": len(results),
        "details": results,
    }
