import argparse
import json
import math
import sys
from pathlib import Path

import bpy
import bmesh
from mathutils import Vector
from mathutils.bvhtree import BVHTree

PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools'))
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools/blender'))
from common.output_policy import validate_output_path


def segments_cross_triangle(first_positions, second_positions):
    def subtract_positions(first, second):
        return tuple(float(first[index]) - float(second[index]) for index in range(3))

    def cross_vectors(first, second):
        return (first[1] * second[2] - first[2] * second[1], first[2] * second[0] - first[0] * second[2], first[0] * second[1] - first[1] * second[0])

    def dot_vectors(first, second):
        return sum(first[index] * second[index] for index in range(3))

    first_edge = subtract_positions(second_positions[1], second_positions[0])
    second_edge = subtract_positions(second_positions[2], second_positions[0])
    triangle_normal = cross_vectors(first_edge, second_edge)
    doubled_area = math.sqrt(dot_vectors(triangle_normal, triangle_normal))
    if doubled_area < 1e-20:
        return False
    for start, finish in zip(first_positions, first_positions[1:] + first_positions[:1]):
        direction = subtract_positions(finish, start)
        segment_length = math.sqrt(dot_vectors(direction, direction))
        perpendicular = cross_vectors(direction, second_edge)
        determinant = dot_vectors(first_edge, perpendicular)
        if segment_length < 1e-10 or abs(determinant) < doubled_area * segment_length * 1e-10:
            continue
        relative = subtract_positions(start, second_positions[0])
        first_fraction = dot_vectors(relative, perpendicular) / determinant
        relative_cross = cross_vectors(relative, first_edge)
        second_fraction = dot_vectors(direction, relative_cross) / determinant
        segment_fraction = dot_vectors(second_edge, relative_cross) / determinant
        third_edge = subtract_positions(second_positions[2], second_positions[1])
        distances = (
            first_fraction * doubled_area / math.sqrt(dot_vectors(second_edge, second_edge)),
            second_fraction * doubled_area / math.sqrt(dot_vectors(first_edge, first_edge)),
            (1 - first_fraction - second_fraction) * doubled_area / math.sqrt(dot_vectors(third_edge, third_edge)),
            min(segment_fraction, 1 - segment_fraction) * segment_length,
        )
        if min(distances) > 0.0000005:
            return True
    return False


def planar_cross_product(first, second):
    return first[0] * second[1] - first[1] * second[0]


def coplanar_overlap_area(first_positions, second_positions):
    normal = (first_positions[1] - first_positions[0]).cross(first_positions[2] - first_positions[0]).normalized()
    if normal.length < 0.5 or max(abs((position - first_positions[0]).dot(normal)) for position in second_positions) > 1e-8:
        return 0
    omitted_axis = max(range(3), key=lambda index: abs(normal[index]))
    axes = [index for index in range(3) if index != omitted_axis]
    subject = [Vector((position[axes[0]], position[axes[1]])) for position in first_positions]
    clip = [Vector((position[axes[0]], position[axes[1]])) for position in second_positions]
    orientation = 1 if planar_cross_product(clip[1] - clip[0], clip[2] - clip[0]) >= 0 else -1
    for start, finish in zip(clip, clip[1:] + clip[:1]):
        previous_polygon = subject
        subject = []
        if not previous_polygon:
            return 0
        previous = previous_polygon[-1]
        previous_distance = orientation * planar_cross_product(finish - start, previous - start)
        for current in previous_polygon:
            current_distance = orientation * planar_cross_product(finish - start, current - start)
            if (current_distance >= 0) != (previous_distance >= 0):
                subject.append(previous.lerp(current, previous_distance / (previous_distance - current_distance)))
            if current_distance >= 0:
                subject.append(current)
            previous = current
            previous_distance = current_distance
    return abs(sum(planar_cross_product(first, second) for first, second in zip(subject, subject[1:] + subject[:1]))) / 2


def inspect_mesh_geometry(scene_object):
    mesh = scene_object.data
    mesh.calc_loop_triangles()
    positions = [scene_object.matrix_world @ vertex.co for vertex in mesh.vertices]
    triangles = [tuple(triangle.vertices) for triangle in mesh.loop_triangles]
    surface = BVHTree.FromPolygons(positions, triangles, all_triangles=True)
    crossing_pairs = []
    coplanar_pairs = []
    inspected_pairs = 0
    candidate_pairs = set(tuple(sorted(pair)) for pair in surface.overlap(surface))
    vertex_triangles = {}
    for triangle_index, triangle in enumerate(triangles):
        for vertex_index in triangle:
            vertex_triangles.setdefault(vertex_index, []).append(triangle_index)
    for connected_triangles in vertex_triangles.values():
        for first_index in connected_triangles:
            for second_index in connected_triangles:
                if first_index < second_index:
                    candidate_pairs.add((first_index, second_index))
    for first_index, second_index in sorted(candidate_pairs):
        if first_index >= second_index or mesh.loop_triangles[first_index].polygon_index == mesh.loop_triangles[second_index].polygon_index:
            continue
        inspected_pairs += 1
        first_positions = [positions[index] for index in triangles[first_index]]
        second_positions = [positions[index] for index in triangles[second_index]]
        if segments_cross_triangle(first_positions, second_positions) or segments_cross_triangle(second_positions, first_positions):
            crossing_pairs.append([first_index, second_index])
        elif coplanar_overlap_area(first_positions, second_positions) > 1e-10:
            coplanar_pairs.append([first_index, second_index])
    topology = bmesh.new()
    topology.from_mesh(mesh)
    measurements = {
        'object': scene_object.name,
        'vertices': len(mesh.vertices),
        'polygons': len(mesh.polygons),
        'triangles': len(triangles),
        'boundary_edges': sum(edge.is_boundary for edge in topology.edges),
        'nonmanifold_edges': sum(not edge.is_manifold for edge in topology.edges),
        'inconsistent_winding_edges': sum(edge.is_manifold and not edge.is_contiguous for edge in topology.edges),
        'zero_area_faces': sum(face.calc_area() < 1e-12 for face in topology.faces),
        'zero_length_edges': sum(edge.calc_length() < 1e-8 for edge in topology.edges),
        'minimum_triangle_area_square_meters': min(triangle.area for triangle in mesh.loop_triangles),
        'signed_volume_cubic_meters': topology.calc_volume(signed=True),
        'bounding_pairs_inspected_including_adjacent_triangles': inspected_pairs,
        'intersection_boundary_tolerance_meters': 0.0000005,
        'triangle_crossing_pairs': crossing_pairs,
        'coplanar_overlap_pairs': coplanar_pairs,
    }
    topology.free()
    measurements['passed'] = not any(measurements[name] for name in ('boundary_edges', 'nonmanifold_edges', 'inconsistent_winding_edges', 'zero_area_faces', 'zero_length_edges', 'triangle_crossing_pairs', 'coplanar_overlap_pairs')) and measurements['signed_volume_cubic_meters'] > 0
    return measurements


