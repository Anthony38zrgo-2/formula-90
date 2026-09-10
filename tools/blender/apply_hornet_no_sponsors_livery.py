"""Apply and bake a sponsor-free hornet livery to the canonical F1 source.

The script changes materials and UV layers only. It deliberately does not alter
mesh vertex positions or create tire tread/normal detail.
"""

from __future__ import annotations

import json
import math
import re
from pathlib import Path

import bpy
from mathutils import Vector


ROOT = Path(__file__).resolve().parents[2]
SOURCE = (
    ROOT
    / "game/assets/models/vehicles/f1-2026-2008/source/f1_2026_2008_source.blend"
)
TEXTURE_DIR = SOURCE.parent / "textures/hornet_no_sponsors"
MOTIF_PATH = TEXTURE_DIR / "hornet_graphic_motif.png"
REPORT_DIR = ROOT / "reports/2026-09-09-hornet-no-sponsors"
TEXTURE_SIZE = 2048

LIVERY_OBJECTS = {
    "Body",
    "AeroPart1",
    "ActiveFront",
    "FrontWing",
    "Endplate",
    "ActiveRear",
    "AeroPart2",
    "Rear Wing",
}


def socket(node, name):
    return node.inputs.get(name)


def set_if_present(node, name, value):
    target = socket(node, name)
    if target is not None:
        target.default_value = value


def clean_name(name: str) -> str:
    return re.sub(r"[^A-Za-z0-9_-]+", "_", name).strip("_")


def new_flat_material(name: str, color, metallic=0.0, roughness=0.35):
    material = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    material.use_nodes = True
    nodes = material.node_tree.nodes
    links = material.node_tree.links
    nodes.clear()
    principled = nodes.new("ShaderNodeBsdfPrincipled")
    principled.inputs["Base Color"].default_value = color
    principled.inputs["Metallic"].default_value = metallic
    principled.inputs["Roughness"].default_value = roughness
    set_if_present(principled, "Coat Weight", 0.22)
    set_if_present(principled, "Coat Roughness", 0.18)
    output = nodes.new("ShaderNodeOutputMaterial")
    links.new(principled.outputs["BSDF"], output.inputs["Surface"])
    material.diffuse_color = color
    return material


def assign_only(obj, material):
    obj.data.materials.clear()
    obj.data.materials.append(material)
    for polygon in obj.data.polygons:
        polygon.material_index = 0


def ensure_bake_uv(obj):
    uv_layers = obj.data.uv_layers
    old = uv_layers.get("HornetBakeUV")
    if old is not None:
        uv_layers.remove(old)
    uv = uv_layers.new(name="HornetBakeUV")
    uv_layers.active = uv
    uv.active_render = True

    bpy.ops.object.select_all(action="DESELECT")
    obj.hide_set(False)
    obj.hide_render = False
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(
        angle_limit=math.radians(66.0),
        island_margin=0.014,
        area_weight=0.1,
        correct_aspect=True,
        scale_to_bounds=False,
    )
    bpy.ops.object.mode_set(mode="OBJECT")
    return uv


