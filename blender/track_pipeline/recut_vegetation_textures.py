from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

import numpy as np
from PIL import Image

from pipeline_common import read_json
from vegetation_texture_common import MAGENTA_RGB, align_bottom, key_background_rgba, pad_transparent_rgb


POSTPROCESS_ID = "vegetation_legacy_card_recut"
POSTPROCESS_VERSION = 2
VEGETATION_CATEGORIES = {"trees", "bushes", "grass"}
ASSET_RE = re.compile(r"_(tree|bush|grass)_\d+\.png$")


def category_for(path: Path) -> str | None:
    match = ASSET_RE.search(path.name)
    if not match:
        return None
    return {"tree": "trees", "bush": "bushes", "grass": "grass"}[match.group(1)]


def recut_image(path: Path, pass_index: int) -> dict:
    image = Image.open(path).convert("RGBA")
    rgba = np.asarray(image, dtype=np.uint8).copy()
    height, width = rgba.shape[:2]
    rgba[..., 3][rgba[..., 3] < 2] = 0

    rgba, key_metrics = key_background_rgba(rgba, MAGENTA_RGB, pass_index)
    rgba, shift_down = align_bottom(rgba)
    rgba = pad_transparent_rgb(rgba, radius=4)

    visible = rgba[..., 3] > 0
    ys, xs = np.where(visible)
    if len(xs) == 0:
        raise RuntimeError(f"Vegetation card became empty: {path}")
    bbox = (int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1)

    Image.fromarray(rgba, "RGBA").save(path)
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    return {
        "path": str(path),
        "size": [width, height],
        "bbox": list(bbox),
        "bottom_gap_px": height - bbox[3],
        "shift_down_px": int(shift_down),
        "key": key_metrics,
        "sha256": digest,
    }


def update_forge_manifest(root: Path, metrics: list[dict], pass_index: int) -> None:
    manifest_path = root / "texture_forge_manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    by_rel = {
        str(Path(item["path"]).resolve().relative_to(root.resolve())).replace("\\", "/"): item
        for item in metrics
    }
    recipe = {
        "id": POSTPROCESS_ID,
        "version": POSTPROCESS_VERSION,
        "pass": pass_index,
        "scope": "legacy_128px_fallback_only",
        "key_rgb": MAGENTA_RGB.tolist(),
        "transparent_rgb": "foreground_edge_padding_4px",
        "bottom_anchor": "last_visible_alpha_row",
    }
    manifest["vegetation_legacy_recut"] = recipe
    for rel, item in by_rel.items():
        entry = manifest["entries"].get(rel)
        if entry is None:
            raise RuntimeError(f"Missing forge manifest entry: {rel}")
        entry["sha256"] = item["sha256"]
        entry["vegetation_postprocess"] = recipe
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Legacy fallback: recut existing 128px vegetation cards. Prefer source rebuild.")
    parser.add_argument("--config", required=True)
    parser.add_argument("--pass-index", type=int, choices=(1, 2), default=1)
    parser.add_argument("--report", default="")
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
    report_path.write_text(json.dumps({
        "postprocess": {"id": POSTPROCESS_ID, "version": POSTPROCESS_VERSION, "pass": ns.pass_index, "scope": "legacy_128px_fallback_only"},
        "assets": metrics,
    }, indent=2) + "\n", encoding="utf-8")
    print(f"[vegetation] legacy_recut={len(metrics)} pass={ns.pass_index} root={root}")
    print(f"[vegetation] report={report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