def inspect_contour_attachment(contour, surface):
    edges = []
    maximum_distance = 0
    sample_count = 0
    perimeter_length = 0
    for index, (first, second) in enumerate(zip(contour, contour[1:] + contour[:1])):
        length = (second - first).length
        subdivisions = max(1, math.ceil(length / 0.001))
        edge_maximum = 0
        for sample_index in range(subdivisions + 1):
            sample = first.lerp(second, sample_index / subdivisions)
            nearest = surface.find_nearest(sample)
            if nearest[0] is None:
                raise RuntimeError('Contour sample has no lining surface')
            edge_maximum = max(edge_maximum, nearest[3])
        edges.append({'edge': index, 'length_meters': length, 'samples': subdivisions + 1, 'maximum_distance_meters': edge_maximum})
        maximum_distance = max(maximum_distance, edge_maximum)
        sample_count += subdivisions + 1
        perimeter_length += length
    return {'edges': len(edges), 'perimeter_length_meters': perimeter_length, 'samples': sample_count, 'maximum_sample_step_meters': 0.001, 'maximum_distance_meters': maximum_distance, 'edge_measurements': edges, 'passed': maximum_distance < 0.00002}


def validate_cockpit_geometry():
    from seal_formula_one_2030_cockpit_interior import cockpit_opening_contour, surface_for_object
    lining = bpy.data.objects['GEO_CHASSIS_CockpitUpperLining']
    interior = bpy.data.objects['GEO_CHASSIS_INTERIOR']
    opening = cockpit_opening_contour()
    lower_contour = [interior.matrix_world @ interior.data.vertices[row * 33].co for row in range(71)]
    lower_contour += [interior.matrix_world @ interior.data.vertices[row * 33 + 32].co for row in reversed(range(71))]
    surface = surface_for_object(lining)
    lining_measurements = inspect_mesh_geometry(lining)
    interior_measurements = inspect_mesh_geometry(interior)
    opening_measurements = inspect_contour_attachment(opening, surface)
    lower_measurements = inspect_contour_attachment(lower_contour, surface)
    body_surface = surface_for_object(bpy.data.objects['GEO_CHASSIS_BODY'])
    roof_clearance_samples = []
    for vertex in lining.data.vertices:
        position = lining.matrix_world @ vertex.co
        if position.x >= min(point.x for point in opening) - 0.01:
            continue
        location, normal, polygon_index, distance = body_surface.ray_cast(Vector((position.x, position.y, 0.02)), Vector((0, 0, 1)), 0.4)
        if location is None or normal.z <= 0.2:
            raise RuntimeError('Footwell roof has no outward body surface above its footprint')
        roof_clearance_samples.append(location.z - position.z)
    vertex_count = len(lining.data.vertices) // 2
    displacements = [(lining.data.vertices[index + vertex_count].co - lining.data.vertices[index].co).length for index in range(vertex_count)]
    roof_measurements = {'samples':len(roof_clearance_samples), 'minimum_vertical_distance_to_exterior_body_meters':min(roof_clearance_samples), 'passed':min(roof_clearance_samples)>0}
    report = {'lining': lining_measurements, 'interior': interior_measurements, 'opening_perimeter': opening_measurements, 'interior_perimeter': lower_measurements, 'footwell_exterior_clearance':roof_measurements, 'vertical_shell_separation_meters': {'minimum': min(displacements), 'maximum': max(displacements), 'construction': 'Upper skin translated vertically by 1.5 millimeters; normal thickness varies with surface slope'}}
    report['passed'] = all(measurements['passed'] for measurements in (lining_measurements, interior_measurements, opening_measurements, lower_measurements, roof_measurements))
    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output-report', required=True)
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
    report = validate_cockpit_geometry()
    destination = validate_output_path(PROJECT_DIRECTORY, options.output_report, 'preview').path
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'passed':report['passed'],'lining_crossings':len(report['lining']['triangle_crossing_pairs']),'interior_crossings':len(report['interior']['triangle_crossing_pairs']),'opening_perimeter_samples':report['opening_perimeter']['samples'],'opening_maximum_gap_meters':report['opening_perimeter']['maximum_distance_meters'],'interior_perimeter_samples':report['interior_perimeter']['samples'],'interior_maximum_gap_meters':report['interior_perimeter']['maximum_distance_meters']},indent=2),flush=True)
    if not report['passed']:
        raise RuntimeError('Cockpit geometry or full perimeter validation failed')


if __name__ == '__main__':
    main()
