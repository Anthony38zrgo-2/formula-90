"""Bake a pixel-continuous 2048 square tile from the generated carbon source."""

from pathlib import Path

import bpy


ROOT = Path(__file__).resolve().parents[2]
DIRECTORY = ROOT / "game/assets/models/vehicles/f1-2026-2008/source/textures/carbon_fiber_seamless"
SOURCE = DIRECTORY / "carbon_fiber_2x2_twill_source.png"
OUTPUT = DIRECTORY / "carbon_fiber_2x2_twill_basecolor.png"

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.mesh.primitive_plane_add(size=2.0)
plane = bpy.context.object
material = bpy.data.materials.new("SeamlessCarbonTileBake")
material.use_nodes = True
plane.data.materials.append(material)
nodes = material.node_tree.nodes
links = material.node_tree.links
nodes.clear()

uv = nodes.new("ShaderNodeTexCoord")
mapping = nodes.new("ShaderNodeMapping")
mapping.inputs["Scale"].default_value = (2.0, 2.0, 1.0)
links.new(uv.outputs["UV"], mapping.inputs["Vector"])
source = bpy.data.images.load(str(SOURCE), check_existing=True)
texture = nodes.new("ShaderNodeTexImage")
texture.image = source
texture.extension = "MIRROR"
texture.interpolation = "Linear"
links.new(mapping.outputs["Vector"], texture.inputs["Vector"])
emission = nodes.new("ShaderNodeEmission")
links.new(texture.outputs["Color"], emission.inputs["Color"])
output = nodes.new("ShaderNodeOutputMaterial")
links.new(emission.outputs["Emission"], output.inputs["Surface"])

target_image = bpy.data.images.new(
    "CarbonFiber_2x2_Twill_PixelSeamless",
    width=2048,
    height=2048,
    alpha=False,
    float_buffer=False,
)
target_image.colorspace_settings.name = "sRGB"
target = nodes.new("ShaderNodeTexImage")
target.image = target_image
target.select = True
nodes.active = target

scene = bpy.context.scene
scene.render.engine = "CYCLES"
scene.cycles.samples = 1
scene.render.bake.margin = 0
bpy.ops.object.select_all(action="DESELECT")
plane.select_set(True)
bpy.context.view_layer.objects.active = plane
bpy.ops.object.bake(type="EMIT", margin=0, use_clear=True)
target_image.filepath_raw = str(OUTPUT)
target_image.file_format = "PNG"
target_image.save()
print(f"SEAMLESS_TILE|{OUTPUT}|{tuple(target_image.size)}")
