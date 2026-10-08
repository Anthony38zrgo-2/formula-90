import bpy
import hashlib
import json
import math
import shutil
import struct
import subprocess
import sys
from pathlib import Path
from mathutils import Vector
from mathutils.bvhtree import BVHTree

project_directory = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(project_directory / 'tools'))
from common.output_policy import validate_output_path
source_path = project_directory / 'game/assets/models/vehicles/f1-2030/source/f1_2030.blend'
requested_family = sys.argv[sys.argv.index('--material-family')+1] if '--material-family' in sys.argv else None
output_directory = validate_output_path(project_directory, 'scratch/vehicle_physical_materials', 'preview').path
output_directory.mkdir(parents=True, exist_ok=True)
texture_directory = validate_output_path(project_directory, source_path.parent / 'textures/baked_physical_materials', 'promote').path
texture_directory.mkdir(parents=True, exist_ok=True)
source_digest = hashlib.sha256(source_path.read_bytes()).hexdigest()
backup_path = validate_output_path(project_directory, output_directory / ('before_physical_material_baking_' + source_digest[:12] + '.blend'), 'preview').path
if not backup_path.exists():
    shutil.copy2(source_path, backup_path)
report = {'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=project_directory, text=True).strip(), 'branch': subprocess.check_output(['git', 'branch', '--show-current'], cwd=project_directory, text=True).strip(), 'status_before': subprocess.check_output(['git', 'status', '--short'], cwd=project_directory, text=True), 'source_before_sha256': hashlib.sha256(backup_path.read_bytes()).hexdigest(), 'groups': [], 'adjusted_decals': []}
assert Path(bpy.data.filepath).resolve() == source_path.resolve()
material_sources = {
    'BakedPhysicalRedLacquer': 'ReferenceRedChassisLacquer',
    'BakedPhysicalSatinCarbon': 'ReferenceSatinChassisCarbon',
    'BakedPhysicalChampagneGold': 'ReferenceChampagneGoldWheelMetal',
    'BakedPhysicalPolishedChrome': 'PolishedChromeExhaustMetal',
}
if requested_family is not None:
    requested_material = 'BakedPhysical'+''.join(word.title() for word in requested_family.split('_'))
    assert requested_material in material_sources,requested_family
    material_sources = {requested_material:material_sources[requested_material]}
previous_manifest_path = texture_directory / 'baking_manifest.json'
previous_manifest = json.loads(previous_manifest_path.read_text()) if previous_manifest_path.exists() else None
for scene_object in bpy.data.objects:
    if scene_object.type != 'MESH':
        continue
    for material_index, material in enumerate(scene_object.data.materials):
        if material.name in material_sources:
            scene_object.data.materials[material_index] = bpy.data.materials[material_sources[material.name]]
for material_name in material_sources:
    previous_material = bpy.data.materials.get(material_name)
    if previous_material is not None:
        previous_images = [node.image for node in previous_material.node_tree.nodes if node.type == 'TEX_IMAGE']
        bpy.data.materials.remove(previous_material)
        for image in previous_images:
            if image.users == 0:
                bpy.data.images.remove(image)

def geometry_digest(scene_object):
    digest = hashlib.sha256()
    digest.update(struct.pack('<16d', *(value for row in scene_object.matrix_world for value in row)))
    for vertex in scene_object.data.vertices:
        digest.update(struct.pack('<3f', *vertex.co))
    for polygon in scene_object.data.polygons:
        digest.update(struct.pack('<' + 'I' * len(polygon.vertices), *polygon.vertices))
    return digest.hexdigest()

original_geometry = {scene_object.name: geometry_digest(scene_object) for scene_object in bpy.data.objects if scene_object.type == 'MESH'}
body = bpy.data.objects['GEO_CHASSIS_BODY']
surface = BVHTree.FromPolygons([body.matrix_world @ vertex.co for vertex in body.data.vertices], [list(polygon.vertices) for polygon in body.data.polygons])
layout_path = source_path.parent / 'textures/reference_red_and_gold/sponsor_decal_layout.json'
layout = json.loads(layout_path.read_text())
for record in layout['decals']:
    if requested_family is not None:
        continue
    if record['logo'] not in ('Zyn', 'DriverNumber') or record.get('side', 0) == 0:
        continue
    scene_object = bpy.data.objects.get(record['object'])
    if scene_object is None:
        continue
    side_sign = record['side']
    center_horizontal, center_vertical = record['center']
    width, height = record['size']
    hit, normal, polygon_index, distance = surface.ray_cast(Vector((center_horizontal, side_sign * 3, center_vertical)), Vector((0,-side_sign,0)), 6)
    assert hit is not None
    horizontal = Vector((-side_sign,0,0))
    horizontal = (horizontal - normal * horizontal.dot(normal)).normalized()
    vertical = normal.cross(horizontal).normalized()
    if vertical.z < 0:
        vertical = -vertical
    changed_positions = []
    for vertex in scene_object.data.vertices:
        row, column = divmod(vertex.index, 25)
        destination = hit + horizontal * ((column / 24 - 0.5) * width) + vertical * ((row / 12 - 0.5) * height)
        position, surface_normal, polygon_index, distance = surface.ray_cast(destination + normal * 0.10, -normal, 0.25)
        if position is None:
            position, surface_normal, polygon_index, distance = surface.find_nearest(destination, 0.08)
        assert position is not None, (scene_object.name, vertex.index)
        changed_positions.append(scene_object.matrix_world.inverted() @ (position + surface_normal * 0.0015))
    for vertex, position in zip(scene_object.data.vertices, changed_positions):
        vertex.co = position
    scene_object.data.update()
    report['adjusted_decals'].append(scene_object.name)

generated_source = texture_directory / 'generated_clean_red_albedo_reference.png'
generated_destination = validate_output_path(project_directory, texture_directory / 'generated_clean_red_albedo_reference.png', 'promote').path
if generated_source.resolve() != generated_destination.resolve():
    shutil.copy2(generated_source, generated_destination)
red_material = bpy.data.materials['ReferenceRedChassisLacquer']
red_shader = next(node for node in red_material.node_tree.nodes if node.type == 'BSDF_PRINCIPLED')
red_texture = next(node for node in red_material.node_tree.nodes if node.type == 'TEX_IMAGE')
red_texture.image = bpy.data.images.load(str(generated_destination), check_existing=True)
red_texture.image.colorspace_settings.name = 'sRGB'
red_mix = red_material.node_tree.nodes.new('ShaderNodeMixRGB')
red_mix.inputs[0].default_value = 0.06
red_mix.inputs[1].default_value = (0.72,0.003,0.009,1)
red_material.node_tree.links.new(red_texture.outputs['Color'], red_mix.inputs[2])
red_material.node_tree.links.new(red_mix.outputs['Color'], red_shader.inputs['Base Color'])
gold_material = bpy.data.materials['ReferenceChampagneGoldWheelMetal']
gold_shader = next(node for node in gold_material.node_tree.nodes if node.type == 'BSDF_PRINCIPLED')
gold_texture = next(node for node in gold_material.node_tree.nodes if node.type == 'TEX_IMAGE')
gold_mix = gold_material.node_tree.nodes.new('ShaderNodeMixRGB')
gold_mix.inputs[0].default_value = 0.08
gold_mix.inputs[1].default_value = (0.58,0.36,0.09,1)
gold_material.node_tree.links.new(gold_texture.outputs['Color'], gold_mix.inputs[2])
gold_material.node_tree.links.new(gold_mix.outputs['Color'], gold_shader.inputs['Base Color'])
red_shader.inputs['Coat Weight'].default_value = 0.40
red_shader.inputs['Coat Roughness'].default_value = 0.16

scene = bpy.context.scene
scene.render.engine = 'CYCLES'
scene.cycles.samples = 16
scene.cycles.device = 'CPU'
scene.render.bake.margin = 12
scene.render.bake.use_clear = True
scene.render.bake.use_selected_to_active = False
scene.render.bake.normal_space = 'TANGENT'
families = (('red_lacquer', red_material, 2048), ('satin_carbon', bpy.data.materials['ReferenceSatinChassisCarbon'], 4096), ('champagne_gold', gold_material, 1024), ('polished_chrome', bpy.data.materials['PolishedChromeExhaustMetal'], 512))
if requested_family is not None:
    families = tuple(family for family in families if family[0] == requested_family)
for family_name, original_material, resolution in families:
    print('BAKE_START=' + family_name, flush=True)
    vertices = []
    faces = []
    source_loops = []
    corner_normals = []
    source_objects = []
    for scene_object in list(bpy.data.objects):
        if scene_object.type != 'MESH' or original_material not in list(scene_object.data.materials):
            continue
        source_objects.append(scene_object.name)
        vertex_offset = len(vertices)
        vertices.extend(scene_object.matrix_world @ vertex.co for vertex in scene_object.data.vertices)
        normal_matrix = scene_object.matrix_world.to_3x3().inverted().transposed()
        for polygon in scene_object.data.polygons:
            if scene_object.data.materials[polygon.material_index] != original_material:
                continue
            faces.append(tuple(vertex_offset + index for index in polygon.vertices))
            for loop_index in polygon.loop_indices:
                source_loops.append((scene_object, loop_index))
                corner_normals.append((normal_matrix @ scene_object.data.corner_normals[loop_index].vector).normalized())
    mesh = bpy.data.meshes.new('PhysicalMaterialBakeSurface')
    mesh.from_pydata(vertices, [], faces)
    mesh.materials.append(original_material)
    for polygon in mesh.polygons:
        polygon.use_smooth = True
    mesh.normals_split_custom_set(corner_normals)
    temporary_object = bpy.data.objects.new('PhysicalMaterialBakeSurface', mesh)
    scene.collection.objects.link(temporary_object)
    original_coordinate_name = 'CarbonSurfaceCoordinates' if family_name == 'satin_carbon' else 'SourcePhysicalMaterialCoordinates'
    source_coordinates = mesh.uv_layers.new(name=original_coordinate_name)
    for index, (scene_object, loop_index) in enumerate(source_loops):
        source_layer = scene_object.data.uv_layers.get('CarbonSurfaceCoordinates') if family_name == 'satin_carbon' else scene_object.data.uv_layers.active
        source_coordinates.data[index].uv = source_layer.data[loop_index].uv if source_layer else (0,0)
    for node in original_material.node_tree.nodes:
        if node.type == 'TEX_IMAGE' and not node.inputs['Vector'].is_linked:
            source_node = original_material.node_tree.nodes.new('ShaderNodeUVMap')
            source_node.uv_map = original_coordinate_name
            original_material.node_tree.links.new(source_node.outputs['UV'], node.inputs['Vector'])
    bake_coordinates = mesh.uv_layers.new(name='BakedPhysicalMaterialCoordinates')
    mesh.uv_layers.active = bake_coordinates
    bake_coordinates.active_render = True
    bpy.ops.object.select_all(action='DESELECT')
    temporary_object.select_set(True)
    bpy.context.view_layer.objects.active = temporary_object
    bpy.ops.object.mode_set(mode='EDIT')
    bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.uv.smart_project(angle_limit=math.radians(66), island_margin=0.007, area_weight=0, correct_aspect=True, scale_to_bounds=True)
    bpy.ops.object.mode_set(mode='OBJECT')
    bake_coordinates = mesh.uv_layers['BakedPhysicalMaterialCoordinates']
    print('COORDINATE_STATE=' + str([(layer.name, layer.active_render) for layer in mesh.uv_layers]), flush=True)
    for index, (scene_object, loop_index) in enumerate(source_loops):
        destination = scene_object.data.uv_layers.get('BakedPhysicalMaterialCoordinates') or scene_object.data.uv_layers.new(name='BakedPhysicalMaterialCoordinates')
        destination.data[loop_index].uv = bake_coordinates.data[index].uv
    for scene_object_name in source_objects:
        bpy.data.objects[scene_object_name].data.update()
    assert all((scene_object.data.uv_layers['BakedPhysicalMaterialCoordinates'].data[loop_index].uv - bake_coordinates.data[index].uv).length < 1e-7 for index, (scene_object,loop_index) in enumerate(source_loops))
    shader = next(node for node in original_material.node_tree.nodes if node.type == 'BSDF_PRINCIPLED')
    output_node = next(node for node in original_material.node_tree.nodes if node.type == 'OUTPUT_MATERIAL')
    emission = original_material.node_tree.nodes.new('ShaderNodeEmission')
    destination_node = original_material.node_tree.nodes.new('ShaderNodeTexImage')
    original_material.node_tree.nodes.active = destination_node
    images = {}
    def bake_output(output_name, bake_type, color_value=None, color_socket=None):
        image = bpy.data.images.new(family_name + '_' + output_name, width=resolution, height=resolution, alpha=False)
        image.colorspace_settings.name = 'sRGB' if output_name == 'base_color' else 'Non-Color'
        image.generated_color = (0.5,0.5,1,1) if output_name == 'normal' else (0,0,0,1)
        destination_node.image = image
        for link in list(emission.inputs['Color'].links):
            original_material.node_tree.links.remove(link)
        if color_socket is not None:
            original_material.node_tree.links.new(color_socket, emission.inputs['Color'])
        elif color_value is not None:
            emission.inputs['Color'].default_value = color_value
        original_material.node_tree.links.new(shader.outputs['BSDF'] if bake_type == 'NORMAL' else emission.outputs['Emission'], output_node.inputs['Surface'])
        bpy.ops.object.bake(type=bake_type, uv_layer='BakedPhysicalMaterialCoordinates')
        images[output_name] = image
        print('BAKE_MAP=' + family_name + '_' + output_name, flush=True)
    base_input = shader.inputs['Base Color']
    bake_output('base_color', 'EMIT', tuple(base_input.default_value), base_input.links[0].from_socket if base_input.is_linked else None)
    bake_output('normal', 'NORMAL')
    ambient_occlusion = original_material.node_tree.nodes.new('ShaderNodeAmbientOcclusion')
    ambient_occlusion.only_local = True
    ambient_occlusion.inputs['Distance'].default_value = 0.12
    combined = original_material.node_tree.nodes.new('ShaderNodeCombineColor')
    original_material.node_tree.links.new(ambient_occlusion.outputs['AO'], combined.inputs['Red'])
    combined.inputs['Green'].default_value = shader.inputs['Roughness'].default_value
    combined.inputs['Blue'].default_value = shader.inputs['Metallic'].default_value
    if shader.inputs['Roughness'].is_linked:
        original_material.node_tree.links.new(shader.inputs['Roughness'].links[0].from_socket, combined.inputs['Green'])
    bake_output('occlusion_roughness_metallic', 'EMIT', color_socket=combined.outputs['Color'])
    original_material.node_tree.links.new(shader.outputs['BSDF'], output_node.inputs['Surface'])
    baked_material = bpy.data.materials.new('BakedPhysical' + ''.join(word.title() for word in family_name.split('_')))
    baked_material.use_nodes = True
    baked_shader = baked_material.node_tree.nodes.get('Principled BSDF')
    for input_name in ('Metallic','Roughness','IOR','Coat Weight','Coat Roughness'):
        baked_shader.inputs[input_name].default_value = shader.inputs[input_name].default_value
    texture_coordinate_node = baked_material.node_tree.nodes.new('ShaderNodeUVMap')
    texture_coordinate_node.uv_map = 'BakedPhysicalMaterialCoordinates'
    texture_nodes = {}
    for output_name, image in images.items():
        destination_path = validate_output_path(project_directory, texture_directory / (family_name + '_' + output_name + '.png'), 'promote').path
        image.filepath_raw = str(destination_path)
        image.file_format = 'PNG'
        image.save()
        texture_node = baked_material.node_tree.nodes.new('ShaderNodeTexImage')
        texture_node.image = image
        baked_material.node_tree.links.new(texture_coordinate_node.outputs['UV'], texture_node.inputs['Vector'])
        texture_nodes[output_name] = texture_node
        image.filepath = str(destination_path)
    baked_material.node_tree.links.new(texture_nodes['base_color'].outputs['Color'], baked_shader.inputs['Base Color'])
    normal_node = baked_material.node_tree.nodes.new('ShaderNodeNormalMap')
    normal_node.uv_map = 'BakedPhysicalMaterialCoordinates'
    baked_material.node_tree.links.new(texture_nodes['normal'].outputs['Color'], normal_node.inputs['Color'])
    baked_material.node_tree.links.new(normal_node.outputs['Normal'], baked_shader.inputs['Normal'])
    separation = baked_material.node_tree.nodes.new('ShaderNodeSeparateColor')
    baked_material.node_tree.links.new(texture_nodes['occlusion_roughness_metallic'].outputs['Color'], separation.inputs['Color'])
    baked_material.node_tree.links.new(separation.outputs['Green'], baked_shader.inputs['Roughness'])
    baked_material.node_tree.links.new(separation.outputs['Blue'], baked_shader.inputs['Metallic'])
    settings = bpy.data.node_groups.get('glTF Material Output')
    if settings is None:
        settings = bpy.data.node_groups.new('glTF Material Output','ShaderNodeTree')
        settings.interface.new_socket(name='Occlusion',in_out='INPUT',socket_type='NodeSocketFloat')
    settings_node = baked_material.node_tree.nodes.new('ShaderNodeGroup')
    settings_node.node_tree = settings
    baked_material.node_tree.links.new(separation.outputs['Red'], settings_node.inputs['Occlusion'])
    for scene_object_name in source_objects:
        scene_object = bpy.data.objects[scene_object_name]
        for material_index, candidate in enumerate(scene_object.data.materials):
            if candidate == original_material:
                scene_object.data.materials[material_index] = baked_material
    for node in (emission,destination_node,ambient_occlusion,combined):
        original_material.node_tree.nodes.remove(node)
    bpy.data.objects.remove(temporary_object, do_unlink=True)
    bpy.data.meshes.remove(mesh)
    report['groups'].append({'family': family_name, 'material': baked_material.name, 'objects': source_objects, 'resolution': resolution, 'maps': {name: {'path': str(texture_directory / (family_name + '_' + name + '.png')), 'sha256': hashlib.sha256((texture_directory / (family_name + '_' + name + '.png')).read_bytes()).hexdigest()} for name in images}})
    original_material.use_fake_user = True

for scene_object in bpy.data.objects:
    if scene_object.type == 'MESH' and scene_object.name not in report['adjusted_decals']:
        assert geometry_digest(scene_object) == original_geometry[scene_object.name], scene_object.name
report['protected_geometry'] = {name: digest for name,digest in original_geometry.items() if name not in report['adjusted_decals']}
report['adjusted_geometry'] = {name: geometry_digest(bpy.data.objects[name]) for name in report['adjusted_decals']}
report['generated_albedo_prompt'] = 'Uniform Ferrari racing red, seamless unlit albedo, extremely subtle pigment grain, no reflections, shadows, gradients or decals.'
report['ambient_occlusion'] = {'only_local': True,'distance_meters': 0.12,'moving_part_shadows_baked': False}
report['decals_preserved_as_editable_layers'] = True
report['source_after'] = str(source_path)
if requested_family is not None and previous_manifest is not None:
    report['groups'] = [group for group in previous_manifest['groups'] if group['family'] != requested_family]+report['groups']
    report['material_family_rebaked'] = requested_family
    report['previous_manifest_sha256'] = hashlib.sha256(previous_manifest_path.read_bytes()).hexdigest()
instructions = validate_output_path(project_directory, texture_directory / 'AGENTS.md', 'promote').path
instructions.write_text('# baked_physical_materials\n\nScope: reproducible vehicle PBR atlases and generated albedo reference. Consumers: canonical f1_2030.blend and its GLB exports. Base color uses sRGB; normal and packed occlusion/roughness/metallic use Non-Color. Packed channels: red occlusion, green roughness, blue metallic. Preserve source material node trees and original UV layers. Historical decal geometry is a projection input; the canonical chassis and tire printing use flat color textures under ../reference_balanced_finish/.\n',encoding='utf-8')
manifest_path = validate_output_path(project_directory, texture_directory / 'baking_manifest.json', 'promote').path
manifest_path.write_text(json.dumps(report,indent=2),encoding='utf-8')
bpy.context.preferences.filepaths.save_version = 0
for group in report['groups']:
    for output_name, record in group['maps'].items():
        image = bpy.data.images[group['family'] + '_' + output_name]
        image.filepath = bpy.path.relpath(record['path'], start=source_path.parent)
bpy.ops.wm.save_as_mainfile(filepath=str(validate_output_path(project_directory,source_path,'promote').path), relative_remap=False)
print('PHYSICAL_MATERIAL_BAKING_COMPLETE',flush=True)
