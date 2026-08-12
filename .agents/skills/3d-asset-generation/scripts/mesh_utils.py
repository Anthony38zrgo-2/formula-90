"""Shared deterministic mesh loading and measurement helpers."""

from __future__ import annotations

import hashlib
from pathlib import Path
from typing import Any

import numpy as np
import trimesh


def sha256_file(path: Path, chunk_size: int = 1024 * 1024) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(chunk_size):
            digest.update(chunk)
    return digest.hexdigest()


def load_scene(path: Path) -> trimesh.Scene:
    """Load a mesh or scene without changing its source topology."""
    loaded = trimesh.load(path, force="scene", process=False)
    if isinstance(loaded, trimesh.Scene):
        return loaded
    if isinstance(loaded, trimesh.Trimesh):
        return trimesh.Scene(loaded)
    raise TypeError(f"Unsupported geometry type: {type(loaded).__name__}")


def scene_meshes(scene: trimesh.Scene) -> list[trimesh.Trimesh]:
    """Return transformed mesh instances, excluding non-mesh scene geometry."""
    dumped = scene.dump(concatenate=False)
    meshes = [item for item in dumped if isinstance(item, trimesh.Trimesh)]
    if meshes:
        return meshes
    return [item for item in scene.geometry.values() if isinstance(item, trimesh.Trimesh)]


def combined_mesh(meshes: list[trimesh.Trimesh]) -> trimesh.Trimesh:
    if not meshes:
        return trimesh.Trimesh(
            vertices=np.empty((0, 3), dtype=np.float64),
            faces=np.empty((0, 3), dtype=np.int64),
            process=False,
        )
    return trimesh.util.concatenate(meshes)


def _safe_bool(value: Any) -> bool | None:
    try:
        return bool(value)
    except (TypeError, ValueError, AttributeError):
        return None


def _safe_float(value: Any) -> float | None:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return None
    return number if np.isfinite(number) else None


def _safe_material_name(mesh: trimesh.Trimesh) -> str | None:
    try:
        material = getattr(mesh.visual, "material", None)
        name = getattr(material, "name", None)
        return str(name) if name else None
    except (AttributeError, TypeError, ValueError):
        return None


def topology_summary(mesh: trimesh.Trimesh) -> dict[str, Any]:
    vertices = np.asarray(mesh.vertices)
    faces = np.asarray(mesh.faces)
    face_count = len(faces)
    finite_vertices = bool(np.isfinite(vertices).all()) if vertices.size else True
    finite_faces = bool(np.isfinite(faces).all()) if faces.size else True
    valid_indices = bool(
        face_count == 0
        or (faces.min() >= 0 and faces.max() < len(vertices))
    )

    degenerate_faces = 0
    if face_count:
        repeated_indices = (faces[:, 0] == faces[:, 1]) | (faces[:, 1] == faces[:, 2]) | (faces[:, 0] == faces[:, 2])
        if valid_indices:
            triangles = vertices[faces]
            cross = np.cross(triangles[:, 1] - triangles[:, 0], triangles[:, 2] - triangles[:, 0])
            scale = max(float(np.linalg.norm(np.ptp(vertices, axis=0))), 1.0)
            tiny_area = np.linalg.norm(cross, axis=1) <= (scale * scale * 1.0e-12)
            degenerate_faces = int(np.count_nonzero(repeated_indices | tiny_area))
        else:
            degenerate_faces = int(np.count_nonzero(repeated_indices))

    duplicate_vertices = 0
    if len(vertices):
        _, counts = np.unique(vertices, axis=0, return_counts=True)
        duplicate_vertices = int(np.sum(counts[counts > 1] - 1))

    boundary_edges = 0
    non_manifold_edges = 0
    if face_count and valid_indices:
        try:
            edges = np.sort(faces[:, [0, 1, 1, 2, 2, 0]].reshape(-1, 2), axis=1)
            _, edge_counts = np.unique(edges, axis=0, return_counts=True)
            boundary_edges = int(np.count_nonzero(edge_counts == 1))
            non_manifold_edges = int(np.count_nonzero(edge_counts > 2))
        except (TypeError, ValueError):
            pass

    try:
        watertight = bool(mesh.is_watertight)
    except (AttributeError, TypeError, ValueError):
        watertight = None

    return {
        "finite_vertices": finite_vertices,
        "finite_faces": finite_faces,
        "valid_face_indices": valid_indices,
        "degenerate_faces": degenerate_faces,
        "duplicate_vertices": duplicate_vertices,
        "boundary_edges": boundary_edges,
        "non_manifold_edges": non_manifold_edges,
        "watertight": watertight,
    }


