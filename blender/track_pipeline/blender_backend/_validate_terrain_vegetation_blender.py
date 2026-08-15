"""Blender headless smoke validation for terrain and vegetation builders.

Runs inside ``blender --background --factory-startup --python`` and verifies
that the terrain heightfield mesh and the vegetation cards are created from the
explicit BuildIR data.

Exit code: 0 = PASS, 1 = FAIL.
"""

from __future__ import annotations

import sys
from pathlib import Path

_PIPELINE = Path(__file__).resolve().parents[1]
if str(_PIPELINE) not in sys.path:
    sys.path.insert(0, str(_PIPELINE))

from blender_backend.build_ir_loader import TerrainCell, VegetationInstance  # noqa: E402
from blender_backend.terrain_builder import build_terrain_in_blender, terrain_grid  # noqa: E402
from blender_backend.vegetation_builder import build_vegetation_in_blender  # noqa: E402


def main() -> int:
    import bpy

    cells = [
        TerrainCell(x=0.0, z=0.0, height=0.0),
        TerrainCell(x=6.0, z=0.0, height=1.0),
        TerrainCell(x=0.0, z=6.0, height=2.0),
        TerrainCell(x=6.0, z=6.0, height=3.0),
    ]
    vertices, faces = terrain_grid(cells)
    ok_terrain_geometry = len(vertices) == 4 and len(faces) == 1
    terrain = build_terrain_in_blender(cells, None)
    ok_terrain_mesh = len(terrain.data.vertices) == 4 and len(terrain.data.polygons) == 1

    vegetation_instances = [
        VegetationInstance(
            instance_id="trees_000_0",
            asset_id="tree_v2_01",
            position=(12.0, 12.0),
            yaw_rad=0.5,
            scale=1.0,
        )
    ]
    vegetation = build_vegetation_in_blender(vegetation_instances[0], None)
    ok_vegetation = vegetation.type == "MESH" and len(vegetation.data.vertices) == 4

    print(f"terrain_geometry: {ok_terrain_geometry}")
    print(f"terrain_mesh:     {ok_terrain_mesh}")
    print(f"vegetation:       {ok_vegetation}")

    if ok_terrain_geometry and ok_terrain_mesh and ok_vegetation:
        print("RESULT: PASS")
        return 0
    print("RESULT: FAIL")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
