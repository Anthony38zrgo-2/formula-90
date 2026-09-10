"""Bake a clean sponsor-free livery matching the supplied side reference.

Only UV layers and materials are changed. Mesh topology and vertex positions are
treated as invariants. Tires remain slick and receive no bump/normal nodes.
"""

from __future__ import annotations

import json
import math
import re
from pathlib import Path

import bpy
from mathutils import Vector


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "game/assets/models/vehicles/f1-2026-2008/source/f1_2026_2008_source.blend"
TEXTURE_DIR = SOURCE.parent / "textures/hornet_reference_match"
REPORT_DIR = ROOT / "reports/2026-09-09-hornet-reference-match"

LIVERY_OBJECTS = (
    "Body",
    "AeroPart1",
    "ActiveFront",
    "FrontWing",
    "Endplate",
    "ActiveRear",
    "AeroPart2",
    "Rear Wing",
)

YELLOW = (1.0, 0.53, 0.002, 1.0)
BLACK = (0.006, 0.008, 0.012, 1.0)
RED = (0.80, 0.012, 0.004, 1.0)
ORANGE = (1.0, 0.19, 0.002, 1.0)
GREY = (0.46, 0.52, 0.60, 1.0)
GOLD = (1.0, 0.48, 0.0, 1.0)


def connect(links, value, target):
    if hasattr(value, "node"):
        links.new(value, target)
    else:
        target.default_value = value


def math_node(nodes, links, operation, a, b=0.0):
    node = nodes.new("ShaderNodeMath")
    node.operation = operation
    connect(links, a, node.inputs[0])
    connect(links, b, node.inputs[1])
    return node.outputs[0]


def multiply(nodes, links, *values):
    output = values[0]
    for value in values[1:]:
        output = math_node(nodes, links, "MULTIPLY", output, value)
    return output


def between(nodes, links, value, low, high):
    return multiply(
        nodes,
        links,
        math_node(nodes, links, "GREATER_THAN", value, low),
        math_node(nodes, links, "LESS_THAN", value, high),
    )


def absolute_difference(nodes, links, value, center):
    return math_node(
        nodes,
        links,
        "ABSOLUTE",
        math_node(nodes, links, "SUBTRACT", value, center),
    )


def ellipse(nodes, links, a, b, center_a, center_b, radius_a, radius_b):
    da = math_node(nodes, links, "DIVIDE", math_node(nodes, links, "SUBTRACT", a, center_a), radius_a)
    db = math_node(nodes, links, "DIVIDE", math_node(nodes, links, "SUBTRACT", b, center_b), radius_b)
    distance = math_node(
        nodes,
        links,
        "ADD",
        math_node(nodes, links, "MULTIPLY", da, da),
        math_node(nodes, links, "MULTIPLY", db, db),
    )
    return math_node(nodes, links, "LESS_THAN", distance, 1.0)


def mix_color(nodes, links, base, overlay, factor):
    node = nodes.new("ShaderNodeMixRGB")
    node.blend_type = "MIX"
    connect(links, factor, node.inputs[0])
    connect(links, base, node.inputs[1])
    connect(links, overlay, node.inputs[2])
    return node.outputs[0]


def generated_axes(nodes, links):
    coordinates = nodes.new("ShaderNodeTexCoord")
    separate = nodes.new("ShaderNodeSeparateXYZ")
    links.new(coordinates.outputs["Generated"], separate.inputs["Vector"])
    return separate.outputs["X"], separate.outputs["Y"], separate.outputs["Z"]


