import argparse
import ast
import hashlib
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree
from mathutils.kdtree import KDTree

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from common.output_policy import validate_output_path
from separate_formula_one_2030_bodywork import mesh_fingerprint, protected_object_fingerprint
from taper_formula_one_2030_engine_cover_rear import exact_surface_intersections


CHANGED_OBJECT_NAMES = (
    'BodyworkSidepodLeft',
    'BodyworkSidepodRight',
    'BodyworkEngineCover',
)

SHOULDER_LOWERING_PROFILE = (
    (0.08, 0.0), (0.3, 0.025), (0.5, 0.045), (0.8, 0.045),
    (1.0, 0.025), (1.2, 0.01), (1.5, 0.0),
)


def smooth_fraction(value, lower, upper):
    fraction = min(1.0, max(0.0, (value - lower) / (upper - lower)))
    return fraction * fraction * (3.0 - 2.0 * fraction)


def monotone_profile_value(position, profile):
    slopes = [(second[1] - first[1]) / (second[0] - first[0]) for first, second in zip(profile, profile[1:])]
    derivatives = [slopes[0]]
    for previous_slope, next_slope in zip(slopes, slopes[1:]):
        derivatives.append(2.0 * previous_slope * next_slope / (previous_slope + next_slope) if previous_slope * next_slope > 0 else 0.0)
    derivatives.append(slopes[-1])
    if position <= profile[0][0]:
        return profile[0][1]
    for control_index, (first_control, second_control) in enumerate(zip(profile, profile[1:])):
        if position <= second_control[0]:
            interval = second_control[0] - first_control[0]
            fraction = (position - first_control[0]) / interval
            return (2 * fraction ** 3 - 3 * fraction ** 2 + 1) * first_control[1] + (fraction ** 3 - 2 * fraction ** 2 + fraction) * interval * derivatives[control_index] + (-2 * fraction ** 3 + 3 * fraction ** 2) * second_control[1] + (fraction ** 3 - fraction ** 2) * interval * derivatives[control_index + 1]
    return profile[-1][1]


def build_attachment_tree(original_meshes, protected_objects):
    protected_vertices = [
        scene_object.matrix_world @ vertex.co
        for scene_object in protected_objects if scene_object.type == 'MESH'
        for vertex in scene_object.data.vertices
    ]
    protected_tree = KDTree(len(protected_vertices))
    for vertex_index, position in enumerate(protected_vertices):
        protected_tree.insert(position, vertex_index)
    protected_tree.balance()
    attachment_positions = [
        vertex.co.copy() for original_mesh in original_meshes.values()
        for vertex in original_mesh.vertices if protected_tree.find(vertex.co)[2] < 0.000002
    ]
    attachment_tree = KDTree(len(attachment_positions))
    for vertex_index, position in enumerate(attachment_positions):
        attachment_tree.insert(position, vertex_index)
    attachment_tree.balance()
    return attachment_tree, len(attachment_positions)


def remodeled_position(position, attachment_tree, strength):
    result = position.copy()
    lateral_distance = abs(position.y)
    if lateral_distance <= 0.016 or position.x <= 0.08 or position.x >= 1.45 or position.z >= 0.24 or position.z <= -0.32:
        return result
    attachment_weight = smooth_fraction(attachment_tree.find(position)[2], 0.000003, 0.12)
    upper_protection = 1.0 - smooth_fraction(position.z, 0.12, 0.24)
    underside_protection = smooth_fraction(position.z, -0.32, -0.22)
    inner_skin_protection = max(smooth_fraction(lateral_distance, 0.13, 0.24), smooth_fraction(position.z, -0.03, 0.12))
    displacement_weight = attachment_weight * upper_protection * underside_protection * inner_skin_protection * strength * smooth_fraction(position.x, 0.08, 0.35) * (1.0 - smooth_fraction(position.x, 1.2, 1.45))
    width_expansion = 0.05 * math.exp(-((position.x - 0.6) / 0.4) ** 2)
    expanded_distance = lateral_distance + (lateral_distance - 0.016) * width_expansion * displacement_weight
    result.y = math.copysign(expanded_distance, position.y)
    rearward_extension = 0.28 * smooth_fraction(position.x, 0.3, 0.7) * (1.0 - smooth_fraction(position.x, 0.7, 1.45))
    result.x += rearward_extension * displacement_weight
    lateral_weight = smooth_fraction(lateral_distance, 0.07, 0.30)
    vertical_weight = smooth_fraction(position.z, -0.34, -0.02)
    result.z -= monotone_profile_value(position.x, SHOULDER_LOWERING_PROFILE) * displacement_weight * lateral_weight * vertical_weight
    return result


