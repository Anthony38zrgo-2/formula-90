import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
import shutil
import sys

import bpy
from mathutils import Matrix

PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools'))
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools/blender'))
from common.output_policy import validate_output_path
from export_f1_2030_v10 import clear_export_objects, duplicate_group, export_selected, restore_sources
from glb_util import read_glb, write_glb

AFFECTED_OBJECT_NAMES = ('GEO_CHASSIS_CockpitHeadrestPadding', 'GEO_CHASSIS_INTERIOR', 'GEO_CHASSIS_SEAT')


def export_fitted_geometry(source_path, conversion, output_path):
    bpy.ops.wm.open_mainfile(filepath=str(source_path), load_ui=False, use_scripts=False)
    source_objects = [bpy.data.objects[name] for name in AFFECTED_OBJECT_NAMES]
    duplicates, source_records = duplicate_group(source_objects, [conversion @ scene_object.matrix_world for scene_object in source_objects])
    headrest = duplicates[0]
    original_coordinates = headrest.data.uv_layers['CockpitHeadrestSurfaceCoordinates']
    minimum = [min(coordinate.uv[axis] for coordinate in original_coordinates.data) for axis in range(2)]
    maximum = [max(coordinate.uv[axis] for coordinate in original_coordinates.data) for axis in range(2)]
    for coordinate in original_coordinates.data:
        coordinate.uv = [0.002 + 0.996 * (coordinate.uv[axis] - minimum[axis]) / (maximum[axis] - minimum[axis]) for axis in range(2)]
    for coordinates in list(headrest.data.uv_layers):
        if coordinates.name != original_coordinates.name:
            headrest.data.uv_layers.remove(coordinates)
    original_coordinates.active_render = True
    for scene_object in duplicates:
        modifier = scene_object.modifiers.new('TriangulateExportSurface', 'TRIANGULATE')
        bpy.context.view_layer.objects.active = scene_object
        bpy.ops.object.modifier_apply(modifier=modifier.name)
    export_selected(duplicates, output_path)
    clear_export_objects()
    restore_sources(source_records)


