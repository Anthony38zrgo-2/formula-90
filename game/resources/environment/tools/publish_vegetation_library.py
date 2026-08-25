"""Publish the canonical procedural vegetation library into the game runtime.

The canonical v4 library (implementation/manifest.json plus the 60 GLBs under
implementation/trees/ and implementation/bushes/) is the published authority.
The v3 generators are deprecated and no longer regenerate assets, so this tool
copies the canonical GLBs into game/resources/environment/assets/, installs the
schema-4 manifest, and rebuilds the two color-library reports, audit previews
and contact sheets from the published GLBs.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import trimesh

from audit_vegetation_3d_library import inspect_glb
from build_tree_color_library import _contact_sheet
from build_vegetation_3d_library import render_audit

VARIANTS = ("", "copper", "red", "golden_beige", "yellow")
COLOR_RECIPES = {
    "trees": "game/resources/environment/recipes/tree_color_library.json",
    "bushes": "game/resources/environment/recipes/bush_color_library.json",
}


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _render_previews(repo: Path, records: list[dict]) -> None:
    for record in records:
        scene = trimesh.load(repo / record["glb"], force="scene", process=False)
        wood, foliage = None, None
        for name, geometry in scene.geometry.items():
            lowered = str(name).lower()
            if "foliage" in lowered:
                foliage = geometry
            elif "wood" in lowered:
                wood = geometry
        if wood is None or foliage is None:
            raise RuntimeError(f"GLB roles missing for {record['id']}: {sorted(scene.geometry)}")
        render_audit(wood, foliage, repo / record["preview"])
        print(f"rendered {record['id']}")


def _color_report(repo: Path, kind: str, category: str, bases: list[dict], recipe_path: Path) -> Path:
    recipe = _load(recipe_path)
    base_recipes = _load(repo / recipe["base_recipe"])
    base_recipes = [item for item in base_recipes["assets"] if item["kind"] == kind]
    records = []
    for variant in recipe["variants"]:
        suffix = str(variant["suffix"])
        for base in bases:
            if base["category"] != category:
                continue
            asset_id = base["id"] if not suffix else f"{base['id']}_{suffix}"
            glb = repo / base["glb"].replace(f"{base['id']}.glb", f"{asset_id}.glb")
            inspect = inspect_glb(glb)
            records.append({
                "id": asset_id,
                "kind": base["kind"],
                "family": base["family"],
                "category": base["category"],
                "glb": glb.relative_to(repo).as_posix(),
                "preview": f"game/resources/environment/generated/library_audit/{asset_id}.png",
                "dimensions_m": inspect["dimensions_m"],
                "cluster_count": base["cluster_count"],
                "leaf_cards": base["leaf_cards"],
                "wood_triangles": inspect["triangles"]["wood"],
                "foliage_triangles": inspect["triangles"]["foliage"],
                "sha256": inspect["sha256"],
                "lod": False,
                "collision": False,
                "foliage_double_sided_material": True,
            })
    review = repo / recipe["review"]
    _contact_sheet(repo, recipe, base_recipes, review)
    report = {
        "schema_version": 1,
        "generator": f"procedural_lanceolate_vegetation_v4_{kind}_color_library",
        "manifest": recipe_path.resolve().relative_to(repo).as_posix(),
        "wood_palette_locked": True,
        "assets": records,
        "review": recipe["review"],
    }
    report_path = repo / recipe["report"]
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report_path


def main() -> int:
    parser = argparse.ArgumentParser(description="Publish the canonical vegetation library into the game runtime.")
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--source", type=Path, default=Path("implementation"))
    args = parser.parse_args()
    repo = args.repo.resolve()
    source = args.source if args.source.is_absolute() else repo / args.source

    manifest_path = source / "manifest.json"
    manifest = _load(manifest_path)
    if manifest.get("schema_version") != 4 or manifest.get("generator") != "procedural_lanceolate_vegetation_v4":
        raise RuntimeError(f"Unsupported canonical manifest: {manifest_path}")

    bases = manifest["assets"]
    for base in bases:
        canonical = source / base["category"] / f"{base['id']}.glb"
        if _sha256(canonical) != base["sha256"]:
            raise RuntimeError(f"Canonical sha mismatch for {base['id']}")

    copied = 0
    for base in bases:
        category = base["category"]
        for suffix in VARIANTS:
            asset_id = base["id"] if not suffix else f"{base['id']}_{suffix}"
            canonical = source / category / f"{asset_id}.glb"
            if not canonical.is_file():
                raise RuntimeError(f"Canonical variant missing: {canonical}")
            target = repo / "game/resources/environment/assets" / category / f"{asset_id}.glb"
            target.write_bytes(canonical.read_bytes())
            if _sha256(target) != _sha256(canonical):
                raise RuntimeError(f"Copy verification failed: {asset_id}")
            copied += 1

    published_manifest = repo / "game/resources/environment/assets/manifest.json"
    published_manifest.write_text(manifest_path.read_text(encoding="utf-8"), encoding="utf-8")

    for kind, category in (("tree", "trees"), ("bush", "bushes")):
        recipe_path = repo / COLOR_RECIPES[category]
        report_path = _color_report(repo, kind, category, bases, recipe_path)
        _render_previews(repo, json.loads(report_path.read_text(encoding="utf-8"))["assets"])
        print(f"[publish] {category} report: {report_path}")

    print(json.dumps({
        "copied_glbs": copied,
        "manifest": str(published_manifest.relative_to(repo)),
        "assets": len(bases),
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())