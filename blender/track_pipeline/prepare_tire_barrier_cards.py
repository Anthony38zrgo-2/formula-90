from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image

from vegetation_texture_common import prepare_precut_card_preserve_aspect


OUTPUT_SIZE = (512, 512)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description="Prepare deterministic La Chutana tire-barrier card textures.")
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--output-dir", required=True)
    args = parser.parse_args()

    manifest_path = Path(args.manifest).resolve()
    output_dir = Path(args.output_dir).resolve()
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("source_contract") != "mixed_rgba_cards_and_opaque_tiles":
        raise RuntimeError("Tire barrier manifest must declare mixed_rgba_cards_and_opaque_tiles")
    if manifest.get("module", {}).get("geometry") != "rectangular_prism":
        raise RuntimeError("Tire barrier module must declare rectangular_prism")

    texture_dir = output_dir / "textures"
    texture_dir.mkdir(parents=True, exist_ok=True)
    prepared = {}
    for source_id, entry in sorted(manifest["sources"].items()):
        source = manifest_path.parent / entry["file"]
        actual_hash = sha256(source)
        if actual_hash != entry["sha256"]:
            raise RuntimeError(f"Source hash mismatch for {source_id}: {actual_hash}")
        rgba = np.asarray(Image.open(source).convert("RGBA"), dtype=np.uint8)
        alpha = rgba[..., 3]
        kind = entry.get("kind", "precut_card")
        if kind == "precut_card" and (not np.any(alpha == 0) or not np.any(alpha > 0)):
            raise RuntimeError(f"Source is not a pre-cut RGBA card: {source}")
        if kind == "opaque_tile" and not np.all(alpha == 255):
            raise RuntimeError(f"Top tread must be an opaque tile: {source}")
        if kind == "opaque_tile":
            resized = Image.fromarray(rgba, "RGBA").resize(OUTPUT_SIZE, Image.Resampling.LANCZOS)
            output = np.asarray(resized, dtype=np.uint8).copy()
            output[..., 3] = 255
            metrics = {
                "source_size": [int(rgba.shape[1]), int(rgba.shape[0])],
                "output_size": list(OUTPUT_SIZE),
                "output_bbox": [0, 0, OUTPUT_SIZE[0], OUTPUT_SIZE[1]],
                "alpha_nonzero": int(OUTPUT_SIZE[0] * OUTPUT_SIZE[1]),
                "resize": "opaque_lanczos_square_tile",
            }
        else:
            output, metrics = prepare_precut_card_preserve_aspect(rgba, OUTPUT_SIZE)
        destination = texture_dir / f"{source_id}.png"
        Image.fromarray(output, "RGBA").save(destination)
        prepared[source_id] = {
            "source": str(source),
            "source_sha256": actual_hash,
            "texture": str(destination),
            "texture_sha256": sha256(destination),
            "kind": kind,
            "metrics": metrics,
        }

    report = {
        "schema_version": 1,
        "track_id": manifest["track_id"],
        "source_manifest": str(manifest_path),
        "source_manifest_sha256": sha256(manifest_path),
        "source_contract": manifest["source_contract"],
        "postprocess": "alpha_bbox_aspect_fit_premultiplied_resize_bottom_anchor",
        "output_size": list(OUTPUT_SIZE),
        "sources": prepared,
        "module": manifest["module"],
    }
    report_path = output_dir / "prepared_manifest.json"
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"prepared_sources": len(prepared), "manifest": str(report_path)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
