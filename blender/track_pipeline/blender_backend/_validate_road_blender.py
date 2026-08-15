"""Blender headless smoke validation for the BuildIR road materializer.

Runs inside ``blender --background --factory-startup --python`` and verifies
that ``blender_backend.road_builder.build_road_in_blender`` creates the road
mesh from explicit BuildIR samples without reinterpreting semantics.

Exit code: 0 = PASS, 1 = FAIL.
"""

from __future__ import annotations

import sys
from pathlib import Path

_PIPELINE = Path(__file__).resolve().parents[1]
if str(_PIPELINE) not in sys.path:
    sys.path.insert(0, str(_PIPELINE))

from blender_backend.build_ir_loader import RoadSample  # noqa: E402
from blender_backend.road_builder import build_road_in_blender  # noqa: E402


def _sample(station: float) -> RoadSample:
    return RoadSample(
        station=station,
        position=(station, 0.0),
        tangent=(1.0, 0.0),
        normal=(0.0, 1.0),
        width_left=6.0,
        width_right=6.0,
        elevation=0.0,
        bank=0.0,
    )


def main() -> int:
    import bpy

    samples = [_sample(float(index) * 10.0) for index in range(4)]
    obj = build_road_in_blender(samples, None)

    mesh = obj.data
    ok_vertices = len(mesh.vertices) == len(samples) * 2
    ok_faces = len(mesh.polygons) == len(samples)
    scene = bpy.context.scene or bpy.data.scenes[0]
    in_scene = obj.name in [o.name for o in scene.collection.objects]

    print(f"vertices: {len(mesh.vertices)} (expected {len(samples) * 2})")
    print(f"faces:    {len(mesh.polygons)} (expected {len(samples)})")
    print(f"in_scene: {in_scene}")

    if ok_vertices and ok_faces and in_scene:
        print("RESULT: PASS")
        return 0
    print("RESULT: FAIL")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
