#!/usr/bin/env python3
"""Export connected mesh components as deterministic individual GLB files."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from mesh_utils import load_scene, scene_meshes, sha256_file


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("asset", type=Path)
    parser.add_argument("output_dir", type=Path)
    parser.add_argument("--min-faces", type=int, default=1)
    parser.add_argument("--report", type=Path)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    asset = args.asset.resolve()
    output_dir = args.output_dir.resolve()
    if not asset.is_file():
        print(f"error: asset does not exist: {asset}", file=sys.stderr)
        return 2
    if args.min_faces < 0:
        print("error: --min-faces must be non-negative", file=sys.stderr)
        return 2

    output_dir.mkdir(parents=True, exist_ok=True)
    outputs = []
    component_index = 0
    for mesh in scene_meshes(load_scene(asset)):
        try:
            components = mesh.split(only_watertight=False)
        except (AttributeError, TypeError, ValueError):
            components = [mesh]
        for component in components:
            if len(component.faces) < args.min_faces:
                continue
            target = output_dir / f"component_{component_index:03d}.glb"
            component.export(target)
            outputs.append(
                {
                    "region": f"Component_{component_index:03d}",
                    "output": str(target),
                    "vertices": int(len(component.vertices)),
                    "faces": int(len(component.faces)),
                }
            )
            component_index += 1

    report = {"input": str(asset), "input_sha256": sha256_file(asset), "components": outputs}
    report_path = (args.report or output_dir / "components.json").resolve()
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
