import argparse
import hashlib
import json
import math
import subprocess
import sys
from pathlib import Path
from functools import lru_cache

import bpy
import bmesh
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools'))
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools/blender'))
from common.output_policy import validate_output_path
from export_f1_2030_v10 import export_chassis
from fit_f1_2030_driver_seat import mesh_fingerprint
from fit_f1_2030_cockpit_clearance import driver_capture_provenance
from shade_formula_one_2030_cockpit_surfaces import shade_surface_with_preserved_rims
from seal_formula_one_2030_cockpit_interior import surface_for_object, validate_side_sealing, validate_steering_sweep
from validate_formula_one_2030_cockpit_geometry import validate_cockpit_geometry


def triangle_samples(positions, triangles, subdivisions):
    samples = []
    for triangle in triangles:
        first, second, third = [positions[index] for index in triangle]
        for first_fraction in range(subdivisions + 1):
            for second_fraction in range(subdivisions + 1 - first_fraction):
                samples.append(first + (second - first) * (first_fraction / subdivisions) + (third - first) * (second_fraction / subdivisions))
    return samples


def topology_surface(topology):
    topology.verts.index_update()
    positions = [vertex.co.copy() for vertex in topology.verts]
    triangles = [tuple(vertex.index for vertex in face.verts) for face in topology.faces]
    return BVHTree.FromPolygons(positions, triangles, all_triangles=True), positions, triangles


def edge_vertex_identifiers(edge, identifier_layer):
    return tuple(sorted(vertex[identifier_layer] for vertex in edge.verts))


def triangulate_vertex_neighborhood(vertex, reference_surface):
    connections = {}
    for face in vertex.link_faces:
        if len(face.verts) != 3:
            return None
        for edge in face.edges:
            if vertex not in edge.verts:
                first, second = edge.verts
                connections.setdefault(first, []).append(second)
                connections.setdefault(second, []).append(first)
    if any(len(neighbors) != 2 for neighbors in connections.values()):
        return None
    first_vertex = next(iter(connections))
    ring = [first_vertex]
    previous_vertex = None
    current_vertex = first_vertex
    while True:
        following_vertex = next(neighbor for neighbor in connections[current_vertex] if neighbor != previous_vertex)
        if following_vertex == first_vertex:
            break
        if following_vertex in ring:
            return None
        ring.append(following_vertex)
        previous_vertex, current_vertex = current_vertex, following_vertex
    normal = sum((face.normal for face in vertex.link_faces), Vector()).normalized()
    polygon_normal = sum((first.co.cross(second.co) for first, second in zip(ring, ring[1:] + ring[:1])), Vector())
    if polygon_normal.dot(normal) < 0:
        ring.reverse()
    dominant_axis = max(range(3), key=lambda axis:abs(normal[axis]))
    projected_axes = [axis for axis in range(3) if axis != dominant_axis]
    projected = [Vector((neighbor.co[projected_axes[0]], neighbor.co[projected_axes[1]])) for neighbor in ring]
    def cross_product(first, second, third):
        return (second.x - first.x) * (third.y - first.y) - (second.y - first.y) * (third.x - first.x)
    signed_area = sum(first.x * second.y - second.x * first.y for first, second in zip(projected, projected[1:] + projected[:1]))
    orientation = 1 if signed_area > 0 else -1
    def point_inside(point):
        inside = False
        for first, second in zip(projected, projected[1:] + projected[:1]):
            if (first.y > point.y) != (second.y > point.y) and point.x < (second.x - first.x) * (point.y - first.y) / (second.y - first.y) + first.x:
                inside = not inside
        return inside
    @lru_cache(None)
    def diagonal_valid(first_index, second_index):
        if second_index - first_index == 1 or (first_index == 0 and second_index == len(ring) - 1):
            return True
        first, second = projected[first_index], projected[second_index]
        if not point_inside((first + second) / 2):
            return False
        for edge_index in range(len(ring)):
            following_index = (edge_index + 1) % len(ring)
            if first_index in (edge_index, following_index) or second_index in (edge_index, following_index):
                continue
            start, end = projected[edge_index], projected[following_index]
            if cross_product(first, second, start) * cross_product(first, second, end) < -1e-20 and cross_product(start, end, first) * cross_product(start, end, second) < -1e-20:
                return False
        return True
    @lru_cache(None)
    def solve(first_index, last_index):
        if last_index - first_index < 2:
            return (0.0, 0.0, [])
        if not diagonal_valid(first_index, last_index):
            return None
        best = None
        for middle_index in range(first_index + 1, last_index):
            if cross_product(projected[first_index], projected[middle_index], projected[last_index]) * orientation <= 1e-12:
                continue
            first_part = solve(first_index, middle_index)
            second_part = solve(middle_index, last_index)
            if first_part is None or second_part is None:
                continue
            positions = [ring[index].co for index in (first_index, middle_index, last_index)]
            area = (positions[1] - positions[0]).cross(positions[2] - positions[0]).length / 2
            if area < 1e-12:
                continue
            distance = max(reference_surface.find_nearest(sample)[3] for sample in triangle_samples(positions, [(0, 1, 2)], 4))
            quality = sum((first - second).length_squared for first, second in zip(positions, positions[1:] + positions[:1])) / area
            cost = (max(first_part[0], second_part[0], distance), first_part[1] + second_part[1] + quality, first_part[2] + second_part[2] + [tuple(ring[index] for index in (first_index, middle_index, last_index))])
            if best is None or cost[:2] < best[:2]:
                best = cost
        return best
    result = solve(0, len(ring) - 1)
    return result[2] if result else None


