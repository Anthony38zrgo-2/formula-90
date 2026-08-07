"""
fix_geometry.py — Limpieza y reparación de geometría OBJ
---------------------------------------------------------
Operaciones:
  - Eliminar caras degeneradas (área ≈ 0)
  - Merge by distance: fusionar vértices duplicados/cercanos
  - Recalcular normales por cara
  - Invertir normales si el volumen parece estar al revés
  - Opcionalmente: decimate (simplificación quadric via trimesh)
"""

from __future__ import annotations

import numpy as np
from typing import Dict, List, Any
import trimesh

from .obj_io import ObjFile, ObjObject, ObjFace


# ──────────────────────────────────────────────────────────────────────────────

def _face_area(v0: np.ndarray, v1: np.ndarray, v2: np.ndarray) -> float:
    edge1 = v1 - v0
    edge2 = v2 - v0
    cross = np.cross(edge1, edge2)
    return float(np.linalg.norm(cross)) * 0.5


def remove_degenerate_faces(obj_file: ObjFile, area_threshold: float = 1e-9) -> Dict[str, Any]:
    """Elimina caras con área <= area_threshold de todos los objetos."""
    total_removed = 0
    per_object = {}

    for obj in obj_file.objects:
        removed = 0
        clean_faces: List[ObjFace] = []
        for face in obj.faces:
            if len(face.raw_indices) < 3:
                removed += 1
                continue
            v0 = obj_file.vertices[face.raw_indices[0][0] - 1]
            v1 = obj_file.vertices[face.raw_indices[1][0] - 1]
            v2 = obj_file.vertices[face.raw_indices[2][0] - 1]
            if _face_area(v0, v1, v2) <= area_threshold:
                removed += 1
            else:
                clean_faces.append(face)
        obj.faces = clean_faces
        per_object[obj.name] = removed
        total_removed += removed

    return {"total_degenerate_removed": total_removed, "per_object": per_object}


def merge_vertices_by_distance(obj_file: ObjFile, threshold: float = 1e-5) -> Dict[str, Any]:
    """
    Fusiona vértices que están a menos de 'threshold' metros entre sí.
    Redirige los índices de cara al representante del cluster.

    Nota: Opera sobre el pool GLOBAL de vértices.
    """
    if len(obj_file.vertices) == 0:
        return {"merged": 0, "original": 0, "final": 0}

    n = len(obj_file.vertices)
    representative = list(range(n))  # representative[i] = índice canónico

    # Algoritmo greedy con tolerancia
    merged = 0
    for i in range(n):
        if representative[i] != i:
            continue
        for j in range(i + 1, n):
            if representative[j] != j:
                continue
            dist = np.linalg.norm(obj_file.vertices[i] - obj_file.vertices[j])
            if dist <= threshold:
                representative[j] = i
                merged += 1

    # Remap face indices (OBJ usa 1-based)
    for obj in obj_file.objects:
        for face in obj.faces:
            new_indices = []
            for v_idx, vt_idx, vn_idx in face.raw_indices:
                new_v = representative[v_idx - 1] + 1  # 1-based
                new_indices.append((new_v, vt_idx, vn_idx))
            face.raw_indices = new_indices

    return {"merged": merged, "original": n, "final": n - merged}


def recalculate_normals(obj_file: ObjFile) -> Dict[str, Any]:
    """
    Recalcula todas las normales del archivo por cara.
    Reemplaza el pool global de normales y actualiza índices.
    """
    new_normals: List[np.ndarray] = []
    normal_map: Dict[bytes, int] = {}

    def get_or_add_normal(n: np.ndarray) -> int:
        key = n.tobytes()
        if key not in normal_map:
            normal_map[key] = len(new_normals) + 1  # 1-based
            new_normals.append(n)
        return normal_map[key]

    total_updated = 0
    for obj in obj_file.objects:
        for face in obj.faces:
            if len(face.raw_indices) < 3:
                continue
            v0 = obj_file.vertices[face.raw_indices[0][0] - 1]
            v1 = obj_file.vertices[face.raw_indices[1][0] - 1]
            v2 = obj_file.vertices[face.raw_indices[2][0] - 1]
            edge1 = v1 - v0
            edge2 = v2 - v0
            normal = np.cross(edge1, edge2)
            norm_len = np.linalg.norm(normal)
            if norm_len > 1e-9:
                normal = normal / norm_len
            else:
                normal = np.array([0.0, 1.0, 0.0])

            n_idx = get_or_add_normal(normal)
            new_raw = []
            for v_idx, vt_idx, _ in face.raw_indices:
                new_raw.append((v_idx, vt_idx, n_idx))
            face.raw_indices = new_raw
            total_updated += 1

    obj_file.normals = np.array(new_normals) if new_normals else np.empty((0, 3))
    return {"faces_updated": total_updated, "new_normal_count": len(new_normals)}


def decimate_object(obj_file: ObjFile, obj_name: str, target_ratio: float = 0.5) -> Dict[str, Any]:
    """
    Reduce los polígonos de un sub-objeto usando simplificación quadric de trimesh.
    target_ratio: fracción de caras a conservar (0.5 = 50%)

    IMPORTANTE: Esta operación re-indexa vértices y caras del objeto.
    Las referencias al pool global son reconstruidas tras la decimación.
    """
    obj = obj_file.get_object(obj_name)
    if obj is None:
        return {"error": f"Objeto '{obj_name}' no encontrado."}
    if not obj.faces:
        return {"error": f"Objeto '{obj_name}' no tiene caras."}

    # Construir mesh local
    v_map: Dict[int, int] = {}  # global_1based → local_0based
    local_verts: List[np.ndarray] = []

    for face in obj.faces:
        for v_idx, _, _ in face.raw_indices:
            if v_idx not in v_map:
                v_map[v_idx] = len(local_verts)
                local_verts.append(obj_file.vertices[v_idx - 1].copy())

    local_verts_arr = np.array(local_verts)
    local_faces: List[List[int]] = []
    for face in obj.faces:
        if len(face.raw_indices) >= 3:
            tri = [v_map[face.raw_indices[i][0]] for i in range(3)]
            local_faces.append(tri)

    if not local_faces:
        return {"error": "No se pudieron construir caras locales."}

    mesh = trimesh.Trimesh(vertices=local_verts_arr, faces=np.array(local_faces), process=False)
    original_face_count = len(mesh.faces)
    target_faces = max(4, int(original_face_count * target_ratio))

    simplified = mesh.simplify_quadric_decimation(target_faces)
    if simplified is None or len(simplified.faces) == 0:
        return {"error": "La simplificación retornó un mesh vacío."}

    # Integrar vértices decimados al pool global
    offset = len(obj_file.vertices)
    new_verts = np.vstack([obj_file.vertices, simplified.vertices])
    obj_file.vertices = new_verts

    # Reconstruir caras del objeto
    new_faces: List[ObjFace] = []
    mat = obj.faces[0].material if obj.faces else ""
    for tri in simplified.faces:
        face = ObjFace(material=mat)
        for local_idx in tri:
            global_idx = offset + local_idx + 1  # 1-based
            face.raw_indices.append((global_idx, 0, 0))
        new_faces.append(face)

    obj.faces = new_faces
    final_face_count = len(new_faces)

    return {
        "object": obj_name,
        "original_faces": original_face_count,
        "final_faces": final_face_count,
        "reduction_pct": (1 - final_face_count / original_face_count) * 100,
    }
