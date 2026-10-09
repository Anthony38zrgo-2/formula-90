import argparse
import hashlib
import json
import sys
from pathlib import Path

import bpy

PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools'))
from common.output_policy import validate_output_path


def digest_file(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def driver_structure():
    return {
        scene_object.name: {
            'type': scene_object.type,
            'transform': [list(row) for row in scene_object.matrix_world],
            'parent': scene_object.parent.name if scene_object.parent else None,
            'parent_bone': scene_object.parent_bone,
            'vertices': [list(vertex.co) for vertex in scene_object.data.vertices] if scene_object.type == 'MESH' else [],
            'polygons': [list(polygon.vertices) for polygon in scene_object.data.polygons] if scene_object.type == 'MESH' else [],
            'weights': [[(group.group, group.weight) for group in vertex.groups] for vertex in scene_object.data.vertices] if scene_object.type == 'MESH' else [],
            'bones': {bone.name: {'parent': bone.parent.name if bone.parent else None, 'rest': [list(row) for row in bone.matrix_local], 'pose': [list(row) for row in scene_object.pose.bones[bone.name].matrix]} for bone in scene_object.data.bones} if scene_object.type == 'ARMATURE' else {},
        } for scene_object in bpy.data.objects
    }


def apply_visor_material(profile):
    material = bpy.data.materials['RT_HELMET_Glass']
    surface = next(node for node in material.node_tree.nodes if node.type == 'BSDF_PRINCIPLED')
    for socket_name in ('Base Color', 'Alpha'):
        for link in list(surface.inputs[socket_name].links):
            material.node_tree.links.remove(link)
    surface.inputs['Metallic'].default_value = profile['visor']['metallic']
    surface.inputs['Roughness'].default_value = profile['visor']['roughness']
    surface.inputs['Alpha'].default_value = 1.0
    surface.inputs['Coat Weight'].default_value = 0.25
    surface.inputs['Coat Roughness'].default_value = 0.12
    head = bpy.data.objects['DriverHeadAndNeck']
    visor_material_index = next(index for index, candidate in enumerate(head.data.materials) if candidate == material)
    visor_loops = [loop_index for polygon in head.data.polygons if polygon.material_index == visor_material_index for loop_index in polygon.loop_indices]
    evaluated = head.evaluated_get(bpy.context.evaluated_depsgraph_get())
    evaluated_mesh = evaluated.to_mesh()
    heights = {head.data.loops[loop_index].vertex_index: (evaluated.matrix_world @ evaluated_mesh.vertices[head.data.loops[loop_index].vertex_index].co).z for loop_index in visor_loops}
    evaluated.to_mesh_clear()
    minimum_height = min(heights.values())
    maximum_height = max(heights.values())
    for existing_colors in list(head.data.color_attributes):
        head.data.color_attributes.remove(existing_colors)
    colors = head.data.color_attributes.new(name='ReflectiveVisorTint', type='FLOAT_COLOR', domain='CORNER')
    head.data.color_attributes.active_color_index = 0
    head.data.color_attributes.render_color_index = 0
    for color in colors.data:
        color.color = (1, 1, 1, 1)
    lower_color, middle_color, upper_color = [profile['visor'][name] for name in ('lower_color', 'middle_color', 'upper_color')]
    for loop_index in visor_loops:
        fraction = (heights[head.data.loops[loop_index].vertex_index] - minimum_height) / max(0.000001, maximum_height - minimum_height)
        if fraction < 0.8:
            first_color, second_color, progress = lower_color, middle_color, fraction / 0.8
        else:
            first_color, second_color, progress = middle_color, upper_color, (fraction - 0.8) / 0.2
        colors.data[loop_index].color = tuple(first * (1 - progress) + second * progress for first, second in zip(first_color, second_color)) + (1,)
    vertex_color = material.node_tree.nodes.new('ShaderNodeVertexColor')
    vertex_color.layer_name = colors.name
    material.node_tree.links.new(vertex_color.outputs['Color'], surface.inputs['Base Color'])


def apply_driver_skin(prepared_source, profile_path, destination, output_mode, base_manifest_path):
    destination = validate_output_path(PROJECT_DIRECTORY, destination, output_mode).path
    destination.mkdir(parents=True, exist_ok=True)
    profile = json.loads(Path(profile_path).read_text(encoding='utf-8-sig'))
    base_manifest = json.loads(Path(base_manifest_path).read_text(encoding='utf-8-sig'))
    if base_manifest.get('geometry_contract_version') != 1 or not base_manifest.get('original_geometry_verification', {}).get('verified'):
        raise RuntimeError('Driver skin requires verified original geometry with uniform scaling; regenerate the source first')
    bpy.ops.wm.open_mainfile(filepath=str(prepared_source), load_ui=False, use_scripts=False)
    if bpy.context.scene.get('driver_appearance'):
        raise RuntimeError('Apply the skin to the original prepared driver or regenerate the original appearance first')
    original_structure = driver_structure()
    original_structure_digest = hashlib.sha256(json.dumps(original_structure, sort_keys=True).encode()).hexdigest()
    replacements = {}
    for texture in profile['textures']:
        texture_path = Path(profile_path).parent / texture['path']
        if not texture_path.is_file():
            raise RuntimeError('Missing skin texture: ' + str(texture_path))
        image = bpy.data.images.load(str(texture_path.resolve()), check_existing=False)
        image.pack()
        original_aspect = texture['original_aspect_ratio']
        new_aspect = image.size[0] / image.size[1]
        horizontal_scale = min(1.0, original_aspect / new_aspect)
        vertical_scale = min(1.0, new_aspect / original_aspect)
        replacements[texture['original_image']] = {'image': image, 'scale': (horizontal_scale, vertical_scale), 'offset': ((1 - horizontal_scale) * 0.5, (1 - vertical_scale) * 0.5), 'path': texture_path}
    material_alignment = {}
    for material in bpy.data.materials:
        if not material.node_tree:
            continue
        for node in material.node_tree.nodes:
            if node.type == 'TEX_IMAGE' and node.image and Path(node.image.name).stem in replacements:
                replacement = replacements[Path(node.image.name).stem]
                node.image = replacement['image']
                material_alignment[material.name] = replacement
        surface = next((node for node in material.node_tree.nodes if node.type == 'BSDF_PRINCIPLED'), None)
        if surface:
            if material.name in ('RT_Helemt', 'RT_HELMET_Strip'):
                surface.inputs['Roughness'].default_value = 0.25
                surface.inputs['Coat Weight'].default_value = 0.35
                surface.inputs['Coat Roughness'].default_value = 0.18
            elif material.name == 'RT_DriverSuit':
                surface.inputs['Roughness'].default_value = 0.82
            elif material.name == 'RT_Gloves':
                surface.inputs['Roughness'].default_value = 0.78
            elif material.name == 'RT_HANS':
                surface.inputs['Roughness'].default_value = 0.7
    if {replacement['image'].name for replacement in material_alignment.values()} != {replacement['image'].name for replacement in replacements.values()}:
        raise RuntimeError('The prepared driver does not use every expected source texture')
    if base_manifest.get('character') != 'low_polygon_race_car_driver':
        raise RuntimeError('This skin requires the current low polygon driver, never the deprecated Racer mesh')
    for scene_object in bpy.data.objects:
        if scene_object.type != 'MESH':
            continue
        coordinates = scene_object.data.uv_layers.active
        for polygon in scene_object.data.polygons:
            material = scene_object.data.materials[polygon.material_index]
            alignment = material_alignment.get(material.name)
            if alignment:
                for loop_index in polygon.loop_indices:
                    coordinates.data[loop_index].uv = [coordinate * scale + offset for coordinate, scale, offset in zip(coordinates.data[loop_index].uv, alignment['scale'], alignment['offset'])]
    apply_visor_material(profile)
    if driver_structure() != original_structure:
        raise RuntimeError('Appearance changes modified geometry, skin weights or the driver rig')
    bpy.context.scene['driver_appearance'] = profile['appearance']
    bpy.ops.object.select_all(action='SELECT')
    prepared_directory = destination / 'source' if output_mode == 'promote' else destination
    prepared_directory = validate_output_path(PROJECT_DIRECTORY, prepared_directory, output_mode).path
    prepared_directory.mkdir(parents=True, exist_ok=True)
    prepared_path = validate_output_path(PROJECT_DIRECTORY, prepared_directory / 'prepared_driver.blend', output_mode).path
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(prepared_path))
    model_path = validate_output_path(PROJECT_DIRECTORY, destination / 'driver.glb', output_mode).path
    bpy.ops.export_scene.gltf(filepath=str(model_path), export_format='GLB', use_selection=True, export_animations=False, export_skins=True, export_yup=True, export_current_frame=True, export_rest_position_armature=False, export_extras=True)
    base_manifest.update({'model_sha256': digest_file(model_path), 'prepared_source_sha256': digest_file(prepared_path), 'appearance': profile['appearance'], 'appearance_profile': str(Path(profile_path)), 'appearance_profile_sha256': digest_file(Path(profile_path)), 'appearance_textures': {replacement['image'].name: {'path': str(replacement['path']), 'sha256': digest_file(replacement['path']), 'dimensions': list(replacement['image'].size), 'coordinate_scale': list(replacement['scale']), 'coordinate_offset': list(replacement['offset'])} for replacement in replacements.values()}, 'geometry_and_rig_preserved': True, 'geometry_and_rig_fingerprint': original_structure_digest})
    (destination / 'driver_manifest.json').write_text(json.dumps(base_manifest, indent=2) + '\n', encoding='utf-8')
    bpy.ops.wm.open_mainfile(filepath=str(prepared_path), load_ui=False, use_scripts=False)
    if driver_structure() != original_structure:
        raise RuntimeError('The saved prepared source changed the driver structure')
    print(json.dumps({'appearance': profile['appearance'], 'geometry_and_rig_preserved': True, 'saved_source_verified': True, 'model_path': str(model_path)}, indent=2))
    return base_manifest


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--prepared-source', type=Path, required=True)
    parser.add_argument('--appearance-profile', type=Path, required=True)
    parser.add_argument('--base-manifest', type=Path, required=True)
    parser.add_argument('--output-directory', type=Path, required=True)
    parser.add_argument('--mode', choices=('preview', 'promote'), default='preview')
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    apply_driver_skin(options.prepared_source, options.appearance_profile, options.output_directory, options.mode, options.base_manifest)


if __name__ == '__main__':
    main()