def body_color(nodes, links):
    x, y, z = generated_axes(nodes, links)
    lateral = absolute_difference(nodes, links, y, 0.5)
    side = math_node(nodes, links, "GREATER_THAN", lateral, 0.27)
    geometry = nodes.new("ShaderNodeNewGeometry")
    normal_axes = nodes.new("ShaderNodeSeparateXYZ")
    links.new(geometry.outputs["Normal"], normal_axes.inputs["Vector"])
    normal_y = normal_axes.outputs["Y"]
    side_facing = math_node(
        nodes,
        links,
        "GREATER_THAN",
        math_node(nodes, links, "ABSOLUTE", normal_y),
        0.24,
    )
    color = YELLOW

    # Black floor and lower body.
    color = mix_color(nodes, links, color, BLACK, math_node(nodes, links, "LESS_THAN", z, 0.19))

    # One large, clean black sidepod panel per side.
    sidepod = multiply(
        nodes,
        links,
        side,
        between(nodes, links, x, 0.42, 0.77),
        between(nodes, links, z, 0.19, 0.52),
    )
    color = mix_color(nodes, links, color, BLACK, sidepod)

    # Yellow lower blocks interrupt the black floor as in the reference.
    for low, high in ((0.31, 0.40), (0.52, 0.63), (0.72, 0.82)):
        block = multiply(
            nodes,
            links,
            side,
            between(nodes, links, x, low, high),
            between(nodes, links, z, 0.105, 0.17),
        )
        color = mix_color(nodes, links, color, GOLD, block)

    # Restrained black stripe below the engine cover.
    engine_stripe = multiply(
        nodes,
        links,
        side,
        between(nodes, links, x, 0.68, 0.88),
        between(nodes, links, z, 0.53, 0.61),
    )
    color = mix_color(nodes, links, color, BLACK, engine_stripe)

    # Tapered black nose region; generated X=0 is the front of this model.
    nose_width = math_node(
        nodes,
        links,
        "ADD",
        0.065,
        math_node(
            nodes,
            links,
            "MULTIPLY",
            math_node(nodes, links, "MAXIMUM", math_node(nodes, links, "SUBTRACT", 0.31, x), 0.0),
            0.78,
        ),
    )
    nose = multiply(
        nodes,
        links,
        math_node(nodes, links, "LESS_THAN", x, 0.31),
        math_node(nodes, links, "GREATER_THAN", z, 0.20),
        math_node(nodes, links, "LESS_THAN", lateral, nose_width),
    )
    color = mix_color(nodes, links, color, BLACK, nose)

    # Symmetric hornet chevrons remain confined to the nose.
    for slope, offset, thickness, stripe_color in (
        (0.66, 0.045, 0.030, GOLD),
        (0.66, 0.105, 0.024, ORANGE),
        (0.66, 0.165, 0.018, YELLOW),
    ):
        target = math_node(nodes, links, "ADD", offset, math_node(nodes, links, "MULTIPLY", x, slope))
        stripe = multiply(
            nodes,
            links,
            nose,
            math_node(nodes, links, "LESS_THAN", absolute_difference(nodes, links, lateral, target), thickness),
        )
        color = mix_color(nodes, links, color, stripe_color, stripe)

    # Red oval on the nose top and red/black side accents.
    nose_oval = multiply(
        nodes,
        links,
        ellipse(nodes, links, x, y, 0.115, 0.5, 0.052, 0.105),
        math_node(nodes, links, "GREATER_THAN", z, 0.28),
    )
    color = mix_color(nodes, links, color, RED, nose_oval)
    side_red = multiply(nodes, links, side_facing, ellipse(nodes, links, x, z, 0.365, 0.45, 0.058, 0.072))
    color = mix_color(nodes, links, color, RED, side_red)
    side_black = multiply(nodes, links, side_facing, ellipse(nodes, links, x, z, 0.815, 0.43, 0.068, 0.058))
    color = mix_color(nodes, links, color, BLACK, side_black)

    # Orange/yellow wing strokes on the nose side faces.
    nose_side_region = multiply(
        nodes,
        links,
        side_facing,
        math_node(nodes, links, "LESS_THAN", x, 0.29),
        between(nodes, links, z, 0.28, 0.58),
    )
    for offset, stripe_color in ((0.18, ORANGE), (0.235, YELLOW), (0.29, ORANGE)):
        target = math_node(nodes, links, "ADD", offset, math_node(nodes, links, "MULTIPLY", x, 0.70))
        stroke = multiply(
            nodes,
            links,
            nose_side_region,
            math_node(nodes, links, "LESS_THAN", absolute_difference(nodes, links, z, target), 0.016),
        )
        color = mix_color(nodes, links, color, stripe_color, stroke)

    # German flag on the negative-Y/reference half only. Generated Y is more
    # reliable here than imported normals, which are irregular around the
    # narrow monocoque seams.
    flag_side = math_node(nodes, links, "LESS_THAN", y, 0.5)
    flag_border = multiply(
        nodes,
        links,
        flag_side,
        between(nodes, links, x, 0.492, 0.623),
        between(nodes, links, z, 0.315, 0.500),
    )
    flag_x = between(nodes, links, x, 0.500, 0.615)
    black_band = multiply(nodes, links, flag_side, flag_x, between(nodes, links, z, 0.435, 0.490))
    red_band = multiply(nodes, links, flag_side, flag_x, between(nodes, links, z, 0.380, 0.435))
    gold_band = multiply(nodes, links, flag_side, flag_x, between(nodes, links, z, 0.325, 0.380))
    color = mix_color(nodes, links, color, GREY, flag_border)
    color = mix_color(nodes, links, color, BLACK, black_band)
    color = mix_color(nodes, links, color, RED, red_band)
    color = mix_color(nodes, links, color, GOLD, gold_band)

    # Thin silver-grey lines on both forward flanks.
    line_region = multiply(
        nodes,
        links,
        side_facing,
        between(nodes, links, x, 0.20, 0.43),
        between(nodes, links, z, 0.32, 0.66),
    )
    for offset in (0.16, 0.205, 0.25, 0.295):
        target = math_node(nodes, links, "ADD", offset, math_node(nodes, links, "MULTIPLY", x, 0.78))
        line = multiply(
            nodes,
            links,
            line_region,
            math_node(nodes, links, "LESS_THAN", absolute_difference(nodes, links, z, target), 0.008),
        )
        color = mix_color(nodes, links, color, GREY, line)

    # Red-to-orange cap on the upper engine intake cover.
    intake_cap = multiply(
        nodes,
        links,
        between(nodes, links, x, 0.665, 0.815),
        math_node(nodes, links, "LESS_THAN", lateral, 0.16),
        math_node(nodes, links, "GREATER_THAN", z, 0.69),
    )
    red_cap = multiply(nodes, links, intake_cap, math_node(nodes, links, "LESS_THAN", x, 0.735))
    orange_cap = multiply(nodes, links, intake_cap, math_node(nodes, links, "GREATER_THAN", x, 0.735))
    color = mix_color(nodes, links, color, RED, red_cap)
    color = mix_color(nodes, links, color, ORANGE, orange_cap)
    return color


