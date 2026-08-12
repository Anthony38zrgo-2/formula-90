"""Build the Blockbench-friendly low-poly Jordan 191 runtime assets.

Run with Blender 5.2 or newer:
    blender --background --factory-startup --python build_jordan_191_lowpoly.py

The source OBJ files are intentionally kept separate from the canonical GLB
assets.  Before replacement, the script copies every canonical GLB and Godot
import sidecar to a timestamped backup directory.
"""

from __future__ import annotations

import json
import os
import shutil
from datetime import datetime

import bpy


HERE = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
ASSET_DIR = os.path.join(
    REPO_ROOT,
    "game",
    "assets",
    "models",
    "vehicles",
    "f1_90s_canonical_1997",
    "candidate_k3_historical",
)
OBJ_DIR = os.path.join(ASSET_DIR, "obj")
TEXTURE_DIR = os.path.join(ASSET_DIR, "lowpoly_textures")
BACKUP_ROOT = os.path.join(
    REPO_ROOT,
    "blender",
    "backup",
    "vehicles",
    "f1_90s_canonical_1997",
    "candidate_k3_historical",
)

# Target triangles keep the full car below ~35k triangles (four wheels), a
# scale which remains comfortable for Blockbench mesh editing.
ASSETS = (
    {
        "source": "jordan_191_candidate_chassis_copy.obj",
        "output": "jordan_191_candidate_chassis.glb",
        "texture": "jordan_191_candidate_chassis_lowpoly_palette.png",
        "ratio": 0.025,
        "roughness": 0.86,
        "metallic": 0.0,
    },
    {
        "source": "jordan_191_candidate_wheel_front_copy.obj",
        "output": "jordan_191_candidate_wheel_front.glb",
        "texture": "jordan_191_candidate_wheel_front_lowpoly_palette.png",
        "ratio": 0.12,
        "roughness": 0.52,
        "metallic": 0.0,
    },
    {
        "source": "jordan_191_candidate_wheel_rear_copy.obj",
        "output": "jordan_191_candidate_wheel_rear.glb",
        "texture": "jordan_191_candidate_wheel_rear_lowpoly_palette.png",
        "ratio": 0.12,
        "roughness": 0.52,
        "metallic": 0.0,
    },
)


def reset_scene() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)


def import_obj(path: str) -> bpy.types.Object:
    bpy.ops.wm.obj_import(filepath=path, forward_axis="NEGATIVE_Z", up_axis="Y")
    meshes = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    if len(meshes) != 1:
        raise RuntimeError(f"Expected one mesh in {path}, got {len(meshes)}")
    obj = meshes[0]
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    return obj


def material_color(material: bpy.types.Material | None) -> tuple[float, float, float, float]:
    if material is None:
        return (0.18, 0.18, 0.18, 1.0)
    return tuple(material.diffuse_color)


def decimate(obj: bpy.types.Object, ratio: float) -> None:
    modifier = obj.modifiers.new("Formula90s_Blockbench_LowPoly", "DECIMATE")
    modifier.decimate_type = "COLLAPSE"
    modifier.ratio = ratio
    modifier.use_symmetry = False
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.modifier_apply(modifier=modifier.name)
    # Decimate can leave a handful of degenerate faces in very dense imported
    # OBJ data. Clean them now so the exported GLB is structurally valid.
    if obj.data.validate(verbose=False, clean_customdata=True):
        obj.data.update()


