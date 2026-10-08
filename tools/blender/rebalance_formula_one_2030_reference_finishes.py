import argparse
import hashlib
import json
import math
import subprocess
import sys
from pathlib import Path

import bpy
import numpy
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools'))
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools/blender'))
from common.output_policy import validate_output_path
from convert_formula_one_2030_chassis_decals_to_textures import bake_flat_decal_colors, decode_color_channel_values, encode_color_channel_values, mesh_shading_fingerprint
from export_f1_2030_v10 import export_chassis, export_wheel, bbox_center_world
from fit_f1_2030_driver_seat import mesh_fingerprint


def image_pixels(image):
    values = numpy.empty(image.size[0] * image.size[1] * 4, dtype=numpy.float32)
    image.pixels.foreach_get(values)
    return values.reshape((image.size[1], image.size[0], 4))


def save_texture(name, pixels, destination, color_space):
    image = bpy.data.images.new(name, width=pixels.shape[1], height=pixels.shape[0], alpha=False)
    image.colorspace_settings.name = color_space
    image.pixels.foreach_set(pixels.ravel())
    path = validate_output_path(PROJECT_DIRECTORY, destination / (name + '.png'), 'preview').path
    image.filepath_raw = str(path)
    image.file_format = 'PNG'
    image.save()
    return image


def sample_texture(pixels, coordinates):
    height, width = pixels.shape[:2]
    horizontal = numpy.clip(coordinates[..., 0] * width - 0.5, 0, width - 1.00001)
    vertical = numpy.clip(coordinates[..., 1] * height - 0.5, 0, height - 1.00001)
    first_horizontal, first_vertical = horizontal.astype(numpy.int32), vertical.astype(numpy.int32)
    horizontal_fraction, vertical_fraction = (horizontal - first_horizontal)[..., None], (vertical - first_vertical)[..., None]
    return (pixels[first_vertical, first_horizontal] * (1 - horizontal_fraction) + pixels[first_vertical, first_horizontal + 1] * horizontal_fraction) * (1 - vertical_fraction) + (pixels[first_vertical + 1, first_horizontal] * (1 - horizontal_fraction) + pixels[first_vertical + 1, first_horizontal + 1] * horizontal_fraction) * vertical_fraction


def structural_fingerprint(scene_object):
    mesh = scene_object.data
    record = {'matrix': [list(row) for row in scene_object.matrix_world], 'vertices': [list(vertex.co) for vertex in mesh.vertices], 'edges': [list(edge.vertices) for edge in mesh.edges], 'faces': [list(polygon.vertices) for polygon in mesh.polygons], 'shading': mesh_shading_fingerprint(scene_object)}
    return hashlib.sha256(json.dumps(record, separators=(',', ':')).encode()).hexdigest()