def aero_color(nodes, links, object_name):
    x, y, z = generated_axes(nodes, links)
    color = YELLOW
    if object_name in {"ActiveFront", "FrontWing"}:
        panel = multiply(
            nodes,
            links,
            between(nodes, links, y, 0.20, 0.80),
            between(nodes, links, z, 0.30, 0.58),
        )
        color = mix_color(nodes, links, color, BLACK, panel)
    elif object_name == "Endplate":
        oval = ellipse(nodes, links, x, z, 0.52, 0.50, 0.31, 0.17)
        color = mix_color(nodes, links, color, BLACK, oval)
    elif object_name in {"ActiveRear", "AeroPart2"}:
        panel = multiply(
            nodes,
            links,
            between(nodes, links, y, 0.16, 0.84),
            between(nodes, links, x, 0.22, 0.78),
        )
        color = mix_color(nodes, links, color, BLACK, panel)
    elif object_name == "Rear Wing":
        panel = multiply(
            nodes,
            links,
            between(nodes, links, x, 0.16, 0.84),
            between(nodes, links, z, 0.56, 0.78),
        )
        oval = ellipse(nodes, links, x, z, 0.52, 0.37, 0.25, 0.115)
        color = mix_color(nodes, links, color, BLACK, panel)
        color = mix_color(nodes, links, color, RED, oval)
    elif object_name == "AeroPart1":
        panel = multiply(
            nodes,
            links,
            between(nodes, links, x, 0.17, 0.83),
            between(nodes, links, z, 0.13, 0.62),
        )
        color = mix_color(nodes, links, color, BLACK, panel)
    return color


