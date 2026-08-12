"""Export the canonical Jordan K3 GLBs as auxiliary OBJ/MTL copies."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import bpy


def args_after_double_dash() -> list[str]:
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True)
    parser.add_argument("--output", required=True)
    ns = parser.parse_args(args_after_double_dash())

    source = Path(ns.source).resolve()
    output = Path(ns.output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(source))

    meshes = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    if not meshes:
        raise RuntimeError(f"No mesh objects imported from {source}")
    bpy.ops.object.select_all(action="DESELECT")
    for obj in meshes:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = meshes[0]

    # Blender 5.2 exposes the OBJ exporter as wm.obj_export. Keep the export
    # selected so imported helper empties/material roots never leak into OBJ.
    bpy.ops.wm.obj_export(
        filepath=str(output),
        export_selected_objects=True,
        export_materials=True,
        export_uv=True,
        export_normals=True,
        export_smooth_groups=True,
        export_object_groups=True,
        export_material_groups=True,
    )
    print(f"OBJ_EXPORT source={source} meshes={len(meshes)} output={output}")


if __name__ == "__main__":
    main()
