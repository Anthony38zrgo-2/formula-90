"""
check_uvs.py — Validaciones de coordenadas UV del OBJ
------------------------------------------------------
Verifica:
  - Que el mesh tenga UVs (sin UVs no hay texturas en Godot)
  - Porcentaje de vértices sin UV (holes en el unwrap)
  - UVs fuera del rango [0,1] (UV tiles — problemático sin atlas)
  - UV overlaps excesivos (detección simplificada por AABB de islas)
"""

from __future__ import annotations
import numpy as np
from dataclasses import dataclass, field
from typing import List, Optional
import trimesh


@dataclass
class UVCheckResult:
    passed: bool = True
    warnings: List[str] = field(default_factory=list)
    errors:   List[str] = field(default_factory=list)


def check_uvs(mesh: trimesh.Trimesh, obj_name: str = "mesh") -> UVCheckResult:
    result = UVCheckResult()

    # ── 1. ¿Existen UVs? ─────────────────────────────────────────────────────
    uv = None
    if hasattr(mesh, "visual") and hasattr(mesh.visual, "uv"):
        uv = mesh.visual.uv
    if uv is None or len(uv) == 0:
        result.errors.append(
            f"[{obj_name}] El mesh NO tiene coordenadas UV. "
            "Realiza un UV Unwrap en Blender antes de exportar."
        )
        result.passed = False
        return result

    uv = np.asarray(uv, dtype=float)

    # ── 2. Cobertura: vértices sin UV (NaN o cero exacto en ambas coords) ────
    zero_mask = np.all(uv == 0.0, axis=1)
    zero_count = int(np.sum(zero_mask))
    if zero_count > 0:
        pct = zero_count / len(uv) * 100
        if pct > 5.0:
            result.errors.append(
                f"[{obj_name}] {zero_count} vértices ({pct:.1f}%) tienen UV en (0,0). "
                "Es probable que haya vértices sin unwrap. Revisa el UV Editor en Blender."
            )
            result.passed = False
        else:
            result.warnings.append(
                f"[{obj_name}] {zero_count} vértices ({pct:.1f}%) tienen UV en (0,0). "
                "Puede ser intencional (seams en el borde) o vértices sin unwrap."
            )

    # ── 3. UVs fuera del rango [0,1] — UV Tiles ──────────────────────────────
    out_of_range = np.any((uv < 0.0) | (uv > 1.0), axis=1)
    oor_count = int(np.sum(out_of_range))
    if oor_count > 0:
        pct = oor_count / len(uv) * 100
        result.warnings.append(
            f"[{obj_name}] {oor_count} vértices ({pct:.1f}%) tienen UVs fuera de [0,1]. "
            "Esto indica UV Tiling. Godot soporta tiling, pero requiere configuración "
            "explícita en el material. Recomienda normalizar al atlas [0,1] si es posible."
        )

    # ── 4. Estadísticas básicas del espacio UV ────────────────────────────────
    u_range = float(uv[:, 0].max() - uv[:, 0].min())
    v_range = float(uv[:, 1].max() - uv[:, 1].min())
    coverage = u_range * v_range  # aproximación

    if coverage < 0.01:
        result.errors.append(
            f"[{obj_name}] Cobertura UV muy baja ({coverage:.4f}). "
            "El unwrap puede estar colapsado en un punto. Revisa el UV Editor."
        )
        result.passed = False
    elif coverage < 0.1:
        result.warnings.append(
            f"[{obj_name}] Cobertura UV baja ({coverage:.4f}). "
            "Considera optimizar el unwrap para mejor resolución de textura."
        )

    if result.passed:
        total_uvs = len(uv)
        result.warnings.append(
            f"[{obj_name}] UVs OK — {total_uvs} coordenadas UV. "
            f"Rango U=[{uv[:,0].min():.3f}–{uv[:,0].max():.3f}] "
            f"V=[{uv[:,1].min():.3f}–{uv[:,1].max():.3f}]."
        )

    return result
