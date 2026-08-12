#!/usr/bin/env python3
"""Run KD-tree vertex queries or true triangle-surface proximity queries."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

import numpy as np
from scipy.spatial import cKDTree
from trimesh.proximity import closest_point

from mesh_utils import combined_mesh, load_scene, scene_meshes


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("asset", type=Path)
    parser.add_argument("--point", nargs=3, type=float, action="append", required=True, metavar=("X", "Y", "Z"))
    parser.add_argument("--nearest", type=int, default=1, help="Number of nearest vertices for KD-tree mode")
    parser.add_argument("--radius", type=float, help="Also return vertex indices within this radius")
    parser.add_argument("--surface", action="store_true", help="Measure distance to triangles instead of vertices")
    parser.add_argument("--json-out", type=Path)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    asset = args.asset.resolve()
    if not asset.is_file():
        print(f"error: asset does not exist: {asset}", file=sys.stderr)
        return 2
    if args.nearest < 1 or (args.radius is not None and args.radius < 0):
        print("error: nearest must be positive and radius must be non-negative", file=sys.stderr)
        return 2

    mesh = combined_mesh(scene_meshes(load_scene(asset)))
    points = np.asarray(args.point, dtype=np.float64)
    results = []
    if args.surface:
        if not len(mesh.faces):
            print("error: surface mode requires triangular faces", file=sys.stderr)
            return 2
        locations, distances, face_ids = closest_point(mesh, points)
        for point, location, distance, face_id in zip(points, locations, distances, face_ids):
            results.append(
                {
                    "query": point.tolist(),
                    "closest_point": np.round(location, 8).tolist(),
                    "distance": float(distance),
                    "face_index": int(face_id),
                }
            )
        report = {"method": "trimesh_surface", "queries": results}
    else:
        vertices = np.asarray(mesh.vertices, dtype=np.float64)
        if not len(vertices):
            print("error: asset has no vertices", file=sys.stderr)
            return 2
        tree = cKDTree(vertices)
        distances, indices = tree.query(points, k=args.nearest)
        for query_index, point in enumerate(points):
            query_distances = np.atleast_1d(distances[query_index])
            query_indices = np.atleast_1d(indices[query_index])
            item = {
                "query": point.tolist(),
                "nearest": [
                    {"index": int(index), "distance": float(distance), "vertex": np.round(vertices[index], 8).tolist()}
                    for distance, index in zip(query_distances, query_indices)
                ],
            }
            if args.radius is not None:
                item["neighbors_within_radius"] = [int(index) for index in tree.query_ball_point(point, args.radius)]
            results.append(item)
        report = {"method": "scipy.spatial.cKDTree", "queries": results}

    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
