#!/usr/bin/env python3
"""Extract connected components and emit geometry-based feature vectors."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

import numpy as np
from scipy.spatial import ConvexHull

from mesh_utils import combined_mesh, load_scene, scene_meshes


def _circularity(vertices: np.ndarray) -> float | None:
    if len(vertices) < 3:
        return None
    best = 0.0
    for axes in ((0, 1), (0, 2), (1, 2)):
        points = vertices[:, axes]
        try:
            hull = ConvexHull(points)
            boundary = points[hull.vertices]
            perimeter = float(np.linalg.norm(np.roll(boundary, -1, axis=0) - boundary, axis=1).sum())
            area = float(hull.volume)
            if perimeter > 0.0:
                best = max(best, 4.0 * np.pi * area / (perimeter * perimeter))
        except (ValueError, RuntimeError):
            continue
    return best


def _features(mesh, vehicle_center: np.ndarray) -> dict:
    vertices = np.asarray(mesh.vertices, dtype=np.float64)
    bounds = np.asarray(mesh.bounds, dtype=np.float64) if len(vertices) else np.zeros((2, 3))
    dimensions = bounds[1] - bounds[0]
    centroid = np.asarray(mesh.centroid, dtype=np.float64) if len(vertices) else np.zeros(3)
    nonzero_dimensions = dimensions[dimensions > 1.0e-12]
    aspect_ratio = float(nonzero_dimensions.max() / nonzero_dimensions.min()) if len(nonzero_dimensions) else None
    try:
        volume = float(mesh.volume) if mesh.is_volume else None
    except (AttributeError, TypeError, ValueError):
        volume = None
    try:
        area = float(mesh.area)
    except (AttributeError, TypeError, ValueError):
        area = None
    return {
        "centroid": np.round(centroid, 8).tolist(),
        "bbox_size_x": float(dimensions[0]),
        "bbox_size_y": float(dimensions[1]),
        "bbox_size_z": float(dimensions[2]),
        "surface_area": area,
        "volume": volume,
        "vertex_count": int(len(mesh.vertices)),
        "face_count": int(len(mesh.faces)),
        "distance_to_vehicle_center": float(np.linalg.norm(centroid - vehicle_center)),
        "height": float(centroid[2]),
        "aspect_ratio": aspect_ratio,
        "circularity": _circularity(vertices),
        "watertight": bool(mesh.is_watertight) if len(mesh.faces) else False,
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("asset", type=Path)
    parser.add_argument("--json-out", type=Path)
    parser.add_argument("--min-faces", type=int, default=1)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    asset = args.asset.resolve()
    if not asset.is_file():
        print(f"error: asset does not exist: {asset}", file=sys.stderr)
        return 2
    if args.min_faces < 0:
        print("error: --min-faces must be non-negative", file=sys.stderr)
        return 2

    meshes = scene_meshes(load_scene(asset))
    aggregate = combined_mesh(meshes)
    vehicle_center = (
        np.asarray(aggregate.bounds, dtype=np.float64).mean(axis=0)
        if len(aggregate.vertices)
        else np.zeros(3)
    )
    components = []
    component_index = 0
    for object_index, mesh in enumerate(meshes):
        try:
            parts = mesh.split(only_watertight=False)
        except (AttributeError, TypeError, ValueError):
            parts = [mesh]
        for part_index, component in enumerate(parts):
            if len(component.faces) < args.min_faces:
                continue
            components.append(
                {
                    "region": f"Component_{component_index:03d}",
                    "source_object": object_index,
                    "source_component": part_index,
                    "classification": "unknown",
                    "confidence": 0.0,
                    "evidence": ["geometric feature vector emitted; no destructive classifier applied"],
                    "features": _features(component, vehicle_center),
                }
            )
            component_index += 1

    report = {
        "file": str(asset),
        "components": len(components),
        "vehicle_center": np.round(vehicle_center, 8).tolist(),
        "regions": components,
    }
    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
