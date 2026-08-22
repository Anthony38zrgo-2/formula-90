from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import os
import shutil
import sys
import time
from pathlib import Path

import bpy

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from procedural_assets_blender import _mesh_object
from procedural_materials_blender import texture_material


def args_after_double_dash():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def copy_canonical_asset(source: Path, destination: Path, attempts: int = 8) -> None:
    """Replace a canonical GLB without exposing a partially written destination.

    Windows indexers/importers can briefly hold generated GLBs immediately after export.
    Copy to a sibling first and retry only the atomic replacement on transient OSErrors.
    """
    temporary = destination.with_name(f".{destination.name}.{os.getpid()}.tmp")
    try:
        shutil.copyfile(source, temporary)
        for attempt in range(attempts):
            try:
                os.replace(temporary, destination)
                return
            except OSError:
                if attempt + 1 == attempts:
                    raise
                time.sleep(0.15 * (attempt + 1))
    finally:
        if temporary.exists():
            temporary.unlink()


def card_mesh(name: str, width: float, height: float, planes: int, material, mirror_uv: bool):
    vertices = []
    faces = []
    uv_by_face = []
    half = float(width) * 0.5
    for plane in range(int(planes)):
        angle = math.pi * plane / max(1, int(planes))
        ca, sa = math.cos(angle), math.sin(angle)
        start = len(vertices)
        vertices.extend([
            (-half * ca, -half * sa, 0.0),
            (half * ca, half * sa, 0.0),
            (half * ca, half * sa, float(height)),
            (-half * ca, -half * sa, float(height)),
        ])
        faces.append((start, start + 1, start + 2, start + 3))
        uv_by_face.append([(1, 0), (0, 0), (0, 1), (1, 1)] if mirror_uv else [(0, 0), (1, 0), (1, 1), (0, 1)])
    return _mesh_object(name, vertices, faces, [material], uv_by_face=uv_by_face)


def export_variant(target: Path, variant: dict, texture: Path, mirror_uv: bool) -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    material = texture_material(f"F90_{variant['id']}", texture, roughness=0.98, alpha=True)
    obj = card_mesh(
        variant["id"],
        float(variant["width_m"]),
        float(variant["height_m"]),
        int(variant["planes"]),
        material,
        mirror_uv,
    )
    obj["formula90s_asset_id"] = variant["id"]
    obj["formula90s_category"] = variant["category"]
    obj["formula90s_source_texture"] = str(texture.as_posix())
    obj["formula90s_collision"] = False
    for candidate in bpy.context.selected_objects:
        candidate.select_set(False)
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj

    target.parent.mkdir(parents=True, exist_ok=True)
    temp = target.with_name(f".{target.stem}.new{target.suffix}")
    if temp.exists():
        temp.unlink()
    bpy.ops.export_scene.gltf(
        filepath=str(temp),
        export_format="GLB",
        export_apply=True,
        export_extras=True,
        use_selection=True,
    )
    os.replace(temp, target)


def main() -> None:
    parser = argparse.ArgumentParser(description="Build reusable crossed-card GLBs from prepared vegetation v2 textures.")
    parser.add_argument("--prepared-manifest", required=True)
    parser.add_argument("--repo", required=True)
    ns = parser.parse_args(args_after_double_dash())

    prepared_path = Path(ns.prepared_manifest).resolve()
    repo = Path(ns.repo).resolve()
    prepared = json.loads(prepared_path.read_text(encoding="utf-8"))
    output_dir = prepared_path.parent
    glb_dir = output_dir / "glb"
    rows = []
    assets = []
    for index, variant in enumerate(prepared["variants"]):
        source = prepared["sources"][variant["source"]]
        texture = Path(source["texture"]).resolve()
        target = glb_dir / f"{variant['id']}.glb"
        export_variant(target, variant, texture, mirror_uv=bool(index % 2))
        canonical_dir = repo / "assets-lowpoly-python" / "nature" / variant["category"] / "glb"
        canonical_dir.mkdir(parents=True, exist_ok=True)
        canonical_target = canonical_dir / f"{variant['id']}.glb"
        copy_canonical_asset(target, canonical_target)
        relative = target.relative_to(repo).as_posix()
        width = float(variant["width_m"])
        height = float(variant["height_m"])
        rows.append({
            "file": relative,
            "category": variant["category"],
            "width_x": width,
            "height_y": height,
            "depth_z": width if int(variant["planes"]) > 1 else 0.02,
        })
        assets.append({
            "id": variant["id"],
            "category": variant["category"],
            "source": variant["source"],
            "texture": str(texture),
            "texture_sha256": sha256(texture),
            "glb": relative,
            "glb_sha256": sha256(target),
            "planes": int(variant["planes"]),
            "width_m": width,
            "height_m": height,
            "collision": False,
        })

    csv_path = output_dir / "manifest.csv"
    csv_temp = csv_path.with_suffix(".new.csv")
    with csv_temp.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=("file", "category", "width_x", "height_y", "depth_z"))
        writer.writeheader()
        writer.writerows(rows)
    os.replace(csv_temp, csv_path)
    shutil.copyfile(csv_path, repo / "assets-lowpoly-python" / "nature" / "manifest.csv")

    report_path = output_dir / "asset_manifest.json"
    report_temp = report_path.with_suffix(".new.json")
    report_temp.write_text(json.dumps({
        "schema_version": 1,
        "track_id": prepared["track_id"],
        "prepared_manifest": str(prepared_path),
        "prepared_manifest_sha256": sha256(prepared_path),
        "asset_count": len(assets),
        "assets": assets,
    }, indent=2) + "\n", encoding="utf-8")
    os.replace(report_temp, report_path)
    print(json.dumps({"assets": len(assets), "manifest": str(report_path), "dimensions": str(csv_path)}, indent=2))


if __name__ == "__main__":
    main()
