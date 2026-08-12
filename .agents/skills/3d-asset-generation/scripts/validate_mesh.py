#!/usr/bin/env python3
"""Validate mesh structure, attributes, and optional before/after deltas."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from typing import Any

import numpy as np

from mesh_utils import analyze_scene, load_scene, scene_meshes, sha256_file


def _checks(report: dict[str, Any]) -> tuple[dict[str, bool], list[str]]:
    topology = report["topology"]
    attributes = report["attributes"]
    checks = {
        "finite_vertices": all(item["finite_vertices"] for item in topology),
        "finite_faces": all(item["finite_faces"] for item in topology),
        "valid_face_indices": all(item["valid_face_indices"] for item in topology),
        "uv_integrity": all(item["uv_valid"] for item in attributes),
        "material_assignments": all(item["material_assignments_valid"] for item in attributes),
    }
    warnings = []
    if any(item["degenerate_faces"] for item in topology):
        warnings.append("degenerate faces present")
    if any(item["duplicate_vertices"] for item in topology):
        warnings.append("duplicate vertices present")
    if any(item["non_manifold_edges"] for item in topology):
        warnings.append("non-manifold edges present")
    if report["faces"] and not report["has_normals"]:
        warnings.append("vertex normals are not present")
    return checks, warnings


def _delta(before: dict[str, Any], after: dict[str, Any]) -> dict[str, Any]:
    delta: dict[str, Any] = {}
    for key in ("objects", "meshes", "vertices", "faces", "components"):
        delta[key] = after[key] - before[key]
    for axis in ("x", "y", "z"):
        delta[f"dimension_{axis}"] = after["dimensions"][axis] - before["dimensions"][axis]
    return delta


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("asset", type=Path)
    parser.add_argument("--compare-to", type=Path, help="Compare metrics with an original asset")
    parser.add_argument("--json-out", type=Path)
    parser.add_argument("--strict", action="store_true", help="Treat topology warnings as validation failures")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    asset = args.asset.resolve()
    if not asset.is_file():
        print(f"error: asset does not exist: {asset}", file=sys.stderr)
        return 2
    report = analyze_scene(load_scene(asset), asset, sha256_file(asset))
    checks, warnings = _checks(report)
    comparison = None
    if args.compare_to:
        original = args.compare_to.resolve()
        if not original.is_file():
            print(f"error: comparison asset does not exist: {original}", file=sys.stderr)
            return 2
        before = analyze_scene(load_scene(original), original, sha256_file(original))
        comparison = {"before": before, "delta": _delta(before, report)}

    fatal_failures = [name for name, passed in checks.items() if not passed]
    status = "FAIL" if fatal_failures else ("PASS_WITH_WARNINGS" if warnings else "PASS")
    output = {
        "asset": report,
        "checks": checks,
        "fatal_failures": fatal_failures,
        "warnings": warnings,
        "status": status,
        "comparison": comparison,
    }
    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(output, indent=2))
    if fatal_failures or (args.strict and warnings):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