def pattern_shader(material, motif, tire=False):
    nodes = material.node_tree.nodes
    links = material.node_tree.links
    nodes.clear()

    texcoord = nodes.new("ShaderNodeTexCoord")
    mapping = nodes.new("ShaderNodeMapping")
    links.new(texcoord.outputs["Generated"], mapping.inputs["Vector"])

    if tire:
        noise = nodes.new("ShaderNodeTexNoise")
        noise.inputs["Scale"].default_value = 18.0
        noise.inputs["Detail"].default_value = 3.0
        noise.inputs["Roughness"].default_value = 0.68
        links.new(mapping.outputs["Vector"], noise.inputs["Vector"])
        ramp = nodes.new("ShaderNodeValToRGB")
        ramp.color_ramp.elements[0].position = 0.18
        ramp.color_ramp.elements[0].color = (0.008, 0.009, 0.010, 1.0)
        ramp.color_ramp.elements[1].position = 0.82
        ramp.color_ramp.elements[1].color = (0.042, 0.047, 0.052, 1.0)
        links.new(noise.outputs["Fac"], ramp.inputs["Fac"])
        base_color = ramp.outputs["Color"]
    else:
        mapping.inputs["Scale"].default_value = (0.72, 0.72, 0.72)
        texture = nodes.new("ShaderNodeTexImage")
        texture.image = motif
        texture.extension = "REPEAT"
        texture.projection = "BOX"
        texture.projection_blend = 0.16
        links.new(mapping.outputs["Vector"], texture.inputs["Vector"])
        base_color = texture.outputs["Color"]

    ao = nodes.new("ShaderNodeAmbientOcclusion")
    ao.inputs["Color"].default_value = (1.0, 1.0, 1.0, 1.0)
    ao.inputs["Distance"].default_value = 0.32 if not tire else 0.18
    ao.inside = False
    multiply_ao = nodes.new("ShaderNodeMixRGB")
    multiply_ao.blend_type = "MULTIPLY"
    multiply_ao.inputs[0].default_value = 0.82
    links.new(base_color, multiply_ao.inputs[1])
    links.new(ao.outputs["Color"], multiply_ao.inputs[2])

    micro = nodes.new("ShaderNodeTexNoise")
    micro.inputs["Scale"].default_value = 145.0 if not tire else 46.0
    micro.inputs["Detail"].default_value = 2.0
    micro.inputs["Roughness"].default_value = 0.55
    links.new(mapping.outputs["Vector"], micro.inputs["Vector"])
    micro_ramp = nodes.new("ShaderNodeValToRGB")
    micro_ramp.color_ramp.elements[0].color = (0.91, 0.91, 0.91, 1.0)
    micro_ramp.color_ramp.elements[1].color = (1.0, 1.0, 1.0, 1.0)
    links.new(micro.outputs["Fac"], micro_ramp.inputs["Fac"])
    multiply_micro = nodes.new("ShaderNodeMixRGB")
    multiply_micro.blend_type = "MULTIPLY"
    multiply_micro.inputs[0].default_value = 0.22 if not tire else 0.36
    links.new(multiply_ao.outputs["Color"], multiply_micro.inputs[1])
    links.new(micro_ramp.outputs["Color"], multiply_micro.inputs[2])

    emission = nodes.new("ShaderNodeEmission")
    links.new(multiply_micro.outputs["Color"], emission.inputs["Color"])
    output = nodes.new("ShaderNodeOutputMaterial")
    links.new(emission.outputs["Emission"], output.inputs["Surface"])
    return micro


def finalize_pbr(material, baked_image, tire=False):
    nodes = material.node_tree.nodes
    links = material.node_tree.links
    nodes.clear()
    texcoord = nodes.new("ShaderNodeTexCoord")
    color = nodes.new("ShaderNodeTexImage")
    color.image = baked_image
    color.interpolation = "Linear"
    links.new(texcoord.outputs["UV"], color.inputs["Vector"])

    principled = nodes.new("ShaderNodeBsdfPrincipled")
    links.new(color.outputs["Color"], principled.inputs["Base Color"])
    principled.inputs["Metallic"].default_value = 0.0
    principled.inputs["Roughness"].default_value = 0.78 if tire else 0.28
    set_if_present(principled, "Coat Weight", 0.0 if tire else 0.42)
    set_if_present(principled, "Coat Roughness", 0.0 if tire else 0.16)

    rough_noise = nodes.new("ShaderNodeTexNoise")
    rough_noise.inputs["Scale"].default_value = 52.0 if tire else 110.0
    rough_noise.inputs["Detail"].default_value = 2.0
    links.new(texcoord.outputs["Generated"], rough_noise.inputs["Vector"])
    rough_ramp = nodes.new("ShaderNodeValToRGB")
    rough_ramp.color_ramp.elements[0].color = (
        (0.66, 0.66, 0.66, 1.0) if tire else (0.19, 0.19, 0.19, 1.0)
    )
    rough_ramp.color_ramp.elements[1].color = (
        (0.88, 0.88, 0.88, 1.0) if tire else (0.36, 0.36, 0.36, 1.0)
    )
    links.new(rough_noise.outputs["Fac"], rough_ramp.inputs["Fac"])
    links.new(rough_ramp.outputs["Color"], principled.inputs["Roughness"])

    output = nodes.new("ShaderNodeOutputMaterial")
    links.new(principled.outputs["BSDF"], output.inputs["Surface"])


