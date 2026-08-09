from __future__ import annotations

import argparse
from pathlib import Path
import json
import sys

import bpy

SUPPORTED = {".blend", ".glb", ".gltf"}


def args_after_double_dash():
    return sys.argv[sys.argv.index("--")+1:] if "--" in sys.argv else []


def category_from_path(path: Path) -> str:
    names = {p.lower() for p in path.parts}
    for category in ("trees", "bushes", "grass", "guardrails"):
        if category in names:
            return category
    return "unknown"


def clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)


def import_asset(path: Path):
    before = set(bpy.data.objects)
    if path.suffix.lower() == ".blend":
        with bpy.data.libraries.load(str(path), link=False) as (src, dst):
            dst.objects = [name for name in src.objects if name]
        for obj in dst.objects:
            if obj is not None and obj.name not in bpy.context.scene.collection.objects:
                bpy.context.scene.collection.objects.link(obj)
    else:
        bpy.ops.import_scene.gltf(filepath=str(path))
    bpy.context.view_layer.update()
    return [o for o in bpy.data.objects if o not in before and o.type == "MESH"]


def bbox_dimensions(objects):
    coords = []
    from mathutils import Vector
    for obj in objects:
        for corner in obj.bound_box:
            world = obj.matrix_world @ Vector(corner)
            coords.append((world.x, world.y, world.z))
    if not coords:
        return (0.0,0.0,0.0)
    xs, ys, zs = zip(*coords)
    return (max(xs)-min(xs), max(ys)-min(ys), max(zs)-min(zs))


def load_sidecar(path: Path) -> dict:
    sidecar = path.with_suffix(path.suffix + ".asset.json")
    if sidecar.exists():
        return json.loads(sidecar.read_text(encoding="utf-8"))
    alt = path.with_suffix(".asset.json")
    if alt.exists():
        return json.loads(alt.read_text(encoding="utf-8"))
    return {}


def plausible(category: str, dims, limits: dict):
    x,y,z = dims
    radius = max(x,y) * 0.5
    if category in {"trees","bushes","grass"}:
        rule = limits.get(category, {})
        h0,h1 = rule.get("height_m",[0,float("inf")])
        r0,r1 = rule.get("radius_m",[0,float("inf")])
        return h0 <= z <= h1 and r0 <= radius <= r1
    if category == "guardrails":
        rule = limits.get("guardrails", {})
        h0,h1 = rule.get("height_m",[0,float("inf")])
        l0,l1 = rule.get("length_m",[0,float("inf")])
        return h0 <= z <= h1 and l0 <= max(x,y) <= l1
    return False


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--config", required=True)
    p.add_argument("--output", required=True)
    ns = p.parse_args(args_after_double_dash())

    repo = Path(ns.config).resolve().parents[3]
    config = json.loads(Path(ns.config).read_text(encoding="utf-8"))
    roots = [repo / config["vegetation"]["asset_root"], repo / config["guardrails"]["asset_root"]]
    limits = config["vegetation"]["plausibility"]
    assets = []

    for asset_root in roots:
        if not asset_root.exists():
            continue
        for path in sorted(p for p in asset_root.rglob("*") if p.suffix.lower() in SUPPORTED):
            clear_scene()
            try:
                meshes = import_asset(path)
                dims = bbox_dimensions(meshes)
                meta = load_sidecar(path)
                category = str(meta.get("category") or category_from_path(path))
                x,y,z = dims
                long_axis = "X" if x >= y else "Y"
                radius = max(x,y)*0.5
                default_scale = config["vegetation"]["default_scale_ranges"].get(category,[1,1])
                record = {
                    "id": meta.get("id") or path.stem,
                    "path": path.relative_to(repo).as_posix(),
                    "category": category,
                    "dimensions_m": [round(x,4),round(y,4),round(z,4)],
                    "radius_m": round(float(meta.get("radius_m", radius)),4),
                    "height_m": round(float(meta.get("height_m", z)),4),
                    "scale_min": float(meta.get("scale_min", default_scale[0])),
                    "scale_max": float(meta.get("scale_max", default_scale[1])),
                    "weight": float(meta.get("weight",1.0)),
                    "long_axis": meta.get("long_axis", long_axis),
                    "rotation_correction_deg": float(meta.get("rotation_correction_deg", 0.0)),
                    "valid": plausible(category,dims,limits) or bool(meta.get("allow_outside_plausibility",False)),
                }
                assets.append(record)
                print(f"[asset] {record['id']}: {category} {record['dimensions_m']} valid={record['valid']}")
            except Exception as exc:
                print(f"[asset] ERROR {path}: {exc}", file=sys.stderr)

    out = repo / ns.output
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps({"assets":assets}, indent=2), encoding="utf-8")
    print(f"[asset] catalog: {out}")


if __name__ == "__main__":
    main()
