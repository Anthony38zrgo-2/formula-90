import copy
import hashlib
import json
import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
from export_f1_2030_v10 import export_wheel, WHEELS
from glb_util import read_glb, write_glb
from restore_formula_one_2030_original_tire_geometry import append_accessor


def bake_carbon_surface(scene_object):
    scene_object.hide_set(False)
    bpy.ops.object.select_all(action='DESELECT')
    scene_object.select_set(True)
    bpy.context.view_layer.objects.active = scene_object
    bpy.ops.object.mode_set(mode='EDIT')
    bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.uv.smart_project(island_margin=0.025)
    bpy.ops.object.mode_set(mode='OBJECT')
    material = scene_object.data.materials[0].copy()
    scene_object.data.materials[0] = material
    image = bpy.data.images.new(scene_object.name + '_CarbonBaseColor', width=512, height=512, alpha=False)
    image.generated_color = (0.02, 0.025, 0.03, 1)
    texture = material.node_tree.nodes.new('ShaderNodeTexImage')
    texture.image = image
    material.node_tree.nodes.active = texture
    bpy.ops.object.bake(type='DIFFUSE', pass_filter={'COLOR'}, margin=4)
    image.pack()
    runtime_material = bpy.data.materials.new(scene_object.name + '_ExportCarbon')
    runtime_material.use_nodes = True
    shader = next(node for node in runtime_material.node_tree.nodes if node.type == 'BSDF_PRINCIPLED')
    shader.inputs['Metallic'].default_value = 0.25
    shader.inputs['Roughness'].default_value = 0.36
    texture = runtime_material.node_tree.nodes.new('ShaderNodeTexImage')
    texture.image = image
    runtime_material.node_tree.links.new(texture.outputs['Color'], shader.inputs['Base Color'])
    scene_object.data.materials[0] = runtime_material


def replace_brakes(baseline_path, donor_path, output_path):
    document, buffer = read_glb(baseline_path)
    donor_document, donor_buffer = read_glb(donor_path)
    protected_buffer = bytes(buffer)
    protected_meshes = {node['name']: copy.deepcopy(document['meshes'][node['mesh']])
                        for node in document['nodes'] if 'mesh' in node and not node.get('name', '').startswith('GEO_BRAKE_')}
    view_mapping = {}
    accessor_mapping = {}
    material_mapping = {}
    texture_mapping = {}

    def copy_texture(texture_index):
        if texture_index in texture_mapping:
            return texture_mapping[texture_index]
        texture = copy.deepcopy(donor_document['textures'][texture_index])
        image = copy.deepcopy(donor_document['images'][texture['source']])
        source_view = donor_document['bufferViews'][image['bufferView']]
        offset = source_view.get('byteOffset', 0)
        buffer.extend(b'\0' * ((-len(buffer)) % 4))
        image_view = copy.deepcopy(source_view)
        image_view['buffer'] = 0
        image_view['byteOffset'] = len(buffer)
        buffer.extend(donor_buffer[offset:offset + source_view['byteLength']])
        image['bufferView'] = len(document['bufferViews'])
        document['bufferViews'].append(image_view)
        texture['source'] = len(document.setdefault('images', []))
        document['images'].append(image)
        if 'sampler' in texture:
            sampler = copy.deepcopy(donor_document['samplers'][texture['sampler']])
            texture['sampler'] = len(document.setdefault('samplers', []))
            document['samplers'].append(sampler)
        texture_mapping[texture_index] = len(document.setdefault('textures', []))
        document['textures'].append(texture)
        return texture_mapping[texture_index]

    def remap_textures(value):
        if isinstance(value, dict):
            for key, child in value.items():
                if key.endswith('Texture') and isinstance(child, dict) and 'index' in child:
                    child['index'] = copy_texture(child['index'])
                else:
                    remap_textures(child)
        elif isinstance(value, list):
            for child in value:
                remap_textures(child)

    def copy_material(material_index):
        if material_index not in material_mapping:
            material = copy.deepcopy(donor_document['materials'][material_index])
            remap_textures(material)
            material_mapping[material_index] = len(document.setdefault('materials', []))
            document['materials'].append(material)
        return material_mapping[material_index]

    brake_root = next(index for index, node in enumerate(document['nodes']) if node.get('name') == 'BrakeStatic')
    replaced_names = []
    for donor_node in donor_document['nodes']:
        name = donor_node.get('name', '')
        if 'mesh' not in donor_node or not name.startswith('GEO_BRAKE_'):
            continue
        mesh = copy.deepcopy(donor_document['meshes'][donor_node['mesh']])
        for primitive in mesh['primitives']:
            primitive['attributes'] = {
                attribute: append_accessor(document, buffer, donor_document, donor_buffer, accessor_index, view_mapping, accessor_mapping)
                for attribute, accessor_index in primitive['attributes'].items()
            }
            primitive['indices'] = append_accessor(document, buffer, donor_document, donor_buffer, primitive['indices'], view_mapping, accessor_mapping)
            primitive['material'] = copy_material(primitive['material'])
        existing_node = next((node for node in document['nodes'] if node.get('name') == name), None)
        if existing_node is None:
            mesh_index = len(document['meshes'])
            document['meshes'].append(mesh)
            node_index = len(document['nodes'])
            document['nodes'].append({'name': name, 'mesh': mesh_index})
            document['nodes'][brake_root].setdefault('children', []).append(node_index)
        else:
            document['meshes'][existing_node['mesh']] = mesh
        replaced_names.append(name)
    if len(replaced_names) != 4:
        raise ValueError('Each wheel requires a disc, caliper, cover and independent intake')
    for node in document['nodes']:
        name = node.get('name', '')
        if name in protected_meshes and document['meshes'][node['mesh']] != protected_meshes[name]:
            raise ValueError('Protected wheel geometry changed: ' + name)
    if bytes(buffer[:len(protected_buffer)]) != protected_buffer:
        raise ValueError('Protected buffer content changed')
    for extension in donor_document.get('extensionsUsed', []):
        if extension not in document.setdefault('extensionsUsed', []):
            document['extensionsUsed'].append(extension)
    write_glb(output_path, document, buffer)
    return replaced_names