def bake_object(obj, motif, tire=False):
    ensure_bake_uv(obj)
    label = clean_name(obj.name)
    material = bpy.data.materials.new(f"HornetNoSponsors_{label}")
    material.use_nodes = True
    assign_only(obj, material)
    pattern_shader(material, motif, tire=tire)

    image_name = f"{label}_BaseColor_AO"
    old = bpy.data.images.get(image_name)
    if old is not None:
        bpy.data.images.remove(old)
    baked = bpy.data.images.new(
        image_name,
        width=TEXTURE_SIZE,
        height=TEXTURE_SIZE,
        alpha=False,
        float_buffer=False,
    )
    baked.generated_color = (0.02, 0.02, 0.02, 1.0) if tire else (1.0, 0.68, 0.0, 1.0)
    baked.colorspace_settings.name = "sRGB"
    target = material.node_tree.nodes.new("ShaderNodeTexImage")
    target.image = baked
    material.node_tree.nodes.active = target
    target.select = True

    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.bake(type="EMIT", margin=18, use_clear=True)

    destination = TEXTURE_DIR / f"{image_name}.png"
    baked.filepath_raw = str(destination)
    baked.file_format = "PNG"
    baked.save()
    baked.pack()
    finalize_pbr(material, baked, tire=tire)
    material.diffuse_color = (0.018, 0.021, 0.024, 1.0) if tire else (1.0, 0.64, 0.0, 1.0)
    if tire:
        for polygon in obj.data.polygons:
            polygon.use_smooth = True
    print(f"BAKED|{obj.name}|{destination}")
    return destination


def configure_rims():
    yellow = new_flat_material(
        "Hornet_Rim_Yellow", (0.95, 0.52, 0.015, 1.0), metallic=0.72, roughness=0.24
    )
    black = new_flat_material(
        "Hornet_Rim_Black", (0.012, 0.014, 0.017, 1.0), metallic=0.62, roughness=0.26
    )
    metal = new_flat_material(
        "Hornet_Hub_Metal", (0.15, 0.17, 0.19, 1.0), metallic=0.92, roughness=0.2
    )
    red = new_flat_material(
        "Hornet_Hub_Red", (0.42, 0.006, 0.004, 1.0), metallic=0.68, roughness=0.22
    )
    for obj in bpy.data.objects:
        if obj.type != "MESH" or "GEO_INDY2010_" not in obj.name:
            continue
        if "_RIM_01" in obj.name or "_RIM_06" in obj.name:
            assign_only(obj, yellow)
        elif "_RIM_" in obj.name:
            assign_only(obj, black)
        elif "_NUT_05" in obj.name:
            assign_only(obj, red)
        elif "_NUT_" in obj.name:
            assign_only(obj, metal)


def configure_untextured_bodywork():
    yellow = new_flat_material(
        "Hornet_Untextured_Yellow",
        (0.95, 0.50, 0.008, 1.0),
        metallic=0.0,
        roughness=0.28,
    )
    black = new_flat_material(
        "Hornet_Untextured_Black",
        (0.009, 0.012, 0.016, 1.0),
        metallic=0.05,
        roughness=0.31,
    )
    mirrors = bpy.data.objects.get("Mirrors")
    aero = bpy.data.objects.get("Aero2")
    if mirrors is not None:
        assign_only(mirrors, yellow)
    if aero is not None:
        assign_only(aero, black)


def render_reviews():
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 1600
    scene.render.resolution_y = 1000
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.film_transparent = False
    scene.render.image_settings.color_mode = "RGBA"
    scene.render.resolution_percentage = 100
    scene.world = bpy.data.worlds.new("HornetReviewWorld")
    scene.world.use_nodes = True
    background = scene.world.node_tree.nodes.get("Background")
    background.inputs["Color"].default_value = (0.055, 0.065, 0.085, 1.0)
    background.inputs["Strength"].default_value = 0.32
    scene.view_settings.look = "AgX - Medium High Contrast"

    bpy.ops.mesh.primitive_plane_add(size=200, location=(0.0, 0.0, -0.34))
    ground = bpy.context.object
    ground.name = "TEMP_REVIEW_GROUND"
    ground.data.materials.append(
        new_flat_material("TEMP_REVIEW_GROUND_MAT", (0.075, 0.085, 0.105, 1.0), roughness=0.82)
    )

    for name, position, energy, size in (
        ("TEMP_KEY", (4.5, 5.5, 7.5), 1450.0, 5.0),
        ("TEMP_FILL", (-4.0, 1.0, 4.0), 900.0, 4.0),
        ("TEMP_RIM", (1.0, -5.5, 5.5), 1650.0, 4.0),
    ):
        data = bpy.data.lights.new(name, "AREA")
        data.energy = energy
        data.shape = "DISK"
        data.size = size
        light = bpy.data.objects.new(name, data)
        scene.collection.objects.link(light)
        light.location = position
        light.rotation_euler = (-light.location).to_track_quat("-Z", "Y").to_euler()

    camera_data = bpy.data.cameras.new("TEMP_REVIEW_CAMERA")
    camera = bpy.data.objects.new("TEMP_REVIEW_CAMERA", camera_data)
    scene.collection.objects.link(camera)
    scene.camera = camera
    camera_data.type = "ORTHO"
    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    views = (
        ("front_three_quarter", (4.2, 6.1, 3.3), (0.0, 0.15, 0.08), 5.8),
        ("rear_three_quarter", (-4.2, -6.1, 3.2), (0.0, -0.1, 0.08), 5.8),
        ("side", (7.2, 0.0, 1.7), (0.0, 0.15, 0.08), 5.7),
        ("top", (0.0, 0.2, 8.0), (0.0, 0.2, 0.0), 7.1),
    )
    for name, position, target, scale in views:
        camera.location = position
        camera.rotation_euler = (Vector(target) - camera.location).to_track_quat("-Z", "Y").to_euler()
        camera_data.ortho_scale = scale
        scene.render.filepath = str(REPORT_DIR / f"{name}.png")
        bpy.ops.render.render(write_still=True)
        print(f"RENDER|{scene.render.filepath}")


