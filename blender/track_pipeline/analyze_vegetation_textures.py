from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

import numpy as np
from PIL import Image

from pipeline_common import read_json
from vegetation_texture_common import DEFAULT_BACKGROUND_HEX, DEFAULT_BACKGROUND_RGB, key_candidate_mask


ASSET_RE = re.compile(r"_(tree|bush|grass)_\d+\.png$")
EXPECTED_SIZE = (128, 128)


def category_for(path: Path) -> str | None:
    match = ASSET_RE.search(path.name)
    if not match:
        return None
    return {"tree": "trees", "bush": "bushes", "grass": "grass"}[match.group(1)]


def parse_rgb(value: str) -> np.ndarray:
    value = value.strip()
    if value.startswith("#") and len(value) == 7:
        return np.asarray([int(value[i:i + 2], 16) for i in (1, 3, 5)], dtype=np.uint8)
    parts = [int(part.strip()) for part in value.split(",")]
    if len(parts) != 3 or any(part < 0 or part > 255 for part in parts):
        raise ValueError(f"Expected #RRGGBB or R,G,B in 0..255, got: {value}")
    return np.asarray(parts, dtype=np.uint8)


def visible_key_mask(rgba: np.ndarray, key_rgb=DEFAULT_BACKGROUND_RGB) -> np.ndarray:
    alpha = rgba[..., 3]
    return (alpha >= 16) & key_candidate_mask(rgba[..., :3], key_rgb=key_rgb, tolerance=170.0)


def main() -> int:
    parser = argparse.ArgumentParser(description="Analyze deterministic vegetation card outputs independently of the keyer.")
    parser.add_argument("--config", required=True)
    parser.add_argument("--report", default="")
    parser.add_argument("--strict", action="store_true")
    parser.add_argument("--pass-index", type=int, choices=(1, 2), default=1)
    parser.add_argument("--key-rgb", default=DEFAULT_BACKGROUND_HEX)
    ns = parser.parse_args()

    key_rgb = parse_rgb(ns.key_rgb)
    key_hex = "#%02X%02X%02X" % tuple(int(v) for v in key_rgb)
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
        key_visible = visible_key_mask(rgba, key_rgb=key_rgb)
        key_count = int(key_visible.sum())
        semi_transparent_key = int((key_visible & (alpha < 224)).sum())
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
            "visible_key_px": key_count,
            "semi_transparent_key_px": semi_transparent_key,
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
        if ns.strict and key_count != 0:
            failures.append(f"visible_key:{path}:{key_count}")

    report_path = Path(ns.report) if ns.report else root / "vegetation_analysis_report.json"
    if not report_path.is_absolute():
        report_path = repo / report_path
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps({
        "root": str(root),
        "expected_size": list(EXPECTED_SIZE),
        "strict": bool(ns.strict),
        "validator": "independent_visible_key_v3",
        "key_rgb": key_rgb.tolist(),
        "key_hex": key_hex,
        "assets": assets,
        "failures": failures,
    }, indent=2) + "\n", encoding="utf-8")
    print(f"[vegetation] analyzed={len(assets)} strict={ns.strict} key={key_hex} failures={len(failures)}")
    print(f"[vegetation] report={report_path}")
    for failure in failures[:20]:
        print(f"FAIL {failure}")
    return 0 if not failures else 2


if __name__ == "__main__":
    raise SystemExit(main())
