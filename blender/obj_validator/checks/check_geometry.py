"""
check_geometry.py — Validaciones geométricas del OBJ con trimesh
----------------------------------------------------------------
Verifica:
  - Triángulos vs N-gons (Godot requiere triángulos)
  - Caras con vértices duplicados o degeneradas (área=0)
  - Normales faltantes o invertidas
  - Escala de bounding box (si el modelo parece fuera de escala)
  - Pivote de origen (si el objeto parece no estar centrado)
"""

from __future__ import annotations
import numpy as np
from dataclasses import dataclass, field
from typing import List
import trimesh


VEHICLE_SIZE_LIMITS = {
    "min_length_m": 0.5,   # Z
    "max_length_m": 10.0,  # Z  — fórmula 1 ~5m
    "min_width_m":  0.2,   # X
    "max_width_m":  3.0,   # X
    "min_height_m": 0.1,   # Y
    "max_height_m": 3.0,   # Y
}


@dataclass
class GeometryCheckResult:
    passed: bool = True
    warnings: List[str] = field(default_factory=list)
    errors:   List[str] = field(default_factory=list)


def check_geometry(mesh: trimesh.Trimesh, obj_name: str = "mesh") -> GeometryCheckResult:
    result = GeometryCheckResult()

    # ── 1. Conteo de caras ───────────────────────────────────────────────────
    n_faces = len(mesh.faces)
    if n_faces == 0:
        result.errors.append(f"[{obj_name}] El mesh no contiene ninguna cara.")
        result.passed = False
        return result

    # ── 2. Caras degeneradas (área == 0) ─────────────────────────────────────
    areas = mesh.area_faces
    degenerate_count = int(np.sum(areas < 1e-9))
    if degenerate_count > 0:
        pct = degenerate_count / n_faces * 100
        result.errors.append(
            f"[{obj_name}] {degenerate_count} caras degeneradas (área≈0), "
            f"el {pct:.1f}% del total ({n_faces} caras). "
            "Ejecuta 'Merge by Distance' y 'Clean Up > Degenerate Dissolve' en Blender."
        )
        result.passed = False

    # ── 3. N-gons (trimesh triangula automáticamente, avisamos si el OBJ tenía quads) ──
    # trimesh siempre produce triángulos; el aviso es informativo
    result.warnings.append(
        f"[{obj_name}] {n_faces} triángulos. "
        "(trimesh triangula automáticamente — asegúrate de exportar con 'Triangulate Faces' en Blender)"
    )

    # ── 4. Normales ──────────────────────────────────────────────────────────
    if not mesh.is_volume:
        result.warnings.append(
            f"[{obj_name}] El mesh NO es watertight (no cierra volumen). "
            "Puede haber huecos, normales invertidas o geometría no cerrada. "
            "Usa 'Recalculate Outside Normals' en Blender."
        )

    # Detectar normales de caras con dirección inconsistente (usando convex hull ratio)
    inconsistent = 0
    try:
        face_normals = mesh.face_normals
        vertex_normals = mesh.vertex_normals
        # Proyectar normales de vértice contra normales de cara vecina promediada
        for fi, face in enumerate(mesh.faces):
            vn_avg = vertex_normals[face].mean(axis=0)
            fn = face_normals[fi]
            if np.dot(vn_avg, fn) < -0.3:
                inconsistent += 1
    except Exception:
        pass

    if inconsistent > 0:
        result.warnings.append(
            f"[{obj_name}] ~{inconsistent} caras con normales potencialmente inconsistentes. "
            "Revisa con 'Face Orientation Overlay' en Blender (las rojas están invertidas)."
        )

    # ── 5. Bounding Box / Escala ─────────────────────────────────────────────
    bounds = mesh.bounds  # [[xmin,ymin,zmin],[xmax,ymax,zmax]]
    size = bounds[1] - bounds[0]  # [W, H, D] == [X, Y, Z]
    width, height, depth = size[0], size[1], size[2]

    lim = VEHICLE_SIZE_LIMITS
    max_dim = max(width, height, depth)

    # Solo disparar error si NINGUNA dimensión llega al mínimo absoluto global.
    # Las piezas pequeñas (alerones, ruedas) pueden tener Z pequeño pero el modelo
    # como conjunto sí tiene la escala correcta.
    if max_dim < lim["min_length_m"] * 0.5:
        result.errors.append(
            f"[{obj_name}] Dimensión máxima = {max_dim:.4f}m — el mesh parece estar "
            f"en escala de maqueta (< {lim['min_length_m']*0.5:.2f}m en todos los ejes). "
            "Aplica escala real en Blender (Ctrl+A > Apply Scale)."
        )
        result.passed = False
    elif max_dim > lim["max_length_m"]:
        result.errors.append(
            f"[{obj_name}] Dimensión máxima = {max_dim:.4f}m — excede el tamaño "
            f"máximo esperado de {lim['max_length_m']}m. "
            "Aplica escala real en Blender."
        )
        result.passed = False
    else:
        result.warnings.append(
            f"[{obj_name}] Dimensiones: X={width:.3f}m Y={height:.3f}m Z={depth:.3f}m — OK"
        )

    # ── 6. Pivote de origen ──────────────────────────────────────────────────
    centroid = mesh.centroid
    origin_offset = np.linalg.norm(centroid)
    if origin_offset > 2.0:
        result.warnings.append(
            f"[{obj_name}] El centroide del mesh está a {origin_offset:.2f}m del origen mundial. "
            "Considera centrar el pivote en Blender (Object > Set Origin > Origin to Geometry)."
        )

    return result

