from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path

from validate_barrier_asset_integration import read_glb_json


def validate(glb_path: Path, asset_manifest_path: Path, report_path: Path) -> dict:
    document = read_glb_json(glb_path)
    manifest = json.loads(asset_manifest_path.read_text(encoding="utf-8"))
    report = json.loads(report_path.read_text(encoding="utf-8"))
    known = {asset["id"] for asset in manifest.get("assets", [])}
    expected = Counter({key: int(value) for key, value in report.get("asset_counts", {}).items()})
    if not expected or not all(value > 0 for value in expected.values()):
        raise ValueError(f"Every building asset must be placed at least once: {dict(expected)}")
    if set(expected) != known:
        raise ValueError(f"Building report/library mismatch: expected={sorted(expected)} known={sorted(known)}")
    actual = Counter(
        node.get("extras", {}).get("building_asset_id")
        for node in document.get("nodes", [])
        if node.get("extras", {}).get("building_asset_id")
    )
    if actual != expected:
        raise ValueError(f"Building instance mismatch: expected={dict(expected)} actual={dict(actual)}")
    collidable = [
        node.get("name", "") for node in document.get("nodes", [])
        if node.get("extras", {}).get("building_asset_id") and
        node.get("extras", {}).get("collision") is not False
    ]
    if collidable:
        raise ValueError(f"Scenic buildings unexpectedly collidable: {collidable}")
    return {"instances": sum(actual.values()), "asset_counts": dict(actual), "collision": False}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--glb", type=Path, required=True)
    parser.add_argument("--asset-manifest", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    print("PASS building asset integration " + json.dumps(
        validate(args.glb, args.asset_manifest, args.report), sort_keys=True,
    ))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
