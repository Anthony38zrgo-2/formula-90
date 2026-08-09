from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image


EXPECTED_SIZE = (128, 128)
TREE_NAMES = {
    "tree_01": "south_america_west_low_tree_01.png",
    "tree_02": "south_america_west_low_tree_02.png",
    "tree_03": "south_america_west_low_tree_03.png",
    "tree_04": "south_america_west_low_tree_04.png",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def parse_mapping(values: list[str]) -> dict[str, Path]:
    out: dict[str, Path] = {}
    for value in values:
        key, separator, raw_path = value.partition("=")
        if separator != "=" or key not in TREE_NAMES or not raw_path:
            raise ValueError(f"Expected tree_01=<path> ... tree_04=<path>, got: {value}")
        if key in out:
            raise ValueError(f"Duplicate source mapping: {key}")
        out[key] = Path(raw_path).expanduser().resolve()
    missing = sorted(set(TREE_NAMES) - set(out))
    if missing:
        raise ValueError(f"Missing source mappings: {', '.join(missing)}")
    return out


def prepare_card(source: Path, destination: Path) -> dict:
    image = Image.open(source).convert("RGBA")
    width, height = image.size
    if width != height:
        side = min(width, height)
        left = (width - side) // 2
        top = (height - side) // 2
        image = image.crop((left, top, left + side, top + side))
    image = image.resize(EXPECTED_SIZE, Image.Resampling.LANCZOS)
    rgba = np.asarray(image, dtype=np.uint8).copy()
    Image.fromarray(rgba).save(destination)
    return {
        "source": str(source),
        "source_sha256": sha256(source),
        "source_size": [width, height],
        "destination": str(destination),
        "destination_size": list(EXPECTED_SIZE),
        "destination_sha256": sha256(destination),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Import four generated tree sources into the active 128x128 card bank.")
    parser.add_argument("--root", required=True, help="Generated La Chutana textures root")
    parser.add_argument("--report", default="", help="Optional provenance report path")
    parser.add_argument("--map", dest="mappings", action="append", required=True)
    ns = parser.parse_args()

    root = Path(ns.root).resolve()
    active = root / "biomes" / "south_america" / "west" / "low"
    sources = parse_mapping(ns.mappings)
    assets = []
    for key, filename in TREE_NAMES.items():
        source = sources[key]
        destination = active / filename
        if not source.exists():
            raise FileNotFoundError(source)
        if not destination.exists():
            raise FileNotFoundError(destination)
        assets.append(prepare_card(source, destination))

    report_path = Path(ns.report) if ns.report else root / "tree_regeneration_import_report.json"
    if not report_path.is_absolute():
        report_path = root / report_path
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps({
        "operation": "reference_tree_regeneration_import",
        "active_biome": "south_america/west/low",
        "expected_size": list(EXPECTED_SIZE),
        "assets": assets,
    }, indent=2) + "\n", encoding="utf-8")
    print(f"[tree-import] imported={len(assets)} size={EXPECTED_SIZE[0]}x{EXPECTED_SIZE[1]}")
    print(f"[tree-import] report={report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
