import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from common.output_policy import validate_output_path
from formula_one_2030_object_names import canonical_object_name, normalize_object_name_document, normalize_object_reference_properties


def geometry_fingerprint(mesh):
    return hashlib.sha256(repr((
        [tuple(vertex.co) for vertex in mesh.vertices],
        [tuple(edge.vertices) for edge in mesh.edges],
        [tuple(polygon.vertices) for polygon in mesh.polygons],
        [polygon.material_index for polygon in mesh.polygons],
        [polygon.use_smooth for polygon in mesh.polygons],
        [[tuple(coordinate.uv) for coordinate in layer.data] for layer in mesh.uv_layers],
        [tuple(normal.vector) for normal in mesh.corner_normals],
        [material.name if material else None for material in mesh.materials],
    )).encode()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output-directory', type=Path, required=True)
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    project_directory = Path(__file__).resolve().parents[2]
    output_directory = validate_output_path(project_directory, options.output_directory, 'preview').path
    output_directory.mkdir(parents=True, exist_ok=True)
    source_path = Path(bpy.data.filepath)
    geometry_records = [(mesh, geometry_fingerprint(mesh)) for mesh in bpy.data.meshes]
    object_records = [(scene_object, scene_object.name, scene_object.matrix_world.copy(), scene_object.parent) for scene_object in bpy.data.objects]
    current_names = [canonical_object_name(previous_name) for scene_object, previous_name, transformation, parent in object_records]
    if len(current_names) != len(set(current_names)):
        raise ValueError('Renaming would create duplicate object names')
    for scene_object, previous_name, transformation, parent in object_records:
        scene_object.name = canonical_object_name(previous_name)
        if scene_object.type == 'MESH':
            scene_object.data.name = scene_object.name + '_MESH'
            runtime_surface_name = scene_object.get('formula_one_2030_runtime_surface_mesh')
            if runtime_surface_name:
                runtime_mesh = bpy.data.meshes[runtime_surface_name]
                runtime_mesh.name = scene_object.name + '_RUNTIME_SURFACE_MESH'
                scene_object['formula_one_2030_runtime_surface_mesh'] = runtime_mesh.name
        normalize_object_reference_properties(scene_object)
    for embedded_text in bpy.data.texts:
        text = embedded_text.as_string()
        try:
            document = json.loads(text)
            updated = json.dumps(normalize_object_name_document(document), indent=2)
        except json.JSONDecodeError:
            updated = re.sub(r'GEO_[A-Za-z0-9_]+|WHEEL_(?:FRONT|REAR)_[LR]_CONTROL|StudioGround|StudioOverheadStrip|StudioFrontThreeQuarter|StudioRearThreeQuarter|StudioSide|StudioTop', lambda match: canonical_object_name(match.group()), text)
        if updated != text:
            embedded_text.clear()
            embedded_text.write(updated)
    for mesh, fingerprint in geometry_records:
        if geometry_fingerprint(mesh) != fingerprint:
            raise ValueError('Renaming changed geometry: ' + mesh.name)
    for scene_object, previous_name, transformation, parent in object_records:
        if scene_object.matrix_world != transformation or scene_object.parent != parent:
            raise ValueError('Renaming changed object placement: ' + previous_name)
    output_path = output_directory / source_path.name
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(output_path))
    report = {
        'source_path': str(source_path),
        'output_path': str(output_path),
        'source_sha256': hashlib.sha256(source_path.read_bytes()).hexdigest(),
        'output_sha256': hashlib.sha256(output_path.read_bytes()).hexdigest(),
        'geometry_preserved': True,
        'transforms_and_parents_preserved': True,
        'objects': [{'previous_name': previous_name, 'current_name': scene_object.name, 'type': scene_object.type} for scene_object, previous_name, transformation, parent in object_records],
        'mesh_fingerprints': {mesh.name: fingerprint for mesh, fingerprint in geometry_records},
    }
    (output_directory / (source_path.stem + '_name_migration_report.json')).write_text(json.dumps(report, indent=2), encoding='utf-8')
    print('OBJECT_NAME_MIGRATION_PASS ' + str(len(object_records)))


if __name__ == '__main__':
    main()
