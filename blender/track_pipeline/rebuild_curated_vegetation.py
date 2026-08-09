from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image

from pipeline_common import read_json
from vegetation_texture_common import POSTPROCESS_ID, POSTPROCESS_VERSION, postprocess_recipe, prepare_vegetation_card_rgba


EXPECTED_SIZE = (128, 128)
SUPPORTED_CATEGORIES = {"trees", "bushes", "grass"}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _center_crop_square(image: Image.Image) -> Image.Image:
    width, height = image.size
    if width == height:
        return image
    side = min(width, height)
    left = (width - side) // 2
    top = (height - side) // 2
    return image.crop((left, top, left + side, top + side))


def _safe_repo_path(repo: Path, raw: str) -> Path:
    path = (repo / raw).resolve()
    try:
        path.relative_to(repo.resolve())
    except ValueError as exc:
        raise RuntimeError(f"Manifest path escapes repository: {raw}") from exc
    return path


def _process_asset(repo: Path, manifest_path: Path, manifest: dict, asset_id: str, entry: dict, pass_index: int) -> dict:
    background_rgb = np.asarray(manifest.get("background_rgb", [255, 0, 255]), dtype=np.uint8)
    if background_rgb.shape != (3,):
        raise RuntimeError(f"Invalid background_rgb in {manifest_path}")
    source = (manifest_path.parent / entry["source"]).resolve()
    destination = _safe_repo_path(repo, entry["output"])
    if not source.exists():
        raise FileNotFoundError(source)

    image = Image.open(source).convert("RGBA")
    original_size = list(image.size)
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
        "asset_id": asset_id,
        "category": manifest.get("category"),
        "manifest": str(manifest_path),
        "source": str(source),
        "source_size": original_size,
        "source_sha256": sha256(source),
        "destination": str(destination),
        "destination_sha256": sha256(destination),
        "declared_output_sha256": entry.get("output_sha256"),
        "postprocess": postprocess_recipe(pass_index, background_rgb),
        "metrics": metrics,
    }


def _update_forge_manifest(texture_root: Path, assets: list[dict], recipe: dict) -> None:
    forge_path = texture_root / "texture_forge_manifest.json"
    if not forge_path.exists():
        raise RuntimeError(f"Missing Texture Forge manifest: {forge_path}")
    forge = json.loads(forge_path.read_text(encoding="utf-8"))
    texture_root_resolved = texture_root.resolve()
    updated = []
    for asset in assets:
        destination = Path(asset["destination"]).resolve()
        try:
            rel = destination.relative_to(texture_root_resolved).as_posix()
        except ValueError as exc:
            raise RuntimeError(f"Curated vegetation output is outside texture root: {destination}") from exc
        entry = forge.get("entries", {}).get(rel)
        if entry is None:
            raise RuntimeError(f"Missing forge manifest entry: {rel}")
        entry["sha256"] = asset["destination_sha256"]
        entry["vegetation_postprocess"] = asset["postprocess"]
        updated.append(rel)
    forge["curated_vegetation_rebuild"] = {
        **recipe,
        "assets": updated,
    }
    forge_path.write_text(json.dumps(forge, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Rebuild source-backed curated vegetation before Blender consumes it.")
    parser.add_argument("--config", required=True)
    parser.add_argument("--pass-index", type=int, choices=(1, 2), default=1)
    parser.add_argument("--report", default="")
    ns = parser.parse_args()

    config_path = Path(ns.config).resolve()
    repo = config_path.parents[3]
    config = read_json(config_path)
    track_id = str(config.get("track_id") or config_path.stem)
    texture_root = repo / config["generated_dir"] / "textures"
    source_root = repo / "blender" / "assets" / "texture_sources" / track_id
    manifests = sorted(source_root.rglob("source_manifest.json")) if source_root.exists() else []

    assets: list[dict] = []
    for manifest_path in manifests:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        category = manifest.get("category")
        if category not in SUPPORTED_CATEGORIES:
            continue
        manifest_assets = manifest.get("assets", {})
        processed_here: list[dict] = []
        for asset_id, entry in sorted(manifest_assets.items()):
            if not isinstance(entry, dict) or "source" not in entry or "output" not in entry:
                raise RuntimeError(f"Invalid asset {asset_id} in {manifest_path}")
            result = _process_asset(repo, manifest_path, manifest, asset_id, entry, ns.pass_index)
            processed_here.append(result)
            assets.append(result)
            entry["output_sha256"] = result["destination_sha256"]
        if processed_here:
            manifest["postprocess"] = POSTPROCESS_ID
            manifest["postprocess_version"] = POSTPROCESS_VERSION
            manifest["postprocess_recipe"] = processed_here[0]["postprocess"]
            manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

    if assets:
        # Per-asset manifests can declare different key colors; the top-level recipe records the algorithm only.
        recipe = postprocess_recipe(ns.pass_index)
        _update_forge_manifest(texture_root, assets, recipe)

    report_path = Path(ns.report) if ns.report else texture_root / "curated_vegetation_rebuild_report.json"
    if not report_path.is_absolute():
        report_path = repo / report_path
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps({
        "operation": "source_backed_curated_vegetation_rebuild",
        "track_id": track_id,
        "manifest_count": len(manifests),
        "asset_count": len(assets),
        "assets": assets,
    }, indent=2) + "\n", encoding="utf-8")
    print(f"[vegetation-rebuild] manifests={len(manifests)} assets={len(assets)}")
    print(f"[vegetation-rebuild] report={report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