def tire_color(nodes, links):
    coordinates = nodes.new("ShaderNodeTexCoord")
    noise = nodes.new("ShaderNodeTexNoise")
    noise.inputs["Scale"].default_value = 165.0
    noise.inputs["Detail"].default_value = 1.4
    noise.inputs["Roughness"].default_value = 0.42
    links.new(coordinates.outputs["Generated"], noise.inputs["Vector"])
    ramp = nodes.new("ShaderNodeValToRGB")
    ramp.color_ramp.elements[0].color = (0.006, 0.007, 0.008, 1.0)
    ramp.color_ramp.elements[1].color = (0.020, 0.022, 0.025, 1.0)
    links.new(noise.outputs["Fac"], ramp.inputs["Fac"])
    return mix_color(nodes, links, (0.010, 0.011, 0.013, 1.0), ramp.outputs["Color"], 0.10)


def add_subtle_microvariation(nodes, links, color, tire=False):
    coordinates = nodes.new("ShaderNodeTexCoord")
    noise = nodes.new("ShaderNodeTexNoise")
    noise.inputs["Scale"].default_value = 190.0 if not tire else 220.0
    noise.inputs["Detail"].default_value = 1.6
    links.new(coordinates.outputs["Generated"], noise.inputs["Vector"])
    ramp = nodes.new("ShaderNodeValToRGB")
    ramp.color_ramp.elements[0].color = (0.94, 0.94, 0.94, 1.0)
    ramp.color_ramp.elements[1].color = (1.0, 1.0, 1.0, 1.0)
    links.new(noise.outputs["Fac"], ramp.inputs["Fac"])
    multiply_node = nodes.new("ShaderNodeMixRGB")
    multiply_node.blend_type = "MULTIPLY"
    multiply_node.inputs[0].default_value = 0.035 if not tire else 0.055
    connect(links, color, multiply_node.inputs[1])
    links.new(ramp.outputs["Color"], multiply_node.inputs[2])
    return multiply_node.outputs["Color"]


def clean_name(name):
    return re.sub(r"[^A-Za-z0-9_-]+", "_", name).strip("_")


def ensure_bake_uv(obj):
    layers = obj.data.uv_layers
    old = layers.get("ReferenceBakeUV")
    if old is not None:
        layers.remove(old)
    uv = layers.new(name="ReferenceBakeUV")
    layers.active = uv
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
        island_margin=0.012,
        area_weight=0.1,
        correct_aspect=True,
        scale_to_bounds=False,
    )
    bpy.ops.object.mode_set(mode="OBJECT")


def new_image(name, size, color_space="sRGB"):
    old = bpy.data.images.get(name)
    if old is not None:
        bpy.data.images.remove(old)
    image = bpy.data.images.new(name, width=size, height=size, alpha=False, float_buffer=False)
    image.colorspace_settings.name = color_space
    return image


def save_and_pack(image, path):
    image.filepath_raw = str(path)
    image.file_format = "PNG"
    image.save()
    image.pack()
    image.use_fake_user = True


def active_image_node(material, image):
    node = material.node_tree.nodes.new("ShaderNodeTexImage")
    node.image = image
    node.select = True
    material.node_tree.nodes.active = node
    return node


def select_for_bake(obj):
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj


