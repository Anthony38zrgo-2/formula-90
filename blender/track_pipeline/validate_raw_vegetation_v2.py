from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate source provenance and generated La Chutana vegetation v2 assets.")
    parser.add_argument("--source-manifest", required=True)
    parser.add_argument("--asset-manifest", required=True)
    args = parser.parse_args()

    source_path = Path(args.source_manifest).resolve()
    asset_path = Path(args.asset_manifest).resolve()
    source = json.loads(source_path.read_text(encoding="utf-8"))
    assets = json.loads(asset_path.read_text(encoding="utf-8"))
    failures = []

    for source_id, entry in source["sources"].items():
        path = source_path.parent / entry["file"]
        if not path.exists() or sha256(path) != entry["sha256"]:
            failures.append(f"source hash mismatch: {source_id}")
            continue
        rgba = np.asarray(Image.open(path).convert("RGBA"), dtype=np.uint8)
        if not np.any(rgba[..., 3] == 0) or not np.any(rgba[..., 3] > 0):
            failures.append(f"source alpha contract failed: {source_id}")

    category_counts = {"trees": 0, "bushes": 0, "grass": 0}
    source_variants = {item["id"]: item for item in source["variants"]}
    for item in assets.get("assets", []):
        glb = source_path.parents[2] / item["glb"]
        if not glb.exists() or sha256(glb) != item["glb_sha256"]:
            failures.append(f"generated GLB hash mismatch: {item['id']}")
        texture = Path(item["texture"])
        rgba = np.asarray(Image.open(texture).convert("RGBA"), dtype=np.uint8)
        ys, _ = np.where(rgba[..., 3] > 0)
        if len(ys) == 0 or int(ys.max()) != rgba.shape[0] - 1:
            failures.append(f"prepared texture is not bottom anchored: {item['id']}")
        if item.get("collision"):
            failures.append(f"visual vegetation cannot have collision: {item['id']}")
        variant = source_variants[item["id"]]
        if item["category"] == "trees" and int(variant.get("planes", 0)) != 2:
            failures.append(f"tree must use exactly two crossed cards: {item['id']}")
        category_counts[item["category"]] += 1

    if category_counts != {"trees": 3, "bushes": 4, "grass": 4}:
        failures.append(f"expected reusable variants trees=3 bushes=4 grass=4, got {category_counts}")
    if failures:
        for failure in failures:
            print(f"FAIL {failure}")
        return 2
    print(f"PASS vegetation v2 sources={len(source['sources'])} assets={len(assets['assets'])} variants={category_counts}")
    print("PASS alpha bottom anchors, hashes, two-card trees and visual-only collision contract")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
