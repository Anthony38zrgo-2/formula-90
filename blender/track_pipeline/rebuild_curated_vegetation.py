from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image

from atomic_json import write_json_atomic
from pipeline_common import read_json
from texture_forge import save_image_atomic
from vegetation_texture_stylizer import palette_from_catalog, stylize_card_rgba, stylizer_recipe
from vegetation_texture_common import (
    POSTPROCESS_ID,
    POSTPROCESS_VERSION,
    pad_transparent_rgb,
    prepare_vegetation_card_rgba,
)


EXPECTED_SIZE = (128, 128)
SUPPORTED_CATEGORIES = {"trees", "bushes", "grass"}
SOURCE_CONTRACT = "precut_rgba_transparent"
PROPAGATION_LONGITUDES = ("west", "center", "east")
PROPAGATION_ALTITUDES = ("low", "medium", "high")


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


def _validate_precut_source(rgba: np.ndarray, source: Path) -> None:
    if rgba.ndim != 3 or rgba.shape[2] != 4:
        raise RuntimeError(f"Source must be RGBA: {source}")
    alpha = rgba[..., 3]
    if not np.any(alpha == 0):
        raise RuntimeError(f"Source is opaque; new vegetation sources must be pre-cut RGBA: {source}")
    if not np.any(alpha > 0):
        raise RuntimeError(f"Source has no visible pixels: {source}")


def _biome_id(manifest: dict) -> str:
    raw = str(manifest.get("biome", "")).replace("/", "_")
    if not raw:
        raise RuntimeError("Source manifest is missing biome")
    return raw


def _propagate_west_low_manifests(source_root: Path) -> int:
    baseline = source_root / "south_america" / "west" / "low" / "vegetation"
    created = 0
    baseline_manifests = {
        path.parent.name: json.loads(path.read_text(encoding="utf-8"))
        for path in baseline.glob("*/source_manifest.json")
    }
    if set(baseline_manifests) != SUPPORTED_CATEGORIES:
        raise RuntimeError(f"west/low propagation requires manifests for {sorted(SUPPORTED_CATEGORIES)}")
    for longitude in PROPAGATION_LONGITUDES:
        for altitude in PROPAGATION_ALTITUDES:
            if longitude == "west" and altitude == "low":
                continue
            biome_path = source_root / "south_america" / longitude / altitude / "vegetation"
            biome_id = f"south_america/{longitude}/{altitude}"
            for category, template in sorted(baseline_manifests.items()):
                target_dir = biome_path / category
                target_dir.mkdir(parents=True, exist_ok=True)
                assets = {}
                relative_source_root = Path("../../../../west/low/vegetation") / category
                singular_category = {"trees": "tree", "bushes": "bush", "grass": "grass"}[category]
                for asset_id, entry in sorted(template.get("assets", {}).items()):
                    filename = str(entry["source"])
                    variant = asset_id.rsplit("_", 1)[-1]
                    output_name = f"south_america_{longitude}_{altitude}_{singular_category}_{variant}.png"
                    assets[asset_id] = {
                        "source": (relative_source_root / filename).as_posix(),
                        "source_sha256": entry["source_sha256"],
                        "output": f"blender/generated/la_chutana/textures/biomes/south_america/{longitude}/{altitude}/{output_name}",
                    }
                manifest = {
                    "schema_version": 2,
                    "track_id": "la_chutana",
                    "biome": biome_id,
                    "category": category,
                    "generator": "propagated latest west/low RGBA reference sources",
                    "source_contract": SOURCE_CONTRACT,
                    "postprocess": POSTPROCESS_ID,
                    "assets": assets,
                }
                write_json_atomic(target_dir / "source_manifest.json", manifest)
                created += 1
    return created


