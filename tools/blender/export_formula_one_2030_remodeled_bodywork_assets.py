import argparse
import copy
import hashlib
import json
import shutil
import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from common.output_policy import validate_output_path
from formula_one_2030_object_names import BODYWORK_COMPONENT_NAMES
from glb_util import read_glb, write_glb
from restore_formula_one_2030_original_tire_geometry import append_accessor
import export_f1_2030_v10


REPLACED_OBJECT_NAMES = ('GEO_ENGINE_AIR_INTAKE_INLET_SURFACE', 'GEO_ENGINE_COVER_REAR_CLOSURE', 'GEO_ENGINE_COVER_SIDE_CAMERA_HOUSINGS', 'GEO_FRONT_WING_MAIN_ELEMENT', 'GEO_FRONT_WING_UPPER_ELEMENT')
BODYWORK_ASSEMBLY_NAME = 'GEO_BODYWORK_NOSE_SIDEPODS_AND_ENGINE_COVER'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest().upper()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--baseline-directory', type=Path, required=True)
    parser.add_argument('--output-directory', type=Path, required=True)
    arguments = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    repository = Path(__file__).resolve().parents[2]
    output_directory = validate_output_path(repository, arguments.output_directory, 'preview').path
    output_directory.mkdir(parents=True, exist_ok=True)
    manifest = json.loads((arguments.baseline_directory / 'manifest.json').read_text())
    bpy.ops.wm.open_mainfile(filepath=str(arguments.source_file))
    preceding_duplicate_function = export_f1_2030_v10.duplicate_mesh_object

    def duplicate_remodeled_surface(source, source_name, matrix_world):
        surface_name = source.get('formula_one_2030_runtime_surface_mesh')
        source_mesh = bpy.data.meshes[surface_name] if surface_name else source.data
        normals = [normal.vector.copy() for normal in source_mesh.corner_normals] if source_name in (*BODYWORK_COMPONENT_NAMES, *REPLACED_OBJECT_NAMES) else None
        duplicate = preceding_duplicate_function(source, source_name, matrix_world)
        if normals is not None:
            transformation = matrix_world.to_3x3().inverted().transposed()
            duplicate.data.normals_split_custom_set([(transformation @ normal).normalized() for normal in normals])
        return duplicate

    donor_path = output_directory / 'remodeled_chassis_geometry.glb'
    export_f1_2030_v10.duplicate_mesh_object = duplicate_remodeled_surface
    try:
        export_f1_2030_v10.export_chassis(donor_path, manifest['authored_alignment']['chassis_vertical_offset'])
    finally:
        export_f1_2030_v10.duplicate_mesh_object = preceding_duplicate_function
    donor_document, donor_buffer = read_glb(donor_path)
    chassis_path = arguments.baseline_directory / 'f1_2030_v10_chassis.glb'
    document, buffer = read_glb(chassis_path)
    original_document = copy.deepcopy(document)
    original_buffer = bytes(buffer)
    donor_nodes = {node['name']: node for node in donor_document['nodes'] if 'mesh' in node}
    nodes = {node['name']: node for node in document['nodes'] if 'mesh' in node}
    bodywork_material = document['meshes'][nodes[BODYWORK_ASSEMBLY_NAME]['mesh']]['primitives'][0]['material']
    inlet_material = document['meshes'][nodes['GEO_ENGINE_AIR_INTAKE_INLET_SURFACE']['mesh']]['primitives'][0]['material']
    view_mapping = {}
    accessor_mapping = {}
    changes = {}

    def replacement_mesh(name, material_index):
        mesh = copy.deepcopy(donor_document['meshes'][donor_nodes[name]['mesh']])
        for primitive in mesh['primitives']:
            donor_material_name = donor_document['materials'][primitive['material']].get('name', '')
            primitive['attributes'] = {attribute: append_accessor(document, buffer, donor_document, donor_buffer, source_index, view_mapping, accessor_mapping) for attribute, source_index in primitive['attributes'].items()}
            primitive['indices'] = append_accessor(document, buffer, donor_document, donor_buffer, primitive['indices'], view_mapping, accessor_mapping)
            primitive['material'] = inlet_material if name == 'BodyworkEngineCover' and donor_material_name == 'UniformBlenderKitCarbonWithProtectedDecals' else material_index
        changes[name] = {'triangles': sum(document['accessors'][primitive['indices']]['count'] // 3 for primitive in mesh['primitives']), 'materials_reused': True}
        return mesh

    for name in REPLACED_OBJECT_NAMES:
        node = nodes[name]
        existing_material = document['meshes'][node['mesh']]['primitives'][0]['material']
        document['meshes'][node['mesh']] = replacement_mesh(name, existing_material)
    assembly = nodes[BODYWORK_ASSEMBLY_NAME]
    assembly.pop('mesh')
    for name in BODYWORK_COMPONENT_NAMES:
        mesh_index = len(document['meshes'])
        document['meshes'].append(replacement_mesh(name, bodywork_material))
        node_index = len(document['nodes'])
        document['nodes'].append({'name': name, 'mesh': mesh_index})
        assembly.setdefault('children', []).append(node_index)
    assert bytes(buffer[:len(original_buffer)]) == original_buffer
    assert all(document.get(key) == original_document.get(key) for key in ('materials', 'images', 'textures', 'samplers'))
    for node in original_document['nodes']:
        if 'mesh' in node and node.get('name') not in (*REPLACED_OBJECT_NAMES, BODYWORK_ASSEMBLY_NAME):
            assert document['meshes'][node['mesh']] == original_document['meshes'][node['mesh']]
    document['buffers'][0]['byteLength'] = len(buffer)
    output_chassis = output_directory / chassis_path.name
    write_glb(output_chassis, document, buffer)
    for key, asset in manifest['geometry_assets'].items():
        output_path = output_directory / asset['path']
        if key != 'chassis':
            shutil.copyfile(arguments.baseline_directory / asset['path'], output_path)
            assert digest(output_path) == digest(arguments.baseline_directory / asset['path'])
        asset['sha256'] = digest(output_path)
        asset['size_bytes'] = output_path.stat().st_size
    manifest['current_chassis_source_sha256'] = digest(arguments.source_file).lower()
    report = {'operation': 'Publish separated bodywork, oval intake, rear engine cover taper and front wing leading arc', 'passed': True, 'source_file': str(arguments.source_file), 'source_digest': digest(arguments.source_file), 'baseline_chassis_digest': digest(chassis_path), 'files': manifest['geometry_assets'], 'changes': changes, 'protected_geometry_and_material_bytes_preserved': True, 'wheel_exports_byte_identical': True, 'bodywork_assembly_retained_as_parent': True}
    manifest['bodywork_geometry_remodel'] = report
    (output_directory / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    (output_directory / 'bodywork_geometry_export_report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps({'changes': changes, 'wheel_exports_byte_identical': True, 'protected_runtime_appearance_preserved': True}))


if __name__ == '__main__':
    main()