def build_emission_material(material, color):
    nodes = material.node_tree.nodes
    links = material.node_tree.links
    emission = nodes.new("ShaderNodeEmission")
    connect(links, color, emission.inputs["Color"])
    output = nodes.new("ShaderNodeOutputMaterial")
    links.new(emission.outputs["Emission"], output.inputs["Surface"])


def combine_base_and_ao(material, base_image, ao_image, combined_image):
    nodes = material.node_tree.nodes
    links = material.node_tree.links
    nodes.clear()
    uv = nodes.new("ShaderNodeUVMap")
    uv.uv_map = "ReferenceBakeUV"
    base = nodes.new("ShaderNodeTexImage")
    base.image = base_image
    ao = nodes.new("ShaderNodeTexImage")
    ao.image = ao_image
    links.new(uv.outputs["UV"], base.inputs["Vector"])
    links.new(uv.outputs["UV"], ao.inputs["Vector"])
    multiply_node = nodes.new("ShaderNodeMixRGB")
    multiply_node.blend_type = "MULTIPLY"
    multiply_node.inputs[0].default_value = 0.46
    links.new(base.outputs["Color"], multiply_node.inputs[1])
    links.new(ao.outputs["Color"], multiply_node.inputs[2])
    build_emission_material(material, multiply_node.outputs["Color"])
    active_image_node(material, combined_image)


def finalize_material(material, combined_image, tire=False):
    nodes = material.node_tree.nodes
    links = material.node_tree.links
    nodes.clear()
    uv = nodes.new("ShaderNodeUVMap")
    uv.uv_map = "ReferenceBakeUV"
    texture = nodes.new("ShaderNodeTexImage")
    texture.image = combined_image
    texture.interpolation = "Linear"
    links.new(uv.outputs["UV"], texture.inputs["Vector"])
    principled = nodes.new("ShaderNodeBsdfPrincipled")
    links.new(texture.outputs["Color"], principled.inputs["Base Color"])
    principled.inputs["Metallic"].default_value = 0.0
    principled.inputs["Roughness"].default_value = 0.77 if tire else 0.29
    if principled.inputs.get("Coat Weight"):
        principled.inputs["Coat Weight"].default_value = 0.0 if tire else 0.34
    if principled.inputs.get("Coat Roughness"):
        principled.inputs["Coat Roughness"].default_value = 0.0 if tire else 0.18
    output = nodes.new("ShaderNodeOutputMaterial")
    links.new(principled.outputs["BSDF"], output.inputs["Surface"])


def bake_object(obj, tire=False):
    ensure_bake_uv(obj)
    label = clean_name(obj.name)
    size = 4096 if obj.name == "Body" else 2048
    material = bpy.data.materials.new(f"ReferenceMatch_{label}")
    material.use_nodes = True
    obj.data.materials.clear()
    obj.data.materials.append(material)
    for polygon in obj.data.polygons:
        polygon.material_index = 0
        if tire:
            polygon.use_smooth = True

    nodes = material.node_tree.nodes
    links = material.node_tree.links
    nodes.clear()
    if tire:
        color = tire_color(nodes, links)
    elif obj.name == "Body":
        color = body_color(nodes, links)
    else:
        color = aero_color(nodes, links, obj.name)
    color = add_subtle_microvariation(nodes, links, color, tire=tire)
    build_emission_material(material, color)

    base = new_image(f"Reference_{label}_BaseColor", size)
    active_image_node(material, base)
    select_for_bake(obj)
    bpy.context.scene.cycles.samples = 1
    bpy.ops.object.bake(type="EMIT", margin=18, use_clear=True)
    base_path = TEXTURE_DIR / f"Reference_{label}_BaseColor.png"
    save_and_pack(base, base_path)

    ao = new_image(f"Reference_{label}_AO", size, "Non-Color")
    active_image_node(material, ao)
    select_for_bake(obj)
    bpy.context.scene.cycles.samples = 4
    bpy.ops.object.bake(type="AO", margin=18, use_clear=True)
    ao_path = TEXTURE_DIR / f"Reference_{label}_AO.png"
    save_and_pack(ao, ao_path)

    combined = new_image(f"Reference_{label}_BaseColor_AO", size)
    combine_base_and_ao(material, base, ao, combined)
    select_for_bake(obj)
    bpy.context.scene.cycles.samples = 1
    bpy.ops.object.bake(type="EMIT", margin=18, use_clear=True)
    combined_path = TEXTURE_DIR / f"Reference_{label}_BaseColor_AO.png"
    save_and_pack(combined, combined_path)
    finalize_material(material, combined, tire=tire)
    print(f"BAKED|{obj.name}|{size}|{combined_path}")
    return {
        "object": obj.name,
        "size": size,
        "base_color": str(base_path),
        "ao": str(ao_path),
        "base_color_ao": str(combined_path),
        "tire": tire,
    }


