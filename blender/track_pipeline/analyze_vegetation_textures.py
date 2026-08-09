from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

import numpy as np
from PIL import Image

from pipeline_common import read_json
from vegetation_texture_common import magenta_key_mask, transparent_edge


ASSET_RE = re.compile(r"_(tree|bush|grass)_\d+\.png$")
EXPECTED_SIZE = (128, 128)


def category_for(path: Path) -> str | None:
    match = ASSET_RE.search(path.name)
    if not match:
        return None
    return {"tree": "trees", "bush": "bushes", "grass": "grass"}[match.group(1)]


def edge_magenta_opaque(rgba: np.ndarray, pass_index: int) -> int:
    candidate = magenta_key_mask(rgba[..., :3], pass_index=pass_index)
    alpha = rgba[..., 3] > 0
    radius = 2 if pass_index == 1 else 5
    near_transparent = transparent_edge(rgba[..., 3], radius=radius)
    return int((candidate & alpha & near_transparent).sum())


def main() -> int:
    parser = argparse.ArgumentParser(description="Analyze deterministic vegetation card outputs.")
    parser.add_argument("--config", required=True)
    parser.add_argument("--report", default="")
    parser.add_argument("--strict", action="store_true")
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
        edge_magenta = edge_magenta_opaque(rgba, ns.pass_index)
        item = {
            "path": str(path),
            "category": category_for(path),
            "mode": image.mode,
            "size": list(image.size),
            "alpha_nonzero": int(visible.sum()),
            "bbox": bbox,
            "bottom_gap_px": bottom_gap,
            "edge_magenta_opaque": edge_magenta,
        }
        assets.append(item)
        if image.size != EXPECTED_SIZE or image.mode not in {"RGBA", "LA"}:
            failures.append(f"format:{path}")
        if bbox is None:
            failures.append(f"empty:{path}")
        if ns.strict and bottom_gap != 0:
            failures.append(f"bottom_gap:{path}:{bottom_gap}")
        if ns.strict and edge_magenta != 0:
            failures.append(f"edge_magenta:{path}:{edge_magenta}")

    report_path = Path(ns.report) if ns.report else root / "vegetation_analysis_report.json"
    if not report_path.is_absolute():
        report_path = repo / report_path
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps({
        "root": str(root),
        "expected_size": list(EXPECTED_SIZE),
        "strict": bool(ns.strict),
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
