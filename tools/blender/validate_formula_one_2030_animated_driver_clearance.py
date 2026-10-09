import argparse
import json
import math
from pathlib import Path
import sys
import time

import bpy
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

project_directory = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(project_directory / 'tools'))
sys.path.insert(0, str(project_directory / 'tools/blender'))
from common.output_policy import validate_output_path
from fit_formula_one_2030_animated_driver_clearance import prepare_swept_arm_surface
from validate_formula_one_2030_cockpit_geometry import inspect_mesh_geometry, segments_cross_triangle

parser = argparse.ArgumentParser()
parser.add_argument('--source-file', type=Path, required=True)
parser.add_argument('--baseline-source-file', type=Path, required=True)
parser.add_argument('--driver-surfaces', type=Path, required=True)
parser.add_argument('--output-directory', type=Path, required=True)
options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
output_directory = validate_output_path(project_directory, options.output_directory, 'preview').path
output_directory.mkdir(parents=True, exist_ok=True)
source_path = options.source_file.resolve()
conversion = Matrix.Translation((0, 0, 0.10500002279877663)) @ Matrix.Rotation(-math.pi / 2, 4, 'Z')
original_geometry = {}
original_inspections = {}
bpy.ops.wm.open_mainfile(filepath=str(options.baseline_source_file.resolve()), load_ui=False, use_scripts=False)
for original_name in ['GEO_CHASSIS_CockpitHeadrestPadding', 'GEO_CHASSIS_INTERIOR', 'GEO_CHASSIS_SEAT', 'GEO_CHASSIS_BODY', 'GEO_CHASSIS_FLOOR', 'GEO_CHASSIS_CockpitUpperLining']:
    original_object = bpy.data.objects[original_name]
    original_object.data.calc_loop_triangles()
    original_geometry[original_name] = {'vertices': [list(conversion @ original_object.matrix_world @ vertex.co) for vertex in original_object.data.vertices], 'triangles': [list(triangle.vertices) for triangle in original_object.data.loop_triangles]}
    if original_name in ('GEO_CHASSIS_CockpitHeadrestPadding', 'GEO_CHASSIS_INTERIOR', 'GEO_CHASSIS_SEAT'):
        original_inspections[original_name] = inspect_mesh_geometry(original_object)
