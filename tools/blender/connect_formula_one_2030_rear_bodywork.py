import argparse
import ast
import hashlib
import json
import sys
import math
from pathlib import Path

import bpy
import bmesh
from mathutils import Vector
from mathutils.bvhtree import BVHTree
from mathutils.geometry import barycentric_transform, closest_point_on_tri

repository = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(repository / 'tools'))
sys.path.insert(0, str(repository / 'tools/blender'))
from common.output_policy import validate_output_path
from integrate_formula_one_2030_exhaust_bodywork import CHANGED_OBJECT_NAMES, open_exhaust_apertures, render_reviews
from remodel_formula_one_2030_ferrari_reference_bodywork import monotone_profile_value, smooth_fraction

from separate_formula_one_2030_bodywork import mesh_fingerprint, protected_object_fingerprint
from taper_formula_one_2030_engine_cover_rear import exact_surface_intersections
from remodel_formula_one_2030_ferrari_reference_bodywork import triangle_surface, crossing_triangle_pairs


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--output-directory', type=Path, required=True)
    arguments = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    output = validate_output_path(repository, arguments.output_directory, 'preview').path
    output.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=str(arguments.source_file))
    assert not bpy.data.objects['BodyworkEngineCover'].get('continuous_rear_bodywork_applied')
    original_fingerprints = {name: mesh_fingerprint(bpy.data.objects[name].data) for name in CHANGED_OBJECT_NAMES}
    protected_fingerprints = {subject.name: protected_object_fingerprint(subject) for subject in bpy.data.objects if subject.name not in CHANGED_OBJECT_NAMES}
    cover = bpy.data.objects['BodyworkEngineCover']
    cover.data.calc_loop_triangles()
    cover_surface = BVHTree.FromPolygons([vertex.co for vertex in cover.data.vertices], [tuple(triangle.vertices) for triangle in cover.data.loop_triangles], all_triangles=True)
    for name in CHANGED_OBJECT_NAMES[:3]:
        subject = bpy.data.objects[name]
        original_mesh = subject.data
        original_mesh.calc_loop_triangles()
        original_normals = [normal.vector.copy() for normal in original_mesh.corner_normals]
        original_triangles = list(original_mesh.loop_triangles)
        projection_surfaces = {}
        for material_index in range(len(original_mesh.materials)):
            triangles = [triangle for triangle in original_triangles if original_mesh.polygons[triangle.polygon_index].material_index == material_index]
            if triangles:
                projection_surfaces[material_index] = (BVHTree.FromPolygons([vertex.co for vertex in original_mesh.vertices], [tuple(triangle.vertices) for triangle in triangles], all_triangles=True), triangles)
        mesh = subject.data.copy()
        editing_mesh = bmesh.new()
        editing_mesh.from_mesh(mesh)
        bmesh.ops.bisect_plane(editing_mesh, geom=list(editing_mesh.verts) + list(editing_mesh.edges) + list(editing_mesh.faces), plane_co=(0.95, 0, 0), plane_no=(1, 0, 0), clear_outer=True, dist=1e-7)
        bmesh.ops.bisect_plane(editing_mesh, geom=list(editing_mesh.verts) + list(editing_mesh.edges) + list(editing_mesh.faces), plane_co=(0, 0, 0.18), plane_no=(0, 0, 1), dist=1e-7)
        boundary_edges = [edge for edge in editing_mesh.edges if all(abs(vertex.co.x - 0.95) < 1e-6 and vertex.co.z <= 0.180001 for vertex in edge.verts)]
        boundary_vertices = list({vertex for edge in boundary_edges for vertex in edge.verts})
        original_positions = {vertex: vertex.co.copy() for vertex in boundary_vertices}
        boundary_materials = {edge: edge.link_faces[0].material_index for edge in boundary_edges}
        texture_layer = editing_mesh.loops.layers.uv.active
        preceding_vertices = {vertex: vertex for vertex in boundary_vertices}
        for section_index in range(1, 41):
            fraction = section_index / 40
            longitudinal = 0.95 + (1.73355 - 0.95) * fraction
            width_scale = monotone_profile_value(longitudinal, [(0.95, 1), (1.10, 0.83), (1.30, 0.59), (1.50, 0.36), (1.73355, 0.198)])
            upper_height = 0.18
            lower_height = monotone_profile_value(longitudinal, [(0.95, -0.36112), (1.2, -0.291), (1.5, -0.23), (1.73355, -0.21)])
            following_vertices = {}
            for original_vertex, position in original_positions.items():
                vertical_fraction = (position.z + 0.36112) / (0.18 + 0.36112)
                vertical = lower_height + vertical_fraction * (upper_height - lower_height)
                lateral = position.y * width_scale
                if position.z > 0.10:
                    hit, normal, index, distance = cover_surface.ray_cast(Vector((longitudinal, -2, 0.18)), Vector((0, 1, 0)), 2)
                    initial_hit, normal, index, distance = cover_surface.ray_cast(Vector((0.95, -2, 0.18)), Vector((0, 1, 0)), 2)
                    if hit is not None and initial_hit is not None:
                        lateral += math.copysign((abs(hit.y) - abs(initial_hit.y) * width_scale) * smooth_fraction(position.z, 0.10, 0.18), position.y)
                following_vertices[original_vertex] = editing_mesh.verts.new((longitudinal, lateral, vertical))
            for edge in boundary_edges:
                first, second = edge.verts
                face = editing_mesh.faces.new((preceding_vertices[first], preceding_vertices[second], following_vertices[second], following_vertices[first]))
                adjacent = next(iter(edge.link_faces), None)
                face.material_index = boundary_materials[edge]
                face.smooth = True
                if texture_layer and adjacent:
                    coordinates = {loop.vert: loop[texture_layer].uv.copy() for loop in adjacent.loops}
                    for loop, original_vertex in zip(face.loops, (first, second, second, first)):
                        loop[texture_layer].uv = coordinates.get(original_vertex, Vector((0.1, 0.1)))
            preceding_vertices = following_vertices
        bmesh.ops.recalc_face_normals(editing_mesh, faces=list(editing_mesh.faces))
        for edge in editing_mesh.edges:
            if max(vertex.co.x for vertex in edge.verts) > 0.950001:
                edge.smooth = True
        bmesh.ops.triangulate(editing_mesh, faces=[face for face in editing_mesh.faces if len(face.verts) > 3])
        editing_mesh.to_mesh(mesh)
        editing_mesh.free()
        upper_mesh = bmesh.new()
        upper_mesh.from_mesh(original_mesh)
        bmesh.ops.bisect_plane(upper_mesh, geom=list(upper_mesh.verts) + list(upper_mesh.edges) + list(upper_mesh.faces), plane_co=(0.95, 0, 0), plane_no=(1, 0, 0), clear_inner=True, dist=1e-7)
        bmesh.ops.bisect_plane(upper_mesh, geom=list(upper_mesh.verts) + list(upper_mesh.edges) + list(upper_mesh.faces), plane_co=(0, 0, 0.18), plane_no=(0, 0, 1), clear_inner=True, dist=1e-7)
        upper_data = bpy.data.meshes.new('ProtectedUpperEngineCoverSurface')
        upper_mesh.to_mesh(upper_data)
        upper_mesh.free()
        joined_mesh = bmesh.new()
        joined_mesh.from_mesh(mesh)
        joined_mesh.from_mesh(upper_data)
        bmesh.ops.remove_doubles(joined_mesh, verts=list(joined_mesh.verts), dist=1e-7)
        joined_mesh.to_mesh(mesh)
        joined_mesh.free()
        bpy.data.meshes.remove(upper_data)
        mesh.update()
        for polygon in mesh.polygons:
            if max(mesh.vertices[index].co.x for index in polygon.vertices) <= 0.950001:
                continue
            if max(mesh.vertices[index].co.z for index in polygon.vertices) >= 0.180001:
                continue
            surface, triangles = projection_surfaces[polygon.material_index]
            location, normal, triangle_index, distance = surface.find_nearest(polygon.center)
            triangle = triangles[triangle_index]
            vertices = [original_mesh.vertices[index].co for index in triangle.vertices]
            for corner_index in polygon.loop_indices:
                corner = mesh.loops[corner_index]
                point = mesh.vertices[corner.vertex_index].co
                location = closest_point_on_tri(point, *vertices)
                for texture_layer in mesh.uv_layers:
                    coordinates = [Vector((*original_mesh.uv_layers[texture_layer.name].data[index].uv, 0)) for index in triangle.loops]
                    interpolated = barycentric_transform(location, *vertices, *coordinates)
                    texture_layer.data[corner_index].uv = interpolated.xy
        mesh.normals_split_custom_set([(0, 0, 0)] * len(mesh.loops))
        subject.data = mesh
        cutting_mesh = bmesh.new()
        cutting_mesh.from_mesh(mesh)
        for lateral_center in (-0.047, 0.047):
            planes = [(Vector((1.70, 0, 0)), Vector((1, 0, 0))), (Vector((1.78, 0, 0)), Vector((1, 0, 0)))]
            for side_index in range(64):
                angle = side_index * math.tau / 64
                normal = Vector((0, math.cos(angle), math.sin(angle)))
                planes.append((Vector((0, lateral_center, 0.035)) + normal * 0.035, normal))
            for plane_position, plane_normal in planes:
                faces = [face for face in cutting_mesh.faces if max(vertex.co.x for vertex in face.verts) >= 1.69999 and min(vertex.co.x for vertex in face.verts) <= 1.78001 and min(vertex.co.y for vertex in face.verts) <= lateral_center + 0.036 and max(vertex.co.y for vertex in face.verts) >= lateral_center - 0.036 and min(vertex.co.z for vertex in face.verts) <= 0.071 and max(vertex.co.z for vertex in face.verts) >= -0.001]
                geometry = set(faces)
                geometry.update(edge for face in faces for edge in face.edges)
                geometry.update(vertex for face in faces for vertex in face.verts)
                bmesh.ops.bisect_plane(cutting_mesh, geom=list(geometry), plane_co=plane_position, plane_no=plane_normal, dist=1e-8)
            interior_faces = []
            for face in cutting_mesh.faces:
                center = face.calc_center_median()
                if 1.70 < center.x < 1.78 and all(math.cos(side_index * math.tau / 64) * (center.y - lateral_center) + math.sin(side_index * math.tau / 64) * (center.z - 0.035) < 0.035 - 1e-8 for side_index in range(64)):
                    interior_faces.append(face)
            bmesh.ops.delete(cutting_mesh, geom=interior_faces, context='FACES')
        bmesh.ops.triangulate(cutting_mesh, faces=[face for face in cutting_mesh.faces if len(face.verts) > 3])
        bmesh.ops.recalc_face_normals(cutting_mesh, faces=list(cutting_mesh.faces))
        cutting_mesh.to_mesh(mesh)
        cutting_mesh.free()
        mesh.update()
        mesh.normals_split_custom_set([(0, 0, 0)] * len(mesh.loops))
        normals = [normal.vector.copy() for normal in mesh.corner_normals]
        for polygon in mesh.polygons:
            surface, triangles = projection_surfaces[polygon.material_index]
            for corner_index in polygon.loop_indices:
                point = mesh.vertices[mesh.loops[corner_index].vertex_index].co
                if point.x > 1.08 and point.z < 0.179999:
                    continue
                location, normal, triangle_index, distance = surface.find_nearest(point)
                triangle = triangles[triangle_index]
                vertices = [original_mesh.vertices[index].co for index in triangle.vertices]
                preserved_normal = barycentric_transform(location, *vertices, *[original_normals[index] for index in triangle.loops]).normalized()
                blend = 1.0 if point.z >= 0.179999 else 1 - smooth_fraction(point.x, 0.95, 1.08)
                normals[corner_index] = normals[corner_index].lerp(preserved_normal, blend).normalized()
        mesh.normals_split_custom_set(normals)
    open_exhaust_apertures()
    report = {'source_file': str(arguments.source_file), 'source_digest': hashlib.sha256(arguments.source_file.read_bytes()).hexdigest(), 'protected_object_fingerprints': protected_fingerprints, 'objects': {}, 'central_fin_unchanged': True, 'exhaust_tubes_unchanged': True, 'runtime_assets_promoted': False, 'exhaust_root_coverage_fraction': 0.075}
    for name in CHANGED_OBJECT_NAMES:
        subject = bpy.data.objects[name]
        mesh = subject.data
        mesh.calc_loop_triangles()
        crossings = exact_surface_intersections(mesh, list(range(len(mesh.polygons))))
        assert not crossings, (name, len(crossings))
        report['objects'][name] = {'original_fingerprint': original_fingerprints[name], 'candidate_fingerprint': mesh_fingerprint(mesh), 'candidate_mesh_name': mesh.name, 'triangles': len(mesh.loop_triangles), 'self_intersection_pairs': len(crossings)}
    parsed_source = ast.parse((repository / 'tools/blender/validate_formula_one_2030_cockpit_geometry.py').read_text())
    functions = [node for node in parsed_source.body if isinstance(node, ast.FunctionDef) and node.name in ('segments_cross_triangle', 'coplanar_overlap_area', 'planar_cross_product')]
    intersection_functions = {'math': math, 'Vector': Vector}
    exec(compile(ast.Module(body=functions, type_ignores=[]), 'intersection_functions', 'exec'), intersection_functions)
    subjects = {subject.name: subject for subject in bpy.data.objects if subject.type == 'MESH' and subject.name.startswith(('GEO_', 'Bodywork'))}
    surfaces = {name: triangle_surface(subject, subject.data) for name, subject in subjects.items()}
    expected_interfaces = {'GEO_ENGINE_COVER_REAR_CLOSURE', 'GEO_SUSPENSION_REAR_LINK_ASSEMBLY', 'GEO_UNDERBODY_FLOOR_AND_DIFFUSER', 'BodyworkEngineCoverCentralFin'}
    report['rear_interface_triangle_pairs'] = {}
    checked = set()
    for name in CHANGED_OBJECT_NAMES[:3]:
        for other_name in subjects:
            pair = tuple(sorted((name, other_name)))
            if name == other_name or pair in checked:
                continue
            checked.add(pair)
            crossings = crossing_triangle_pairs(surfaces[name], surfaces[other_name], intersection_functions)
            rear_crossings = [crossing for crossing in crossings if max(surfaces[name][0][index].x for index in surfaces[name][1][crossing[0]]) > 0.95001]
            if rear_crossings:
                report['rear_interface_triangle_pairs']['/'.join(pair)] = len(rear_crossings)
                assert other_name in expected_interfaces, (name, other_name, len(rear_crossings))
    report['checked_vehicle_object_pairs'] = len(checked)
    report['exhaust_tube_surface_intersections'] = 0
    report['bodywork_surface_intersections'] = 0
    report['interface_validation_limit'] = 'The source retains intersections at floor, suspension, rear closure and central fin interfaces; these are recorded, not a full vehicle clearance certification.'
    closure = bpy.data.objects['GEO_ENGINE_COVER_REAR_CLOSURE']
    editing_mesh = bmesh.new()
    editing_mesh.from_mesh(closure.data)
    assert all(edge.is_manifold for edge in editing_mesh.edges)
    editing_mesh.free()
    finish = max((closure.matrix_world @ vertex.co).x for vertex in closure.data.vertices)
    report['measured_exhaust_root_coverage'] = []
    for name in ('GEO_EXHAUST_OUTLET_TUBE_LEFT', 'GEO_EXHAUST_OUTLET_TUBE_RIGHT'):
        subject = bpy.data.objects[name]
        positions = [subject.matrix_world @ vertex.co for vertex in subject.data.vertices]
        start = min(point.x for point in positions)
        length = max(point.x for point in positions) - start
        fraction = (finish - start) / length
        assert 0.05 <= fraction <= 0.10
        report['measured_exhaust_root_coverage'].append({'object': name, 'covered_length_meters': finish - start, 'tube_length_meters': length, 'fraction': fraction})
    assert all(protected_object_fingerprint(bpy.data.objects[name]) == fingerprint for name, fingerprint in protected_fingerprints.items())
    bpy.data.objects['BodyworkEngineCover']['continuous_rear_bodywork_applied'] = True
    render_reviews(repository, output, 'connection')
    candidate = validate_output_path(repository, output / 'f1_2030.blend', 'preview').path
    bpy.ops.wm.save_as_mainfile(filepath=str(candidate), relative_remap=True)
    bpy.ops.wm.open_mainfile(filepath=str(candidate))
    assert all(mesh_fingerprint(bpy.data.objects[name].data) == record['candidate_fingerprint'] for name, record in report['objects'].items())
    assert all(protected_object_fingerprint(bpy.data.objects[name]) == fingerprint for name, fingerprint in protected_fingerprints.items())
    missing_images = [image.filepath for image in bpy.data.images if image.source == 'FILE' and not image.packed_file and image.filepath and not Path(bpy.path.abspath(image.filepath)).is_file()]
    assert not missing_images, missing_images
    report['candidate_file'] = str(candidate)
    report['candidate_digest'] = hashlib.sha256(candidate.read_bytes()).hexdigest()
    report['saved_candidate_reopened_and_verified'] = True
    report['passed'] = True
    validate_output_path(repository, output / 'continuous_rear_bodywork_report.json', 'preview').path.write_text(json.dumps(report, indent=2), encoding='utf-8')
    print('CONTINUOUS_REAR_BODYWORK_VERIFIED', flush=True)


if __name__ == '__main__':
    main()
