#!/usr/bin/env python3
"""Jordan 197 entry point for the single-T_vehicle runtime pipeline."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from generate_vehicle_runtime import REPO_ROOT, build


DEFAULT_SOURCE = (
    REPO_ROOT
    / "assets-lowpoly-python/vehicles/canonical/formula_reference_livery_contract_annotated.glb"
)
DEFAULT_DATUMS = (
    REPO_ROOT
    / "assets-lowpoly-python/vehicles/canonical/formula_reference_livery_contract_annotated_datums.json"
)
DEFAULT_OUTPUT = REPO_ROOT / "game/assets/models/vehicles/jordan_197"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--datums", type=Path, default=DEFAULT_DATUMS)
    parser.add_argument("--output-dir", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--datum-tolerance", type=float, default=0.002)
    parser.add_argument("--cluster-tolerance", type=float, default=0.15)
    parser.add_argument("--assembly-tolerance-mm", type=float, default=2.0)
    args = parser.parse_args()
    args.candidate_id = "jordan_197"
    args.wheel_prefix = None
    manifest = build(args)
    print(json.dumps(manifest, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