def flat_material(name, color, metallic=0.0, roughness=0.3, coat=0.25):
    material = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    material.use_nodes = True
    nodes = material.node_tree.nodes
    links = material.node_tree.links
    nodes.clear()
    principled = nodes.new("ShaderNodeBsdfPrincipled")
    principled.inputs["Base Color"].default_value = color
    principled.inputs["Metallic"].default_value = metallic
    principled.inputs["Roughness"].default_value = roughness
    if principled.inputs.get("Coat Weight"):
        principled.inputs["Coat Weight"].default_value = coat
    output = nodes.new("ShaderNodeOutputMaterial")
    links.new(principled.outputs["BSDF"], output.inputs["Surface"])
    return material


def assign_material(obj, material):
    obj.data.materials.clear()
    obj.data.materials.append(material)
    for polygon in obj.data.polygons:
        polygon.material_index = 0


def configure_details():
    rim_yellow = flat_material("Reference_Rim_Yellow", (0.95, 0.46, 0.004, 1.0), 0.70, 0.24)
    rim_black = flat_material("Reference_Rim_Black", BLACK, 0.58, 0.27)
    hub = flat_material("Reference_Hub_Metal", (0.11, 0.13, 0.15, 1.0), 0.92, 0.20, 0.0)
    hub_red = flat_material("Reference_Hub_Red", (0.34, 0.006, 0.004, 1.0), 0.66, 0.23)
    yellow = flat_material("Reference_Untextured_Yellow", YELLOW, 0.0, 0.29)
    black = flat_material("Reference_Untextured_Black", BLACK, 0.04, 0.32, 0.05)
    for obj in bpy.data.objects:
        if obj.type != "MESH" or "GEO_INDY2010_" not in obj.name:
            continue
        if "_RIM_01" in obj.name or "_RIM_06" in obj.name:
            assign_material(obj, rim_yellow)
        elif "_RIM_" in obj.name:
            assign_material(obj, rim_black)
        elif "_NUT_05" in obj.name:
            assign_material(obj, hub_red)
        elif "_NUT_" in obj.name:
            assign_material(obj, hub)
    if bpy.data.objects.get("Mirrors"):
        assign_material(bpy.data.objects["Mirrors"], yellow)
    if bpy.data.objects.get("Aero2"):
        assign_material(bpy.data.objects["Aero2"], black)