def clean_chassis_material(source_material, family, destination):
    material = source_material.copy()
    material.name = 'FormulaOne2030RedPaint' if family == 'red_lacquer' else 'FormulaOne2030CarbonSurface'
    shader = next(node for node in material.node_tree.nodes if node.type == 'BSDF_PRINCIPLED')
    directory = PROJECT_DIRECTORY / 'game/assets/models/vehicles/f1-2030/source/textures/baked_physical_materials'
    original = bpy.data.images.load(str(directory / (family + '_base_color.png')), check_existing=False)
    original.scale(4096, 4096)
    pixels = image_pixels(original)
    if family == 'red_lacquer':
        pixels[..., :3] = numpy.clip(pixels[..., :3] * numpy.array([0.94, 0.7, 0.85]), 0, 1)
    else:
        vertical, horizontal = numpy.indices(pixels.shape[:2], dtype=numpy.float32)
        crossing = (numpy.floor(horizontal / 4) - numpy.floor(vertical / 4)) % 4 < 2
        fiber_coordinate = numpy.where(crossing, horizontal, vertical)
        fiber = numpy.sin(fiber_coordinate * math.pi / 4) ** 2
        weave = 0.075 + 0.048 * fiber + 0.012 * crossing
        pixels[..., :3] = weave[..., None] * numpy.array([0.98, 1, 1.03])
    pixels[..., 3] = 1
    color_image = save_texture(material.name + 'CleanColor', pixels, destination, 'sRGB')
    shader.inputs['Base Color'].links[0].from_node.image = color_image
    original_parameters = bpy.data.images.load(str(directory / (family + '_occlusion_roughness_metallic.png')), check_existing=False)
    parameters = image_pixels(original_parameters)
    if family == 'red_lacquer':
        parameters[..., 1] = numpy.where(parameters[..., 1] > 0, numpy.clip(parameters[..., 1] * 0.89, 0.24, 0.32), 0)
    else:
        vertical, horizontal = numpy.indices(parameters.shape[:2], dtype=numpy.float32)
        parameters[..., 1] = numpy.where(parameters[..., 1] > 0, 0.36 + 0.025 * numpy.sin((horizontal + vertical) * math.pi / 8) ** 2, 0)
    parameter_image = save_texture(material.name + 'MaterialParameters', parameters, destination, 'Non-Color')
    for node in material.node_tree.nodes:
        if node.type == 'TEX_IMAGE' and node.image and 'occlusion_roughness_metallic' in node.image.name:
            node.image = parameter_image
        if node.type == 'NORMAL_MAP':
            node.inputs['Strength'].default_value = 0.65 if family == 'red_lacquer' else 0.55
    shader.inputs['Coat Weight'].default_value = 0.28 if family == 'red_lacquer' else 0.16
    shader.inputs['Coat Roughness'].default_value = 0.18 if family == 'red_lacquer' else 0.26
    for scene_object in bpy.data.objects:
        if scene_object.type == 'MESH' and scene_object.name.startswith('GEO_CHASSIS_'):
            for index, current_material in enumerate(scene_object.data.materials):
                if current_material == source_material:
                    scene_object.data.materials[index] = material
    return material


def receiver_surface(names):
    positions, faces = [], []
    for name in names:
        scene_object = bpy.data.objects[name]
        mesh = scene_object.data
        mesh.calc_loop_triangles()
        offset = len(positions)
        positions.extend(scene_object.matrix_world @ vertex.co for vertex in mesh.vertices)
        faces.extend(tuple(offset + index for index in triangle.vertices) for triangle in mesh.loop_triangles)
    return BVHTree.FromPolygons(positions, faces, all_triangles=True)


def decal_adjustment(name):
    adjustments = {
        'HewlettPackard0': (-0.03, 0.012, 0.94),
        'DriverNumber1': (-0.025, 0.018, 0.95),
        'CevaLogistics4': (-0.025, -0.015, 0.96),
        'Shell3': (-0.015, 0.0, 0.96),
        'FerrariShield5': (-0.015, 0.006, 1.0),
        'RichardMille6': (-0.005, 0.006, 0.99),
        'InternationalBusinessMachines7': (0.0, 0.008, 0.98),
        'UniCredit2': (-0.02, 0.005, 0.98),
    }
    return next((values for suffix, values in adjustments.items() if name.endswith(suffix)), (0, 0, 1))


