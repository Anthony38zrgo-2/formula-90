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
    parser = argparse.ArgumentParser(description="Validate La Chutana tire-barrier card provenance and alpha contract.")
    parser.add_argument("--source-manifest", required=True)
    parser.add_argument("--prepared-manifest", required=True)
    args = parser.parse_args()
    source_path = Path(args.source_manifest).resolve()
    prepared_path = Path(args.prepared_manifest).resolve()
    source = json.loads(source_path.read_text(encoding="utf-8"))
    prepared = json.loads(prepared_path.read_text(encoding="utf-8"))
    failures = []

    if source.get("module", {}).get("geometry") != "rectangular_prism":
        failures.append("source geometry is not rectangular_prism")
    if int(source.get("module", {}).get("stack_count", 0)) != 5:
        failures.append("source stack_count must be 5")
    for source_id, entry in source["sources"].items():
        source_image = source_path.parent / entry["file"]
        if not source_image.exists() or sha256(source_image) != entry["sha256"]:
            failures.append(f"source hash mismatch: {source_id}")
        output = Path(prepared["sources"][source_id]["texture"])
        if not output.exists() or sha256(output) != prepared["sources"][source_id]["texture_sha256"]:
            failures.append(f"prepared hash mismatch: {source_id}")
            continue
        rgba = np.asarray(Image.open(output).convert("RGBA"), dtype=np.uint8)
        alpha = rgba[..., 3]
        ys, _ = np.where(alpha > 0)
        kind = entry.get("kind", "precut_card")
        if kind == "precut_card" and (not np.any(alpha == 0) or len(ys) == 0):
            failures.append(f"prepared alpha contract failed: {source_id}")
        elif kind == "precut_card" and int(ys.max()) != rgba.shape[0] - 1:
            failures.append(f"prepared texture is not bottom anchored: {source_id}")
        elif kind == "opaque_tile" and not np.all(alpha == 255):
            failures.append(f"top tile must remain opaque: {source_id}")

    if failures:
        for failure in failures:
            print(f"FAIL {failure}")
        return 2
    print(f"PASS tire barrier sources={len(source['sources'])} geometry=rectangular_prism stack_count=5")
    print("PASS source hashes, prepared hashes, alpha and bottom anchors")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
