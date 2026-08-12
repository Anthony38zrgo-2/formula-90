from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

import bpy
from mathutils import Vector


def args_after_double_dash():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def main() -> None:
    parser = argparse.ArgumentParser(description="Validate the assembled La Chutana raw vegetation Blender file.")
    parser.add_argument("--blend", required=True)
    parser.add_argument("--compiled", required=True)
    ns = parser.parse_args(args_after_double_dash())

    blend = Path(ns.blend).resolve()
    compiled = json.loads(Path(ns.compiled).read_text(encoding="utf-8"))
    bpy.ops.wm.open_mainfile(filepath=str(blend))
    env = bpy.data.collections.get("F90_RAW_ENVIRONMENT")
    if env is None:
        raise RuntimeError("F90_RAW_ENVIRONMENT collection is missing")

    roots = [obj for obj in env.all_objects if obj.get("formula90s_raw_asset")]
    counts = Counter(str(obj.get("formula90s_category")) for obj in roots)
    expected = Counter(item["category"] for item in compiled["vegetation"])
    failures = []
    if counts != expected:
        failures.append(f"root counts mismatch expected={dict(expected)} actual={dict(counts)}")
    legacy = [obj.name for obj in roots if "/assets_v2/glb/" not in str(obj.get("formula90s_raw_asset", "")).replace("\\", "/")]
    if legacy:
        failures.append(f"legacy raw assets active={len(legacy)}")
    collidable = [obj.name for obj in roots if bool(obj.get("formula90s_collision", False))]
    if collidable:
        failures.append(f"visual vegetation roots with collision={len(collidable)}")
    tree_plane_errors = [
        root.name for root in roots
        if root.get("formula90s_category") == "trees"
        # glTF triangulates each authored quad, so two crossed cards arrive as four triangles.
        and sum(len(mesh.data.polygons) for mesh in ([root] if root.type == "MESH" else root.children) if mesh.type == "MESH") != 4
    ]
    if tree_plane_errors:
        failures.append(f"tree roots without exactly two mesh cards={len(tree_plane_errors)}")

    bottom_errors = []
    heights = {"trees": [], "bushes": [], "grass": []}
    for root in roots:
        root_meshes = [root] if root.type == "MESH" else [child for child in root.children if child.type == "MESH"]
        corners = [mesh.matrix_world @ Vector(corner) for mesh in root_meshes for corner in mesh.bound_box]
        if not corners:
            bottom_errors.append(root.name)
            continue
        minimum = min(point.z for point in corners)
        maximum = max(point.z for point in corners)
        if abs(minimum - root.location.z) > 1e-3:
            bottom_errors.append(root.name)
        heights[str(root.get("formula90s_category"))].append(maximum - minimum)
    if bottom_errors:
        failures.append(f"bottom anchor failures={len(bottom_errors)}")

    v2_materials = [mat for mat in bpy.data.materials if "v2" in mat.name.lower()]
    alpha_failures = []
    for material in v2_materials:
        bsdf = material.node_tree.nodes.get("Principled BSDF") if material.use_nodes and material.node_tree else None
        if bsdf is None or not bsdf.inputs["Alpha"].is_linked:
            alpha_failures.append(material.name)
    if alpha_failures:
        failures.append(f"v2 alpha material failures={alpha_failures}")

    building_like = [obj.name for obj in env.all_objects if "fake_building" in obj.name.lower() or "building_" in obj.name.lower()]
    raw_collisions = [obj.name for obj in env.all_objects if obj.get("formula90s_raw_collision")]
    if len(raw_collisions) != 1:
        failures.append(f"expected one simplified raw barrier collision object, got {len(raw_collisions)}")
    barrier_visuals = [obj for obj in env.all_objects if obj.get("formula90s_barrier_geometry") == "continuous_rectangular_ribbon"]
    if len(barrier_visuals) != 1:
        failures.append(f"expected one continuous rectangular barrier visual object, got {len(barrier_visuals)}")
    elif (
        barrier_visuals[0].get("formula90s_collision")
        or int(barrier_visuals[0].get("formula90s_quads_per_segment", 0)) != 4
        or int(barrier_visuals[0].get("formula90s_vertices_per_segment", 0)) != 4
    ):
        failures.append("barrier visual geometry/collision contract failed")

    margins = [float(item["barrier_distance_m"]) - float(item["required_barrier_distance_m"]) for item in compiled["vegetation"] if item["category"] in {"trees", "bushes"}]
    if margins and min(margins) < -1e-4:
        failures.append(f"negative barrier footprint margin={min(margins):.4f}")

    if failures:
        for failure in failures:
            print(f"FAIL {failure}")
        raise SystemExit(2)
    summary = {
        "blend": str(blend),
        "vegetation_roots": len(roots),
        "counts": dict(counts),
        "height_ranges_m": {key: [round(min(values), 4), round(max(values), 4)] for key, values in heights.items() if values},
        "minimum_barrier_footprint_margin_m": round(min(margins), 4) if margins else None,
        "v2_materials": len(v2_materials),
        "simplified_barrier_collision_objects": len(raw_collisions),
        "barrier_card_visual_objects": len(barrier_visuals),
        "buildings": len(building_like),
    }
    print("PASS raw Blender vegetation v2 assembly")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