def create_tire_textures(atlas, lettering, destination):
    resolution = 2048
    vertical, horizontal = numpy.indices((resolution, resolution), dtype=numpy.float32)
    horizontal = (horizontal + 0.5) / resolution
    vertical = (vertical + 0.5) / resolution
    pixels = numpy.ones((resolution, resolution, 4), dtype=numpy.float32)
    grain = numpy.sin(horizontal * 941 + vertical * 577) * numpy.sin(horizontal * 1907 - vertical * 1321)
    rubber = 0.092 + grain * 0.006
    pixels[..., :3] = rubber[..., None] * numpy.array([0.96, 0.99, 1])
    for center_horizontal in (0.25, 0.75):
        normalized_horizontal = (horizontal - center_horizontal) / 0.235
        normalized_vertical = (vertical - 0.27) / 0.235
        radius = numpy.sqrt(normalized_horizontal ** 2 + normalized_vertical ** 2)
        band = numpy.clip((radius - 0.882) / 0.006, 0, 1) * numpy.clip((0.926 - radius) / 0.006, 0, 1)
        pixels[..., :3] = pixels[..., :3] * (1 - band[..., None]) + numpy.array([0.79, 0.79, 0.77]) * band[..., None]
        for name, center_vertical, lettering_width, lettering_height in [('Pirelli', 0.745, 0.55, 0.083), ('PZero', -0.745, 0.62, 0.09)]:
            minimum, maximum = lettering[name]
            coordinate_horizontal = normalized_horizontal / lettering_width + 0.5
            coordinate_vertical = (normalized_vertical - center_vertical) / lettering_height + 0.5
            inside = (coordinate_horizontal >= 0) & (coordinate_horizontal <= 1) & (coordinate_vertical >= 0) & (coordinate_vertical <= 1)
            coordinates = numpy.stack([minimum[0] + coordinate_horizontal[inside] * (maximum[0] - minimum[0]), minimum[1] + coordinate_vertical[inside] * (maximum[1] - minimum[1])], axis=-1)
            colors = sample_texture(atlas, coordinates)
            coverage = numpy.clip(colors[:, 3:4], 0, 1)
            existing = decode_color_channel_values(pixels[inside, :3])
            printed = decode_color_channel_values(colors[:, :3]) * 0.84
            pixels[inside, :3] = encode_color_channel_values(existing * (1 - coverage) + printed * coverage)
    tread = vertical >= 0.57
    abrasion = 0.011 * numpy.sin(horizontal * 31 + vertical * 9) ** 2 + grain * 0.005
    pixels[tread, :3] = (0.086 + abrasion[tread])[..., None] * numpy.array([0.96, 0.99, 1])
    color = save_texture('FormulaOne2030TireColor', pixels, destination, 'sRGB')
    parameters = numpy.ones((1024, 1024, 4), dtype=numpy.float32)
    vertical, horizontal = numpy.indices((1024, 1024), dtype=numpy.float32)
    parameters[..., 0] = 1
    parameters[..., 1] = numpy.where((vertical + 0.5) / 1024 >= 0.57, 0.79, 0.68) + 0.015 * numpy.sin(horizontal * 0.031) ** 2
    parameters[..., 2] = 0
    return color, save_texture('FormulaOne2030TireParameters', parameters, destination, 'Non-Color')


def apply_tire_coordinates(mesh, transformation, center, radius, half_width):
    coordinates = mesh.uv_layers.get('BakedPhysicalMaterialCoordinates') or mesh.uv_layers.new(name='BakedPhysicalMaterialCoordinates')
    coordinates.active_render = True
    mesh.uv_layers.active_index = list(mesh.uv_layers).index(coordinates)
    normal_transformation = transformation.to_3x3().inverted().transposed()
    for polygon in mesh.polygons:
        normal = (normal_transformation @ polygon.normal).normalized()
        positions = [transformation @ mesh.vertices[mesh.loops[index].vertex_index].co - center for index in polygon.loop_indices]
        if abs(normal.y) > 0.42:
            side = 1 if sum(position.y for position in positions) > 0 else -1
            center_horizontal = 0.75 if side > 0 else 0.25
            for index, position in zip(polygon.loop_indices, positions):
                coordinates.data[index].uv = (center_horizontal - side * position.x / radius * 0.235, 0.27 + position.z / radius * 0.235)
        else:
            angles = [math.atan2(position.z, position.x) / (2 * math.pi) + 0.5 for position in positions]
            if max(angles) - min(angles) > 0.5:
                angles = [value + 1 if value < 0.5 else value for value in angles]
            for index, position, angle in zip(polygon.loop_indices, positions, angles):
                coordinates.data[index].uv = (angle, 0.6 + 0.37 * (position.y / half_width + 1) * 0.5)