def main():
    arguments = sys.argv[sys.argv.index('--') + 1:]
    output_directory = Path(arguments[0]).resolve()
    project_directory = Path(__file__).resolve().parents[2]
    from common.output_policy import validate_output_path
    validate_output_path(project_directory, output_directory, 'preview')
    output_directory.mkdir(parents=True, exist_ok=True)
    physics_path = project_directory / 'game/data/vehicles/f1_2030/f1_2030_v10_geometric.json'
    physics = json.loads(physics_path.read_text())
    source_path = Path(bpy.data.filepath)
    source_hash = hashlib.sha256(source_path.read_bytes()).hexdigest()
    bpy.context.scene.render.engine = 'CYCLES'
    bpy.context.scene.cycles.device = 'CPU'
    bpy.context.scene.cycles.samples = 1
    for scene_object in list(bpy.data.objects):
        if scene_object.type == 'MESH' and scene_object.name.startswith('GEO_BRAKE_') and scene_object.name.endswith(('_DUCT', '_INBOARD_DUCT_COVER')):
            bake_carbon_surface(scene_object)
    donor_directory = output_directory / 'donors'
    donor_directory.mkdir(exist_ok=True)
    for corner, position in WHEELS.items():
        specification = physics['tires']['front' if corner.startswith('F') else 'rear']
        export_wheel(position, donor_directory / ('f1_2030_v10_wheel_' + corner + '.glb'), specification['radius'], specification['width'])
    canonical_directory = project_directory / 'game/assets/models/vehicles/f1-2030'
    for variant, baseline_directory in [('original', canonical_directory), ('senna', canonical_directory / 'liveries/mp4_6_senna_1')]:
        variant_directory = output_directory / variant
        variant_directory.mkdir(exist_ok=True)
        manifest = json.loads((baseline_directory / 'manifest.json').read_text())
        changes = {}
        for asset_name, asset in manifest['geometry_assets'].items():
            if asset_name == 'chassis':
                continue
            destination = variant_directory / asset['path']
            changes[asset_name] = replace_brakes(baseline_directory / asset['path'], donor_directory / asset['path'], destination)
            asset['sha256'] = hashlib.sha256(destination.read_bytes()).hexdigest().upper()
            asset['size_bytes'] = destination.stat().st_size
        manifest['brake_assembly_export'] = {'source_sha256': source_hash, 'changes': changes, 'protected_chassis_and_tires_preserved': True}
        manifest['wheel_visual_contract']['BrakeStatic'] = 'caliper, carbon cover and independent cooling intake follow suspension and steering without wheel spin'
        (variant_directory / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
        report = json.loads((baseline_directory / 'export_report.json').read_text())
        report['brake_assembly_export'] = manifest['brake_assembly_export']
        report['files'] = manifest['geometry_assets']
        (variant_directory / 'export_report.json').write_text(json.dumps(report, indent=2) + '\n')
    if hashlib.sha256(source_path.read_bytes()).hexdigest() != source_hash:
        raise ValueError('Authoring source changed during export')
    print('BRAKE_EXPORT_PASS eight wheels; protected chassis and tire data preserved', flush=True)


if __name__ == '__main__':
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    main()
