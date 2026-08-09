from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image

from vegetation_texture_common import DEFAULT_BACKGROUND_HEX, DEFAULT_BACKGROUND_RGB, postprocess_recipe, prepare_vegetation_card_rgba


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


def parse_rgb(value: str) -> np.ndarray:
    value = value.strip()
    if value.startswith("#") and len(value) == 7:
        return np.asarray([int(value[i:i + 2], 16) for i in (1, 3, 5)], dtype=np.uint8)
    parts = [int(part.strip()) for part in value.split(",")]
    if len(parts) != 3 or any(part < 0 or part > 255 for part in parts):
        raise ValueError(f"Expected #RRGGBB or R,G,B in 0..255, got: {value}")
    return np.asarray(parts, dtype=np.uint8)


def _center_crop_square(image: Image.Image) -> Image.Image:
    width, height = image.size
    if width == height:
        return image
    side = min(width, height)
    left = (width - side) // 2
    top = (height - side) // 2
    return image.crop((left, top, left + side, top + side))


def prepare_card(source: Path, destination: Path, background_rgb, pass_index: int) -> dict:
    image = Image.open(source).convert("RGBA")
    width, height = image.size
    image = _center_crop_square(image)
    rgba = np.asarray(image, dtype=np.uint8)
    output, metrics = prepare_vegetation_card_rgba(
        rgba,
        output_size=EXPECTED_SIZE,
        background_rgb=background_rgb,
        pass_index=pass_index,
    )
    destination.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(output, "RGBA").save(destination)
    return {
        "source": str(source),
        "source_sha256": sha256(source),
        "source_size": [width, height],
        "destination": str(destination),
        "destination_size": list(EXPECTED_SIZE),
        "destination_sha256": sha256(destination),
        "postprocess": postprocess_recipe(pass_index, background_rgb),
        "metrics": metrics,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Import four generated tree sources using source-resolution chroma keying.")
    parser.add_argument("--root", required=True, help="Generated La Chutana textures root")
    parser.add_argument("--report", default="", help="Optional provenance report path")
    parser.add_argument("--map", dest="mappings", action="append", required=True)
    parser.add_argument("--background-rgb", default=DEFAULT_BACKGROUND_HEX)
    parser.add_argument("--pass-index", type=int, choices=(1, 2), default=1)
    ns = parser.parse_args()

    root = Path(ns.root).resolve()
    active = root / "biomes" / "south_america" / "west" / "low"
    sources = parse_mapping(ns.mappings)
    background_rgb = parse_rgb(ns.background_rgb)
    assets = []
    for key, filename in TREE_NAMES.items():
        source = sources[key]
        destination = active / filename
        if not source.exists():
            raise FileNotFoundError(source)
        assets.append(prepare_card(source, destination, background_rgb, ns.pass_index))

    report_path = Path(ns.report) if ns.report else root / "tree_regeneration_import_report.json"
    if not report_path.is_absolute():
        report_path = root / report_path
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps({
        "operation": "reference_tree_regeneration_import",
        "active_biome": "south_america/west/low",
        "expected_size": list(EXPECTED_SIZE),
        "required_source_background_key": {
            "name": "electric_cyan",
            "hex": DEFAULT_BACKGROUND_HEX,
            "rgb": DEFAULT_BACKGROUND_RGB.tolist(),
        },
        "postprocess": postprocess_recipe(ns.pass_index, background_rgb),
        "assets": assets,
    }, indent=2) + "\n", encoding="utf-8")
    print(f"[tree-import] imported={len(assets)} size={EXPECTED_SIZE[0]}x{EXPECTED_SIZE[1]} key={DEFAULT_BACKGROUND_HEX}")
    print(f"[tree-import] report={report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
