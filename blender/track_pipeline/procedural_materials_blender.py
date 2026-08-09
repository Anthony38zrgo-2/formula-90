from __future__ import annotations

from pathlib import Path

import bpy


def _image(path: Path):
    key = str(path.resolve())
    existing = bpy.data.images.get(path.name)
    if existing and Path(existing.filepath).resolve() == path.resolve():
        return existing
    return bpy.data.images.load(key, check_existing=True)


def texture_material(name: str, texture_path: Path, roughness: float, metallic: float = 0.0, alpha: bool = False, tint=(1.0, 1.0, 1.0, 1.0)):
    existing = bpy.data.materials.get(name)
    if existing:
        return existing
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    links = mat.node_tree.links
    for node in list(nodes):
        nodes.remove(node)

    out = nodes.new("ShaderNodeOutputMaterial")
    bsdf = nodes.new("ShaderNodeBsdfPrincipled")
    tex = nodes.new("ShaderNodeTexImage")
    tex.image = _image(texture_path)
    tex.interpolation = "Closest"
    bsdf.inputs["Base Color"].default_value = tint
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metallic
    links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    links.new(bsdf.outputs["BSDF"], out.inputs["Surface"])

    if alpha:
        links.new(tex.outputs["Alpha"], bsdf.inputs["Alpha"])
        if hasattr(mat, "surface_render_method"):
            mat.surface_render_method = "DITHERED"
        elif hasattr(mat, "blend_method"):
            mat.blend_method = "CLIP"
            mat.alpha_threshold = 0.35
        if hasattr(mat, "use_transparency_overlap"):
            mat.use_transparency_overlap = False

    return mat


def flat_material(name: str, color, roughness: float = 0.8, metallic: float = 0.0):
    existing = bpy.data.materials.get(name)
    if existing:
        return existing
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = (*color, 1.0)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = (*color, 1.0)
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metallic
    return mat


def build_material_library(texture_dir: str | Path) -> dict[str, bpy.types.Material]:
    root = Path(texture_dir)
    return {
        "asphalt": texture_material("F90_Asphalt", root / "asphalt.png", roughness=0.95),
        "ground": texture_material("F90_DryGround", root / "dry_ground.png", roughness=1.0),
        "bark": texture_material("F90_Bark", root / "bark.png", roughness=0.90),
        "foliage_green": flat_material("F90_FoliageGreen", (0.24, 0.42, 0.19), roughness=0.94),
        "foliage_dry": flat_material("F90_FoliageDry", (0.40, 0.43, 0.20), roughness=0.96),
        "grass_green": texture_material("F90_GrassGreen", root / "grass_green.png", roughness=1.0, alpha=True),
        "grass_dry": texture_material("F90_GrassDry", root / "grass_dry.png", roughness=1.0, alpha=True),
        "bush_green": texture_material("F90_BushGreen", root / "bush_green.png", roughness=1.0, alpha=True),
        "bush_dry": texture_material("F90_BushDry", root / "bush_dry.png", roughness=1.0, alpha=True),
        "building_low": texture_material("F90_BuildingLow", root / "building_low.png", roughness=0.92),
        "building_warehouse": texture_material("F90_BuildingWarehouse", root / "building_warehouse.png", roughness=0.92),
        "guardrail": texture_material("F90_Guardrail", root / "guardrail.png", roughness=0.48, metallic=0.72),
        "start_finish": texture_material("F90_StartFinish", root / "start_finish.png", roughness=0.72),
        "curb_red": flat_material("F90_CurbRed", (0.78, 0.055, 0.035), roughness=0.78),
        "curb_white": flat_material("F90_CurbWhite", (0.92, 0.92, 0.88), roughness=0.82),
        "edge_line": flat_material("F90_EdgeLine", (0.94, 0.94, 0.91), roughness=0.86),
        "trunk": flat_material("F90_Trunk", (0.28, 0.18, 0.10), roughness=0.92),
    }