def _process_asset(repo: Path, palette_catalog: Path, manifest_path: Path, manifest: dict, asset_id: str, entry: dict, pass_index: int) -> dict:
    category = str(manifest.get("category", ""))
    if category not in SUPPORTED_CATEGORIES:
        raise RuntimeError(f"Unsupported source-backed vegetation category: {category}")
    if manifest.get("source_contract", SOURCE_CONTRACT) != SOURCE_CONTRACT:
        raise RuntimeError(f"Source manifest must declare {SOURCE_CONTRACT}: {manifest_path}")
    source = (manifest_path.parent / entry["source"]).resolve()
    destination = _safe_repo_path(repo, entry["output"])
    if not source.exists():
        raise FileNotFoundError(source)

    image = Image.open(source).convert("RGBA")
    original_size = list(image.size)
    image = _center_crop_square(image)
    rgba = np.asarray(image, dtype=np.uint8)
    _validate_precut_source(rgba, source)
    output, metrics = prepare_vegetation_card_rgba(
        rgba,
        output_size=EXPECTED_SIZE,
        background_rgb=None,
    )
    palette, palette_hex = palette_from_catalog(palette_catalog, _biome_id(manifest), category)
    output, stylizer_metrics = stylize_card_rgba(output, palette, category)
    output = pad_transparent_rgb(output, radius=4)
    metrics["stylizer"] = stylizer_metrics

    destination.parent.mkdir(parents=True, exist_ok=True)
    save_image_atomic(Image.fromarray(output), destination)
    recipe = {
        "id": POSTPROCESS_ID,
        "version": POSTPROCESS_VERSION,
        "pass": int(pass_index),
        "source_contract": SOURCE_CONTRACT,
        "key_stage": "not_applied_to_precut_sources",
        "resize": "premultiplied_lanczos4",
        "transparent_rgb": "foreground_edge_padding_4px",
        "bottom_anchor": "last_visible_alpha_row",
        "stylizer": stylizer_recipe(category, palette_hex),
    }
    return {
        "asset_id": asset_id,
        "category": category,
        "manifest": str(manifest_path),
        "source": str(source),
        "source_size": original_size,
        "source_sha256": sha256(source),
        "destination": str(destination),
        "destination_sha256": sha256(destination),
        "declared_output_sha256": entry.get("output_sha256"),
        "source_contract": SOURCE_CONTRACT,
        "postprocess": recipe,
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
    tmp_path = forge_path.with_name("texture_forge_manifest.tmp")
    tmp_path.write_text(json.dumps(forge, indent=2) + "\n", encoding="utf-8")
    tmp_path.replace(forge_path)


def _aggregate_recipe(assets: list[dict], pass_index: int) -> dict:
    recipe = dict(assets[0]["postprocess"]) if assets else {
        "id": POSTPROCESS_ID,
        "version": POSTPROCESS_VERSION,
        "pass": int(pass_index),
    }
    recipe["scope"] = "source_backed_manifest_assets"
    recipe["source_contract"] = SOURCE_CONTRACT
    return recipe


def main() -> int:
    parser = argparse.ArgumentParser(description="Rebuild source-backed curated vegetation before Blender consumes it.")
    parser.add_argument("--config", required=True)
    parser.add_argument("--pass-index", type=int, choices=(1, 2), default=1)
    parser.add_argument("--report", default="")
    parser.add_argument("--propagate-west-low", action="store_true", help="Create shared-source manifests for all South America biomes from the approved west/low bank.")
    ns = parser.parse_args()

    config_path = Path(ns.config).resolve()
    repo = config_path.parents[3]
    config = read_json(config_path)
    track_id = str(config.get("track_id") or config_path.stem)
    texture_root = repo / config["generated_dir"] / "textures"
    source_root = repo / "blender" / "assets" / "texture_sources" / track_id
    propagated = _propagate_west_low_manifests(source_root) if ns.propagate_west_low else 0
    palette_catalog = repo / "blender" / "assets" / "texture_sources" / track_id / "vegetation" / "vegetation_palette_reference.json"
    if not palette_catalog.exists():
        raise FileNotFoundError(palette_catalog)
    manifests = sorted(source_root.rglob("source_manifest.json")) if source_root.exists() else []

    assets: list[dict] = []
    for manifest_path in manifests:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        category = manifest.get("category")
        if category not in SUPPORTED_CATEGORIES:
            continue
        manifest["source_contract"] = SOURCE_CONTRACT
        manifest.pop("background_rgb", None)
        manifest.pop("background_key_name", None)
        manifest.pop("background_key_hex", None)
        manifest.pop("source_key_migration", None)
        manifest_assets = manifest.get("assets", {})
        processed_here: list[dict] = []
        for asset_id, entry in sorted(manifest_assets.items()):
            if not isinstance(entry, dict) or "source" not in entry or "output" not in entry:
                raise RuntimeError(f"Invalid asset {asset_id} in {manifest_path}")
            result = _process_asset(repo, palette_catalog, manifest_path, manifest, asset_id, entry, ns.pass_index)
            processed_here.append(result)
            assets.append(result)
            entry["output_sha256"] = result["destination_sha256"]
        if processed_here:
            manifest["postprocess"] = POSTPROCESS_ID
            manifest["postprocess_version"] = POSTPROCESS_VERSION
            manifest["postprocess_recipe"] = processed_here[0]["postprocess"]
            manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

    if assets:
        recipe = _aggregate_recipe(assets, ns.pass_index)
        _update_forge_manifest(texture_root, assets, recipe)

    report_path = Path(ns.report) if ns.report else texture_root / "curated_vegetation_rebuild_report.json"
    if not report_path.is_absolute():
        report_path = repo / report_path
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps({
        "operation": "source_backed_curated_vegetation_rebuild",
        "track_id": track_id,
        "source_contract": SOURCE_CONTRACT,
        "source_key_policy": "not_applied_to_new_pre-cut_sources; legacy assets remain outside this source-backed path",
        "manifest_count": len(manifests),
        "propagated_manifest_count": propagated,
        "asset_count": len(assets),
        "assets": assets,
    }, indent=2) + "\n", encoding="utf-8")
    print(f"[vegetation-rebuild] manifests={len(manifests)} assets={len(assets)} propagated={propagated} source_contract={SOURCE_CONTRACT} stylizer=deterministic")
    print(f"[vegetation-rebuild] report={report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
