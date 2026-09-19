"""Lower the F1 2030 rear support so it carries the intermediate plane.

The rear support is compressed vertically about its base until its arch apex
reaches the underside of GEO_CHASSIS_REAR_WING_MID instead of the upper wing
region, then the top contact band is snapped onto that underside with a 0.5 mm
embed. The base attachment to the floor and every other object stay untouched.
"""

import bpy
import sys
from pathlib import Path
from mathutils import Vector
from mathutils.bvhtree import BVHTree


SUPPORT_NAME = "GEO_CHASSIS_REARSUPPORT"
MID_NAME = "GEO_CHASSIS_REAR_WING_MID"
UPPER_NAME = "GEO_CHASSIS_AEROPART2"
REAR_WING_NAME = "GEO_CHASSIS_REAR_WING"
CONTACT_BAND_M = 0.002
CONTACT_EMBED_M = 0.0005
EXPECTED_TOPOLOGY = (1752, 1730)


def arguments():
    values = sys.argv[sys.argv.index("--") + 1 :]
    if len(values) != 2:
        raise SystemExit("usage: blender --background --python SCRIPT -- INPUT.blend OUTPUT.blend")
    return Path(values[0]).resolve(), Path(values[1]).resolve()


def world_points(obj):
    return [obj.matrix_world @ vertex.co for vertex in obj.data.vertices]


def world_bvh(obj):
    vertices = world_points(obj)
    polygons = [tuple(polygon.vertices) for polygon in obj.data.polygons]
    return BVHTree.FromPolygons(vertices, polygons, all_triangles=False)


def main():
    input_path, output_path = arguments()
    bpy.ops.wm.open_mainfile(filepath=str(input_path))
    support = bpy.data.objects.get(SUPPORT_NAME)
    mid = bpy.data.objects.get(MID_NAME)
    if support is None or support.type != "MESH":
        raise RuntimeError(f"required rear support missing: {SUPPORT_NAME}")
    if mid is None or mid.type != "MESH":
        raise RuntimeError(f"required intermediate plane missing: {MID_NAME}")
    if (len(support.data.vertices), len(support.data.polygons)) != EXPECTED_TOPOLOGY:
        raise RuntimeError(
            "rear support topology changed from the expected base: "
            + str((len(support.data.vertices), len(support.data.polygons)))
        )

    protected = {
        name: tuple(round(value, 9) for row in obj.matrix_world for value in row)
        for name, obj in bpy.data.objects.items()
        if name != SUPPORT_NAME
    }

    points = world_points(support)
    z_min = min(point.z for point in points)
    z_max = max(point.z for point in points)
    inverse = support.matrix_world.inverted()

    mid_tree = world_bvh(mid)
    apex_vertex = max(support.data.vertices, key=lambda vertex: (support.matrix_world @ vertex.co).z)
    apex_world = support.matrix_world @ apex_vertex.co
    hit, _normal, _index, _distance = mid_tree.ray_cast(Vector((apex_world.x, apex_world.y, -0.25)), Vector((0.0, 0.0, 1.0)), 1.0)
    if hit is None:
        raise RuntimeError("intermediate plane underside not found above the support apex")
    target_z = hit.z
    scale = (target_z - z_min) / (z_max - z_min)
    if not 0.0 < scale < 1.0:
        raise RuntimeError(f"invalid support compression scale: {scale}")

    band = []
    snapped = []
    for vertex, point in zip(support.data.vertices, points):
        compressed_z = z_min + (point.z - z_min) * scale
        moved = Vector((point.x, point.y, compressed_z))
        if point.z >= z_max - CONTACT_BAND_M:
            upward = mid_tree.ray_cast(Vector((point.x, point.y, compressed_z - 0.05)), Vector((0.0, 0.0, 1.0)), 0.5)
            contact_z = upward[0].z if upward[0] is not None else target_z
            moved.z = contact_z + CONTACT_EMBED_M
            band.append((vertex.index, contact_z))
            snapped.append(moved.z)
        vertex.co = inverse @ moved
    support.data.update()

    if not band:
        raise RuntimeError("support contact band was empty")
    if max(snapped) - min(snapped) > 0.02:
        raise RuntimeError("support contact band is not conforming to the intermediate plane")

    for name, matrix_values in protected.items():
        current = tuple(round(value, 9) for row in bpy.data.objects[name].matrix_world for value in row)
        if current != matrix_values:
            raise RuntimeError(f"protected object transform changed: {name}")

    new_points = world_points(support)
    new_z_max = max(point.z for point in new_points)
    support["f1_2030_rear_support_role"] = "carries_intermediate_plane"
    support["f1_2030_rear_support_compression"] = scale
    support["f1_2030_rear_support_contact_embed_m"] = CONTACT_EMBED_M
    support["f1_2030_rear_support_band_vertices"] = len(band)
    support["f1_2030_rear_support_previous_z_max_m"] = z_max

    report = {
        "compression_scale": scale,
        "z_min_m": z_min,
        "z_max_before_m": z_max,
        "z_max_after_m": new_z_max,
        "apex_before_m": list(apex_world),
        "target_underside_z_m": target_z,
        "band_vertices": len(band),
        "band_contact_z_range_m": [min(contact for _i, contact in band), max(contact for _i, contact in band)],
        "band_embed_range_m": [min(snapped) - min(contact for _i, contact in band), max(snapped) - max(contact for _i, contact in band)],
    }

    other = bpy.data.objects.get(REAR_WING_NAME)
    if other is not None:
        tree = world_bvh(support)
        report["overlap_with_rear_wing"] = len(tree.overlap(world_bvh(other)))
    upper = bpy.data.objects.get(UPPER_NAME)
    if upper is not None:
        tree = world_bvh(support)
        upper_points = world_points(upper)
        report["clearance_to_upper_plane_m"] = min(point.z for point in upper_points) - new_z_max

    output_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(output_path), check_existing=False)
    print("F90_REAR_SUPPORT=" + str(report))


if __name__ == "__main__":
    main()