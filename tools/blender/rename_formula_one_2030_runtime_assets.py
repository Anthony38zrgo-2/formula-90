import argparse
import copy
import hashlib
import json
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from common.output_policy import validate_output_path
from formula_one_2030_object_names import canonical_object_name, normalize_object_name_document


def rename_runtime_document(source_path, output_path):
    source_bytes = source_path.read_bytes()
    magic, version, total_length = struct.unpack_from('<III', source_bytes)
    if magic != 0x46546C67 or version != 2 or total_length != len(source_bytes):
        raise ValueError('Invalid runtime container: ' + str(source_path))
    chunks = []
    offset = 12
    while offset < total_length:
        chunk_length, chunk_type = struct.unpack_from('<II', source_bytes, offset)
        chunks.append((chunk_type, source_bytes[offset + 8:offset + 8 + chunk_length]))
        offset += chunk_length + 8
    document = json.loads(chunks[0][1].decode('utf-8'))
    original_document = copy.deepcopy(document)
    renamed_objects = []
    for collection_name in ('nodes', 'meshes'):
        for record in document.get(collection_name, []):
            if 'name' in record:
                previous_name = record['name']
                record['name'] = canonical_object_name(previous_name)
                if record['name'] != previous_name:
                    renamed_objects.append({'collection': collection_name, 'previous_name': previous_name, 'current_name': record['name']})
            if 'extras' in record:
                record['extras'] = normalize_object_name_document(record['extras'])
    preserved_document = copy.deepcopy(document)
    for collection_name in ('nodes', 'meshes'):
        for previous, current in zip(original_document.get(collection_name, []), preserved_document.get(collection_name, [])):
            if 'name' in previous:
                current['name'] = previous['name']
            if 'extras' in previous:
                current['extras'] = previous['extras']
    if preserved_document != original_document:
        raise ValueError('Runtime migration changed fields beyond names')
    encoded_document = json.dumps(document, separators=(',', ':'), ensure_ascii=False).encode('utf-8')
    encoded_document += b' ' * (-len(encoded_document) % 4)
    output_chunks = [(chunks[0][0], encoded_document), *chunks[1:]]
    output_bytes = struct.pack('<III', magic, version, 12 + sum(len(content) + 8 for chunk_type, content in output_chunks))
    output_bytes += b''.join(struct.pack('<II', len(content), chunk_type) + content for chunk_type, content in output_chunks)
    output_path.write_bytes(output_bytes)
    return {
        'source_sha256': hashlib.sha256(source_bytes).hexdigest(),
        'output_sha256': hashlib.sha256(output_bytes).hexdigest(),
        'binary_chunk_sha256': [hashlib.sha256(content).hexdigest() for chunk_type, content in chunks[1:]],
        'non_name_document_fields_preserved': True,
        'binary_chunks_preserved': True,
        'renamed_objects': renamed_objects,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-directory', type=Path, required=True)
    parser.add_argument('--output-directory', type=Path, required=True)
    options = parser.parse_args()
    project_directory = Path(__file__).resolve().parents[2]
    output_directory = validate_output_path(project_directory, options.output_directory, 'preview').path
    output_directory.mkdir(parents=True, exist_ok=True)
    manifest = json.loads((options.source_directory / 'manifest.json').read_text(encoding='utf-8'))
    migration_records = {}
    for asset_name, asset in manifest['geometry_assets'].items():
        source_path = options.source_directory / asset['path']
        output_path = output_directory / asset['path']
        migration_records[asset_name] = rename_runtime_document(source_path, output_path)
        asset['sha256'] = migration_records[asset_name]['output_sha256'].upper()
        asset['size_bytes'] = output_path.stat().st_size
    manifest['object_name_migration'] = {
        'operation': 'Rename runtime nodes and meshes while preserving all geometry, materials, images, transforms and hierarchy',
        'assets': migration_records,
        'physics_parameters_preserved': True,
    }
    (output_directory / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    previous_report_path = options.source_directory / 'export_report.json'
    if previous_report_path.exists():
        report = normalize_object_name_document(json.loads(previous_report_path.read_text(encoding='utf-8')))
        report['object_name_migration'] = manifest['object_name_migration']
        (output_directory / 'export_report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    (output_directory / 'object_name_migration_report.json').write_text(json.dumps(migration_records, indent=2), encoding='utf-8')
    print('RUNTIME_NAME_MIGRATION_PASS ' + str(len(migration_records)))


if __name__ == '__main__':
    main()
