import argparse
import copy
import hashlib
import json
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from common.output_policy import validate_output_path
from glb_util import read_glb, write_glb


def append_accessor(document, buffer, source_document, source_buffer, source_index, view_mapping, accessor_mapping):
    if source_index in accessor_mapping:
        return accessor_mapping[source_index]
    accessor = copy.deepcopy(source_document['accessors'][source_index])
    source_view_index = accessor['bufferView']
    if source_view_index not in view_mapping:
        source_view = source_document['bufferViews'][source_view_index]
        source_offset = source_view.get('byteOffset', 0)
        buffer.extend(b'\0' * ((-len(buffer)) % 4))
        output_view = copy.deepcopy(source_view)
        output_view['buffer'] = 0
        output_view['byteOffset'] = len(buffer)
        buffer.extend(source_buffer[source_offset:source_offset + source_view['byteLength']])
        view_mapping[source_view_index] = len(document['bufferViews'])
        document['bufferViews'].append(output_view)
    accessor['bufferView'] = view_mapping[source_view_index]
    accessor_mapping[source_index] = len(document['accessors'])
    document['accessors'].append(accessor)
    return accessor_mapping[source_index]


def tire_mesh(document):
    matches = [document['meshes'][node['mesh']] for node in document['nodes']
               if 'mesh' in node and node.get('name', '').endswith('_TIRE')]
    if len(matches) != 1 or len(matches[0]['primitives']) != 1:
        raise ValueError('Exactly one tire with one material surface is required')
    return matches[0]


def replace_tire_geometry(baseline_path, geometry_path, output_path):
    document, buffer = read_glb(baseline_path)
    geometry_document, geometry_buffer = read_glb(geometry_path)
    original_document = copy.deepcopy(document)
    original_buffer = bytes(buffer)
    original_primitive = copy.deepcopy(tire_mesh(document)['primitives'][0])
    geometry_primitive = tire_mesh(geometry_document)['primitives'][0]
    required_attributes = {attribute for attribute in original_primitive['attributes'] if not attribute.startswith('COLOR_')}
    if not required_attributes.issubset(geometry_primitive['attributes']):
        raise ValueError('Original tire export lacks required runtime attributes')
    view_mapping = {}
    accessor_mapping = {}
    replacement = copy.deepcopy(original_primitive)
    replacement['attributes'] = {
        attribute: append_accessor(document, buffer, geometry_document, geometry_buffer,
                                   geometry_primitive['attributes'][attribute], view_mapping, accessor_mapping)
        for attribute in geometry_primitive['attributes']
    }
    replacement['indices'] = append_accessor(document, buffer, geometry_document, geometry_buffer,
                                             geometry_primitive['indices'], view_mapping, accessor_mapping)
    tire_mesh(document)['primitives'] = [replacement]
    if bytes(buffer[:len(original_buffer)]) != original_buffer:
        raise ValueError('Protected geometry or embedded image bytes changed')
    for key in original_document:
        if key not in ('meshes', 'accessors', 'bufferViews', 'buffers') and document[key] != original_document[key]:
            raise ValueError('Protected runtime field changed: ' + key)
    for current_mesh, original_mesh in zip(document['meshes'], original_document['meshes']):
        if current_mesh is not tire_mesh(document) and current_mesh != original_mesh:
            raise ValueError('A non-tire mesh changed')
    write_glb(output_path, document, buffer)
    previous_indices = original_document['accessors'][original_primitive['indices']]
    current_indices = document['accessors'][replacement['indices']]
    return {
        'previous_triangles': previous_indices['count'] // 3,
        'current_triangles': current_indices['count'] // 3,
        'previous_vertices': original_document['accessors'][original_primitive['attributes']['POSITION']]['count'],
        'current_vertices': document['accessors'][replacement['attributes']['POSITION']]['count'],
        'previous_bytes': baseline_path.stat().st_size,
        'current_bytes': output_path.stat().st_size,
        'original_tire_bounds': document['accessors'][replacement['attributes']['POSITION']],
        'materials_images_nodes_and_other_meshes_preserved': True,
        'source_geometry_sha256': hashlib.sha256(geometry_path.read_bytes()).hexdigest(),
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--baseline-directory', type=Path, required=True)
    parser.add_argument('--geometry-directory', type=Path, required=True)
    parser.add_argument('--authoring-source-file', type=Path, required=True)
    parser.add_argument('--surface-mode', choices=('original', 'optimized'), default='original')
    parser.add_argument('--output-directory', type=Path, required=True)
    options = parser.parse_args()
    project_directory = Path(__file__).resolve().parents[2]
    output_directory = validate_output_path(project_directory, options.output_directory, 'preview').path
    output_directory.mkdir(parents=True, exist_ok=True)
    manifest = json.loads((options.baseline_directory / 'manifest.json').read_text())
    changes = {}
    for asset_name, asset in manifest['geometry_assets'].items():
        baseline_path = options.baseline_directory / asset['path']
        output_path = output_directory / asset['path']
        if asset_name == 'chassis':
            shutil.copy2(baseline_path, output_path)
        else:
            changes[asset_name] = replace_tire_geometry(
                baseline_path, options.geometry_directory / asset['path'], output_path)
            asset['sha256'] = hashlib.sha256(output_path.read_bytes()).hexdigest().upper()
            asset['size_bytes'] = output_path.stat().st_size
    geometry_record_name = options.surface_mode + '_tire_geometry'
    manifest.pop('original_tire_geometry', None)
    manifest.pop('optimized_tire_geometry', None)
    manifest[geometry_record_name] = {
        'operation': 'Replace tire surfaces using ' + options.surface_mode + ' geometry with physical diameter and width preserved',
        'authoring_source_path': str(options.authoring_source_file.resolve()),
        'authoring_source_sha256': hashlib.sha256(options.authoring_source_file.read_bytes()).hexdigest(),
        'changes': changes,
        'physics_parameters_preserved': True,
    }
    (output_directory / 'manifest.json').write_text(json.dumps(manifest, indent=2))
    report = json.loads((options.baseline_directory / 'export_report.json').read_text())
    report.pop('original_tire_geometry', None)
    report.pop('optimized_tire_geometry', None)
    report[geometry_record_name] = manifest[geometry_record_name]
    if 'files' in report:
        report['files'] = copy.deepcopy(manifest['geometry_assets'])
    (output_directory / 'export_report.json').write_text(json.dumps(report, indent=2))
    (output_directory / (geometry_record_name + '_report.json')).write_text(json.dumps(changes, indent=2))
    print('TIRE_GEOMETRY_REPLACEMENT_PASS ' + str(len(changes)))


if __name__ == '__main__':
    main()