def deformation_derivative(position, attachment_tree, strength):
    derivatives = []
    for axis in range(3):
        first_position = position.copy()
        second_position = position.copy()
        first_position[axis] -= 0.00001
        second_position[axis] += 0.00001
        derivatives.append((remodeled_position(second_position, attachment_tree, strength) - remodeled_position(first_position, attachment_tree, strength)) / 0.00002)
    return Matrix(derivatives).transposed()


def remodel_mesh(original_mesh, object_name, attachment_tree, strength):
    remodeled_mesh = original_mesh.copy()
    remodeled_mesh.name = object_name + 'FerrariReferenceSurface'
    derivative_matrices = []
    for vertex in remodeled_mesh.vertices:
        original_position = original_mesh.vertices[vertex.index].co
        vertex.co = remodeled_position(original_position, attachment_tree, strength)
        derivative_matrices.append(deformation_derivative(original_position, attachment_tree, strength))
    minimum_determinant = min(matrix.determinant() for matrix in derivative_matrices)
    assert minimum_determinant > 0.10, (object_name, minimum_determinant)
    remodeled_mesh.update()
    corner_normals = [
        (derivative_matrices[corner.vertex_index].inverted_safe().transposed() @ original_mesh.corner_normals[corner.index].vector).normalized()
        for corner in original_mesh.loops
    ]
    remodeled_mesh.normals_split_custom_set(corner_normals)
    remodeled_mesh.update()
    original_mesh.calc_loop_triangles()
    remodeled_mesh.calc_loop_triangles()
    assert len(original_mesh.loop_triangles) == len(remodeled_mesh.loop_triangles)
    assert all(tuple(original.vertices) == tuple(current.vertices) for original, current in zip(original_mesh.loop_triangles, remodeled_mesh.loop_triangles))
    assert all(current.area > 1e-12 or preceding.area <= 1e-12 for preceding, current in zip(original_mesh.loop_triangles, remodeled_mesh.loop_triangles))
    assert all(tuple(vertex.co) == tuple(original_mesh.vertices[vertex.index].co) for vertex in remodeled_mesh.vertices if abs(original_mesh.vertices[vertex.index].co.y) <= 0.016)
    assert all([tuple(entry.uv) for entry in layer.data] == [tuple(entry.uv) for entry in remodeled_mesh.uv_layers[layer.name].data] for layer in original_mesh.uv_layers)
    return remodeled_mesh, minimum_determinant


def render_reviews(repository, output_directory, label):
    original_scene = bpy.context.scene
    review_scene = bpy.data.scenes.new('FerrariReferenceReview')
    review_scene.world = bpy.data.worlds.new('FerrariReferenceReviewWorld')
    review_scene.world.color = (0.13, 0.13, 0.13)
    review_scene.render.engine = 'BLENDER_WORKBENCH'
    review_scene.render.resolution_x = 1500
    review_scene.render.resolution_y = 900
    review_scene.render.resolution_percentage = 100
    review_scene.render.image_settings.file_format = 'PNG'
    review_scene.display.shading.light = 'STUDIO'
    review_scene.display.shading.color_type = 'OBJECT'
    review_scene.display.shading.show_cavity = True
    review_scene.display.shading.cavity_type = 'BOTH'
    review_scene.display.shading.background_type = 'WORLD'
    copied_objects = []
    for scene_object in list(bpy.data.objects):
        if scene_object.type != 'MESH' or not scene_object.name.startswith(('GEO_', 'Bodywork')):
            continue
        copied_object = bpy.data.objects.new('Review' + scene_object.name, scene_object.data)
        copied_object.matrix_world = scene_object.matrix_world.copy()
        copied_object.color = (0.68, 0.13, 0.06, 1.0) if scene_object.name.startswith('Bodywork') else (0.25, 0.27, 0.30, 1.0)
        review_scene.collection.objects.link(copied_object)
        copied_objects.append(copied_object)
    camera_data = bpy.data.cameras.new('FerrariReferenceReviewCamera')
    camera = bpy.data.objects.new('FerrariReferenceReviewCamera', camera_data)
    review_scene.collection.objects.link(camera)
    review_scene.camera = camera
    camera_data.type = 'ORTHO'
    for view_name, direction in [('side', (0, -1, 0.06)), ('top', (0, 0, 1)), ('three_quarter', (-0.8, -1, 0.65)), ('rear_three_quarter', (0.9, -1, 0.5))]:
        target = Vector((0.55, 0, 0.0))
        camera.location = target + Vector(direction).normalized() * 10
        camera.rotation_euler = (target - camera.location).to_track_quat('-Z', 'Y').to_euler()
        camera_data.ortho_scale = 4.2
        review_scene.render.filepath = str(validate_output_path(repository, output_directory / (label + '_' + view_name + '.png'), 'preview').path)
        bpy.ops.render.render(write_still=True, scene=review_scene.name)
    bpy.data.scenes.remove(review_scene)
    for copied_object in copied_objects:
        bpy.data.objects.remove(copied_object, do_unlink=True)
    bpy.data.objects.remove(camera, do_unlink=True)
    assert bpy.context.scene == original_scene


