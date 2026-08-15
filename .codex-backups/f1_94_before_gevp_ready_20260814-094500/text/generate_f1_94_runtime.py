#!/usr/bin/env python3
"""F1-94 entry point for the canonical single-T_vehicle runtime pipeline."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from generate_vehicle_runtime import REPO_ROOT, build


DEFAULT_SOURCE = REPO_ROOT / "assets-lowpoly-python/vehicles/canonical/f1_94_normalized.glb"
DEFAULT_DATUMS = REPO_ROOT / "assets-lowpoly-python/vehicles/canonical/f1_94_normalized_datums.json"
DEFAULT_OUTPUT = REPO_ROOT / "game/assets/models/vehicles/f1_94"
ORIGINAL_SOURCE = REPO_ROOT / "assets-lowpoly-python/vehicles/F194/F1-94.obj"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--datums", type=Path, default=DEFAULT_DATUMS)
    parser.add_argument("--output-dir", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--datum-tolerance", type=float, default=0.002)
    parser.add_argument("--cluster-tolerance", type=float, default=0.15)
    parser.add_argument("--assembly-tolerance-mm", type=float, default=2.0)
    args = parser.parse_args()
    args.candidate_id = "f1_94"
    args.wheel_prefix = None
    manifest = build(args)
    if ORIGINAL_SOURCE.is_file():
        manifest["source"]["original_asset"] = "assets-lowpoly-python/vehicles/F194/F1-94.obj"
        manifest["source"]["original_sha256"] = hashlib.sha256(ORIGINAL_SOURCE.read_bytes()).hexdigest()
        manifest_path = args.output_dir / "vehicle_runtime_manifest.json"
        manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(manifest, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
