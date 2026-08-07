"""
fix_symmetry.py — Corrección de simetría en eje X
--------------------------------------------------
Estrategias disponibles:
  - mirror_right_to_left: Toma el lado X>0 y lo espeja como espejo en X<0
  - mirror_left_to_right: Toma el lado X<0 y lo espeja en X>0
  - auto:                 Detecta cuál lado tiene más vértices y lo usa como fuente

Opera sobre sub-objetos individuales o sobre el modelo completo.

Nota: La simetría perfecta se consigue duplicando vértices del lado fuente
y negando su coordenada X. Las caras del lado espejo se generan con el
orden de vértices invertido (para mantener las normales hacia afuera).
"""

from __future__ import annotations

import numpy as np
from typing import Dict, Any, Optional, List

from .obj_io import ObjFile, ObjObject, ObjFace


def analyze_symmetry(obj_file: ObjFile, obj_name: Optional[str] = None) -> Dict[str, Any]:
    """
    Analiza cuán simétrico es el modelo en X.
    Devuelve un score de 0 (totalmente asimétrico) a 1 (perfectamente simétrico).
    """
    if obj_name:
        obj = obj_file.get_object(obj_name)
        if obj is None:
            return {"error": f"Objeto '{obj_name}' no encontrado."}
        used_v = set()
        for face in obj.faces:
            for v_idx, _, _ in face.raw_indices:
                used_v.add(v_idx - 1)
        verts = obj_file.vertices[list(used_v)]
    else:
        verts = obj_file.vertices

    if len(verts) == 0:
        return {"error": "No hay vértices para analizar."}

    # Centro en X
    cx = float(verts[:, 0].mean())
    left_verts  = verts[verts[:, 0] < cx]
    right_verts = verts[verts[:, 0] >= cx]

    n_left  = len(left_verts)
    n_right = len(right_verts)

    # Distancia de Hausdorff simplificada: para cada vértice del lado derecho,
    # buscar el más cercano en el lado izquierdo espejado
    if n_left == 0 or n_right == 0:
        return {
            "n_left": n_left,
            "n_right": n_right,
            "symmetry_score": 0.0,
            "center_x": cx,
            "recommendation": "Un lado no tiene vértices. Modelo no simétrico.",
        }

    right_mirrored = right_verts.copy()
    right_mirrored[:, 0] = 2 * cx - right_mirrored[:, 0]

    # Score: % de vértices espejados con un match cercano en el lado izquierdo
    threshold = float(np.linalg.norm(verts.max(axis=0) - verts.min(axis=0))) * 0.02  # 2% del tamaño
    matches = 0
    for rv in right_mirrored:
        dists = np.linalg.norm(left_verts - rv, axis=1)
        if dists.min() <= threshold:
            matches += 1

    score = matches / len(right_mirrored)

    rec = "Simetria correcta" if score > 0.85 else \
          "Simetria parcial — considera aplicar mirror" if score > 0.4 else \
          "Asimetrico — se recomienda mirror desde el lado con mas geometria"

    return {
        "n_left": n_left,
        "n_right": n_right,
        "symmetry_score": round(score, 3),
        "center_x": round(cx, 6),
        "threshold_m": round(threshold, 6),
        "recommendation": rec,
    }


def force_symmetry(
    obj_file: ObjFile,
    obj_name: str,
    strategy: str = "auto",
    center_x: Optional[float] = None,
) -> Dict[str, Any]:
    """
    Fuerza simetría perfecta en un sub-objeto.

    strategy:
        'mirror_right_to_left' — el lado X>center_x se espeja al lado izquierdo
        'mirror_left_to_right' — el lado X<center_x se espeja al lado derecho
        'auto'                 — elige el lado con más vértices como fuente

    Devuelve stats del proceso.
    """
    obj = obj_file.get_object(obj_name)
    if obj is None:
        return {"error": f"Objeto '{obj_name}' no encontrado."}

    # Recoger vértices usados por este objeto
    used_v_indices = set()
    for face in obj.faces:
        for v_idx, _, _ in face.raw_indices:
            used_v_indices.add(v_idx - 1)  # 0-based

    if not used_v_indices:
        return {"error": "El objeto no tiene vértices referenciados."}

    verts_local = {idx: obj_file.vertices[idx].copy() for idx in used_v_indices}

    cx = center_x if center_x is not None else float(np.mean([v[0] for v in verts_local.values()]))

    left_indices  = {i for i, v in verts_local.items() if v[0] < cx}
    right_indices = {i for i, v in verts_local.items() if v[0] >= cx}

    if strategy == "auto":
        strategy = "mirror_right_to_left" if len(right_indices) >= len(left_indices) \
                   else "mirror_left_to_right"

    if strategy == "mirror_right_to_left":
        source_indices = right_indices
    else:
        source_indices = left_indices

    # Crear vértices espejados
    new_v_remap: Dict[int, int] = {}  # source_global_0based → new_global_0based
    new_verts_list: List[np.ndarray] = []
    base_offset = len(obj_file.vertices)

    for src_idx in sorted(source_indices):
        src_v = obj_file.vertices[src_idx].copy()
        mirrored = src_v.copy()
        mirrored[0] = 2 * cx - src_v[0]  # espejo en X respecto al centro
        new_v_remap[src_idx] = base_offset + len(new_verts_list)
        new_verts_list.append(mirrored)

    # Agregar al pool global
    if new_verts_list:
        obj_file.vertices = np.vstack([obj_file.vertices, np.array(new_verts_list)])

    # Generar caras espejadas
    # Identificar caras del lado fuente
    mirrored_faces: List[ObjFace] = []
    for face in obj.faces:
        # Ver si todos los vértices de la cara son del lado fuente
        face_vs = [vi for vi, _, _ in face.raw_indices]
        face_v_set = {v - 1 for v in face_vs}
        if not face_v_set.issubset(source_indices):
            continue

        # Crear cara espejada con vértices nuevos e invertir orden (para flip normal)
        new_face = ObjFace(material=face.material)
        reversed_indices = list(reversed(face.raw_indices))
        for v_idx, vt_idx, vn_idx in reversed_indices:
            new_global = new_v_remap.get(v_idx - 1, None)
            if new_global is None:
                new_global = v_idx - 1
            new_face.raw_indices.append((new_global + 1, vt_idx, 0))  # sin normal (se recalcula)
        mirrored_faces.append(new_face)

    # Eliminar caras del lado destino originales y reemplazar con las espejadas
    if strategy == "mirror_right_to_left":
        dest_indices = left_indices
    else:
        dest_indices = right_indices

    kept_faces = []
    removed_dest = 0
    for face in obj.faces:
        face_v_set = {v - 1 for v, _, _ in face.raw_indices}
        if face_v_set.issubset(dest_indices):
            removed_dest += 1
        else:
            kept_faces.append(face)

    obj.faces = kept_faces + mirrored_faces

    return {
        "object": obj_name,
        "strategy": strategy,
        "center_x": round(cx, 6),
        "source_verts": len(source_indices),
        "mirrored_new_verts": len(new_verts_list),
        "dest_faces_removed": removed_dest,
        "mirrored_faces_added": len(mirrored_faces),
    }
