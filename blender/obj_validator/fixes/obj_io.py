"""
obj_io.py — Parser y Writer OBJ que preserva la estructura completa
--------------------------------------------------------------------
trimesh pierde sub-objetos (grupos 'o') al cargar. Este módulo
mantiene la estructura fiel del archivo y permite modificar cada
sub-mesh independientemente antes de re-exportar.

Estructura interna:
    ObjFile
    ├── header: List[str]          — líneas de comentario iniciales
    ├── mtl_files: List[str]       — librerías de material referenciadas
    └── objects: List[ObjObject]
            ├── name: str
            ├── vertices: np.ndarray   shape (N, 3)  X Y Z
            ├── normals:  np.ndarray   shape (M, 3)
            ├── uvs:      np.ndarray   shape (K, 2)
            └── faces: List[ObjFace]
                    ├── material: str
                    └── indices: List[Tuple[int,int,int]]  (v, vt, vn) 0-based

Nota: Los índices de vértice en OBJ son GLOBALES y 1-based.
      Internamente los almacenamos como 0-based locales por objeto.
"""

from __future__ import annotations

import os
import re
from dataclasses import dataclass, field
from typing import List, Optional, Tuple, Dict

import numpy as np


# ──────────────────────────────────────────────────────────────────────────────

@dataclass
class ObjFace:
    material: str = ""
    # Cada elemento es (v_idx, vt_idx, vn_idx) — índices GLOBALES 1-based del OBJ original
    raw_indices: List[Tuple[int, int, int]] = field(default_factory=list)


@dataclass
class ObjObject:
    name: str = "__root__"
    # Vértices globales referenciados por este objeto (índice global 1-based → local 0-based)
    _v_global: List[int] = field(default_factory=list)   # lista de índices globales usados
    _vn_global: List[int] = field(default_factory=list)
    _vt_global: List[int] = field(default_factory=list)
    faces: List[ObjFace] = field(default_factory=list)
    current_material: str = ""

    def face_count(self) -> int:
        return len(self.faces)