def main():
    if Path(bpy.data.filepath).resolve() != SOURCE.resolve():
        raise RuntimeError(f"Expected canonical source {SOURCE}, opened {bpy.data.filepath}")
    if not MOTIF_PATH.exists():
        raise FileNotFoundError(MOTIF_PATH)
    TEXTURE_DIR.mkdir(parents=True, exist_ok=True)
    motif = bpy.data.images.load(str(MOTIF_PATH), check_existing=True)
    motif.colorspace_settings.name = "sRGB"

    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    # AO is deliberately broad/painted; four Cycles samples preserve that soft
    # character while keeping a twelve-atlas bake practical on CPU.
    scene.cycles.samples = 4
    scene.cycles.use_denoising = False
    scene.cycles.device = "CPU"
    scene.render.bake.use_clear = True
    scene.render.bake.margin = 18

    before = {
        obj.name: (len(obj.data.vertices), len(obj.data.polygons))
        for obj in bpy.data.objects
        if obj.type == "MESH"
    }
    outputs = []
    for name in sorted(LIVERY_OBJECTS):
        obj = bpy.data.objects.get(name)
        if obj is None or obj.type != "MESH":
            raise RuntimeError(f"Missing expected livery mesh: {name}")
        outputs.append(str(bake_object(obj, motif, tire=False)))

    tire_objects = sorted(
        (
            obj
            for obj in bpy.data.objects
            if obj.type == "MESH" and obj.name.endswith("_TIRE")
        ),
        key=lambda obj: obj.name,
    )
    if len(tire_objects) != 4:
        raise RuntimeError(f"Expected four slick tire meshes, found {len(tire_objects)}")
    for obj in tire_objects:
        outputs.append(str(bake_object(obj, motif, tire=True)))
    configure_rims()
    configure_untextured_bodywork()

    after = {
        obj.name: (len(obj.data.vertices), len(obj.data.polygons))
        for obj in bpy.data.objects
        if obj.type == "MESH"
    }
    if before != after:
        raise RuntimeError("Geometry topology changed while applying textures")

    scene["livery_name"] = "Hornet Yellow Black - No Sponsors"
    scene["livery_reference_motif"] = str(MOTIF_PATH)
    scene["livery_baked_ao"] = True
    scene["tire_surface"] = "slick_no_tread_texture_no_normal_map"
    bpy.ops.wm.save_as_mainfile(filepath=str(SOURCE), compress=True)

    report = {
        "source": str(SOURCE),
        "motif": str(MOTIF_PATH),
        "texture_size": TEXTURE_SIZE,
        "baked_ambient_occlusion": True,
        "sponsors_or_text_added": False,
        "tire_tread_texture_or_normal_added": False,
        "geometry_topology_unchanged": True,
        "livery_objects": sorted(LIVERY_OBJECTS),
        "tire_objects": [obj.name for obj in tire_objects],
        "textures": outputs,
    }
    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    (REPORT_DIR / "validation.json").write_text(
        json.dumps(report, indent=2), encoding="utf-8"
    )
    print("VALIDATION|" + json.dumps(report, sort_keys=True))

    render_reviews()


if __name__ == "__main__":
    main()