def append_geometry_accessor(document, buffer, geometry_document, geometry_buffer, accessor_index, buffer_view_mapping, accessor_mapping):
    if accessor_index in accessor_mapping:
        return accessor_mapping[accessor_index]
    accessor = copy.deepcopy(geometry_document['accessors'][accessor_index])
    source_view_index = accessor['bufferView']
    if source_view_index not in buffer_view_mapping:
        source_view = geometry_document['bufferViews'][source_view_index]
        source_offset = source_view.get('byteOffset', 0)
        buffer.extend(b'\0' * ((-len(buffer)) % 4))
        output_view = copy.deepcopy(source_view)
        output_view['buffer'] = 0
        output_view['byteOffset'] = len(buffer)
        buffer.extend(geometry_buffer[source_offset:source_offset + source_view['byteLength']])
        buffer_view_mapping[source_view_index] = len(document['bufferViews'])
        document['bufferViews'].append(output_view)
    accessor['bufferView'] = buffer_view_mapping[source_view_index]
    accessor_mapping[accessor_index] = len(document['accessors'])
    document['accessors'].append(accessor)
    return accessor_mapping[accessor_index]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--baseline-source-file', type=Path, required=True)
    parser.add_argument('--baseline-chassis-file', type=Path, required=True)
    parser.add_argument('--baseline-manifest-file', type=Path, required=True)
    parser.add_argument('--output-directory', type=Path, required=True)
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    output_directory = validate_output_path(PROJECT_DIRECTORY, options.output_directory, 'preview').path
    output_directory.mkdir(parents=True, exist_ok=True)
    manifest = json.loads(options.baseline_manifest_file.read_text())
    conversion = Matrix.Translation((0, 0, manifest['authored_alignment']['chassis_vertical_offset'])) @ Matrix.Rotation(-math.pi / 2, 4, 'Z')
    geometry_path = validate_output_path(PROJECT_DIRECTORY, output_directory / 'fitted_driver_clearance_geometry.glb', 'preview').path
    export_fitted_geometry(options.source_file, conversion, geometry_path)
    geometry_document, geometry_buffer = read_glb(geometry_path)
    document, buffer = read_glb(options.baseline_chassis_file)
    original_buffer = bytes(buffer)
    original_document = copy.deepcopy(document)
    fitted_meshes = {node['name']: geometry_document['meshes'][node['mesh']] for node in geometry_document['nodes'] if 'mesh' in node}
    changes = {}
    buffer_view_mapping = {}
    accessor_mapping = {}
    for node in document['nodes']:
        name = node.get('name')
        if name not in AFFECTED_OBJECT_NAMES:
            continue
        original_mesh = document['meshes'][node['mesh']]
        fitted_mesh = fitted_meshes[name]
        if len(original_mesh['primitives']) != 1 or len(fitted_mesh['primitives']) != 1:
            raise RuntimeError('Fitted mesh material layout is unsupported: ' + name)
        original_primitive = original_mesh['primitives'][0]
        fitted_primitive = fitted_mesh['primitives'][0]
        replacement = copy.deepcopy(original_primitive)
        replacement['attributes'] = {attribute: append_geometry_accessor(document, buffer, geometry_document, geometry_buffer, accessor_index, buffer_view_mapping, accessor_mapping) for attribute, accessor_index in fitted_primitive['attributes'].items() if attribute in original_primitive['attributes']}
        if set(replacement['attributes']) != set(original_primitive['attributes']):
            raise RuntimeError('Fitted export lost a required surface attribute: ' + name)
        replacement['indices'] = append_geometry_accessor(document, buffer, geometry_document, geometry_buffer, fitted_primitive['indices'], buffer_view_mapping, accessor_mapping)
        original_mesh['primitives'] = [replacement]
        changes[name] = {'runtime_vertex_count': document['accessors'][replacement['attributes']['POSITION']]['count'], 'triangle_count': document['accessors'][replacement['indices']]['count'] // 3, 'material_index_preserved': replacement['material'] == original_primitive['material']}
    if set(changes) != set(AFFECTED_OBJECT_NAMES):
        raise RuntimeError('The baseline chassis does not contain every fitted mesh')
    if bytes(buffer[:len(original_buffer)]) != original_buffer:
        raise RuntimeError('Original geometry or embedded textures were overwritten')
    if any(document.get(key) != original_document.get(key) for key in ('materials', 'images', 'textures', 'samplers', 'nodes')):
        raise RuntimeError('Protected runtime appearance or node hierarchy changed')
    for node in document['nodes']:
        if 'mesh' in node and node.get('name') not in AFFECTED_OBJECT_NAMES and document['meshes'][node['mesh']] != original_document['meshes'][node['mesh']]:
            raise RuntimeError('Protected runtime mesh changed: ' + node.get('name', ''))
    document['buffers'][0]['byteLength'] = len(buffer)
    candidate_chassis = validate_output_path(PROJECT_DIRECTORY, output_directory / 'f1_2030_v10_chassis.glb', 'preview').path
    write_glb(candidate_chassis, document, buffer)
    for asset_name, asset in manifest['geometry_assets'].items():
        output_path = validate_output_path(PROJECT_DIRECTORY, output_directory / asset['path'], 'preview').path
        if asset_name != 'chassis':
            shutil.copyfile(PROJECT_DIRECTORY / 'game/assets/models/vehicles/f1-2030' / asset['path'], output_path)
        asset['sha256'] = hashlib.sha256(output_path.read_bytes()).hexdigest().upper()
        asset['size_bytes'] = output_path.stat().st_size
    report = {'operation': 'Export fitted source meshes and replace only the headrest, cockpit interior and seat primitive geometry in the canonical chassis container', 'passed': True, 'source_path': str(options.source_file.resolve()), 'source_sha256': hashlib.sha256(options.source_file.read_bytes()).hexdigest(), 'baseline_source_sha256': hashlib.sha256(options.baseline_source_file.read_bytes()).hexdigest(), 'baseline_chassis_sha256': hashlib.sha256(options.baseline_chassis_file.read_bytes()).hexdigest(), 'files': manifest['geometry_assets'], 'changes': changes, 'protected_geometry_attributes_preserved': True, 'materials_and_embedded_texture_bytes_preserved': True, 'wheel_exports_byte_identical': True}
    (output_directory / 'manifest.json').write_text(json.dumps(manifest, indent=2))
    (output_directory / 'driver_clearance_export_report.json').write_text(json.dumps(report, indent=2))
    print(json.dumps(report), flush=True)


if __name__ == '__main__':
    main()