@dataclass
class ObjFile:
    """Representación en memoria de un archivo .obj completo."""
    source_path: str = ""
    header: List[str] = field(default_factory=list)
    mtl_files: List[str] = field(default_factory=list)

    # Pools globales (indexados 0-based internamente)
    vertices:  np.ndarray = field(default_factory=lambda: np.empty((0, 3)))
    normals:   np.ndarray = field(default_factory=lambda: np.empty((0, 3)))
    uvs:       np.ndarray = field(default_factory=lambda: np.empty((0, 2)))

    objects: List[ObjObject] = field(default_factory=list)

    # ── Consultas ─────────────────────────────────────────────────────────────

    def object_names(self) -> List[str]:
        return [o.name for o in self.objects]

    def get_object(self, name: str) -> Optional[ObjObject]:
        for o in self.objects:
            if o.name == name:
                return o
        return None

    def vertices_of(self, obj: ObjObject) -> np.ndarray:
        """Devuelve los vértices únicos usados por el objeto (orden de aparición)."""
        seen = {}
        out = []
        for face in obj.faces:
            for v_idx, vt_idx, vn_idx in face.raw_indices:
                if v_idx not in seen:
                    seen[v_idx] = len(out)
                    out.append(self.vertices[v_idx - 1])
        return np.array(out) if out else np.empty((0, 3))

    def bounding_box_of(self, obj: ObjObject) -> Tuple[np.ndarray, np.ndarray]:
        """Devuelve (min_xyz, max_xyz) del objeto."""
        verts = self.vertices_of(obj)
        if len(verts) == 0:
            return np.zeros(3), np.zeros(3)
        return verts.min(axis=0), verts.max(axis=0)

    def centroid_of(self, obj: ObjObject) -> np.ndarray:
        verts = self.vertices_of(obj)
        if len(verts) == 0:
            return np.zeros(3)
        return verts.mean(axis=0)

    # ── Modificaciones in-place ───────────────────────────────────────────────

    def apply_scale(self, factor: float) -> None:
        """Escala todos los vértices globales."""
        self.vertices = self.vertices * factor

    def apply_scale_xyz(self, sx: float, sy: float, sz: float) -> None:
        self.vertices[:, 0] *= sx
        self.vertices[:, 1] *= sy
        self.vertices[:, 2] *= sz

    def translate_object(self, obj: ObjObject, delta: np.ndarray) -> None:
        """Traslada los vértices de un sub-objeto (modifica el pool global)."""
        # Recolectar índices globales usados
        used_v = set()
        for face in obj.faces:
            for v_idx, _, _ in face.raw_indices:
                used_v.add(v_idx - 1)  # 0-based
        for idx in used_v:
            self.vertices[idx] += delta

    def center_object_origin(self, obj: ObjObject) -> np.ndarray:
        """
        Mueve el objeto para que su centroide quede en el origen local
        (equivalente a 'Set Origin > Origin to Geometry' en Blender).
        Devuelve el delta aplicado.
        """
        centroid = self.centroid_of(obj)
        self.translate_object(obj, -centroid)
        return -centroid

    def mirror_object_x(self, obj: ObjObject) -> None:
        """Espeja en X los vértices del objeto (simetría)."""
        used_v = set()
        for face in obj.faces:
            for v_idx, _, _ in face.raw_indices:
                used_v.add(v_idx - 1)
        for idx in used_v:
            self.vertices[idx][0] = -self.vertices[idx][0]
        # Invertir también normales en X
        used_n = set()
        for face in obj.faces:
            for _, _, vn_idx in face.raw_indices:
                if vn_idx > 0:
                    used_n.add(vn_idx - 1)
        for idx in used_n:
            self.normals[idx][0] = -self.normals[idx][0]

    def rename_object(self, old_name: str, new_name: str) -> bool:
        for obj in self.objects:
            if obj.name == old_name:
                obj.name = new_name
                return True
        return False

    def recalc_normals_for_object(self, obj: ObjObject) -> None:
        """Recalcula normales por cara para el objeto (modifica pool global de normales)."""
        for face in obj.faces:
            if len(face.raw_indices) < 3:
                continue
            v0 = self.vertices[face.raw_indices[0][0] - 1]
            v1 = self.vertices[face.raw_indices[1][0] - 1]
            v2 = self.vertices[face.raw_indices[2][0] - 1]
            edge1 = v1 - v0
            edge2 = v2 - v0
            normal = np.cross(edge1, edge2)
            norm_len = np.linalg.norm(normal)
            if norm_len > 1e-9:
                normal /= norm_len
            # Actualizar normales referenciadas
            for _, _, vn_idx in face.raw_indices:
                if vn_idx > 0:
                    self.normals[vn_idx - 1] = normal


# ──────────────────────────────────────────────────────────────────────────────
# Parser
# ──────────────────────────────────────────────────────────────────────────────

def parse_face_token(token: str) -> Tuple[int, int, int]:
    """Parsea 'v', 'v/vt', 'v//vn', 'v/vt/vn' → (v, vt, vn) 1-based (0 si ausente)."""
    parts = token.split("/")
    v  = int(parts[0]) if parts[0] else 0
    vt = int(parts[1]) if len(parts) > 1 and parts[1] else 0
    vn = int(parts[2]) if len(parts) > 2 and parts[2] else 0
    return v, vt, vn