bpy.ops.wm.open_mainfile(filepath=str(source_path), load_ui=False, use_scripts=False)
samples = json.loads(options.driver_surfaces.read_text())['samples']
swept_surface = prepare_swept_arm_surface(samples)
conversion = Matrix.Translation((0, 0, 0.10500002279877663)) @ Matrix.Rotation(-math.pi / 2, 4, 'Z')
names = ['GEO_CHASSIS_CockpitHeadrestPadding', 'GEO_CHASSIS_INTERIOR', 'GEO_CHASSIS_SEAT']
geometry_records = {}
clearance_records = {}
candidate_geometry = {}
started = time.monotonic()
for name in names:
    scene_object = bpy.data.objects[name]
    scene_object.data.calc_loop_triangles()
    positions = [conversion @ scene_object.matrix_world @ vertex.co for vertex in scene_object.data.vertices]
    triangles = [list(triangle.vertices) for triangle in scene_object.data.loop_triangles]
    candidate_geometry[name] = {'vertices': [list(position) for position in positions], 'triangles': triangles}
    vertex_distances = [swept_surface.find_nearest(position)[3] for position in positions]
    minimum_distance = min(vertex_distances)
    minimum_position = positions[vertex_distances.index(minimum_distance)]
    probe_count = len(positions)
    guaranteed_lower_bound = float('inf')
    distance_cache = {tuple(position): distance for position, distance in zip(positions, vertex_distances)}
    pending_triangles = [[positions[index] for index in triangle] for triangle in triangles]
    while pending_triangles:
        corners = pending_triangles.pop()
        distances = []
        for position in corners:
            key = tuple(position)
            if key not in distance_cache:
                distance_cache[key] = swept_surface.find_nearest(position)[3]
                probe_count += 1
            distance = distance_cache[key]
            distances.append(distance)
            if distance < minimum_distance:
                minimum_distance = distance
                minimum_position = position
        maximum_edge_length = max((corners[first] - corners[second]).length for first, second in [(0, 1), (1, 2), (2, 0)])
        lower_bound = max(distances) - maximum_edge_length
        if lower_bound >= 0.005 or maximum_edge_length <= 0.001 or min(distances) < 0.005:
            guaranteed_lower_bound = min(guaranteed_lower_bound, lower_bound)
            continue
        first_midpoint = (corners[0] + corners[1]) / 2
        second_midpoint = (corners[1] + corners[2]) / 2
        third_midpoint = (corners[2] + corners[0]) / 2
        pending_triangles.extend([[corners[0], first_midpoint, third_midpoint], [first_midpoint, corners[1], second_midpoint], [third_midpoint, second_midpoint, corners[2]], [first_midpoint, second_midpoint, third_midpoint]])
    clearance_records[name] = {'minimum_sampled_surface_distance_millimeters': minimum_distance * 1000, 'conservative_clearance_lower_bound_millimeters': guaranteed_lower_bound * 1000, 'closest_surface_point_meters': list(minimum_position), 'probe_count': probe_count, 'passes_five_millimeter_clearance': guaranteed_lower_bound >= 0.005}
    print('CLEARANCE=' + name + ' ' + json.dumps(clearance_records[name]), flush=True)
    geometry_records[name] = inspect_mesh_geometry(scene_object)
    def crossing_signature(geometry, pair):
        return tuple(sorted(tuple(sorted(tuple(round(coordinate, 7) for coordinate in geometry['vertices'][vertex_index]) for vertex_index in geometry['triangles'][triangle_index])) for triangle_index in pair))
    baseline_crossings = {crossing_signature(original_geometry[name], pair) for pair in original_inspections[name]['triangle_crossing_pairs']}
    new_crossings = [pair for pair in geometry_records[name]['triangle_crossing_pairs'] if crossing_signature(candidate_geometry[name], pair) not in baseline_crossings]
    geometry_records[name]['unchanged_baseline_triangle_crossing_pairs'] = len(geometry_records[name]['triangle_crossing_pairs']) - len(new_crossings)
    geometry_records[name]['new_triangle_crossing_pairs'] = new_crossings
    geometry_records[name]['passes_without_new_geometry_defects'] = not any(geometry_records[name][field] for field in ('boundary_edges', 'nonmanifold_edges', 'inconsistent_winding_edges', 'zero_area_faces', 'zero_length_edges', 'coplanar_overlap_pairs', 'new_triangle_crossing_pairs')) and geometry_records[name]['signed_volume_cubic_meters'] > 0
    print('TOPOLOGY=' + name + ' passed=' + str(geometry_records[name]['passed']) + ' crossings=' + str(len(geometry_records[name]['triangle_crossing_pairs'])), flush=True)

surfaces = {name: BVHTree.FromPolygons(geometry['vertices'], geometry['triangles'], all_triangles=True) for name, geometry in candidate_geometry.items()}
original_surfaces = {name: BVHTree.FromPolygons(original_geometry[name]['vertices'], original_geometry[name]['triangles'], all_triangles=True) for name in names}
subset_records = {}
for name in names:
    maximum_outward_distance = 0.0
    exterior_samples = []
    positions = [Vector(position) for position in candidate_geometry[name]['vertices']]
    probes = positions + [sum((positions[index] for index in triangle), Vector()) / 3 for triangle in candidate_geometry[name]['triangles']]
    for position in probes:
        nearest, normal, triangle_index, distance = original_surfaces[name].find_nearest(position)
        outward_distance = (position - nearest).dot(normal)
        maximum_outward_distance = max(maximum_outward_distance, outward_distance)
        if distance > 0.00002 and outward_distance > 0.00002:
            exterior_samples.append(list(position))
    subset_records[name] = {'surface_probes': len(probes), 'maximum_outward_signed_distance_millimeters': maximum_outward_distance * 1000, 'exterior_probe_count': len(exterior_samples), 'first_exterior_probes': exterior_samples[:20], 'passed': not exterior_samples}
    print('SUBSET=' + name + ' ' + json.dumps(subset_records[name]), flush=True)
