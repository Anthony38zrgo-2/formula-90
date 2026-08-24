"""Create a temporary Blend fixture; executed only inside Blender tests."""

from pathlib import Path
import sys

import bpy


def main():
    argv = sys.argv
    if "--" not in argv:
        raise SystemExit("missing output path")
    output = Path(argv[argv.index("--") + 1]).resolve()
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.mesh.primitive_cube_add(size=2.0, location=(0.0, 0.5, 0.0))
    obj = bpy.context.object
    obj.name = "GEO_TEST_CHASSIS"
    obj["semantic_role"] = "chassis"
    modifier = obj.modifiers.new(name="MirrorSafety", type="MIRROR")
    modifier.use_axis[0] = True
    material = bpy.data.materials.new(name="MAT_TEST_PAINT")
    material.use_nodes = True
    obj.data.materials.append(material)
    if not obj.data.uv_layers:
        obj.data.uv_layers.new(name="UVMap")
    bpy.context.scene["fixture_id"] = "vehicle-studio-blend-scan"
    bpy.ops.wm.save_as_mainfile(filepath=str(output), check_existing=False)


if __name__ == "__main__":
    main()

