"""Validate the canonical Asset Registry.

Exits non-zero if the registry contains duplicate ids, nonexistent sources or
previews, invalid kinds/collision classes, degenerate dimensions, inconsistent
budgets or provenance hash mismatches.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import sys

from asset_registry import load_registry, validate_registry

REPO = Path(__file__).resolve().parents[2]
DEFAULT = REPO / "blender" / "track_pipeline" / "configs" / "asset_registry.json"


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate the F90 Asset Registry.")
    parser.add_argument("--registry", default=str(DEFAULT))
    args = parser.parse_args()

    errors = validate_registry(load_registry(args.registry, repo_root=REPO))
    for error in errors:
        print(f"FAIL {error}", file=sys.stderr)
    if not errors:
        print("ASSET REGISTRY VALID")
        return 0
    print(f"{len(errors)} registry error(s)", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