def reconstruct_lining(lining, maximum_deviation):
    mesh = lining.data
    original_vertex_count = len(mesh.vertices)
    mesh.calc_loop_triangles()
    original_triangle_count = len(mesh.loop_triangles)
    skin_vertex_count = len(mesh.vertices) // 2
    original_positions = [vertex.co.copy() for vertex in mesh.vertices[:skin_vertex_count]]
    original_faces = [tuple(polygon.vertices) for polygon in mesh.polygons if all(index < skin_vertex_count for index in polygon.vertices)]
    original_surface = BVHTree.FromPolygons(original_positions, original_faces, all_triangles=True)
    source_samples = triangle_samples(original_positions, original_faces, 8)
    topology = bmesh.new()
    identifier_layer = topology.verts.layers.int.new('OriginalSurfaceVertexIdentifier')
    vertices = [topology.verts.new(position) for position in original_positions]
    for index, vertex in enumerate(vertices):
        vertex[identifier_layer] = index
    for face in original_faces:
        topology.faces.new([vertices[index] for index in face])
    topology.normal_update()
    original_boundary = {edge_vertex_identifiers(edge, identifier_layer) for edge in topology.edges if edge.is_boundary}
    original_sharp_pairs = {tuple(sorted(edge.vertices)) for edge in mesh.edges if edge.use_edge_sharp and all(index < skin_vertex_count for index in edge.vertices)}
    original_creases = {edge_vertex_identifiers(edge, identifier_layer) for edge in topology.edges if edge.is_manifold and (edge.calc_face_angle() > math.radians(35) or edge_vertex_identifiers(edge, identifier_layer) in original_sharp_pairs)}
    protected_vertices = {index for edge in original_boundary | original_creases for index in edge}
    candidates = [(max((face.normal - vertex.normal).length for face in vertex.link_faces), vertex[identifier_layer]) for vertex in topology.verts if vertex[identifier_layer] not in protected_vertices]
    removed_identifiers = []
    rejections = {}
    for candidate_number, (curvature, identifier) in enumerate(sorted(candidates)):
        proposal = topology.copy()
        proposal_layer = proposal.verts.layers.int['OriginalSurfaceVertexIdentifier']
        proposal_vertex = next(vertex for vertex in proposal.verts if vertex[proposal_layer] == identifier)
        replacement_triangles = triangulate_vertex_neighborhood(proposal_vertex, original_surface)
        if replacement_triangles is None:
            proposal.free()
            rejections['complex_vertex_neighborhood'] = rejections.get('complex_vertex_neighborhood', 0) + 1
            continue
        bmesh.ops.delete(proposal, geom=[proposal_vertex], context='VERTS')
        for triangle in replacement_triangles:
            proposal.faces.new(triangle)
        proposal.normal_update()
        boundary = {edge_vertex_identifiers(edge, proposal_layer) for edge in proposal.edges if edge.is_boundary}
        all_edges = {edge_vertex_identifiers(edge, proposal_layer) for edge in proposal.edges}
        rejection = None
        if boundary != original_boundary or not original_creases.issubset(all_edges):
            rejection = 'protected_contour_or_crease'
        elif any(face.calc_area() < 1e-12 for face in proposal.faces) or any(edge.is_manifold and not edge.is_contiguous for edge in proposal.edges) or any(not edge.is_manifold and not edge.is_boundary for edge in proposal.edges):
            rejection = 'topology_or_degenerate_face'
        else:
            proposal_surface, proposal_positions, proposal_triangles = topology_surface(proposal)
            maximum_source_distance = max(proposal_surface.find_nearest(sample)[3] for sample in source_samples)
            if maximum_source_distance > maximum_deviation * 0.9:
                rejection = 'source_surface_deviation'
            else:
                proposal_samples = triangle_samples(proposal_positions, proposal_triangles, 8)
                maximum_proposal_distance = max(original_surface.find_nearest(sample)[3] for sample in proposal_samples)
                if maximum_proposal_distance > maximum_deviation * 0.9:
                    rejection = 'reconstructed_surface_deviation'
        if rejection:
            rejections[rejection] = rejections.get(rejection, 0) + 1
            proposal.free()
        else:
            topology.free()
            topology = proposal
            removed_identifiers.append(identifier)
        if candidate_number % 20 == 0:
            print('RECONSTRUCTION', candidate_number + 1, '/', len(candidates), 'removed', len(removed_identifiers), flush=True)
    final_surface, final_positions, final_faces = topology_surface(topology)
    final_layer = topology.verts.layers.int['OriginalSurfaceVertexIdentifier']
    final_identifiers = [vertex[final_layer] for vertex in topology.verts]
    final_boundary = [tuple(vertex.index for vertex in edge.verts) for edge in topology.edges if edge.is_boundary]
    final_dense_samples = triangle_samples(final_positions, final_faces, 12)
    original_dense_samples = triangle_samples(original_positions, original_faces, 12)
    source_distance = max(final_surface.find_nearest(sample)[3] for sample in original_dense_samples)
    reconstructed_distance = max(original_surface.find_nearest(sample)[3] for sample in final_dense_samples)
    print('DENSE_DEVIATION', source_distance, reconstructed_distance, 'REJECTIONS', rejections, flush=True)
    if max(source_distance, reconstructed_distance) > maximum_deviation:
        raise RuntimeError('Dense surface deviation validation failed')
    topology.free()
    shell_vertex_count = len(final_positions)
    shell_positions = final_positions + [position + Vector((0, 0, 0.0015)) for position in final_positions]
    shell_faces = final_faces + [tuple(index + shell_vertex_count for index in reversed(face)) for face in final_faces]
    shell_faces += [(first, second, second + shell_vertex_count, first + shell_vertex_count) for first, second in final_boundary]
    texture_layer_names = [layer.name for layer in mesh.uv_layers]
    if texture_layer_names != ['CockpitLiningTextureCoordinates']:
        raise RuntimeError('Unexpected lining texture coordinates')
    for polygon in mesh.polygons:
        for loop_index in polygon.loop_indices:
            position = mesh.vertices[mesh.loops[loop_index].vertex_index].co
            expected = Vector(((position.x + 1.4) / 2, (position.y + 0.4) / 0.8))
            if (mesh.uv_layers[0].data[loop_index].uv - expected).length > 2e-7:
                raise RuntimeError('Source texture mapping is not the expected continuous projection')
    mesh.clear_geometry()
    mesh.from_pydata(shell_positions, [], shell_faces)
    shell_topology = bmesh.new()
    shell_topology.from_mesh(mesh)
    bmesh.ops.recalc_face_normals(shell_topology, faces=list(shell_topology.faces))
    if shell_topology.calc_volume(signed=True) < 0:
        bmesh.ops.reverse_faces(shell_topology, faces=list(shell_topology.faces))
    shell_topology.to_mesh(mesh)
    shell_topology.free()
    texture_coordinates = mesh.uv_layers.get(texture_layer_names[0]) or mesh.uv_layers.new(name=texture_layer_names[0])
    for loop_index, loop in enumerate(mesh.loops):
        position = mesh.vertices[loop.vertex_index].co
        texture_coordinates.data[loop_index].uv = ((position.x + 1.4) / 2, (position.y + 0.4) / 0.8)
    mesh.update()
    preserved_crease_edge_indices = []
    for edge in mesh.edges:
        if all(index < shell_vertex_count for index in edge.vertices) or all(index >= shell_vertex_count for index in edge.vertices):
            original_identifiers = tuple(sorted(final_identifiers[index % shell_vertex_count] for index in edge.vertices))
            if original_identifiers in original_creases:
                preserved_crease_edge_indices.append(edge.index)
    if len(preserved_crease_edge_indices) != len(original_creases) * 2:
        raise RuntimeError('Original crease edges were lost from the reconstructed shell')
    lining['cockpit_lining_preserved_crease_vertex_pairs'] = [index for edge_index in preserved_crease_edge_indices for index in mesh.edges[edge_index].vertices]
    shading = shade_surface_with_preserved_rims(lining, preserved_crease_edge_indices)
    return {'original_vertices':original_vertex_count, 'vertices':len(mesh.vertices), 'original_triangles':original_triangle_count, 'triangles':shading['triangles'], 'removed_skin_vertices':len(removed_identifiers), 'evaluated_skin_vertices':len(candidates), 'removed_original_skin_vertex_identifiers':removed_identifiers, 'retained_original_skin_vertex_identifiers':final_identifiers, 'locked_boundary_edges':len(original_boundary), 'locked_crease_edges':len(original_creases), 'all_boundary_segments_and_crease_edges_preserved':True, 'maximum_allowed_surface_deviation_meters':maximum_deviation, 'maximum_source_to_reconstruction_distance_meters':source_distance, 'maximum_reconstruction_to_source_distance_meters':reconstructed_distance, 'source_surface_samples':len(original_dense_samples), 'reconstructed_surface_samples':len(final_dense_samples), 'sample_barycentric_subdivisions':12, 'rejected_vertex_removals':rejections, 'texture_projection_preserved':True, 'shading':shading}


