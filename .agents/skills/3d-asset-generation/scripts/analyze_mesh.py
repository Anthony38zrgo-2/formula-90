#!/usr/bin/env python3
"""Inspect a mesh or scene and cache a JSON report by asset SHA-256."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys

from mesh_utils import analyze_scene, load_scene, sha256_file


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("asset", type=Path)
    parser.add_argument("--reports-dir", type=Path, default=Path("reports"))
    parser.add_argument("--json-out", type=Path, help="Also copy the report to this path")
    parser.add_argument("--json", action="store_true", dest="json_stdout", help="Print JSON instead of a human summary")
    parser.add_argument("--no-cache", action="store_true", help="Recompute even when the SHA-256 report exists")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    asset = args.asset.resolve()
    if not asset.is_file():
        print(f"error: asset does not exist: {asset}", file=sys.stderr)
        return 2

    digest = sha256_file(asset)
    cache_path = args.reports_dir / digest / "analysis.json"
    cache_hit = cache_path.is_file() and not args.no_cache
    if cache_hit:
        report = json.loads(cache_path.read_text(encoding="utf-8"))
        report["file"] = str(asset)
        report["sha256"] = digest
    else:
        report = analyze_scene(load_scene(asset), asset, digest)
        cache_path.parent.mkdir(parents=True, exist_ok=True)
        cache_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    if args.json_stdout:
        print(json.dumps(report, indent=2))
    else:
        print(f"asset: {report['file']}")
        print(f"format: {report['format']}  sha256: {report['sha256']}")
        print(
            f"objects: {report['objects']}  vertices: {report['vertices']}  "
            f"faces: {report['faces']}  components: {report['components']}"
        )
        dims = report["dimensions"]
        print(f"dimensions: x={dims['x']} y={dims['y']} z={dims['z']}")
        print(f"cache: {'hit' if cache_hit else 'miss'} ({cache_path})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
