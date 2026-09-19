"""Make the F1 2030 nose slender and upward-pitched, stretching the supports.

The nose region of GEO_CHASSIS_BODY (x < -1.75) is transformed with a single
analytic map driven by a smooth longitudinal weight w (0 at the blend start,
1 at the nose tip): lateral slimming toward y=0, vertical thinning toward the
local section midline c(x), and a uniform tip rise. The front wing plane is
left untouched, so the nose-to-wing clearance grows.

GEO_CHASSIS_FRONTSUPPORT receives the exact same map (a function of the
original world position), which keeps its top flange flush with the new nose
underside, and is then re-parameterized between the deformed nose underside
and the fixed wing surface (1 mm overlap), stretching both blades uniformly.
"""

import bpy
import bisect
import math
import sys
from pathlib import Path
from mathutils import Vector
from mathutils.bvhtree import BVHTree


NOSE_BLEND_START_X = -1.75
NOSE_BLEND_LENGTH_M = 1.0
NOSE_TIP_RISE_M = 0.055
NOSE_LATERAL_SLIM = 0.25
NOSE_VERTICAL_THIN = 0.15
SUPPORT_WING_OVERLAP_M = 0.001
BODY_NAME = "GEO_CHASSIS_BODY"
SUPPORT_NAME = "GEO_CHASSIS_FRONTSUPPORT"
WING_NAME = "GEO_CHASSIS_FRONTWING"
MODIFIED_OBJECTS = {BODY_NAME, SUPPORT_NAME}


def arguments():
    values = sys.argv[sys.argv.index("--") + 1 :]
    if len(values) != 2:
        raise SystemExit("usage: blender --background --python SCRIPT -- INPUT.blend OUTPUT.blend")
    return Path(values[0]).resolve(), Path(values[1]).resolve()


def smoothstep(value):
    t = max(0.0, min(1.0, value))
    return t * t * (3.0 - 2.0 * t)


def world_points(obj):
    return [obj.matrix_world @ vertex.co for vertex in obj.data.vertices]


def world_bvh(obj, points=None):
    vertices = points if points is not None else world_points(obj)
    polygons = [tuple(polygon.vertices) for polygon in obj.data.polygons]
    return BVHTree.FromPolygons(vertices, polygons, all_triangles=False)


class NoseTransform:
    """Pure function of the original world position; identical for BODY and FRONTSUPPORT."""

    def __init__(self, body_points):
        bins = {}
        for point in body_points:
            if point.x < NOSE_BLEND_START_X and abs(point.y) < 0.30:
                key = round(point.x / 0.05) * 0.05
                low, high = bins.get(key, (1.0e9, -1.0e9))
                bins[key] = (min(low, point.z), max(high, point.z))
        self.keys = sorted(bins)
        self.midpoints = [(key, (bins[key][0] + bins[key][1]) * 0.5) for key in self.keys]

    def centerline(self, x):
        keys = self.keys
        if x <= keys[0]:
            return self.midpoints[0][1]
        if x >= keys[-1]:
            return self.midpoints[-1][1]
        index = bisect.bisect_left(keys, x)
        x0, midline0 = self.midpoints[index - 1]
        x1, midline1 = self.midpoints[index]
        t = (x - x0) / (x1 - x0)
        return midline0 * (1.0 - t) + midline1 * t

    def __call__(self, point):
        if point.x >= NOSE_BLEND_START_X:
            return point.copy()
        weight = smoothstep((NOSE_BLEND_START_X - point.x) / NOSE_BLEND_LENGTH_M)
        if weight <= 0.0:
            return point.copy()
        midline = self.centerline(point.x)
        return Vector((
            point.x,
            point.y * (1.0 - NOSE_LATERAL_SLIM * weight),
            midline + (point.z - midline) * (1.0 - NOSE_VERTICAL_THIN * weight) + NOSE_TIP_RISE_M * weight,
        ))


def nose_metrics(points, anchors):
    metrics = {}
    for anchor in anchors:
        selected = [point for point in points if abs(point.x - anchor) < 0.012 and abs(point.y) < 0.30]
        if not selected:
            continue
        metrics[anchor] = {
            "half_width": max(abs(point.y) for point in selected),
            "z_min": min(point.z for point in selected),
            "z_max": max(point.z for point in selected),
        }
    return metrics


def remodel_body_nose(body, transform):
    original = world_points(body)
    transformed = [transform(point) for point in original]
    changed = sum(1 for a, b in zip(original, transformed) if (a - b).length > 1.0e-9)
    if changed == 0:
        raise RuntimeError("nose selection was empty")
    moved_behind_start = max(
        (b.x for a, b in zip(original, transformed) if a.x >= NOSE_BLEND_START_X and (a - b).length > 1.0e-9),
        default=0.0,
    )
    if moved_behind_start != 0.0:
        raise RuntimeError(f"vertices behind the blend start moved: x={moved_behind_start:.6f}")
    inverse = body.matrix_world.inverted()
    for vertex, point in zip(body.data.vertices, transformed):
        vertex.co = inverse @ point
    body.data.update()
    body["f1_2030_nose_slender_reference"] = "slender_up_pitch"
    body["f1_2030_nose_tip_rise_m"] = NOSE_TIP_RISE_M
    body["f1_2030_nose_lateral_slim"] = NOSE_LATERAL_SLIM
    body["f1_2030_nose_vertical_thin"] = NOSE_VERTICAL_THIN
    body["f1_2030_nose_blend_start_x"] = NOSE_BLEND_START_X
    maximum_shift = max((b - a).length for a, b in zip(original, transformed))
    return {
        "changed_vertices": changed,
        "maximum_shift_m": maximum_shift,
        "bounds_before": nose_metrics(original, (-2.7, -2.6, -2.5)),
        "bounds_after": nose_metrics(transformed, (-2.7, -2.6, -2.5)),
    }


