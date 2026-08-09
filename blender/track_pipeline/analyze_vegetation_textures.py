from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

import numpy as np
from PIL import Image

from pipeline_common import read_json


ASSET_RE = re.compile(r"_(tree|bush|grass)_\d+\.png$")
EXPECTED_SIZE = (128, 128)


def category_for(path: Path) -> str | None:
    match = ASSET_RE.search(path.name)
    if not match:
        return None
    return {"tree": "trees", "bush": "bushes", "grass": "grass"}[match.group(1)]


def visible_magenta_mask(rgba: np.ndarray) -> np.ndarray:
    """Independent QA rule; intentionally does not reuse the production key classifier."""
    rgb = rgba[..., :3].astype(np.int32)
    alpha = rgba[..., 3]
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    distance = np.sqrt((255 - r) ** 2 + g ** 2 + (255 - b) ** 2)
    balance = np.minimum(r, b) / np.maximum(np.maximum(r, b), 1)
    return (
        (alpha >= 16)
        & (distance <= 170.0)
        & (r >= 140)
        & (b >= 140)
        & (g <= 105)
        & ((np.minimum(r, b) - g) >= 55)
        & (balance >= 0.80)
        & (np.abs(r - b) <= 95)
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Analyze deterministic vegetation card outputs independently of the keyer.")
    parser.add_argument("--config", required=True)
    parser.add_argument("--report", default="")
    parser.add_argument("--strict", action="store_true")
    # Kept for command-line compatibility; validation thresholds are intentionally independent.
    parser.add_argument("--pass-index", type=int, choices=(1, 2), default=1)
    ns = parser.parse_args()

    config_path = Path(ns.config).resolve()
    repo = config_path.parents[3]
    config = read_json(config_path)
    root = repo / config["generated_dir"] / "textures"
    paths = sorted(p for p in root.glob("biomes/**/*.png") if category_for(p))
    failures: list[str] = []
    assets: list[dict] = []
    for path in paths:
        image = Image.open(path)
        rgba = np.asarray(image.convert("RGBA"), dtype=np.uint8)
        alpha = rgba[..., 3]
        visible = alpha > 0
        ys, xs = np.where(visible)
        bbox = None if len(xs) == 0 else [int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1]
        bottom_gap = None if bbox is None else rgba.shape[0] - bbox[3]
        magenta_visible = visible_magenta_mask(rgba)
        magenta_count = int(magenta_visible.sum())
        semi_transparent_magenta = int((magenta_visible & (alpha < 224)).sum())
        alpha_coverage = float(visible.mean())
        item = {
            "path": str(path),
            "category": category_for(path),
            "mode": image.mode,
            "size": list(image.size),
            "alpha_nonzero": int(visible.sum()),
            "alpha_coverage": alpha_coverage,
            "bbox": bbox,
            "bottom_gap_px": bottom_gap,
            "visible_magenta_px": magenta_count,
            "semi_transparent_magenta_px": semi_transparent_magenta,
        }
        assets.append(item)
        if image.size != EXPECTED_SIZE or image.mode not in {"RGBA", "LA"}:
            failures.append(f"format:{path}")
        if bbox is None:
            failures.append(f"empty:{path}")
        elif alpha_coverage >= 0.985:
            failures.append(f"alpha_coverage:{path}:{alpha_coverage:.4f}")
        if ns.strict and bottom_gap != 0:
            failures.append(f"bottom_gap:{path}:{bottom_gap}")
        if ns.strict and magenta_count != 0:
            failures.append(f"visible_magenta:{path}:{magenta_count}")

    report_path = Path(ns.report) if ns.report else root / "vegetation_analysis_report.json"
    if not report_path.is_absolute():
        report_path = repo / report_path
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps({
        "root": str(root),
        "expected_size": list(EXPECTED_SIZE),
        "strict": bool(ns.strict),
        "validator": "independent_visible_magenta_v2",
        "assets": assets,
        "failures": failures,
    }, indent=2) + "\n", encoding="utf-8")
    print(f"[vegetation] analyzed={len(assets)} strict={ns.strict} failures={len(failures)}")
    print(f"[vegetation] report={report_path}")
    for failure in failures[:20]:
        print(f"FAIL {failure}")
    return 0 if not failures else 2


if __name__ == "__main__":
    raise SystemExit(main())