def _mesh_attributes(mesh: trimesh.Trimesh) -> dict[str, Any]:
    visual = mesh.visual
    uv_count = 0
    uv_valid = True
    try:
        uv = getattr(visual, "uv", None)
        if uv is not None:
            uv_array = np.asarray(uv)
            uv_count = int(len(uv_array))
            uv_valid = bool(uv_array.ndim == 2 and uv_array.shape[1] >= 2 and np.isfinite(uv_array).all())
    except (AttributeError, TypeError, ValueError):
        uv_valid = False

    vertex_color_count = 0
    try:
        colors = getattr(visual, "vertex_colors", None)
        if colors is not None:
            vertex_color_count = int(len(np.asarray(colors)))
    except (AttributeError, TypeError, ValueError):
        pass

    material_count = 0
    face_materials_valid = True
    try:
        material = getattr(visual, "material", None)
        materials = getattr(material, "materials", None)
        if materials is not None:
            material_count = int(len(materials))
        elif material is not None:
            material_count = 1
        face_materials = getattr(visual, "face_materials", None)
        if face_materials is not None:
            assignments = np.asarray(face_materials)
            face_materials_valid = bool(
                len(assignments) == len(mesh.faces)
                and np.isfinite(assignments).all()
                and (material_count == 0 or (assignments.size == 0 or assignments.max() < material_count))
            )
    except (AttributeError, TypeError, ValueError):
        face_materials_valid = False

    try:
        normals = np.asarray(mesh.vertex_normals)
        normals_present = bool(normals.ndim == 2 and len(normals) == len(mesh.vertices) and np.isfinite(normals).all())
    except (AttributeError, TypeError, ValueError):
        normals_present = False

    return {
        "materials": material_count,
        "material_name": _safe_material_name(mesh),
        "uv": uv_count > 0,
        "uv_count": uv_count,
        "uv_valid": uv_valid,
        "vertex_colors": vertex_color_count > 0,
        "vertex_color_count": vertex_color_count,
        "normals": normals_present,
        "material_assignments_valid": face_materials_valid,
    }


def principal_orientation(vertices: np.ndarray) -> dict[str, Any]:
    if len(vertices) < 2:
        return {"principal_axes": [], "note": "not enough vertices"}
    centered = vertices - vertices.mean(axis=0)
    covariance = np.cov(centered, rowvar=False)
    values, vectors = np.linalg.eigh(np.atleast_2d(covariance))
    order = np.argsort(values)[::-1]
    axes = vectors[:, order]
    return {
        "principal_axes": np.round(axes, 8).tolist(),
        "variance": np.round(values[order], 8).tolist(),
        "note": "PCA axes; sign is arbitrary",
    }


def analyze_scene(scene: trimesh.Scene, source_file: Path, source_hash: str | None = None) -> dict[str, Any]:
    meshes = scene_meshes(scene)
    all_vertices = [np.asarray(mesh.vertices, dtype=np.float64) for mesh in meshes if len(mesh.vertices)]
    if all_vertices:
        vertices = np.concatenate(all_vertices, axis=0)
        bounds = np.vstack((vertices.min(axis=0), vertices.max(axis=0)))
        dimensions = bounds[1] - bounds[0]
        center = (bounds[0] + bounds[1]) / 2.0
        orientation = principal_orientation(vertices)
    else:
        bounds = np.zeros((2, 3), dtype=np.float64)
        dimensions = np.zeros(3, dtype=np.float64)
        center = np.zeros(3, dtype=np.float64)
        orientation = {"principal_axes": [], "note": "empty asset"}

    topology = [topology_summary(mesh) for mesh in meshes]
    attributes = [_mesh_attributes(mesh) for mesh in meshes]
    components = 0
    for mesh in meshes:
        try:
            components += len(mesh.split(only_watertight=False))
        except (AttributeError, TypeError, ValueError):
            components += 1 if len(mesh.vertices) else 0

    report: dict[str, Any] = {
        "file": str(source_file.resolve()),
        "format": source_file.suffix.lower().lstrip("."),
        "sha256": source_hash,
        "objects": len(scene.geometry),
        "meshes": len(meshes),
        "vertices": int(sum(len(mesh.vertices) for mesh in meshes)),
        "faces": int(sum(len(mesh.faces) for mesh in meshes)),
        "components": int(components),
        "bounds": {
            "min": np.round(bounds[0], 8).tolist(),
            "max": np.round(bounds[1], 8).tolist(),
        },
        "dimensions": {
            "x": _safe_float(dimensions[0]),
            "y": _safe_float(dimensions[1]),
            "z": _safe_float(dimensions[2]),
        },
        "center": np.round(center, 8).tolist(),
        "orientation": orientation,
        "materials": sorted({item["material_name"] for item in attributes if item["material_name"]}),
        "has_uv": any(item["uv"] for item in attributes),
        "has_vertex_colors": any(item["vertex_colors"] for item in attributes),
        "has_normals": any(item["normals"] for item in attributes),
        "disconnected_components": max(0, components - len(meshes)),
        "topology": topology,
        "attributes": attributes,
    }
    return report