def validate_driver_clearance(lining, driver_path, conversion):
    provenance = json.loads(driver_path.with_suffix('.provenance.json').read_text())
    if any(provenance.get(name) != value for name, value in driver_capture_provenance().items()):
        raise RuntimeError('Driver animation capture does not match current sources')
    for relative_path, expected_digest in provenance['camera_dependencies_sha256'].items():
        if hashlib.sha256((PROJECT_DIRECTORY / 'game' / relative_path).read_bytes()).hexdigest() != expected_digest:
            raise RuntimeError('Driver camera dependency changed')
    if hashlib.sha256(driver_path.read_bytes()).hexdigest() != provenance['surfaces_sha256'] or provenance['seated_position'] != [0, -0.011, -0.24]:
        raise RuntimeError('Driver capture content or position differs from recorded provenance')
    captured = json.loads(driver_path.read_text())
    if len(captured['samples']) < 1000 or min(sample['steering_degrees'] for sample in captured['samples']) != -180 or max(sample['steering_degrees'] for sample in captured['samples']) != 180:
        raise RuntimeError('Driver animation capture does not cover the required steering sequence')
    lining_surface = surface_for_object(lining, conversion)
    failures = []
    for sample_index, sample in enumerate(captured['samples']):
        driver_surface = BVHTree.FromPolygons([Vector(position) for position in sample['vertices']], sample['triangles'], all_triangles=True)
        intersections = lining_surface.overlap(driver_surface)
        if intersections:
            failures.append({'sample':sample_index, 'steering_degrees':sample['steering_degrees'], 'pairs':len(intersections)})
        if sample_index % 100 == 0:
            print('DRIVER_VALIDATION', sample_index + 1, '/', len(captured['samples']), 'failures', len(failures), flush=True)
    return {'samples':len(captured['samples']), 'intersection_samples':failures, 'capture_provenance':provenance, 'passed':not failures}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--output-directory', required=True)
    parser.add_argument('--driver-surfaces', type=Path)
    parser.add_argument('--maximum-deviation-millimeters', type=float, default=0.1)
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
    if not 0 < options.maximum_deviation_millimeters <= 0.25:
        raise RuntimeError('Surface deviation must be positive and no greater than 0.25 millimeters')
    destination = validate_output_path(PROJECT_DIRECTORY, options.output_directory, 'preview').path
    destination.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=str(options.source_file.resolve()))
    protected_meshes = {scene_object.name:mesh_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.type == 'MESH' and scene_object.name != 'GEO_CHASSIS_CockpitUpperLining'}
    report = {'source_before_sha256':hashlib.sha256(options.source_file.read_bytes()).hexdigest(), 'source_commit':subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=PROJECT_DIRECTORY, text=True).strip(), 'source_branch':subprocess.check_output(['git', 'branch', '--show-current'], cwd=PROJECT_DIRECTORY, text=True).strip(), 'source_status':subprocess.check_output(['git', 'status', '--short'], cwd=PROJECT_DIRECTORY, text=True).splitlines(), 'protected_mesh_fingerprints':protected_meshes}
    lining = bpy.data.objects['GEO_CHASSIS_CockpitUpperLining']
    if lining.get('cockpit_lining_preserved_crease_vertex_pairs'):
        raise RuntimeError('This lining was already reconstructed; use the texture-resolved source_before_reconstruction.blend recorded by the reconstruction report')
    report['backup_texture_resolved_source'] = str(destination / 'source_before_reconstruction.blend')
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(validate_output_path(PROJECT_DIRECTORY, destination / 'source_before_reconstruction.blend', 'preview').path), relative_remap=True)
    report['lining'] = reconstruct_lining(lining, options.maximum_deviation_millimeters / 1000)
    report['full_geometry_validation'] = validate_cockpit_geometry()
    manifest = json.loads((PROJECT_DIRECTORY / 'game/assets/models/vehicles/f1-2030/manifest.json').read_text())
    conversion = Matrix.Translation((0, 0, manifest['authored_alignment']['chassis_vertical_offset'])) @ Matrix.Rotation(-math.pi / 2, 4, 'Z')
    lining_surface = surface_for_object(lining, conversion)
    report['seat_intersection_pairs'] = len(lining_surface.overlap(surface_for_object(bpy.data.objects['GEO_CHASSIS_SEAT'], conversion)))
    report['steering_intersections'] = validate_steering_sweep(lining_surface, conversion)
    report['sealed_side_samples'] = validate_side_sealing(lining)
    report['protected_meshes_unchanged'] = all(mesh_fingerprint(bpy.data.objects[name]) == fingerprint for name, fingerprint in protected_meshes.items())
    report['driver_validation'] = validate_driver_clearance(lining, options.driver_surfaces.resolve(), conversion) if options.driver_surfaces else {'passed':False, 'pending':True}
    report['geometry_passed'] = report['full_geometry_validation']['passed'] and not report['seat_intersection_pairs'] and not report['steering_intersections'] and all(sample['sealed'] for sample in report['sealed_side_samples']) and report['protected_meshes_unchanged']
    report['passed'] = report['geometry_passed'] and report['driver_validation']['passed']
    report_path = validate_output_path(PROJECT_DIRECTORY, destination / 'cockpit_lining_reconstruction_report.json', 'preview').path
    report_path.write_text(json.dumps(report, indent=2) + '\n')
    if not report['geometry_passed'] or (options.driver_surfaces and not report['passed']):
        raise RuntimeError('Lining reconstruction failed validation; inspect the report')
    bpy.context.preferences.filepaths.save_version = 0
    candidate_path = validate_output_path(PROJECT_DIRECTORY, destination / 'f1_2030_reconstructed_lining.blend', 'preview').path
    bpy.ops.wm.save_as_mainfile(filepath=str(candidate_path), relative_remap=True)
    chassis_path = validate_output_path(PROJECT_DIRECTORY, destination / 'f1_2030_v10_chassis.glb', 'preview').path
    report['chassis_mesh_statistics'] = export_chassis(chassis_path, manifest['authored_alignment']['chassis_vertical_offset'])
    report['candidate_sha256'] = hashlib.sha256(candidate_path.read_bytes()).hexdigest()
    report['chassis_sha256'] = hashlib.sha256(chassis_path.read_bytes()).hexdigest()
    report_path.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'passed':report['passed'], 'geometry_passed':report['geometry_passed'], 'triangles_before':report['lining']['original_triangles'], 'triangles_after':report['lining']['triangles'], 'maximum_surface_deviation_millimeters':max(report['lining']['maximum_source_to_reconstruction_distance_meters'], report['lining']['maximum_reconstruction_to_source_distance_meters']) * 1000, 'protected_meshes':len(protected_meshes)}), flush=True)


if __name__ == '__main__':
    main()
