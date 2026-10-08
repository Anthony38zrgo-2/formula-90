import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

import bpy
import numpy

PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools'))
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools/blender'))
from common.output_policy import validate_output_path
from fit_f1_2030_driver_seat import mesh_fingerprint
from shade_formula_one_2030_cockpit_surfaces import geometry_fingerprint
from export_f1_2030_v10 import export_chassis


def mesh_shading_fingerprint(scene_object):
    mesh = scene_object.data
    shading = {
        'sharp_edges': [edge.use_edge_sharp for edge in mesh.edges],
        'smooth_faces': [polygon.use_smooth for polygon in mesh.polygons],
        'corner_normals': [tuple(normal.vector) for normal in mesh.corner_normals],
    }
    return hashlib.sha256(json.dumps(shading, separators=(',', ':')).encode()).hexdigest()


def decode_color_channel_values(values):
    return numpy.where(values <= 0.04045, values / 12.92, numpy.power((values + 0.055) / 1.055, 2.4))


def encode_color_channel_values(values):
    return numpy.where(values <= 0.0031308, values * 12.92, 1.055 * numpy.power(numpy.maximum(values, 0), 1 / 2.4) - 0.055)


def bake_flat_decal_colors(scene, decal_objects, source_material, destination, resolution):
    original_shader = next(node for node in source_material.node_tree.nodes if node.type == 'BSDF_PRINCIPLED')
    original_texture = original_shader.inputs['Base Color'].links[0].from_node.image
    vertices = []
    faces = []
    coordinates = []
    corner_normals = []
    recipients = []
    for scene_object in bpy.data.objects:
        if scene_object.type != 'MESH' or not scene_object.name.startswith('GEO_CHASSIS_') or scene_object in decal_objects:
            continue
        if source_material not in list(scene_object.data.materials):
            continue
        recipients.append(scene_object)
        mesh = scene_object.data
        mesh.calc_loop_triangles()
        offset = len(vertices)
        vertices.extend(scene_object.matrix_world @ vertex.co for vertex in mesh.vertices)
        normal_transformation = scene_object.matrix_world.to_3x3().inverted().transposed()
        for triangle in mesh.loop_triangles:
            if mesh.materials[triangle.material_index] != source_material:
                continue
            faces.append(tuple(offset + index for index in triangle.vertices))
            coordinates.extend(mesh.uv_layers['BakedPhysicalMaterialCoordinates'].data[index].uv.copy() for index in triangle.loops)
            corner_normals.extend((normal_transformation @ mesh.corner_normals[index].vector).normalized() for index in triangle.loops)
    mesh = bpy.data.meshes.new('ExistingChassisTextureTransferSurface')
    mesh.from_pydata(vertices, [], faces)
    for polygon in mesh.polygons:
        polygon.use_smooth = True
    mesh.normals_split_custom_set(corner_normals)
    texture_coordinates = mesh.uv_layers.new(name='BakedPhysicalMaterialCoordinates')
    texture_coordinates.active_render = True
    for index, coordinate in enumerate(coordinates):
        texture_coordinates.data[index].uv = coordinate
    recipient_material = bpy.data.materials.new('TemporaryFlatDecalTextureRecipient')
    recipient_material.use_nodes = True
    destination_node = recipient_material.node_tree.nodes.new('ShaderNodeTexImage')
    recipient_material.node_tree.nodes.active = destination_node
    mesh.materials.append(recipient_material)
    recipient_object = bpy.data.objects.new('ExistingChassisTextureTransferSurface', mesh)
    scene.collection.objects.link(recipient_object)
    output_images = {}
    for output_name, color_space in [('color', 'sRGB'), ('coverage', 'Non-Color')]:
        for scene_object in decal_objects:
            material = scene_object.active_material
            emission = material.node_tree.nodes['FlatDecalTransferEmission']
            shader = next(node for node in material.node_tree.nodes if node.type == 'BSDF_PRINCIPLED')
            source_input = shader.inputs['Base Color' if output_name == 'color' else 'Alpha']
            for link in list(emission.inputs['Color'].links):
                material.node_tree.links.remove(link)
            if source_input.is_linked:
                material.node_tree.links.new(source_input.links[0].from_socket, emission.inputs['Color'])
            else:
                value = source_input.default_value
                emission.inputs['Color'].default_value = tuple(value) if output_name == 'color' else (value, value, value, 1)
        output_image = bpy.data.images.new(source_material.name + '_flat_decal_' + output_name, width=resolution, height=resolution, alpha=False)
        output_image.colorspace_settings.name = color_space
        destination_node.image = output_image
        bpy.ops.object.select_all(action='DESELECT')
        for scene_object in decal_objects:
            scene_object.select_set(True)
        recipient_object.select_set(True)
        bpy.context.view_layer.objects.active = recipient_object
        print('BAKING_FLAT_DECALS', source_material.name, output_name, resolution, flush=True)
        bpy.ops.object.bake(type='EMIT', use_selected_to_active=True, cage_extrusion=0.008, max_ray_distance=0.012, margin=8, use_clear=True)
        output_images[output_name] = output_image
    scaled_original = original_texture.copy()
    scaled_original.scale(resolution, resolution)
    original_pixels = numpy.empty(resolution * resolution * 4, dtype=numpy.float32)
    color_pixels = numpy.empty_like(original_pixels)
    coverage_pixels = numpy.empty_like(original_pixels)
    scaled_original.pixels.foreach_get(original_pixels)
    output_images['color'].pixels.foreach_get(color_pixels)
    output_images['coverage'].pixels.foreach_get(coverage_pixels)
    original_pixels = original_pixels.reshape((-1, 4))
    color_pixels = color_pixels.reshape((-1, 4))
    coverage = numpy.clip(coverage_pixels.reshape((-1, 4))[:, 0:1], 0, 1)
    original_pixels[:, :3] = encode_color_channel_values(decode_color_channel_values(original_pixels[:, :3]) * (1 - coverage) + decode_color_channel_values(color_pixels[:, :3]) * coverage)
    original_pixels[:, 3] = 1
    image = bpy.data.images.new(source_material.name + 'WithFlatChassisDecals', width=resolution, height=resolution, alpha=False)
    image.colorspace_settings.name = 'sRGB'
    image.pixels.foreach_set(original_pixels.ravel())
    image_path = validate_output_path(PROJECT_DIRECTORY, destination / (source_material.name + '_flat_chassis_decals.png'), 'preview').path
    image.filepath_raw = str(image_path)
    image.file_format = 'PNG'
    image.save()
    replacement_material = source_material.copy()
    replacement_material.name = source_material.name + 'WithFlatChassisDecals'
    replacement_shader = next(node for node in replacement_material.node_tree.nodes if node.type == 'BSDF_PRINCIPLED')
    replacement_shader.inputs['Base Color'].links[0].from_node.image = image
    for scene_object in recipients:
        for index, material in enumerate(scene_object.data.materials):
            if material == source_material:
                scene_object.data.materials[index] = replacement_material
    result = {'source_material':source_material.name, 'material':replacement_material.name, 'objects':[scene_object.name for scene_object in recipients], 'resolution':resolution, 'texture_path':str(image_path), 'texture_sha256':hashlib.sha256(image_path.read_bytes()).hexdigest(), 'covered_pixels':int(numpy.count_nonzero(coverage > 0.01)), 'coverage_fraction':float(numpy.mean(coverage)), 'normal_roughness_metallic_and_occlusion_maps_preserved':True}
    bpy.data.objects.remove(recipient_object, do_unlink=True)
    bpy.data.meshes.remove(mesh)
    bpy.data.materials.remove(recipient_material)
    bpy.data.images.remove(scaled_original)
    for output_image in output_images.values():
        bpy.data.images.remove(output_image)
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--output-directory', required=True)
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
    destination = validate_output_path(PROJECT_DIRECTORY, options.output_directory, 'preview').path
    destination.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=str(options.source_file.resolve()))
    decal_objects = [scene_object for scene_object in bpy.data.objects if scene_object.type == 'MESH' and scene_object.name.startswith('GEO_CHASSIS_DECAL_')]
    if not decal_objects:
        raise RuntimeError('Chassis decals are already textures or the source has no decal meshes')
    source_meshes = [scene_object for scene_object in bpy.data.objects if scene_object.type == 'MESH' and scene_object not in decal_objects]
    material_names = {scene_object.name:[material.name if material else None for material in scene_object.data.materials] for scene_object in source_meshes}
    protected_meshes = {scene_object.name:mesh_fingerprint(scene_object) for scene_object in source_meshes}
    geometry_fingerprints = {scene_object.name:geometry_fingerprint(scene_object) for scene_object in source_meshes}
    shading_fingerprints = {scene_object.name:mesh_shading_fingerprint(scene_object) for scene_object in source_meshes}
    report = {'source_before_sha256':hashlib.sha256(options.source_file.read_bytes()).hexdigest(), 'source_commit':subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=PROJECT_DIRECTORY, text=True).strip(), 'source_branch':subprocess.check_output(['git', 'branch', '--show-current'], cwd=PROJECT_DIRECTORY, text=True).strip(), 'protected_mesh_fingerprints_before':protected_meshes, 'geometry_fingerprints_before':geometry_fingerprints, 'protected_mesh_shading_fingerprints':shading_fingerprints, 'removed_decals':[], 'texture_groups':[]}
    for scene_object in decal_objects:
        scene_object.data.calc_loop_triangles()
        report['removed_decals'].append({'name':scene_object.name, 'triangles':len(scene_object.data.loop_triangles), 'fingerprint':mesh_fingerprint(scene_object)})
    bpy.context.preferences.filepaths.save_version = 0
    backup_path = validate_output_path(PROJECT_DIRECTORY, destination / 'source_before_flat_decal_conversion.blend', 'preview').path
    bpy.ops.wm.save_as_mainfile(filepath=str(backup_path), relative_remap=True)
    report['texture_resolved_source_backup'] = str(backup_path)
    original_scene = bpy.context.scene
    transfer_scene = bpy.data.scenes.new('FlatChassisDecalTextureTransfer')
    bpy.context.window.scene = transfer_scene
    transfer_scene.render.engine = 'CYCLES'
    transfer_scene.cycles.samples = 1
    device_preferences = bpy.context.preferences.addons['cycles'].preferences
    device_preferences.compute_device_type = 'OPTIX'
    device_preferences.get_devices()
    for device in device_preferences.devices:
        device.use = device.type == 'OPTIX'
    transfer_scene.cycles.device = 'GPU' if any(device.type == 'OPTIX' for device in device_preferences.devices) else 'CPU'
    temporary_materials = {}
    for scene_object in decal_objects:
        transfer_scene.collection.objects.link(scene_object)
        original_material = scene_object.active_material
        if original_material.name not in temporary_materials:
            material = original_material.copy()
            emission = material.node_tree.nodes.new('ShaderNodeEmission')
            emission.name = 'FlatDecalTransferEmission'
            output = next(node for node in material.node_tree.nodes if node.type == 'OUTPUT_MATERIAL')
            material.node_tree.links.new(emission.outputs['Emission'], output.inputs['Surface'])
            temporary_materials[original_material.name] = material
        scene_object.active_material = temporary_materials[original_material.name]
    for material_name in ('BakedPhysicalRedLacquer', 'BakedPhysicalSatinCarbon'):
        report['texture_groups'].append(bake_flat_decal_colors(transfer_scene, decal_objects, bpy.data.materials[material_name], destination, 4096))
    bpy.context.window.scene = original_scene
    decal_meshes = {scene_object.data for scene_object in decal_objects}
    for scene_object in decal_objects:
        bpy.data.objects.remove(scene_object, do_unlink=True)
    for decal_mesh in decal_meshes:
        if decal_mesh.users == 0:
            bpy.data.meshes.remove(decal_mesh)
    bpy.data.scenes.remove(transfer_scene)
    for material in temporary_materials.values():
        bpy.data.materials.remove(material)
    report['removed_triangles'] = sum(record['triangles'] for record in report['removed_decals'])
    report['replacement_decal_triangles'] = 0
    report['geometry_and_texture_coordinates_preserved'] = True
    report['protected_mesh_fingerprints_after'] = {scene_object.name:mesh_fingerprint(scene_object) for scene_object in source_meshes}
    for scene_object in source_meshes:
        if mesh_shading_fingerprint(scene_object) != shading_fingerprints[scene_object.name]:
            raise RuntimeError('Decal conversion changed sharp edges, smooth shading or corner normals: ' + scene_object.name)
        current_materials = list(scene_object.data.materials)
        for index, material_name in enumerate(material_names[scene_object.name]):
            scene_object.data.materials[index] = bpy.data.materials[material_name] if material_name else None
        if geometry_fingerprint(scene_object) != geometry_fingerprints[scene_object.name] or mesh_fingerprint(scene_object) != protected_meshes[scene_object.name]:
            raise RuntimeError('Decal conversion modified existing geometry, shading or texture coordinates: ' + scene_object.name)
        for index, material in enumerate(current_materials):
            scene_object.data.materials[index] = material
    if any(scene_object.type == 'MESH' and scene_object.name.startswith('GEO_CHASSIS_DECAL_') for scene_object in bpy.data.objects):
        raise RuntimeError('Chassis decal geometry remains in the source')
    candidate_path = validate_output_path(PROJECT_DIRECTORY, destination / 'f1_2030_flat_chassis_decals.blend', 'preview').path
    bpy.ops.wm.save_as_mainfile(filepath=str(candidate_path), relative_remap=True)
    manifest = json.loads((PROJECT_DIRECTORY / 'game/assets/models/vehicles/f1-2030/manifest.json').read_text())
    chassis_path = validate_output_path(PROJECT_DIRECTORY, destination / 'f1_2030_v10_chassis.glb', 'preview').path
    report['chassis_mesh_statistics'] = export_chassis(chassis_path, manifest['authored_alignment']['chassis_vertical_offset'])
    report['candidate_sha256'] = hashlib.sha256(candidate_path.read_bytes()).hexdigest()
    report['chassis_sha256'] = hashlib.sha256(chassis_path.read_bytes()).hexdigest()
    report['passed'] = True
    validate_output_path(PROJECT_DIRECTORY, destination / 'chassis_flat_decal_report.json', 'preview').path.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'passed':True, 'removed_meshes':len(decal_objects), 'removed_triangles':report['removed_triangles'], 'texture_groups':report['texture_groups']}), flush=True)


if __name__ == '__main__':
    main()