def render_reviews():
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 1600
    scene.render.resolution_y = 1000
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.world = bpy.data.worlds.new("ReferenceReviewWorld")
    scene.world.use_nodes = True
    bg = scene.world.node_tree.nodes["Background"]
    bg.inputs["Color"].default_value = (0.06, 0.07, 0.09, 1.0)
    bg.inputs["Strength"].default_value = 0.30
    scene.view_settings.look = "AgX - Medium High Contrast"
    bpy.ops.mesh.primitive_plane_add(size=200, location=(0.0, 0.0, -0.38))
    ground = bpy.context.object
    ground.data.materials.append(flat_material("TEMP_REFERENCE_GROUND", (0.09, 0.10, 0.12, 1.0), roughness=0.86))
    for name, position, energy, size in (
        ("TEMP_KEY", (-3.8, -4.5, 7.0), 1500.0, 5.0),
        ("TEMP_FILL", (3.0, 4.0, 4.2), 850.0, 4.5),
        ("TEMP_RIM", (4.5, -2.0, 5.0), 1450.0, 4.0),
    ):
        data = bpy.data.lights.new(name, "AREA")
        data.energy = energy
        data.shape = "DISK"
        data.size = size
        light = bpy.data.objects.new(name, data)
        scene.collection.objects.link(light)
        light.location = position
        light.rotation_euler = (-light.location).to_track_quat("-Z", "Y").to_euler()
    camera_data = bpy.data.cameras.new("TEMP_REFERENCE_CAMERA")
    camera = bpy.data.objects.new("TEMP_REFERENCE_CAMERA", camera_data)
    scene.collection.objects.link(camera)
    scene.camera = camera
    camera_data.type = "ORTHO"
    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    views = (
        ("right_side", (0.0, -7.0, 1.15), (0.0, 0.0, 0.05), 6.1),
        ("left_side", (0.0, 7.0, 1.15), (0.0, 0.0, 0.05), 6.1),
        ("front", (-7.0, 0.0, 1.25), (-0.25, 0.0, 0.02), 3.0),
        ("rear", (7.0, 0.0, 1.35), (0.30, 0.0, 0.08), 3.0),
        ("front_three_quarter", (-5.2, -5.2, 3.1), (-0.10, 0.0, 0.08), 5.9),
        ("top", (0.0, 0.0, 8.0), (0.0, 0.0, 0.0), 6.4),
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
        raise RuntimeError(f"Expected {SOURCE}, opened {bpy.data.filepath}")
    TEXTURE_DIR.mkdir(parents=True, exist_ok=True)
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.use_denoising = False
    scene.render.bake.margin = 18
    before = {
        obj.name: (len(obj.data.vertices), len(obj.data.polygons))
        for obj in bpy.data.objects
        if obj.type == "MESH"
    }
    outputs = []
    for name in LIVERY_OBJECTS:
        obj = bpy.data.objects.get(name)
        if obj is None or obj.type != "MESH":
            raise RuntimeError(f"Missing expected mesh {name}")
        outputs.append(bake_object(obj, tire=False))
    tires = sorted(
        (obj for obj in bpy.data.objects if obj.type == "MESH" and obj.name.endswith("_TIRE")),
        key=lambda obj: obj.name,
    )
    if len(tires) != 4:
        raise RuntimeError(f"Expected four slick tires, found {len(tires)}")
    for tire in tires:
        outputs.append(bake_object(tire, tire=True))
    configure_details()
    after = {
        obj.name: (len(obj.data.vertices), len(obj.data.polygons))
        for obj in bpy.data.objects
        if obj.type == "MESH"
    }
    if before != after:
        raise RuntimeError("Geometry topology changed during texture work")
    scene["livery_name"] = "Reference Match Yellow Black - No Sponsors"
    scene["livery_reference"] = "ChatGPT Image 9 sept 2026, 11_04_00 a.m..png"
    scene["livery_ao_separate_and_combined"] = True
    scene["tire_surface"] = "slick_anthracite_no_bump_no_normal_no_grooves"
    bpy.ops.wm.save_as_mainfile(filepath=str(SOURCE), compress=True)
    report = {
        "source": str(SOURCE),
        "reference": scene["livery_reference"],
        "geometry_topology_unchanged": True,
        "sponsors_text_logos_added": False,
        "ao_baked_separately": True,
        "ao_multiplied_into_final_base_color": True,
        "tires_slick_without_bump_normal_or_grooves": True,
        "outputs": outputs,
    }
    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    (REPORT_DIR / "validation.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print("VALIDATION|" + json.dumps(report, sort_keys=True))
    render_reviews()


if __name__ == "__main__":
    main()
