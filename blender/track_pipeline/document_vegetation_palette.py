from __future__ import annotations

import argparse
import json
from pathlib import Path

from pipeline_common import read_json
from vegetation_texture_common import DEFAULT_BACKGROUND_HEX, DEFAULT_BACKGROUND_NAME, DEFAULT_BACKGROUND_RGB


def _hex(color) -> str:
    return "#%02X%02X%02X" % tuple(int(round(float(c) * 255.0)) for c in color)


def build_palette_reference():
    try:
        from procedural_catalog import palette_for, supported_biomes  # type: ignore
    except Exception:
        return {"warning": "procedural_catalog import unavailable in this environment"}

    out = {}
    for biome in supported_biomes():
        palette = palette_for(biome)
        out[biome.id] = {
            "tree": [_hex(c) for c in palette["tree"]],
            "bush": [_hex(c) for c in palette["bush"]],
            "grass": [_hex(c) for c in palette["grass"]],
            "terrain": {
                "green": _hex(palette["terrain"]["green"]),
                "dry": _hex(palette["terrain"]["dry"]),
                "dirt": _hex(palette["terrain"]["dirt"]),
            },
        }
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description="Write vegetation key-color and palette reference documentation.")
    parser.add_argument("--config", required=True)
    ns = parser.parse_args()

    config_path = Path(ns.config).resolve()
    repo = config_path.parents[3]
    config = read_json(config_path)
    track_id = str(config.get("track_id") or config_path.stem)
    doc_root = repo / "blender" / "assets" / "texture_sources" / track_id / "vegetation"
    doc_root.mkdir(parents=True, exist_ok=True)

    palette_reference = build_palette_reference()
    (doc_root / "vegetation_palette_reference.json").write_text(
        json.dumps({
            "source_contract": "precut_rgba_transparent",
            "source_key": {
                "name": DEFAULT_BACKGROUND_NAME,
                "hex": DEFAULT_BACKGROUND_HEX,
                "rgb": DEFAULT_BACKGROUND_RGB.tolist(),
            },
            "legacy_compatibility_key": {
                "name": "legacy_magenta",
                "hex": "#FF00FF",
                "rgb": [255, 0, 255],
            },
            "palette": palette_reference,
        }, indent=2) + "\n",
        encoding="utf-8",
    )

    md = f"""# Vegetation color policy

## Source contract

New vegetation sources must be delivered as pre-cut RGBA images:

- transparent background in the alpha channel;
- no opaque studio background;
- no chroma-key recut in the normal source-backed path;
- bottom of the visible subject aligned by the deterministic pipeline.

The canonical compatibility color remains documented as `{DEFAULT_BACKGROUND_NAME}` / `{DEFAULT_BACKGROUND_HEX}` / `{DEFAULT_BACKGROUND_RGB.tolist()}`.
It may be used only when an external generator cannot emit transparency and a temporary local extraction is explicitly recorded.

Legacy magenta (`#FF00FF`) is compatibility-only for old assets. It must never be the default key for new sources.

## Palette lock

The current procedural vegetation palette is snapshotted in `vegetation_palette_reference.json`.
Use that file as the canonical reference to avoid drift in tree, bush, grass, and terrain colors across biomes.

## Deterministic stylization

`vegetation_texture_stylizer.py` maps visible RGB to this catalog, applies fixed Bayer dithering and derives fake occlusion from luminance and alpha topology. Alpha, bounding box and placement semantics are preserved.
"""
    (doc_root / "VEGETATION_COLOR_POLICY.md").write_text(md, encoding="utf-8")
    print(f"[vegetation-docs] root={doc_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