def stretch_support(support, transform, body_bvh, wing_bvh, deformed_body_bvh):
    original = world_points(support)
    inverse = support.matrix_world.inverted()
    worst_coincidence = 0.0
    worst_overlap = 0.0
    low_t = 1.0
    high_t = 0.0
    stretched = []
    for vertex, point in zip(support.data.vertices, original):
        # The support flange sits on the lowest body surface (the nose
        # underside), so the upward anchor ray must start below the body. The
        # z-map of the nose transform is x-only, so the anchor column uses the
        # original (x, y); the wing anchor uses the final slimmed (x, y).
        upward = body_bvh.ray_cast(Vector((point.x, point.y, point.z - 0.06)), Vector((0.0, 0.0, 1.0)), 2.0)
        if upward[0] is None:
            raise RuntimeError(f"support vertex without body surface above: {point}")
        top_z = upward[0].z
        moved = transform(point)
        downward = wing_bvh.ray_cast(
            Vector((moved.x, moved.y, point.z + 0.003)), Vector((0.0, 0.0, -1.0)), 2.0
        )
        if downward[0] is None:
            raise RuntimeError(f"support vertex without wing surface below: {point}")
        bottom_z = downward[0].z
        span = bottom_z - top_z
        t = 0.0 if abs(span) <= 1.0e-9 else max(0.0, min(1.0, (point.z - top_z) / span))
        low_t = min(low_t, t)
        high_t = max(high_t, t)
        moved_top = transform(Vector((point.x, point.y, top_z)))
        moved.z = moved_top.z * (1.0 - t) + (bottom_z - SUPPORT_WING_OVERLAP_M) * t
        if t > 0.98:
            moved.z = bottom_z - SUPPORT_WING_OVERLAP_M
        if t < 0.02:
            nearest = deformed_body_bvh.find_nearest(moved, 0.2)
            if nearest[0] is not None:
                worst_coincidence = max(worst_coincidence, (nearest[0] - moved).length)
        if t > 0.98:
            worst_overlap = max(worst_overlap, abs(moved.z - (bottom_z - SUPPORT_WING_OVERLAP_M)))
        vertex.co = inverse @ moved
        stretched.append(moved)
    support.data.update()
    support["f1_2030_front_support_stretch"] = "uniform_top_to_wing"
    support["f1_2030_front_support_overlap_m"] = SUPPORT_WING_OVERLAP_M
    return {
        "parameter_range": [low_t, high_t],
        "worst_top_coincidence_m": worst_coincidence,
        "worst_bottom_overlap_error_m": worst_overlap,
        "z_span_before": [min(point.z for point in original), max(point.z for point in original)],
        "z_span_after": [min(point.z for point in stretched), max(point.z for point in stretched)],
    }


def wing_top_z(wing_bvh, x, y):
    hit = wing_bvh.ray_cast(Vector((x, y, 0.2)), Vector((0.0, 0.0, -1.0)), 1.0)
    return hit[0].z if hit[0] is not None else None


def main():
    input_path, output_path = arguments()
    bpy.ops.wm.open_mainfile(filepath=str(input_path))
    body = bpy.data.objects.get(BODY_NAME)
    support = bpy.data.objects.get(SUPPORT_NAME)
    wing = bpy.data.objects.get(WING_NAME)
    for name, expected in ((BODY_NAME, (8177, 14342)), (SUPPORT_NAME, (224, 384))):
        obj = bpy.data.objects.get(name)
        if obj is None or obj.type != "MESH":
            raise RuntimeError(f"required mesh missing: {name}")
        if (len(obj.data.vertices), len(obj.data.polygons)) != expected:
            raise RuntimeError(
                f"{name} topology changed from the expected base: "
                + str((len(obj.data.vertices), len(obj.data.polygons)))
            )
    if wing is None or wing.type != "MESH":
        raise RuntimeError("required front-wing mesh missing")

    protected = {
        name: tuple(round(value, 9) for row in obj.matrix_world for value in row)
        for name, obj in bpy.data.objects.items()
        if name not in MODIFIED_OBJECTS
    }

    body_points = world_points(body)
    transform = NoseTransform(body_points)
    body_bvh = world_bvh(body, body_points)
    wing_bvh = world_bvh(wing)
    body_report = remodel_body_nose(body, transform)
    deformed_body_points = world_points(body)
    deformed_body_bvh = world_bvh(body, deformed_body_points)
    support_report = stretch_support(support, transform, body_bvh, wing_bvh, deformed_body_bvh)

    for name, matrix_values in protected.items():
        current = tuple(round(value, 9) for row in bpy.data.objects[name].matrix_world for value in row)
        if current != matrix_values:
            raise RuntimeError(f"protected object transform changed: {name}")

    gap_before = {}
    gap_after = {}
    for anchor in (-2.7, -2.6, -2.55):
        wing_z = wing_top_z(wing_bvh, anchor, 0.0)
        if wing_z is None:
            continue
        before = body_report["bounds_before"].get(anchor)
        after = body_report["bounds_after"].get(anchor)
        if before is not None and after is not None:
            gap_before[anchor] = before["z_min"] - wing_z
            gap_after[anchor] = after["z_min"] - wing_z

    scene = bpy.context.scene
    scene["f1_2030_nose_slender_scope"] = (
        "slender lateral and vertical nose taper, upward tip pitch, stretched front supports, fixed front wing plane"
    )
    output_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(output_path), check_existing=False)
    print("BODY", body_report)
    print("SUPPORT", support_report)
    print("GAP_BEFORE", gap_before)
    print("GAP_AFTER", gap_after)


if __name__ == "__main__":
    main()