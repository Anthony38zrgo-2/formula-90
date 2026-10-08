import argparse
import hashlib
import json
import math
import struct
import subprocess
import sys
from pathlib import Path

import bpy
import bmesh
import mathutils

PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools'))
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools/blender'))
from common.output_policy import validate_output_path
from export_f1_2030_v10 import export_chassis
from fit_f1_2030_driver_seat import mesh_fingerprint

TARGET_OBJECT_NAMES = ('GEO_CHASSIS_CockpitUpperLining', 'GEO_CHASSIS_SEAT')


def preserve_unrelated_export_tangents(reference_path, exported_path):
    def load_export(path):
        contents = bytearray(path.read_bytes())
        document_length = struct.unpack_from('<I', contents, 12)[0]
        document = json.loads(contents[20:20 + document_length])
        binary_offset = 28 + document_length
        def accessor_layout(index):
            accessor = document['accessors'][index]
            view = document['bufferViews'][accessor['bufferView']]
            components = {'SCALAR':1, 'VEC2':2, 'VEC3':3, 'VEC4':4}[accessor['type']]
            format_string = '<' + {5121:'B', 5123:'H', 5125:'I', 5126:'f'}[accessor['componentType']] * components
            offset = binary_offset + view.get('byteOffset', 0) + accessor.get('byteOffset', 0)
            stride = view.get('byteStride', struct.calcsize(format_string))
            return format_string, offset, stride, accessor['count']
        def read_accessor(index):
            format_string, offset, stride, count = accessor_layout(index)
            return [struct.unpack_from(format_string, contents, offset + element * stride) for element in range(count)]
        meshes = {node['name']:document['meshes'][node['mesh']]['primitives'] for node in document['nodes'] if 'mesh' in node}
        return contents, meshes, accessor_layout, read_accessor
    original_contents, original_meshes, original_layout, original_accessor = load_export(reference_path)
    exported_contents, exported_meshes, exported_layout, exported_accessor = load_export(exported_path)
    restored_meshes = []
    for name, primitives in original_meshes.items():
        if name in TARGET_OBJECT_NAMES:
            continue
        if name not in exported_meshes or len(primitives) != len(exported_meshes[name]):
            raise RuntimeError('Unrelated exported mesh changed: ' + name)
        for original, exported in zip(primitives, exported_meshes[name]):
            if original_accessor(original['indices']) != exported_accessor(exported['indices']) or original['attributes'].keys() != exported['attributes'].keys() or original.get('material') != exported.get('material'):
                raise RuntimeError('Unrelated exported connectivity or materials changed: ' + name)
            for attribute, original_index in original['attributes'].items():
                original_values = original_accessor(original_index)
                exported_index = exported['attributes'][attribute]
                exported_values = exported_accessor(exported_index)
                if original_values == exported_values:
                    continue
                if attribute != 'TANGENT' or len(original_values) != len(exported_values) or max(abs(first - second) for original_value, exported_value in zip(original_values, exported_values) for first, second in zip(original_value, exported_value)) > 0.001:
                    raise RuntimeError('Unrelated exported attribute changed: ' + name + ' ' + attribute)
                format_string, offset, stride, count = exported_layout(exported_index)
                for element, value in enumerate(original_values):
                    struct.pack_into(format_string, exported_contents, offset + element * stride, *value)
                restored_meshes.append(name)
    validate_output_path(PROJECT_DIRECTORY, exported_path, 'preview').path.write_bytes(exported_contents)
    return restored_meshes


def geometry_fingerprint(scene_object):
    mesh = scene_object.data
    digest = hashlib.sha256()
    digest.update(struct.pack('<16d', *(value for row in scene_object.matrix_world for value in row)))
    for vertex in mesh.vertices:
        digest.update(struct.pack('<3f', *vertex.co))
    for edge in mesh.edges:
        digest.update(struct.pack('<2I', *edge.vertices))
    for polygon in mesh.polygons:
        digest.update(struct.pack('<' + 'I' * len(polygon.vertices), *polygon.vertices))
        digest.update(struct.pack('<I', polygon.material_index))
    for layer in mesh.uv_layers:
        digest.update(layer.name.encode())
        for coordinate in layer.data:
            digest.update(struct.pack('<2f', *coordinate.uv))
    digest.update(str([material.name if material else None for material in mesh.materials]).encode())
    return digest.hexdigest()