def preserve_runtime_wheel_surfaces(scene_object, position, original_wheel_path, material, tire_radius, tire_width):
    existing_objects = set(bpy.data.objects)
    existing_materials = set(bpy.data.materials)
    renamed_materials = {current_material: current_material.name for current_material in bpy.data.materials}
    for current_material, name in renamed_materials.items():
        current_material.name = 'AuthoringMaterialPreserved_' + name
    bpy.ops.import_scene.gltf(filepath=str(original_wheel_path))
    imported_objects = set(bpy.data.objects) - existing_objects
    center = bbox_center_world(scene_object)
    corners = [scene_object.matrix_world @ Vector(corner) for corner in scene_object.bound_box]
    extents = Vector(tuple(max(corner[axis] for corner in corners) - min(corner[axis] for corner in corners) for axis in range(3)))
    radial_scale = tire_radius * 2 / max(extents.x, extents.z)
    width_scale = tire_width / extents.y
    wheel_conversion = Matrix(((0, 1, 0, 0), (-1, 0, 0, 0), (0, 0, 1, 0), (0, 0, 0, 1)))
    export_transformation = Matrix.Diagonal((width_scale, radial_scale, radial_scale, 1)) @ wheel_conversion @ Matrix.Translation(-center) @ scene_object.matrix_world
    records = {}
    prefix = 'GEO_WHEEL_' + position + '_'
    authored_objects = [current_object for current_object in existing_objects if current_object.type == 'MESH' and current_object.name.startswith(prefix) and '_DECAL_' not in current_object.name]
    for authored_object in authored_objects:
        runtime_object = next(current_object for current_object in imported_objects if current_object.type == 'MESH' and current_object.name.startswith(authored_object.name))
        authored_export = Matrix.Diagonal((width_scale, radial_scale, radial_scale, 1)) @ wheel_conversion @ Matrix.Translation(-center) @ authored_object.matrix_world
        to_authoring = authored_export.inverted() @ runtime_object.matrix_world
        surface = runtime_object.data.copy()
        corner_normals = [normal.vector.copy() for normal in surface.corner_normals]
        surface.transform(to_authoring)
        normal_transformation = to_authoring.to_3x3().inverted().transposed()
        surface.normals_split_custom_set([(normal_transformation @ normal).normalized() for normal in corner_normals])
        previous_surface = bpy.data.meshes.get(authored_object.get('formula_one_2030_runtime_surface_mesh', ''))
        if previous_surface:
            previous_surface.use_fake_user = False
            if previous_surface.users == 0:
                bpy.data.meshes.remove(previous_surface)
        suffix = authored_object.name[len(prefix):].title().replace('_', '')
        surface.name = 'FormulaOne2030RuntimeWheelSurface' + position.title().replace('_', '') + suffix
        surface.use_fake_user = True
        if authored_object == scene_object:
            surface.materials.clear()
            surface.materials.append(material)
            apply_tire_coordinates(surface, authored_object.matrix_world, center, max(extents.x, extents.z) * 0.5, extents.y * 0.5)
        authored_object['formula_one_2030_runtime_surface_mesh'] = surface.name
        surface.calc_loop_triangles()
        records[authored_object.name] = {'source_vertices': len(authored_object.data.vertices), 'runtime_vertices': len(surface.vertices), 'runtime_triangles': len(surface.loop_triangles), 'embedded_surface': surface.name, 'baseline_wheel_sha256': hashlib.sha256(original_wheel_path.read_bytes()).hexdigest()}
    for imported_object in imported_objects:
        bpy.data.objects.remove(imported_object, do_unlink=True)
    for imported_material in set(bpy.data.materials) - existing_materials:
        if imported_material.users == 0:
            bpy.data.materials.remove(imported_material)
        else:
            imported_material.name = 'FormulaOne2030PreservedWheel' + imported_material.name
    for current_material, name in renamed_materials.items():
        current_material.name = name
    return records


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--decal-baseline', type=Path, required=True)
    parser.add_argument('--wheel-baseline-directory', type=Path, required=True)
    parser.add_argument('--projection-receivers', type=Path, default=PROJECT_DIRECTORY / 'game/assets/models/vehicles/f1-2030/source/reference_finish_projection_receivers.json')
    parser.add_argument('--output-directory', required=True)
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    destination = validate_output_path(PROJECT_DIRECTORY, options.output_directory, 'preview').path
    destination.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=str(options.decal_baseline.resolve()))
    decal_records = []
    sponsor_material = bpy.data.materials['ReferenceSponsorPrintedDecals.001']
    sponsor_shader = next(node for node in sponsor_material.node_tree.nodes if node.type == 'BSDF_PRINCIPLED')
    sponsor_atlas = image_pixels(sponsor_shader.inputs['Base Color'].links[0].from_node.image)
    lettering = {}
    for label in ('Pirelli', 'PZero'):
        scene_object = bpy.data.objects['GEO_WHEEL_FRONT_L_DECAL_' + label]
        coordinates = numpy.array([tuple(corner.uv) for corner in scene_object.data.uv_layers.active.data])
        lettering[label] = coordinates.min(axis=0), coordinates.max(axis=0)
    for scene_object in bpy.data.objects:
        if scene_object.type == 'MESH' and scene_object.name.startswith('GEO_CHASSIS_DECAL_'):
            material = scene_object.active_material
            shader = next(node for node in material.node_tree.nodes if node.type == 'BSDF_PRINCIPLED')
            decal_records.append({'name': scene_object.name, 'positions': [list(scene_object.matrix_world @ vertex.co) for vertex in scene_object.data.vertices], 'faces': [list(polygon.vertices) for polygon in scene_object.data.polygons], 'coordinates': [list(corner.uv) for corner in scene_object.data.uv_layers.active.data], 'color': list(shader.inputs['Base Color'].default_value), 'printed': shader.inputs['Base Color'].is_linked})
    bpy.ops.wm.open_mainfile(filepath=str(options.source_file.resolve()))
    if 'FormulaOne2030RedPaintWithFlatChassisDecals' in bpy.data.materials:
        raise RuntimeError('Reference finish rebalance is already applied')
    bpy.context.preferences.filepaths.save_version = 0
    source_meshes = [scene_object for scene_object in bpy.data.objects if scene_object.type == 'MESH' and '_DECAL_' not in scene_object.name]
    report = {'source_before_sha256': hashlib.sha256(options.source_file.read_bytes()).hexdigest(), 'source_branch': subprocess.check_output(['git', 'branch', '--show-current'], cwd=PROJECT_DIRECTORY, text=True).strip(), 'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=PROJECT_DIRECTORY, text=True).strip(), 'structural_fingerprints_before': {scene_object.name: structural_fingerprint(scene_object) for scene_object in source_meshes}, 'mesh_fingerprints_before': {scene_object.name: mesh_fingerprint(scene_object) for scene_object in source_meshes}, 'texture_groups': [], 'decal_placements': [], 'runtime_tire_surfaces': {}}
    backup = validate_output_path(PROJECT_DIRECTORY, destination / 'source_before_reference_finish.blend', 'preview').path
    bpy.ops.wm.save_as_mainfile(filepath=str(backup), relative_remap=True)
    report['texture_resolved_source_backup'] = str(backup)
    paint = clean_chassis_material(bpy.data.materials['BakedPhysicalRedLacquerWithFlatChassisDecals'], 'red_lacquer', destination)
    carbon = clean_chassis_material(bpy.data.materials['BakedPhysicalSatinCarbonWithFlatChassisDecals'], 'satin_carbon', destination)
    receiver_names = json.loads(options.projection_receivers.read_text())['receivers_by_design']
    if set(receiver_names) != {record['name'] for record in decal_records}:
        raise RuntimeError('Projection receiver layout does not match the source sponsor designs')
    original_scene = bpy.context.scene
    transfer_scene = bpy.data.scenes.new('ReferenceFinishTextureTransfer')
    bpy.context.window.scene = transfer_scene
    transfer_scene.render.engine = 'CYCLES'
    transfer_scene.cycles.samples = 1
    device_preferences = bpy.context.preferences.addons['cycles'].preferences
    device_preferences.compute_device_type = 'OPTIX'
    device_preferences.get_devices()
    for device in device_preferences.devices:
        device.use = device.type == 'OPTIX'
    transfer_scene.cycles.device = 'GPU'
    atlas_image = bpy.data.images.new('ReferenceSponsorSourceColors', width=sponsor_atlas.shape[1], height=sponsor_atlas.shape[0], alpha=True)
    atlas_image.colorspace_settings.name = 'sRGB'
    atlas_image.pixels.foreach_set(sponsor_atlas.ravel())
    temporary_objects, temporary_materials = [], []
    for record in decal_records:
        vertices = numpy.array(record['positions'])
        center = (vertices.min(axis=0) + vertices.max(axis=0)) * 0.5
        horizontal_offset, vertical_offset, scale = decal_adjustment(record['name'])
        vertices = (vertices - center) * scale + center + numpy.array([horizontal_offset, 0, vertical_offset])
        surface = receiver_surface(receiver_names[record['name']])
        relocated_vertices = []
        maximum_projection_distance = 0
        for position in vertices:
            if horizontal_offset == 0 and vertical_offset == 0 and scale == 1:
                relocated_vertices.append(Vector(position))
                continue
            nearest, normal, triangle_index, distance = surface.find_nearest(Vector(position), 0.08)
            if nearest is None:
                relocated_vertices.append(Vector(position))
                continue
            maximum_projection_distance = max(maximum_projection_distance, distance)
            relocated_vertices.append(nearest + normal * 0.0015)
        mesh = bpy.data.meshes.new('TemporaryProjectionSurface_' + record['name'])
        mesh.from_pydata(relocated_vertices, [], record['faces'])
        coordinates = mesh.uv_layers.new(name='PrintedDesignCoordinates')
        for index, coordinate in enumerate(record['coordinates']):
            coordinates.data[index].uv = coordinate
        material = bpy.data.materials.new('TemporaryPrintedDesign_' + record['name'])
        material.use_nodes = True
        shader = material.node_tree.nodes.get('Principled BSDF')
        shader.inputs['Base Color'].default_value = record['color']
        if record['printed']:
            texture = material.node_tree.nodes.new('ShaderNodeTexImage')
            texture.image = atlas_image
            material.node_tree.links.new(texture.outputs['Color'], shader.inputs['Base Color'])
            material.node_tree.links.new(texture.outputs['Alpha'], shader.inputs['Alpha'])
        emission = material.node_tree.nodes.new('ShaderNodeEmission')
        emission.name = 'FlatDecalTransferEmission'
        output = material.node_tree.nodes.get('Material Output')
        material.node_tree.links.new(emission.outputs['Emission'], output.inputs['Surface'])
        mesh.materials.append(material)
        scene_object = bpy.data.objects.new(record['name'], mesh)
        transfer_scene.collection.objects.link(scene_object)
        temporary_objects.append(scene_object)
        temporary_materials.append(material)
        report['decal_placements'].append({'name': record['name'], 'horizontal_offset_meters': horizontal_offset, 'vertical_offset_meters': vertical_offset, 'scale': scale, 'source_baseline': str(options.decal_baseline.resolve()), 'receivers': receiver_names[record['name']], 'maximum_projection_distance_meters': maximum_projection_distance, 'relocated_vertices': [list(position) for position in relocated_vertices], 'faces': record['faces'], 'coordinates': record['coordinates'], 'printed': record['printed'], 'constant_color': record['color']})
    for material in (paint, carbon):
        report['texture_groups'].append(bake_flat_decal_colors(transfer_scene, temporary_objects, material, destination, 4096))
    bpy.context.window.scene = original_scene
    for scene_object in temporary_objects:
        mesh = scene_object.data
        bpy.data.objects.remove(scene_object, do_unlink=True)
        bpy.data.meshes.remove(mesh)
    bpy.data.scenes.remove(transfer_scene)
    for material in temporary_materials:
        bpy.data.materials.remove(material)
    bpy.data.images.remove(atlas_image)
    tire_color, tire_parameters = create_tire_textures(sponsor_atlas, lettering, destination)
    tire_material = bpy.data.materials.new('FormulaOne2030TireSurface')
    tire_material.use_nodes = True
    shader = tire_material.node_tree.nodes.get('Principled BSDF')
    coordinates = tire_material.node_tree.nodes.new('ShaderNodeUVMap')
    coordinates.uv_map = 'BakedPhysicalMaterialCoordinates'
    color_node = tire_material.node_tree.nodes.new('ShaderNodeTexImage')
    color_node.image = tire_color
    parameters_node = tire_material.node_tree.nodes.new('ShaderNodeTexImage')
    parameters_node.image = tire_parameters
    separator = tire_material.node_tree.nodes.new('ShaderNodeSeparateColor')
    tire_material.node_tree.links.new(coordinates.outputs['UV'], color_node.inputs['Vector'])
    tire_material.node_tree.links.new(coordinates.outputs['UV'], parameters_node.inputs['Vector'])
    tire_material.node_tree.links.new(color_node.outputs['Color'], shader.inputs['Base Color'])
    tire_material.node_tree.links.new(parameters_node.outputs['Color'], separator.inputs['Color'])
    tire_material.node_tree.links.new(separator.outputs['Green'], shader.inputs['Roughness'])
    shader.inputs['Metallic'].default_value = 0
    shader.inputs['Specular IOR Level'].default_value = 0.35
    wheel_specs = json.loads((PROJECT_DIRECTORY / 'game/data/vehicles/f1_2030/f1_2030_v10_geometric.json').read_text())['tires']
    for short_name, position in [('FL', 'FRONT_L'), ('FR', 'FRONT_R'), ('RL', 'REAR_L'), ('RR', 'REAR_R')]:
        scene_object = bpy.data.objects['GEO_WHEEL_' + position + '_TIRE']
        center = bbox_center_world(scene_object)
        world_positions = numpy.array([list(scene_object.matrix_world @ vertex.co) for vertex in scene_object.data.vertices])
        extents = world_positions.max(axis=0) - world_positions.min(axis=0)
        apply_tire_coordinates(scene_object.data, scene_object.matrix_world, center, max(extents[0], extents[2]) * 0.5, extents[1] * 0.5)
        scene_object.data.materials.clear()
        scene_object.data.materials.append(tire_material)
        specification = wheel_specs['front' if short_name.startswith('F') else 'rear']
        preserved_surfaces = preserve_runtime_wheel_surfaces(scene_object, position, options.wheel_baseline_directory / ('f1_2030_v10_wheel_' + short_name + '.glb'), tire_material, specification['radius'], specification['width'])
        report.setdefault('runtime_wheel_surfaces', {}).update(preserved_surfaces)
        report['runtime_tire_surfaces'][position] = preserved_surfaces[scene_object.name]
    wheel_decals = [scene_object for scene_object in bpy.data.objects if scene_object.type == 'MESH' and scene_object.name.startswith('GEO_WHEEL_') and '_DECAL_' in scene_object.name]
    report['removed_wheel_decal_meshes'] = [scene_object.name for scene_object in wheel_decals]
    report['removed_wheel_decal_triangles'] = 0
    for scene_object in wheel_decals:
        scene_object.data.calc_loop_triangles()
        report['removed_wheel_decal_triangles'] += len(scene_object.data.loop_triangles)
        mesh = scene_object.data
        bpy.data.objects.remove(scene_object, do_unlink=True)
        if mesh.users == 0:
            bpy.data.meshes.remove(mesh)
    for scene_object in source_meshes:
        if structural_fingerprint(scene_object) != report['structural_fingerprints_before'][scene_object.name]:
            raise RuntimeError('Reference finish changed structural geometry or shading: ' + scene_object.name)
    report['structural_geometry_and_shading_preserved'] = True
    report['mesh_fingerprints_after'] = {scene_object.name: mesh_fingerprint(scene_object) for scene_object in source_meshes}
    report['chassis_decal_triangles'] = 0
    report['tires'] = {'color_texture': str(Path(tire_color.filepath_raw)), 'parameters_texture': str(Path(tire_parameters.filepath_raw)), 'color_resolution': 2048, 'parameters_resolution': 1024, 'lettering_and_white_band_are_color_only': True, 'shared_material': tire_material.name, 'new_texture_coordinates_only_on_tires': True}
    candidate_path = validate_output_path(PROJECT_DIRECTORY, destination / 'f1_2030_rebalanced_finishes.blend', 'preview').path
    bpy.ops.wm.save_as_mainfile(filepath=str(candidate_path), relative_remap=True)
    manifest = json.loads((PROJECT_DIRECTORY / 'game/assets/models/vehicles/f1-2030/manifest.json').read_text())
    report['mesh_statistics'] = {'chassis': export_chassis(destination / 'f1_2030_v10_chassis.glb', manifest['authored_alignment']['chassis_vertical_offset'])}
    for short_name, position in [('FL', 'FRONT_L'), ('FR', 'FRONT_R'), ('RL', 'REAR_L'), ('RR', 'REAR_R')]:
        specification = wheel_specs['front' if short_name.startswith('F') else 'rear']
        report['mesh_statistics']['wheel_' + short_name] = export_wheel(position, destination / ('f1_2030_v10_wheel_' + short_name + '.glb'), specification['radius'], specification['width'])
    report['candidate_sha256'] = hashlib.sha256(candidate_path.read_bytes()).hexdigest()
    report['exports'] = {path.name: {'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'size_bytes': path.stat().st_size} for path in destination.glob('*.glb')}
    report['passed'] = True
    validate_output_path(PROJECT_DIRECTORY, destination / 'reference_finish_report.json', 'preview').path.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'passed': True, 'wheel_decal_triangles_removed': report['removed_wheel_decal_triangles'], 'embedded_runtime_tire_surfaces': report['runtime_tire_surfaces']}, indent=2), flush=True)


if __name__ == '__main__':
    main()
