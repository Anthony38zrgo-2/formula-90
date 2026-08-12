#!/usr/bin/env python3
"""Select geometry regions with cKDTree instead of hardcoded vertex indices."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

import numpy as np
from scipy.spatial import cKDTree

from mesh_utils import combined_mesh, load_scene, scene_meshes


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("asset", type=Path)
    parser.add_argument("--center", nargs=3, type=float, required=True, metavar=("X", "Y", "Z"))
    parser.add_argument("--radius", type=float, required=True)
    parser.add_argument("--label", default="region")
    parser.add_argument("--json-out", type=Path)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    asset = args.asset.resolve()
    if not asset.is_file() or args.radius < 0:
        print("error: asset must exist and radius must be non-negative", file=sys.stderr)
        return 2
    mesh = combined_mesh(scene_meshes(load_scene(asset)))
    vertices = np.asarray(mesh.vertices, dtype=np.float64)
    center = np.asarray(args.center, dtype=np.float64)
    if not len(vertices):
        print("error: asset has no vertices", file=sys.stderr)
        return 2
    tree = cKDTree(vertices)
    indices = sorted(int(index) for index in tree.query_ball_point(center, args.radius))
    selected = vertices[indices]
    report = {
        "label": args.label,
        "center": center.tolist(),
        "radius": args.radius,
        "vertex_indices": indices,
        "count": len(indices),
        "bounds": {
            "min": selected.min(axis=0).tolist() if len(selected) else None,
            "max": selected.max(axis=0).tolist() if len(selected) else None,
        },
        "selection_method": "scipy.spatial.cKDTree.query_ball_point",
    }
    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