def load_obj(path: str) -> ObjFile:
    """Carga un archivo .obj en memoria preservando la estructura completa."""
    obj_file = ObjFile(source_path=os.path.abspath(path))

    raw_verts:   List[List[float]] = []
    raw_normals: List[List[float]] = []
    raw_uvs:     List[List[float]] = []

    current_obj = ObjObject(name="__root__")
    obj_file.objects.append(current_obj)
    current_material = ""
    in_header = True

    with open(path, "r", encoding="utf-8", errors="replace") as f:
        for line in f:
            line_stripped = line.rstrip("\n\r")
            stripped = line_stripped.strip()

            if not stripped or stripped.startswith("#"):
                if in_header:
                    obj_file.header.append(line_stripped)
                continue

            in_header = False
            parts = stripped.split()
            keyword = parts[0].lower()

            if keyword == "mtllib":
                obj_file.mtl_files.append(" ".join(parts[1:]))

            elif keyword == "o" or keyword == "g":
                name = " ".join(parts[1:]) if len(parts) > 1 else "__unnamed__"
                current_obj = ObjObject(name=name, current_material=current_material)
                obj_file.objects.append(current_obj)

            elif keyword == "v":
                raw_verts.append([float(x) for x in parts[1:4]])

            elif keyword == "vn":
                raw_normals.append([float(x) for x in parts[1:4]])

            elif keyword == "vt":
                raw_uvs.append([float(x) for x in parts[1:3]])

            elif keyword == "usemtl":
                current_material = " ".join(parts[1:])
                current_obj.current_material = current_material

            elif keyword == "f":
                face = ObjFace(material=current_material)
                for token in parts[1:]:
                    face.raw_indices.append(parse_face_token(token))
                current_obj.faces.append(face)

            elif keyword == "s":
                pass  # smooth shading — ignorado

    # Convertir a numpy arrays
    obj_file.vertices = np.array(raw_verts, dtype=float) if raw_verts else np.empty((0, 3))
    obj_file.normals  = np.array(raw_normals, dtype=float) if raw_normals else np.empty((0, 3))
    obj_file.uvs      = np.array(raw_uvs, dtype=float) if raw_uvs else np.empty((0, 2))

    # Eliminar objeto raíz si está vacío
    obj_file.objects = [o for o in obj_file.objects
                        if o.name != "__root__" or len(o.faces) > 0]

    return obj_file


# ──────────────────────────────────────────────────────────────────────────────
# Writer
# ──────────────────────────────────────────────────────────────────────────────

def save_obj(obj_file: ObjFile, out_path: str) -> None:
    """
    Re-exporta el ObjFile a disco preservando la estructura de sub-objetos.
    Escribe vértices/normales/UVs globales primero y luego las caras por objeto.
    """
    os.makedirs(os.path.dirname(os.path.abspath(out_path)), exist_ok=True)

    with open(out_path, "w", encoding="utf-8") as f:
        # Header
        for line in obj_file.header:
            f.write(line + "\n")
        if not obj_file.header:
            f.write("# Generated by OBJ Validator/Fixer\n")

        # MTL references
        for mtl in obj_file.mtl_files:
            f.write(f"mtllib {mtl}\n")

        f.write("\n")

        # Vértices globales
        for v in obj_file.vertices:
            f.write(f"v {v[0]:.6f} {v[1]:.6f} {v[2]:.6f}\n")
        f.write("\n")

        # Normales globales
        for vn in obj_file.normals:
            f.write(f"vn {vn[0]:.6f} {vn[1]:.6f} {vn[2]:.6f}\n")
        if len(obj_file.normals) > 0:
            f.write("\n")

        # UVs globales
        for vt in obj_file.uvs:
            f.write(f"vt {vt[0]:.6f} {vt[1]:.6f}\n")
        if len(obj_file.uvs) > 0:
            f.write("\n")

        # Objetos y caras
        for obj in obj_file.objects:
            f.write(f"o {obj.name}\n")
            current_mat = None
            for face in obj.faces:
                if face.material != current_mat:
                    if face.material:
                        f.write(f"usemtl {face.material}\n")
                    current_mat = face.material
                tokens = []
                for v_idx, vt_idx, vn_idx in face.raw_indices:
                    if vt_idx and vn_idx:
                        tokens.append(f"{v_idx}/{vt_idx}/{vn_idx}")
                    elif vt_idx:
                        tokens.append(f"{v_idx}/{vt_idx}")
                    elif vn_idx:
                        tokens.append(f"{v_idx}//{vn_idx}")
                    else:
                        tokens.append(str(v_idx))
                f.write("f " + " ".join(tokens) + "\n")
            f.write("\n")