def shade_surface_with_preserved_rims(scene_object, additional_sharp_edges=()):
    mesh = scene_object.data
    original_geometry = geometry_fingerprint(scene_object)
    mesh.calc_loop_triangles()
    original_triangles = len(mesh.loop_triangles)
    layer_vertex_count = len(mesh.vertices) // 2
    if len(mesh.vertices) % 2 or (scene_object.name == 'GEO_CHASSIS_SEAT' and len(mesh.vertices) != 600):
        raise RuntimeError('Unexpected shell topology: ' + scene_object.name)
    if scene_object.name == 'GEO_CHASSIS_CockpitUpperLining' and any((mesh.vertices[index + layer_vertex_count].co - mesh.vertices[index].co - mathutils.Vector((0, 0, 0.0015))).length > 1e-7 for index in range(layer_vertex_count)):
        raise RuntimeError('Unexpected lining shell separation')
    edge_regions = [set() for edge in mesh.edges]
    for polygon in mesh.polygons:
        if all(index < layer_vertex_count for index in polygon.vertices):
            region = 'first_skin'
        elif all(index >= layer_vertex_count for index in polygon.vertices):
            region = 'second_skin'
        else:
            region = 'thickness_rim'
        polygon.use_smooth = True
        for loop_index in polygon.loop_indices:
            edge_regions[mesh.loops[loop_index].edge_index].add(region)
    mesh.set_sharp_from_angle(angle=math.radians(35))
    preserved_vertex_pairs = list(scene_object.get('cockpit_lining_preserved_crease_vertex_pairs', []))
    preserved_vertex_pairs = {tuple(sorted(pair)) for pair in zip(preserved_vertex_pairs[::2], preserved_vertex_pairs[1::2])}
    additional_sharp_edges = set(additional_sharp_edges) | {edge.index for edge in mesh.edges if tuple(sorted(edge.vertices)) in preserved_vertex_pairs}
    preserved_rim_edges = []
    for edge, regions in zip(mesh.edges, edge_regions):
        if 'thickness_rim' in regions and len(regions) > 1:
            edge.use_edge_sharp = True
            preserved_rim_edges.append(edge.index)
        if edge.index in additional_sharp_edges:
            edge.use_edge_sharp = True
    mesh.update()
    mesh.calc_loop_triangles()
    if geometry_fingerprint(scene_object) != original_geometry or len(mesh.loop_triangles) != original_triangles:
        raise RuntimeError('Shading changed geometry, material assignments or texture coordinates')
    topology = bmesh.new()
    topology.from_mesh(mesh)
    topology.edges.index_update()
    topology.normal_update()
    steep_edges = [edge.index for edge in topology.edges if edge.is_manifold and edge.calc_face_angle() > math.radians(35) + 1e-5]
    topology.free()
    if any(not mesh.edges[index].use_edge_sharp for index in steep_edges + preserved_rim_edges + list(additional_sharp_edges)):
        raise RuntimeError('A required hard edge was smoothed')
    corner_normals = [normal.vector.copy() for normal in mesh.corner_normals]
    edge_corners = [[] for edge in mesh.edges]
    for polygon in mesh.polygons:
        for loop_index in polygon.loop_indices:
            next_loop_index = polygon.loop_start + (loop_index - polygon.loop_start + 1) % polygon.loop_total
            edge_corners[mesh.loops[loop_index].edge_index].append({mesh.loops[loop_index].vertex_index:corner_normals[loop_index], mesh.loops[next_loop_index].vertex_index:corner_normals[next_loop_index]})
    maximum_smooth_normal_difference = 0
    for edge, corners in zip(mesh.edges, edge_corners):
        if edge.use_edge_sharp or len(corners) != 2:
            continue
        maximum_smooth_normal_difference = max([maximum_smooth_normal_difference] + [(corners[0][index] - corners[1][index]).length for index in edge.vertices])
    if maximum_smooth_normal_difference > 1e-6:
        raise RuntimeError('Smooth edge has discontinuous corner normals')
    return {'vertices':len(mesh.vertices), 'polygons':len(mesh.polygons), 'triangles':original_triangles, 'geometry_sha256':original_geometry, 'geometry_texture_coordinates_material_assignments_unchanged':True, 'smooth_faces':sum(polygon.use_smooth for polygon in mesh.polygons), 'sharp_edges':sum(edge.use_edge_sharp for edge in mesh.edges), 'preserved_skin_rim_edges':len(preserved_rim_edges), 'steep_edges_preserved':len(steep_edges), 'sharp_angle_degrees':35, 'maximum_smooth_edge_corner_normal_difference':maximum_smooth_normal_difference}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--output-directory', required=True)
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
    destination = validate_output_path(PROJECT_DIRECTORY, options.output_directory, 'preview').path
    destination.mkdir(parents=True, exist_ok=True)
    source_path = options.source_file.resolve()
    bpy.ops.wm.open_mainfile(filepath=str(source_path))
    protected_meshes = {scene_object.name:mesh_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.type == 'MESH' and scene_object.name not in TARGET_OBJECT_NAMES}
    report = {'source_before_sha256':hashlib.sha256(source_path.read_bytes()).hexdigest(), 'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=PROJECT_DIRECTORY,text=True).strip(), 'source_branch':subprocess.check_output(['git','branch','--show-current'],cwd=PROJECT_DIRECTORY,text=True).strip(), 'source_status':subprocess.check_output(['git','status','--short'],cwd=PROJECT_DIRECTORY,text=True).splitlines(), 'protected_mesh_fingerprints':protected_meshes, 'surfaces':{}}
    for name in TARGET_OBJECT_NAMES:
        report['surfaces'][name] = shade_surface_with_preserved_rims(bpy.data.objects[name])
    if any(mesh_fingerprint(bpy.data.objects[name]) != fingerprint for name,fingerprint in protected_meshes.items()):
        raise RuntimeError('Shading changed a protected mesh')
    for source_image in bpy.data.images:
        if source_image.source == 'FILE' and source_image.filepath and not source_image.packed_file and not Path(bpy.path.abspath(source_image.filepath)).is_file():
            raise RuntimeError('Source texture missing: ' + source_image.filepath)
    bpy.context.preferences.filepaths.save_version = 0
    candidate = validate_output_path(PROJECT_DIRECTORY, destination / 'f1_2030_cockpit_shaded.blend', 'preview').path
    bpy.ops.wm.save_as_mainfile(filepath=str(candidate), relative_remap=True)
    manifest = json.loads((PROJECT_DIRECTORY/'game/assets/models/vehicles/f1-2030/manifest.json').read_text())
    chassis_path = validate_output_path(PROJECT_DIRECTORY, destination / 'f1_2030_v10_chassis.glb', 'preview').path
    report['chassis_mesh_statistics'] = export_chassis(chassis_path, manifest['authored_alignment']['chassis_vertical_offset'])
    report['unrelated_export_tangent_rounding_restored'] = preserve_unrelated_export_tangents(PROJECT_DIRECTORY / 'game/assets/models/vehicles/f1-2030/f1_2030_v10_chassis.glb', chassis_path)
    report['candidate_sha256'] = hashlib.sha256(candidate.read_bytes()).hexdigest()
    report['chassis_sha256'] = hashlib.sha256(chassis_path.read_bytes()).hexdigest()
    report['passed'] = True
    report_path = validate_output_path(PROJECT_DIRECTORY, destination / 'cockpit_shading_report.json', 'preview').path
    report_path.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'passed':True, 'surfaces':report['surfaces'], 'protected_meshes':len(protected_meshes)}), flush=True)


if __name__ == '__main__':
    main()
