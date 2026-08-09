from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image

from pipeline_common import read_json
from vegetation_texture_common import (
    DEFAULT_BACKGROUND_HEX,
    DEFAULT_BACKGROUND_NAME,
    DEFAULT_BACKGROUND_RGB,
    LEGACY_MAGENTA_RGB,
    key_background_rgba,
    key_name_for_rgb,
)

SUPPORTED_CATEGORIES = {"trees", "bushes", "grass"}
MIGRATION_ID = "vegetation_source_key_recomposite"
MIGRATION_VERSION = 1


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _key_hex(key_rgb) -> str:
    key = np.asarray(key_rgb, dtype=np.uint8).reshape(3)
    return "#%02X%02X%02X" % tuple(int(v) for v in key)


def _rgb(value, manifest_path: Path) -> np.ndarray:
    arr = np.asarray(value, dtype=np.uint8)
    if arr.shape != (3,):
        raise RuntimeError(f"Invalid background_rgb in {manifest_path}")
    return arr


def rekey_source_rgba(
    rgba: np.ndarray,
    source_key_rgb=LEGACY_MAGENTA_RGB,
    target_key_rgb=DEFAULT_BACKGROUND_RGB,
    pass_index: int = 1,
) -> tuple[np.ndarray, dict]:
    """Extract foreground at source resolution, then recomposite it onto a new opaque key color."""
    keyed, key_metrics = key_background_rgba(rgba, source_key_rgb, pass_index)
    alpha = keyed[..., 3:4].astype(np.float32) / 255.0
    foreground = keyed[..., :3].astype(np.float32)
    target = np.asarray(target_key_rgb, dtype=np.float32).reshape(1, 1, 3)
    composite = foreground * alpha + target * (1.0 - alpha)
    out = np.empty_like(keyed)
    out[..., :3] = np.clip(np.rint(composite), 0, 255).astype(np.uint8)
    out[..., 3] = 255
    return out, {
        "id": MIGRATION_ID,
        "version": MIGRATION_VERSION,
        "source_key_name": key_name_for_rgb(source_key_rgb),
        "source_key_hex": _key_hex(source_key_rgb),
        "target_key_name": key_name_for_rgb(target_key_rgb),
        "target_key_hex": _key_hex(target_key_rgb),
        "pass": int(pass_index),
        "key_metrics": key_metrics,
    }


def _migrate_manifest(manifest_path: Path, pass_index: int) -> dict:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    category = manifest.get("category")
    if category not in SUPPORTED_CATEGORIES:
        return {"manifest": str(manifest_path), "category": category, "status": "ignored", "assets": []}

    source_key = _rgb(manifest.get("background_rgb", DEFAULT_BACKGROUND_RGB.tolist()), manifest_path)
    if np.array_equal(source_key, DEFAULT_BACKGROUND_RGB):
        return {
            "manifest": str(manifest_path),
            "category": category,
            "status": "already_canonical",
            "source_key_hex": DEFAULT_BACKGROUND_HEX,
            "assets": [],
        }
    if not np.array_equal(source_key, LEGACY_MAGENTA_RGB):
        raise RuntimeError(
            f"Unsupported vegetation source key {_key_hex(source_key)} in {manifest_path}; "
            f"expected legacy #FF00FF or canonical {DEFAULT_BACKGROUND_HEX}."
        )

    migrated_assets = []
    for asset_id, entry in sorted(manifest.get("assets", {}).items()):
        if not isinstance(entry, dict) or "source" not in entry:
            raise RuntimeError(f"Invalid asset {asset_id} in {manifest_path}")
        source = (manifest_path.parent / entry["source"]).resolve()
        if not source.exists():
            raise FileNotFoundError(source)
        image = Image.open(source).convert("RGBA")
        rgba = np.asarray(image, dtype=np.uint8)
        migrated, metrics = rekey_source_rgba(
            rgba,
            source_key_rgb=LEGACY_MAGENTA_RGB,
            target_key_rgb=DEFAULT_BACKGROUND_RGB,
            pass_index=pass_index,
        )
        Image.fromarray(migrated, "RGBA").save(source)
        digest = sha256(source)
        entry["source_sha256"] = digest
        migrated_assets.append({
            "asset_id": asset_id,
            "source": str(source),
            "source_sha256": digest,
            "source_size": list(image.size),
            "migration": metrics,
        })

    manifest["background_rgb"] = DEFAULT_BACKGROUND_RGB.tolist()
    manifest["background_key_name"] = DEFAULT_BACKGROUND_NAME
    manifest["background_key_hex"] = DEFAULT_BACKGROUND_HEX
    manifest["source_key_migration"] = {
        "id": MIGRATION_ID,
        "version": MIGRATION_VERSION,
        "from": {"name": key_name_for_rgb(source_key), "hex": _key_hex(source_key), "rgb": source_key.tolist()},
        "to": {"name": DEFAULT_BACKGROUND_NAME, "hex": DEFAULT_BACKGROUND_HEX, "rgb": DEFAULT_BACKGROUND_RGB.tolist()},
        "pass": int(pass_index),
        "method": "source-resolution key extraction and opaque recomposite",
    }
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return {
        "manifest": str(manifest_path),
        "category": category,
        "status": "migrated",
        "source_key_hex": "#FF00FF",
        "target_key_hex": DEFAULT_BACKGROUND_HEX,
        "assets": migrated_assets,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Migrate legacy solid-key vegetation sources to canonical electric cyan.")
    parser.add_argument("--config", required=True)
    parser.add_argument("--pass-index", type=int, choices=(1, 2), default=1)
    parser.add_argument("--report", default="")
    ns = parser.parse_args()

    config_path = Path(ns.config).resolve()
    repo = config_path.parents[3]
    config = read_json(config_path)
    track_id = str(config.get("track_id") or config_path.stem)
    source_root = repo / "blender" / "assets" / "texture_sources" / track_id
    texture_root = repo / config["generated_dir"] / "textures"
    manifests = sorted(source_root.rglob("source_manifest.json")) if source_root.exists() else []

    results = [_migrate_manifest(path, ns.pass_index) for path in manifests]
    supported = [item for item in results if item["status"] != "ignored"]
    migrated = [item for item in supported if item["status"] == "migrated"]
    migrated_assets = sum(len(item["assets"]) for item in migrated)

    for item in supported:
        manifest_path = Path(item["manifest"])
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        key = _rgb(manifest.get("background_rgb"), manifest_path)
        if not np.array_equal(key, DEFAULT_BACKGROUND_RGB):
            raise RuntimeError(f"Source key migration incomplete for {manifest_path}")

    report_path = Path(ns.report) if ns.report else texture_root / "vegetation_source_key_migration_report.json"
    if not report_path.is_absolute():
        report_path = repo / report_path
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps({
        "operation": "vegetation_source_key_migration",
        "canonical_key": {"name": DEFAULT_BACKGROUND_NAME, "hex": DEFAULT_BACKGROUND_HEX, "rgb": DEFAULT_BACKGROUND_RGB.tolist()},
        "manifest_count": len(supported),
        "migrated_manifest_count": len(migrated),
        "migrated_asset_count": migrated_assets,
        "results": results,
    }, indent=2) + "\n", encoding="utf-8")
    print(
        f"[vegetation-source-key] manifests={len(supported)} migrated_manifests={len(migrated)} "
        f"migrated_assets={migrated_assets} canonical={DEFAULT_BACKGROUND_HEX}"
    )
    print(f"[vegetation-source-key] report={report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