def triangle_surface(scene_object, mesh):
    mesh.calc_loop_triangles()
    positions = [scene_object.matrix_world @ vertex.co for vertex in mesh.vertices]
    triangles = [tuple(triangle.vertices) for triangle in mesh.loop_triangles]
    return positions, triangles, BVHTree.FromPolygons(positions, triangles, all_triangles=True)


def crossing_triangle_pairs(first_surface, second_surface, intersection_functions):
    first_positions, first_triangles, first_tree = first_surface
    second_positions, second_triangles, second_tree = second_surface
    crossings = set()
    for first_index, second_index in first_tree.overlap(second_tree):
        first_points = [first_positions[index] for index in first_triangles[first_index]]
        second_points = [second_positions[index] for index in second_triangles[second_index]]
        if any((first - second).length < 0.000001 for first in first_points for second in second_points):
            continue
        if intersection_functions['segments_cross_triangle'](first_points, second_points) or intersection_functions['segments_cross_triangle'](second_points, first_points) or intersection_functions['coplanar_overlap_area'](first_points, second_points) > 1e-12:
            crossings.add((first_index, second_index))
    return crossings


def validate_external_attachments(original_meshes):
    intersection_source = Path(__file__).with_name('validate_formula_one_2030_cockpit_geometry.py')
    parsed_source = ast.parse(intersection_source.read_text(encoding='utf-8'))
    selected_functions = [node for node in parsed_source.body if isinstance(node, ast.FunctionDef) and node.name in ('segments_cross_triangle', 'coplanar_overlap_area', 'planar_cross_product')]
    intersection_functions = {'math': math, 'Vector': Vector}
    exec(compile(ast.Module(body=selected_functions, type_ignores=[]), str(intersection_source), 'exec'), intersection_functions)
    mesh_objects = {scene_object.name: scene_object for scene_object in bpy.data.objects if scene_object.type == 'MESH' and scene_object.name.startswith(('Bodywork', 'GEO_'))}
    original_surfaces = {name: triangle_surface(scene_object, original_meshes.get(name, scene_object.data)) for name, scene_object in mesh_objects.items()}
    candidate_surfaces = {name: triangle_surface(mesh_objects[name], mesh_objects[name].data) for name in original_meshes}
    measurements = {}
    checked_pairs = set()
    for first_name in original_meshes:
        for second_name in mesh_objects:
            pair = tuple(sorted((first_name, second_name)))
            if first_name == second_name or pair in checked_pairs:
                continue
            checked_pairs.add(pair)
            preceding_crossings = crossing_triangle_pairs(original_surfaces[first_name], original_surfaces[second_name], intersection_functions)
            current_crossings = crossing_triangle_pairs(candidate_surfaces[first_name], candidate_surfaces.get(second_name, original_surfaces[second_name]), intersection_functions)
            new_crossings = current_crossings - preceding_crossings
            if preceding_crossings or current_crossings:
                measurements[first_name + '/' + second_name] = {'baseline_crossings': len(preceding_crossings), 'candidate_crossings': len(current_crossings), 'new_crossing_pairs': sorted(new_crossings)}
    maximum_attachment_gap = 0.0
    attachment_count = 0
    for first_name in original_meshes:
        for second_name in mesh_objects:
            if first_name == second_name:
                continue
            second_positions = original_surfaces[second_name][0]
            second_tree = KDTree(len(second_positions))
            for vertex_index, position in enumerate(second_positions):
                second_tree.insert(position, vertex_index)
            second_tree.balance()
            for vertex_index, position in enumerate(original_surfaces[first_name][0]):
                original_position, other_index, distance = second_tree.find(position)
                if distance < 0.000002:
                    candidate_position = candidate_surfaces[first_name][0][vertex_index]
                    other_position = candidate_surfaces.get(second_name, original_surfaces[second_name])[0][other_index]
                    maximum_attachment_gap = max(maximum_attachment_gap, (candidate_position - other_position).length)
                    attachment_count += 1
    return {'object_pairs_checked': len(checked_pairs), 'object_pair_intersections': measurements, 'coincident_attachment_vertices_checked': attachment_count, 'maximum_attachment_gap_meters': maximum_attachment_gap, 'passed': maximum_attachment_gap < 0.000004 and all(not value['new_crossing_pairs'] for value in measurements.values())}


