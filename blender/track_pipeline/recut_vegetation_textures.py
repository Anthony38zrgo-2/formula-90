from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

import numpy as np
from PIL import Image

from atomic_json import write_json_atomic
from pipeline_common import read_json


POSTPROCESS_ID = "vegetation_bottom_anchor_validation"
POSTPROCESS_VERSION = 1
VEGETATION_CATEGORIES = {"trees", "bushes", "grass"}
ASSET_RE = re.compile(r"_(tree|bush|grass)_\d+\.png$")


def category_for(path: Path) -> str | None:
    match = ASSET_RE.search(path.name)
    if not match:
        return None
    return {"tree": "trees", "bush": "bushes", "grass": "grass"}[match.group(1)]


def recut_image(path: Path, pass_index: int = 1) -> dict:
    """Validate a pre-cut card without changing its pixels."""
    del pass_index
    before = hashlib.sha256(path.read_bytes()).hexdigest()
    image = Image.open(path)
    rgba = np.asarray(image.convert("RGBA"), dtype=np.uint8)
    alpha = rgba[..., 3]
    visible = alpha > 0
    ys, xs = np.where(visible)
    if len(xs) == 0:
        raise RuntimeError(f"Vegetation card became empty: {path}")
    bbox = (int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1)
    bottom_gap = int(rgba.shape[0] - bbox[3])
    if bottom_gap != 0:
        raise RuntimeError(f"Vegetation card is not bottom anchored: {path} gap={bottom_gap}px")
    after = hashlib.sha256(path.read_bytes()).hexdigest()
    if before != after:
        raise RuntimeError(f"Anchor validation modified source unexpectedly: {path}")
    return {
        "path": str(path),
        "size": [int(rgba.shape[1]), int(rgba.shape[0])],
        "bbox": list(bbox),
        "bottom_gap_px": bottom_gap,
        "alpha_nonzero": int(visible.sum()),
        "sha256": after,
        "modified": False,
    }


def relative_texture_path(path: Path, root: Path) -> str:
    return path.resolve().relative_to(root.resolve()).as_posix()


def update_forge_manifest(root: Path, metrics: list[dict], pass_index: int) -> None:
    manifest_path = root / "texture_forge_manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    recipe = {
        "id": POSTPROCESS_ID,
        "version": POSTPROCESS_VERSION,
        "pass": int(pass_index),
        "scope": "all_pre_cut_vegetation_cards",
        "operation": "validate_only",
        "bottom_anchor": "last_visible_alpha_row",
        "chroma_key": "not_applied",
    }
    manifest["vegetation_bottom_anchor_validation"] = recipe
    manifest["vegetation_legacy_recut"] = {
        "id": "vegetation_legacy_card_recut",
        "version": 4,
        "scope": "legacy_assets_without_source_manifest",
        "operation": "compatibility_only_not_run_for_pre_cut_sources",
        "key_name": "legacy_magenta",
        "key_rgb": [255, 0, 255],
        "key_hex": "#FF00FF",
        "legacy_assets": 0,
        "skipped_source_backed": len(metrics),
    }
    for item in metrics:
        rel = relative_texture_path(Path(item["path"]), root)
        entry = manifest.get("entries", {}).get(rel)
        if entry is None:
            raise RuntimeError(f"Missing forge manifest entry: {rel}")
        entry["sha256"] = item["sha256"]
        entry["vegetation_anchor_validation"] = recipe
    write_json_atomic(manifest_path, manifest)


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate pre-cut vegetation card bottom anchors without recutting pixels.")
    parser.add_argument("--config", required=True)
    parser.add_argument("--pass-index", type=int, choices=(1, 2), default=1)
    parser.add_argument("--report", default="")
    parser.add_argument("--skip-source-backed", action="store_true", help="Deprecated compatibility flag; validation remains read-only.")
    ns = parser.parse_args()

    config_path = Path(ns.config).resolve()
    repo = config_path.parents[3]
    config = read_json(config_path)
    root = repo / config["generated_dir"] / "textures"
    paths = sorted(p for p in root.glob("biomes/**/*.png") if category_for(p) in VEGETATION_CATEGORIES)
    if not paths:
        raise RuntimeError(f"No vegetation cards found under {root}")
    metrics = [recut_image(path, ns.pass_index) for path in paths]
    update_forge_manifest(root, metrics, ns.pass_index)

    report_path = Path(ns.report) if ns.report else root / "vegetation_recut_report.json"
    if not report_path.is_absolute():
        report_path = repo / report_path
    report_path.parent.mkdir(parents=True, exist_ok=True)
    write_json_atomic(report_path, {
        "postprocess": {
            "id": POSTPROCESS_ID,
            "version": POSTPROCESS_VERSION,
            "pass": ns.pass_index,
            "scope": "all_pre_cut_vegetation_cards",
            "operation": "validate_only",
        },
        "assets": metrics,
    })
    print(f"[vegetation] anchor_validation={len(metrics)} pass={ns.pass_index} root={root}")
    print(f"[vegetation] report={report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
