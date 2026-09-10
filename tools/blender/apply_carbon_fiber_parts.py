"""Apply one seamless mirrored 2x2 twill carbon material to named F1 parts."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path

import bpy


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "game/assets/models/vehicles/f1-2026-2008/source/f1_2026_2008_source.blend"
TEXTURE = SOURCE.parent / "textures/carbon_fiber_seamless/carbon_fiber_2x2_twill_basecolor.png"
REPORT_DIR = ROOT / "reports/2026-09-09-carbon-fiber-parts"
REFERENCE_RENDER_SCRIPT = ROOT / "tools/blender/apply_reference_match_livery.py"

TARGETS = (
    "ActiveFront",
    "ActiveRear",
    "Aero2",
    "Floor",
    "FrontWing",
    "Front Suspension",
    "Rear Suspension",
    "RearSupport",
    "AeroPart3",
    "Carbon5",
)


def create_material(image):
    material = bpy.data.materials.get("CarbonFiber_Seamless_2x2_Twill")
    if material is None:
        material = bpy.data.materials.new("CarbonFiber_Seamless_2x2_Twill")
    material.use_nodes = True
    nodes = material.node_tree.nodes
    links = material.node_tree.links
    nodes.clear()

    geometry = nodes.new("ShaderNodeNewGeometry")
    scale = nodes.new("ShaderNodeVectorMath")
    scale.operation = "SCALE"
    scale.inputs[3].default_value = 4.0
    links.new(geometry.outputs["Position"], scale.inputs[0])

    texture = nodes.new("ShaderNodeTexImage")
    texture.name = "CarbonFiberSeamlessBaseColor"
    texture.label = "2x2 twill - mirrored seamless repeat"
    texture.image = image
    texture.projection = "BOX"
    texture.projection_blend = 0.12
    texture.extension = "REPEAT"
    texture.interpolation = "Linear"
    links.new(scale.outputs["Vector"], texture.inputs["Vector"])

    grayscale = nodes.new("ShaderNodeRGBToBW")
    links.new(texture.outputs["Color"], grayscale.inputs["Color"])

    color_ramp = nodes.new("ShaderNodeValToRGB")
    color_ramp.name = "CarbonGraphiteColorRange"
    color_ramp.color_ramp.elements[0].position = 0.08
    color_ramp.color_ramp.elements[0].color = (0.002, 0.003, 0.004, 1.0)
    color_ramp.color_ramp.elements[1].position = 0.72
    color_ramp.color_ramp.elements[1].color = (0.070, 0.078, 0.086, 1.0)
    links.new(grayscale.outputs["Val"], color_ramp.inputs["Fac"])

    roughness_ramp = nodes.new("ShaderNodeValToRGB")
    roughness_ramp.name = "CarbonRoughnessRange"
    roughness_ramp.color_ramp.elements[0].color = (0.24, 0.24, 0.24, 1.0)
    roughness_ramp.color_ramp.elements[1].color = (0.37, 0.37, 0.37, 1.0)
    links.new(grayscale.outputs["Val"], roughness_ramp.inputs["Fac"])

    bump = nodes.new("ShaderNodeBump")
    bump.name = "CarbonMicroWeaveBump"
    bump.inputs["Strength"].default_value = 0.16
    bump.inputs["Distance"].default_value = 0.006
    links.new(grayscale.outputs["Val"], bump.inputs["Height"])

    principled = nodes.new("ShaderNodeBsdfPrincipled")
    principled.name = "CarbonFiberPrincipled"
    links.new(color_ramp.outputs["Color"], principled.inputs["Base Color"])
    links.new(roughness_ramp.outputs["Color"], principled.inputs["Roughness"])
    links.new(bump.outputs["Normal"], principled.inputs["Normal"])
    principled.inputs["Metallic"].default_value = 0.0
    principled.inputs["IOR"].default_value = 1.52
    if principled.inputs.get("Coat Weight"):
        principled.inputs["Coat Weight"].default_value = 0.48
    if principled.inputs.get("Coat Roughness"):
        principled.inputs["Coat Roughness"].default_value = 0.13
    if principled.inputs.get("Anisotropic IOR Level"):
        principled.inputs["Anisotropic IOR Level"].default_value = 0.28

    output = nodes.new("ShaderNodeOutputMaterial")
    links.new(principled.outputs["BSDF"], output.inputs["Surface"])
    material.diffuse_color = (0.012, 0.014, 0.017, 1.0)
    return material


def assign_material(obj, material):
    obj.data.materials.clear()
    obj.data.materials.append(material)
    for polygon in obj.data.polygons:
        polygon.material_index = 0


def render_reviews():
    spec = importlib.util.spec_from_file_location("reference_render", REFERENCE_RENDER_SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.REPORT_DIR = REPORT_DIR
    module.render_reviews()


def main():
    if Path(bpy.data.filepath).resolve() != SOURCE.resolve():
        raise RuntimeError(f"Expected {SOURCE}, opened {bpy.data.filepath}")
    missing = [name for name in TARGETS if bpy.data.objects.get(name) is None]
    if missing:
        raise RuntimeError(f"Missing target objects: {missing}")
    if not TEXTURE.exists():
        raise FileNotFoundError(TEXTURE)

    before = {
        name: (
            len(bpy.data.objects[name].data.vertices),
            len(bpy.data.objects[name].data.polygons),
        )
        for name in TARGETS
    }
    image = bpy.data.images.load(str(TEXTURE), check_existing=True)
    image.name = "CarbonFiber_2x2_Twill_Seamless"
    image.colorspace_settings.name = "sRGB"
    if image.packed_file:
        image.unpack(method="USE_ORIGINAL")
    image.reload()
    if tuple(image.size) != (2048, 2048):
        image.scale(2048, 2048)
        image.filepath_raw = str(TEXTURE)
        image.file_format = "PNG"
        image.save()
    image.pack()
    image.use_fake_user = True

    material = create_material(image)
    for name in TARGETS:
        assign_material(bpy.data.objects[name], material)

    after = {
        name: (
            len(bpy.data.objects[name].data.vertices),
            len(bpy.data.objects[name].data.polygons),
        )
        for name in TARGETS
    }
    if before != after:
        raise RuntimeError("Target topology changed while assigning carbon material")

    scene = bpy.context.scene
    scene["carbon_fiber_texture"] = str(TEXTURE)
    scene["carbon_fiber_targets"] = json.dumps(TARGETS)
    scene["carbon_fiber_repeat"] = "pixel_seamless_repeat_box_projection"
    scene["carbon_fiber_weave"] = "2x2_twill"
    bpy.ops.wm.save_as_mainfile(filepath=str(SOURCE), compress=True)

    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    report = {
        "source": str(SOURCE),
        "texture": str(TEXTURE),
        "texture_size": list(image.size),
        "texture_packed": bool(image.packed_file),
        "material": material.name,
        "projection": "BOX",
        "repeat": "REPEAT",
        "weave": "2x2 twill",
        "targets": list(TARGETS),
        "all_targets_fully_assigned": all(
            len(bpy.data.objects[name].data.materials) == 1
            and bpy.data.objects[name].data.materials[0] == material
            for name in TARGETS
        ),
        "topology_unchanged": before == after,
    }
    (REPORT_DIR / "validation.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print("VALIDATION|" + json.dumps(report, sort_keys=True))
    render_reviews()


if __name__ == "__main__":
    main()