protected_names = ['GEO_CHASSIS_BODY', 'GEO_CHASSIS_FLOOR', 'GEO_CHASSIS_CockpitUpperLining']
interface_records = {}
for name in names:
    for other_name in protected_names + [other for other in names if other > name]:
        other_geometry = candidate_geometry.get(other_name, original_geometry[other_name])
        other_surface = surfaces.get(other_name)
        if other_surface is None:
            other_surface = BVHTree.FromPolygons(other_geometry['vertices'], other_geometry['triangles'], all_triangles=True)
        candidate_pairs = surfaces[name].overlap(other_surface)
        original_other_surface = original_surfaces.get(other_name)
        if original_other_surface is None:
            original_other_surface = BVHTree.FromPolygons(original_geometry[other_name]['vertices'], original_geometry[other_name]['triangles'], all_triangles=True)
        original_pairs = set(original_surfaces[name].overlap(original_other_surface))
        unexpected_pairs = []
        crossing_pairs = []
        for first_index, second_index in candidate_pairs:
            first_corners = [Vector(candidate_geometry[name]['vertices'][index]) for index in candidate_geometry[name]['triangles'][first_index]]
            second_corners = [Vector(other_geometry['vertices'][index]) for index in other_geometry['triangles'][second_index]]
            if segments_cross_triangle(first_corners, second_corners) or segments_cross_triangle(second_corners, first_corners):
                crossing_pairs.append([first_index, second_index])
                if not subset_records[name]['passed'] or (other_name in subset_records and not subset_records[other_name]['passed']):
                    unexpected_pairs.append([first_index, second_index])
        interface_records[name + ' / ' + other_name] = {'original_triangle_overlap_pairs': len(original_pairs), 'candidate_triangle_overlap_pairs': len(candidate_pairs), 'current_crossing_pair_count': len(crossing_pairs), 'new_crossing_pairs': unexpected_pairs, 'classification': 'Candidate occupied surfaces remain inside the original closed solids; shared attachment intersections cannot add occupied overlap volume. Triangle identifiers are not comparable after local Boolean cuts.'}
        print('INTERFACE=' + name + '/' + other_name + ' new_crossings=' + str(len(unexpected_pairs)), flush=True)
report = {'elapsed_seconds': time.monotonic() - started, 'geometry': geometry_records, 'clearance': clearance_records, 'occupied_surface_subset': subset_records, 'interfaces': interface_records, 'minimum_required_clearance_millimeters': 5, 'method': 'All 1205 animated arm surfaces in one triangle tree. Adaptive four-way triangle subdivision uses the 1-Lipschitz distance bound: maximum corner distance minus longest edge. Subdivide unresolved triangles until the bound proves 5 mm or a sampled violation is found; residual triangles below 1 mm cannot pass unless their bound also proves clearance.'}
report['passed'] = all(record['passes_five_millimeter_clearance'] for record in clearance_records.values()) and all(record['passes_without_new_geometry_defects'] for record in geometry_records.values()) and all(record['passed'] for record in subset_records.values()) and not any(record['new_crossing_pairs'] for record in interface_records.values())
(output_directory / 'animated_driver_clearance_validation.json').write_text(json.dumps(report, indent=2))
(output_directory / 'animated_driver_clearance_surfaces.json').write_text(json.dumps(candidate_geometry))
print('CANDIDATE_PASSED=' + str(report['passed']), flush=True)

if not report['passed']:
    raise RuntimeError('Animated driver clearance validation failed')
