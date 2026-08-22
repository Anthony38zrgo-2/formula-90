"""Generate configs/asset_registry.json from existing asset manifests.

Deterministic, auditable seed for the Asset Registry. Reads the raw vegetation
manifest and the semantic object catalog, computes source SHA-256 hashes and
writes the canonical registry. Run again to refresh (output is deterministic).
"""

from __future__ import annotations

import argparse
from pathlib import Path
import csv
import hashlib
import json

from asset_registry import SCHEMA_VERSION, PROFILE_VERSION

REPO = Path(__file__).resolve().parents[2]

MANIFEST = REPO / "blender" / "generated" / "la_chutana" / "raw_vegetation" / "assets_v2" / "manifest.csv"
CATALOG = REPO / "blender" / "track_pipeline" / "layouts" / "la_chutana" / "object_catalog.json"
OUTPUT = REPO / "blender" / "track_pipeline" / "configs" / "asset_registry.json"
HYBRID_VEGETATION = REPO / "game" / "resources" / "environment" / "assets" / "manifest.json"


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def budget_for(category: str, catalog: dict) -> dict[str, int]:
    zone = catalog.get("zones", {}).get(category, {})
    return {
        # Track-scoped placement totals live on the catalog zones, so no single
        # vegetation variant is mandatory here. Each variant only gets a ceiling
        # equal to the whole category total, so one variant can never exceed the
        # category's budget by itself. The importer/compiler enforces the exact
        # per-track totals from the catalog.
        "min_instances": 0,
        "max_instances": int(zone.get("max_count", 0)),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", default=str(OUTPUT))
    args = parser.parse_args()

    assets: list[dict] = []

    with MANIFEST.open(encoding="utf-8", newline="") as handle:
        for row in csv.DictReader(handle):
            rel = row["file"].replace("\\", "/")
            path = REPO / rel
            assets.append({
                "id": Path(rel).stem,
                "kind": "vegetation",
                "category": row["category"],
                "source": rel,
                "source_sha256": sha256_file(path) if path.exists() else None,
                "dimensions_m": {
                    "width": float(row["width_x"]),
                    "height": float(row["height_y"]),
                    "depth": float(row["depth_z"]),
                },
                "preview": None,
                "collision_class": "none",
                "budget": {"min_instances": 0, "max_instances": 0},
                "metadata": {"origin": "assets_v2/manifest.csv"},
            })

    if HYBRID_VEGETATION.exists():
        library = json.loads(HYBRID_VEGETATION.read_text(encoding="utf-8"))
        for item in library.get("assets", []):
            source = item["glb"]
            path = REPO / source
            assets.append({
                "id": item["id"],
                "kind": "vegetation",
                "category": item["category"],
                "source": source,
                "source_sha256": sha256_file(path),
                "dimensions_m": item["dimensions_m"],
                "preview": item.get("preview"),
                "collision_class": "none",
                "budget": {"min_instances": 0, "max_instances": 0},
                "metadata": {
                    "origin": "game/resources/environment/assets/manifest.json",
                    "generator": library.get("generator"),
                    "lod": False,
                    "leaf_cards": item.get("leaf_cards"),
                    "triangles": item.get("wood_triangles", 0) + item.get("foliage_triangles", 0),
                },
            })

    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    for key, spec in catalog["objects"].items():
        assets.append({
            "id": spec["asset_id"],
            "kind": "flag" if spec["kind"] == "procedural_flag" else "card",
            "category": spec["category"],
            "source": spec.get("source"),
            "source_sha256": sha256_file(REPO / spec["source"]) if spec.get("source") else None,
            "dimensions_m": {
                "width": float(spec.get("target_width_m", 0.8)),
                "height": float(spec.get("target_height_m", 1.6)),
                "depth": 0.05,
            },
            "preview": None,
            "collision_class": "none",
            "budget": {"min_instances": 0, "max_instances": 0},
            "metadata": {"origin": f"object_catalog marker {key}", "marker_index": key},
        })

    # Track-scoped budgets are carried on catalog zones, not the general registry.
    for zone_name in ("trees", "bushes", "grass"):
        for asset in assets:
            if asset["kind"] == "vegetation" and asset["category"] == zone_name:
                asset["budget"] = budget_for(zone_name, catalog)

    assets.sort(key=lambda item: item["id"])
    registry = {
        "schema_version": SCHEMA_VERSION,
        "profile_version": PROFILE_VERSION,
        "note": "Assets are referenced by semantic id only; generated by build_asset_registry.py.",
        "asset_count": len(assets),
        "assets": assets,
    }

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(registry, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {output} with {len(assets)} assets")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