def build_palette_material(
    obj: bpy.types.Object,
    texture_path: str,
    roughness: float,
    metallic: float,
) -> None:
    """Encode source material colours into a tiny nearest-filtered palette atlas."""
    source_colours = [material_color(slot) for slot in obj.data.materials]
    if not source_colours:
        source_colours = [(0.18, 0.18, 0.18, 1.0)]

    # 8 x 8 gives each original material a distinct, safely filterable cell.
    cells = 8
    size = 64
    image = bpy.data.images.new("Formula90s_LowPoly_Palette", width=size, height=size, alpha=True)
    pixels = [0.0] * (size * size * 4)
    cell_size = size // cells
    for index, colour in enumerate(source_colours):
        cell_x = index % cells
        cell_y = index // cells
        for y in range(cell_y * cell_size, (cell_y + 1) * cell_size):
            for x in range(cell_x * cell_size, (cell_x + 1) * cell_size):
                offset = (y * size + x) * 4
                pixels[offset : offset + 4] = colour
    image.pixels = pixels
    image.filepath_raw = texture_path
    image.file_format = "PNG"
    image.save()
    image.colorspace_settings.name = "sRGB"

    uv = obj.data.uv_layers.get("UVMap") or obj.data.uv_layers.new(name="UVMap")
    for polygon in obj.data.polygons:
        material_index = min(polygon.material_index, len(source_colours) - 1)
        cell_x = material_index % cells
        cell_y = material_index // cells
        u = (cell_x + 0.5) / cells
        v = (cell_y + 0.5) / cells
        for loop_index in polygon.loop_indices:
            uv.data[loop_index].uv = (u, v)

    obj.data.materials.clear()
    material = bpy.data.materials.new("Formula90s_LowPoly")
    material.use_nodes = True
    nodes = material.node_tree.nodes
    links = material.node_tree.links
    principled = nodes.get("Principled BSDF")
    principled.inputs["Roughness"].default_value = roughness
    principled.inputs["Metallic"].default_value = metallic
    texture_node = nodes.new("ShaderNodeTexImage")
    texture_node.image = image
    texture_node.interpolation = "Closest"
    # Refer to the asset-side texture path. This prevents the GLB exporter from
    # emitting a duplicate PNG next to each GLB on Godot re-import.
    image.filepath = "//lowpoly_textures/" + os.path.basename(texture_path)
    links.new(texture_node.outputs["Color"], principled.inputs["Base Color"])
    obj.data.materials.append(material)


def backup_canonical_files() -> str:
    stamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    # Keep source backups outside res://. Godot scans everything under game and
    # therefore treats duplicate GLB UIDs there as conflicting runtime assets.
    backup_dir = os.path.join(BACKUP_ROOT, f"pre_lowpoly_{stamp}")
    os.makedirs(backup_dir, exist_ok=False)
    for item in ASSETS:
        output = item["output"]
        for name in (output, f"{output}.import"):
            source = os.path.join(ASSET_DIR, name)
            if os.path.isfile(source):
                shutil.copy2(source, os.path.join(backup_dir, name))
    return backup_dir


def export_glb(obj: bpy.types.Object, output_path: str) -> None:
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.export_scene.gltf(
        filepath=output_path,
        export_format="GLB",
        use_selection=True,
        export_materials="EXPORT",
        export_image_format="AUTO",
        export_normals=True,
        export_tangents=False,
        export_yup=True,
        export_apply=True,
    )


def build_asset(spec: dict[str, object]) -> dict[str, object]:
    reset_scene()
    source_path = os.path.join(OBJ_DIR, str(spec["source"]))
    texture_path = os.path.join(TEXTURE_DIR, str(spec["texture"]))
    output_path = os.path.join(ASSET_DIR, str(spec["output"]))
    obj = import_obj(source_path)
    before = len(obj.data.polygons)
    decimate(obj, float(spec["ratio"]))
    after = len(obj.data.polygons)
    build_palette_material(obj, texture_path, float(spec["roughness"]), float(spec["metallic"]))
    export_glb(obj, output_path)
    return {
        "source": str(spec["source"]),
        "output": str(spec["output"]),
        "faces_before": before,
        "faces_after": after,
        "reduction": round(1.0 - after / before, 4),
        "texture": os.path.basename(texture_path),
        "output_bytes": os.path.getsize(output_path),
    }


def main() -> None:
    for spec in ASSETS:
        if not os.path.isfile(os.path.join(OBJ_DIR, str(spec["source"]))):
            raise FileNotFoundError(spec["source"])
    os.makedirs(TEXTURE_DIR, exist_ok=True)
    skip_backup = os.environ.get("FORMULA90S_SKIP_BACKUP") == "1"
    backup_dir = None if skip_backup else backup_canonical_files()
    report = {
        "backup": os.path.relpath(backup_dir, REPO_ROOT) if backup_dir else None,
        "assets": [],
    }
    for spec in ASSETS:
        report["assets"].append(build_asset(spec))
    report_path = os.path.join(ASSET_DIR, "lowpoly_build_report.json")
    with open(report_path, "w", encoding="utf-8") as report_file:
        json.dump(report, report_file, indent=2)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