def main():
    argument_parser = argparse.ArgumentParser()
    argument_parser.add_argument('--source-file', type=Path, required=True)
    argument_parser.add_argument('--output-directory', type=Path, required=True)
    argument_parser.add_argument('--strength', type=float, default=1.0)
    argument_parser.add_argument('--reference-archive', type=Path)
    arguments = argument_parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    assert 0.0 < arguments.strength <= 1.0
    repository = Path(__file__).resolve().parents[2]
    output_directory = validate_output_path(repository, arguments.output_directory, 'preview').path
    output_directory.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=str(arguments.source_file))
    assert not bpy.data.objects['BodyworkEngineCover'].get('ferrari_reference_remodel_applied')
    protected_objects = [scene_object for scene_object in bpy.data.objects if scene_object.name not in CHANGED_OBJECT_NAMES]
    protected_fingerprints = {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in protected_objects}
    original_meshes = {name: bpy.data.objects[name].data for name in CHANGED_OBJECT_NAMES}
    attachment_tree, attachment_count = build_attachment_tree(original_meshes, protected_objects)
    original_intersections = {}
    report = {'source_file': str(arguments.source_file), 'source_digest': hashlib.sha256(arguments.source_file.read_bytes()).hexdigest(), 'strength': arguments.strength, 'attachment_vertex_count': attachment_count, 'objects': {}, 'central_fin_unchanged': True, 'runtime_assets_promoted': False}
    if arguments.reference_archive:
        report['reference_archive'] = str(arguments.reference_archive)
        report['reference_archive_digest'] = hashlib.sha256(arguments.reference_archive.read_bytes()).hexdigest()
    render_reviews(repository, output_directory, 'before')
    for object_name, original_mesh in original_meshes.items():
        original_intersections[object_name] = exact_surface_intersections(original_mesh, list(range(len(original_mesh.polygons))))
        remodeled_mesh, minimum_determinant = remodel_mesh(original_mesh, object_name, attachment_tree, arguments.strength)
        bpy.data.objects[object_name].data = remodeled_mesh
        intersections = exact_surface_intersections(remodeled_mesh, list(range(len(remodeled_mesh.polygons))))
        new_intersections = intersections - original_intersections[object_name]
        report['objects'][object_name] = {
            'original_fingerprint': mesh_fingerprint(original_mesh),
            'remodeled_fingerprint': mesh_fingerprint(remodeled_mesh),
            'triangles': len(remodeled_mesh.loop_triangles),
            'maximum_displacement_meters': max((vertex.co - original_mesh.vertices[vertex.index].co).length for vertex in remodeled_mesh.vertices),
            'minimum_deformation_determinant': minimum_determinant,
            'baseline_self_intersections': len(original_intersections[object_name]),
            'candidate_self_intersections': len(intersections),
            'new_self_intersection_pairs': sorted(new_intersections),
        }
        print(json.dumps({object_name: report['objects'][object_name]}), flush=True)
    report['passed'] = all(not record['new_self_intersection_pairs'] for record in report['objects'].values())
    if report['passed']:
        report['external_attachments'] = validate_external_attachments(original_meshes)
        report['passed'] = report['external_attachments']['passed']
    report_path = validate_output_path(repository, output_directory / 'ferrari_reference_bodywork_report.json', 'preview').path
    report_path.write_text(json.dumps(report, indent=2), encoding='utf-8')
    render_reviews(repository, output_directory, 'after')
    assert report['passed'], 'Candidate failed geometry or attachment validation; canonical source remains unchanged'
    assert all(protected_object_fingerprint(bpy.data.objects[name]) == fingerprint for name, fingerprint in protected_fingerprints.items())
    report['protected_object_fingerprints'] = protected_fingerprints
    report['all_texture_coordinates_unchanged'] = True
    report['central_attachment_strip_unchanged'] = True
    bpy.data.objects['BodyworkEngineCover']['ferrari_reference_remodel_applied'] = True
    output_file = validate_output_path(repository, output_directory / 'f1_2030.blend', 'preview').path
    bpy.ops.wm.save_as_mainfile(filepath=str(output_file), relative_remap=True)
    bpy.ops.wm.open_mainfile(filepath=str(output_file))
    assert all(mesh_fingerprint(bpy.data.objects[name].data) == record['remodeled_fingerprint'] for name, record in report['objects'].items())
    assert all(protected_object_fingerprint(bpy.data.objects[name]) == fingerprint for name, fingerprint in protected_fingerprints.items())
    report['output_file'] = str(output_file)
    report['output_digest'] = hashlib.sha256(output_file.read_bytes()).hexdigest()
    report['saved_source_reopened_and_verified'] = True
    report_path.write_text(json.dumps(report, indent=2), encoding='utf-8')
    print('FERRARI_REFERENCE_CANDIDATE_VERIFIED', flush=True)


if __name__ == '__main__':
    main()
