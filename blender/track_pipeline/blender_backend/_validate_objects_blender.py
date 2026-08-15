"""Blender headless smoke validation for object instancing and GLB export.

Runs inside ``blender --background --factory-startup --python`` and verifies
that the object builder creates cards at the explicit BuildIR positions and
that the exporter produces a non-empty GLB file.

Exit code: 0 = PASS, 1 = FAIL.
"""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

_PIPELINE = Path(__file__).resolve().parents[1]
if str(_PIPELINE) not in sys.path:
    sys.path.insert(0, str(_PIPELINE))

from blender_backend.build_ir_loader import ExplicitAssetInstance  # noqa: E402
from blender_backend.exporter import export_glb  # noqa: E402
from blender_backend.object_builder import build_object_in_blender, object_card_geometry  # noqa: E402


def main() -> int:
    import bpy

    instances = [
        ExplicitAssetInstance(
            instance_id="tree_a",
            asset_id="tree_v2_01",
            position=(5.0, 5.0),
            yaw_rad=0.5,
            scale=1.0,
        ),
        ExplicitAssetInstance(
            instance_id="tree_b",
            asset_id="tree_v2_01",
            position=(20.0, 8.0),
            yaw_rad=-0.25,
            scale=1.2,
        ),
    ]

    objects = [build_object_in_blender(instance, None) for instance in instances]
    ok_count = len([o for o in objects if o.type == "MESH"]) == len(instances)

    # The builder bakes the world position into the mesh vertices, so compare
    # the first vertex against the pure geometry helper.
    expected = object_card_geometry(instances[0])[0][0]
    actual = objects[0].data.vertices[0].co
    ok_positions = (
        abs(actual.x - expected[0]) < 1e-4 and abs(actual.z - expected[2]) < 1e-4
    )

    with tempfile.TemporaryDirectory() as directory:
        glb_path = Path(directory) / "objects.glb"
        export_glb(objects, glb_path)
        ok_export = glb_path.exists() and glb_path.stat().st_size > 0

    print(f"object_count: {ok_count}")
    print(f"positions:    {ok_positions}")
    print(f"glb_export:   {ok_export}")

    if ok_count and ok_positions and ok_export:
        print("RESULT: PASS")
        return 0
    print("RESULT: FAIL")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
