"""
fix_scale.py — Corrección de escala del modelo OBJ
----------------------------------------------------
Modos:
  - auto:   Calcula el factor comparando el bounding box del modelo
            contra las dimensiones reales de un F1 (por defecto).
  - factor: Multiplica todos los vértices por el factor dado.
  - target_length: Escala para que el eje Z (largo) mida N metros.

La escala se aplica al pool global de vértices del ObjFile.
"""

from __future__ import annotations

import numpy as np
from typing import Optional, Tuple

from .obj_io import ObjFile

# Longitud real aproximada de un F1 en metros (eje Z en Blender = profundidad)
F1_REAL_LENGTH_M = 5.0
F1_REAL_WIDTH_M  = 2.0
F1_REAL_HEIGHT_M = 1.0


def compute_global_bbox(obj_file: ObjFile) -> Tuple[np.ndarray, np.ndarray]:
    """Devuelve (min_xyz, max_xyz) de todos los vértices del archivo."""
    if len(obj_file.vertices) == 0:
        return np.zeros(3), np.zeros(3)
    return obj_file.vertices.min(axis=0), obj_file.vertices.max(axis=0)


def fix_scale(
    obj_file: ObjFile,
    mode: str = "auto",
    factor: float = 1.0,
    target_length_m: float = F1_REAL_LENGTH_M,
    axis: str = "max",
) -> dict:
    """
    Aplica corrección de escala al ObjFile in-place.

    Parámetros:
        mode           : 'auto' | 'factor' | 'target'
        factor         : factor manual (solo si mode='factor')
        target_length_m: longitud objetivo en metros (solo si mode='target')
        axis           : 'max' usa el eje más largo, 'z' usa Z, 'x' usa X

    Devuelve:
        dict con: factor_applied, bbox_before, bbox_after
    """
    vmin, vmax = compute_global_bbox(obj_file)
    size = vmax - vmin
    bbox_before = {"min": vmin.tolist(), "max": vmax.tolist(), "size": size.tolist()}

    if mode == "factor":
        scale_factor = factor

    elif mode == "auto":
        # Calcula el factor tomando el eje más largo y comparando con F1 real
        if axis == "max":
            current_max_dim = float(size.max())
            target = max(F1_REAL_LENGTH_M, F1_REAL_WIDTH_M, F1_REAL_HEIGHT_M)
        elif axis == "z":
            current_max_dim = float(abs(size[2]))
            target = target_length_m
        elif axis == "x":
            current_max_dim = float(abs(size[0]))
            target = F1_REAL_WIDTH_M
        else:
            current_max_dim = float(size.max())
            target = target_length_m

        if current_max_dim < 1e-9:
            return {"error": "Bounding box con dimension cero, no se puede escalar.", "factor_applied": 1.0}

        scale_factor = target / current_max_dim

    elif mode == "target":
        current_len = float(abs(size[2]))  # largo en Z
        if current_len < 1e-9:
            return {"error": "Largo Z es cero, imposible escalar.", "factor_applied": 1.0}
        scale_factor = target_length_m / current_len

    else:
        return {"error": f"Modo desconocido: {mode}", "factor_applied": 1.0}

    # Aplicar escala
    obj_file.apply_scale(scale_factor)

    vmin2, vmax2 = compute_global_bbox(obj_file)
    size2 = vmax2 - vmin2

    return {
        "factor_applied": scale_factor,
        "bbox_before": bbox_before,
        "bbox_after": {
            "min": vmin2.tolist(),
            "max": vmax2.tolist(),
            "size": size2.tolist(),
        },
        "mode": mode,
    }
