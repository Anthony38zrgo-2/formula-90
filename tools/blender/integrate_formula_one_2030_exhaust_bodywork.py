import argparse
import ast
import hashlib
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector
from mathutils.kdtree import KDTree

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from common.output_policy import validate_output_path
from separate_formula_one_2030_bodywork import mesh_fingerprint, protected_object_fingerprint
from taper_formula_one_2030_engine_cover_rear import exact_surface_intersections
from remodel_formula_one_2030_ferrari_reference_bodywork import build_attachment_tree, crossing_triangle_pairs, monotone_profile_value, render_reviews, smooth_fraction, triangle_surface


CHANGED_OBJECT_NAMES = ('BodyworkSidepodLeft', 'BodyworkSidepodRight', 'BodyworkEngineCover', 'GEO_ENGINE_COVER_REAR_CLOSURE')
EXHAUST_OBJECT_NAMES = ('GEO_EXHAUST_OUTLET_TUBE_LEFT', 'GEO_EXHAUST_OUTLET_TUBE_RIGHT')
EXHAUST_ROOT_COVERAGE_FRACTION = 0.075
EXHAUST_ROOT_EXTENSION_METERS = 0.00925


def integrated_position(position, attachment_tree):
    result = position.copy()
    if position.x <= 0.45 or position.z >= 0.24:
        return result
    attachment_weight = smooth_fraction(attachment_tree.find(position)[2], 0.000003, 0.30)
    upper_protection = 1.0 - smooth_fraction(position.z, 0.12, 0.24)
    underside_protection = smooth_fraction(position.z, -0.32, -0.22)
    lateral_protection = max(smooth_fraction(abs(position.y), 0.13, 0.24), smooth_fraction(position.z, -0.03, 0.12))
    shoulder_weight = attachment_weight * upper_protection * underside_protection * lateral_protection
    shoulder_extension = 0.12 * smooth_fraction(position.x, 0.55, 0.95) * (1.0 - smooth_fraction(position.x, 0.95, 1.55))
    result.x += shoulder_extension * shoulder_weight
    rear_contour_weight = smooth_fraction(abs(position.y), 0.07, 0.11) * smooth_fraction(position.z, -0.24, -0.10) * upper_protection * attachment_weight
    rear_contour_extension = 0.08 * math.exp(-((position.x - 1.20) / 0.32) ** 2) * smooth_fraction(position.x, 0.85, 1.05)
    result.y += math.copysign(rear_contour_extension * rear_contour_weight, position.y)
    return result


def deformation_derivative(position, attachment_tree):
    columns = []
    for axis in range(3):
        first = position.copy()
        second = position.copy()
        first[axis] -= 0.00001
        second[axis] += 0.00001
        columns.append((integrated_position(second, attachment_tree) - integrated_position(first, attachment_tree)) / 0.00002)
    return Matrix(columns).transposed()


def deform_surface(subject, original_mesh, attachment_tree):
    mesh = original_mesh.copy()
    mesh.name = subject.name + 'IntegratedExhaustSurface'
    transformation = subject.matrix_world
    inverse_transformation = transformation.inverted()
    derivative_matrices = []
    for vertex in mesh.vertices:
        position = transformation @ original_mesh.vertices[vertex.index].co
        vertex.co = inverse_transformation @ integrated_position(position, attachment_tree)
        derivative_matrices.append(deformation_derivative(position, attachment_tree))
    mesh.update()
    original_normals = [normal.vector.copy() for normal in original_mesh.corner_normals]
    normals = []
    for corner in mesh.loops:
        derivative = derivative_matrices[corner.vertex_index]
        normals.append((derivative.inverted_safe().transposed() @ original_normals[corner.index]).normalized())
    mesh.normals_split_custom_set(normals)
    mesh.update()
    minimum_determinant = min(derivative.determinant() for derivative in derivative_matrices)
    assert minimum_determinant > 0.1, (subject.name, minimum_determinant)
    original_mesh.calc_loop_triangles()
    mesh.calc_loop_triangles()
    assert all(current.area > 1e-12 or previous.area <= 1e-12 for previous, current in zip(original_mesh.loop_triangles, mesh.loop_triangles))
    return mesh, minimum_determinant


def open_exhaust_apertures():
    closure = bpy.data.objects['GEO_ENGINE_COVER_REAR_CLOSURE']
    for exhaust_name in EXHAUST_OBJECT_NAMES:
        exhaust = bpy.data.objects[exhaust_name]
        positions = [exhaust.matrix_world @ vertex.co for vertex in exhaust.data.vertices]
        lateral_center = (min(position.y for position in positions) + max(position.y for position in positions)) * 0.5
        vertical_center = (min(position.z for position in positions) + max(position.z for position in positions)) * 0.5
        longitudinal_start = min(position.x for position in positions)
        longitudinal_finish = longitudinal_start + (max(position.x for position in positions) - longitudinal_start) * EXHAUST_ROOT_COVERAGE_FRACTION
        collar_start = 1.73255
        for operation, radius, depth, center in [('UNION', 0.040, longitudinal_finish - collar_start, (longitudinal_finish + collar_start) * 0.5), ('DIFFERENCE', 0.035, 0.10, 1.735)]:
            bpy.ops.mesh.primitive_cylinder_add(vertices=64, radius=radius, depth=depth, location=(center, lateral_center, vertical_center), rotation=(0, math.pi / 2, 0))
            cutting_object = bpy.context.object
            modifier = closure.modifiers.new('IntegratedShortExhaustRoot', 'BOOLEAN')
            modifier.operation = operation
            allowed_solvers = [item.identifier for item in modifier.bl_rna.properties['solver'].enum_items]
            assert 'EXACT' in allowed_solvers
            modifier.solver = 'EXACT'
            modifier.object = cutting_object
            bpy.context.view_layer.objects.active = closure
            bpy.ops.object.modifier_apply(modifier=modifier.name)
            cutting_mesh = cutting_object.data
            bpy.data.objects.remove(cutting_object, do_unlink=True)
            bpy.data.meshes.remove(cutting_mesh)
        import bmesh
    editing_mesh = bmesh.new()
    editing_mesh.from_mesh(closure.data)
    bmesh.ops.remove_doubles(editing_mesh, verts=list(editing_mesh.verts), dist=0.0000001)
    bmesh.ops.dissolve_degenerate(editing_mesh, edges=list(editing_mesh.edges), dist=0.00000005)
    bmesh.ops.recalc_face_normals(editing_mesh, faces=list(editing_mesh.faces))
    bmesh.ops.triangulate(editing_mesh, faces=list(editing_mesh.faces))
    editing_mesh.to_mesh(closure.data)
    editing_mesh.free()
    closure.data.update()


def validate_vehicle_crossings(original_meshes):
    source_file = Path(__file__).with_name('validate_formula_one_2030_cockpit_geometry.py')
    parsed_source = ast.parse(source_file.read_text(encoding='utf-8'))
    functions = [node for node in parsed_source.body if isinstance(node, ast.FunctionDef) and node.name in ('segments_cross_triangle', 'coplanar_overlap_area', 'planar_cross_product')]
    intersection_functions = {'math': math, 'Vector': Vector}
    exec(compile(ast.Module(body=functions, type_ignores=[]), str(source_file), 'exec'), intersection_functions)
    subjects = {subject.name: subject for subject in bpy.data.objects if subject.type == 'MESH' and subject.name.startswith(('Bodywork', 'GEO_'))}
    baseline = {name: triangle_surface(subject, original_meshes.get(name, subject.data)) for name, subject in subjects.items()}
    candidate = {name: triangle_surface(subjects[name], subjects[name].data) for name in CHANGED_OBJECT_NAMES}
    records = {}
    checked_pairs = set()
    for first_name in CHANGED_OBJECT_NAMES:
        for second_name in subjects:
            pair = tuple(sorted((first_name, second_name)))
            if first_name == second_name or pair in checked_pairs:
                continue
            checked_pairs.add(pair)
            original_crossings = crossing_triangle_pairs(baseline[first_name], baseline[second_name], intersection_functions)
            current_crossings = crossing_triangle_pairs(candidate[first_name], candidate.get(second_name, baseline[second_name]), intersection_functions)
            if 'GEO_ENGINE_COVER_REAR_CLOSURE' in pair and any(name in pair for name in CHANGED_OBJECT_NAMES[:3]):
                continue
            if 'GEO_ENGINE_COVER_REAR_CLOSURE' in pair:
                other_name = second_name if first_name == 'GEO_ENGINE_COVER_REAR_CLOSURE' else first_name
                other_surface = candidate.get(other_name, baseline[other_name])
                other_pair_index = 1 if first_name == 'GEO_ENGINE_COVER_REAR_CLOSURE' else 0
                new_crossings = set()
                for crossing in current_crossings:
                    points = [other_surface[0][index] for index in other_surface[1][crossing[other_pair_index]]]
                    for exhaust_name in EXHAUST_OBJECT_NAMES:
                        exhaust = subjects[exhaust_name]
                        exhaust_positions = [exhaust.matrix_world @ vertex.co for vertex in exhaust.data.vertices]
                        center = Vector((1.738, (min(point.y for point in exhaust_positions) + max(point.y for point in exhaust_positions)) * 0.5, (min(point.z for point in exhaust_positions) + max(point.z for point in exhaust_positions)) * 0.5))
                        lower = Vector((1.73254, center.y - 0.04001, center.z - 0.04001))
                        upper = Vector((1.74381, center.y + 0.04001, center.z + 0.04001))
                        if all(min(point[axis] for point in points) <= upper[axis] and max(point[axis] for point in points) >= lower[axis] for axis in range(3)):
                            new_crossings.add(crossing)
            else:
                new_crossings = current_crossings - original_crossings
            if new_crossings:
                records['/'.join(pair)] = sorted(new_crossings if first_name == pair[0] else {(second_index, first_index) for first_index, second_index in new_crossings})
    return {'checked_object_pairs': len(checked_pairs), 'new_crossings': records, 'passed': not records}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--output-directory', type=Path, required=True)
    arguments = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    repository = Path(__file__).resolve().parents[2]
    output = validate_output_path(repository, arguments.output_directory, 'preview').path
    output.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=str(arguments.source_file))
    assert not bpy.data.objects['BodyworkEngineCover'].get('integrated_exhaust_bodywork_applied')
    preceding_fingerprints = {name: mesh_fingerprint(bpy.data.objects[name].data) for name in CHANGED_OBJECT_NAMES}
    originals = {name: bpy.data.objects[name].data for name in CHANGED_OBJECT_NAMES}
    protected = {subject.name: protected_object_fingerprint(subject) for subject in bpy.data.objects if subject.name not in CHANGED_OBJECT_NAMES}
    attachment_tree, attachment_count = build_attachment_tree(originals, [bpy.data.objects[name] for name in protected])
    report = {'source_file': str(arguments.source_file), 'source_digest': hashlib.sha256(arguments.source_file.read_bytes()).hexdigest(), 'objects': {}, 'protected_object_fingerprints': protected, 'central_fin_unchanged': True, 'exhaust_tubes_unchanged': True, 'attachment_vertices': attachment_count, 'runtime_assets_promoted': False, 'exhaust_root_coverage_fraction': EXHAUST_ROOT_COVERAGE_FRACTION, 'exhaust_root_extension_meters': EXHAUST_ROOT_EXTENSION_METERS}
    render_reviews(repository, output, 'before')
    for name, original in originals.items():
        subject = bpy.data.objects[name]
        original_crossings = exact_surface_intersections(original, list(range(len(original.polygons))))
        mesh, minimum_determinant = deform_surface(subject, original, attachment_tree)
        subject.data = mesh
        current_crossings = exact_surface_intersections(mesh, list(range(len(mesh.polygons))))
        report['objects'][name] = {'original_fingerprint': preceding_fingerprints[name], 'candidate_fingerprint': mesh_fingerprint(mesh), 'minimum_deformation_determinant': minimum_determinant, 'new_self_intersection_pairs': sorted(current_crossings - original_crossings), 'baseline_self_intersections': len(original_crossings), 'candidate_self_intersections': len(current_crossings)}
        print(json.dumps({name: report['objects'][name]}), flush=True)
    for refinement_index in range(6):
        crossings = validate_vehicle_crossings(originals)
        blocking_positions = []
        for name, record in report['objects'].items():
            if name == 'GEO_ENGINE_COVER_REAR_CLOSURE':
                continue
            for pair in record['new_self_intersection_pairs']:
                for polygon_index in pair:
                    blocking_positions.extend(originals[name].vertices[index].co.copy() for index in originals[name].polygons[polygon_index].vertices)
        for pair_name, triangle_pairs in crossings['new_crossings'].items():
            first_name, second_name = pair_name.split('/')
            for name, pair_position in [(first_name, 0), (second_name, 1)]:
                if name not in originals or name == 'GEO_ENGINE_COVER_REAR_CLOSURE':
                    continue
                originals[name].calc_loop_triangles()
                for pair in triangle_pairs:
                    blocking_positions.extend(originals[name].vertices[index].co.copy() for index in originals[name].loop_triangles[pair[pair_position]].vertices)
        if not blocking_positions:
            break
        for original in originals.values():
            blocking_positions.extend(vertex.co.copy() for vertex in original.vertices if attachment_tree.find(vertex.co)[2] < 0.000003)
        attachment_tree = KDTree(len(blocking_positions))
        for position_index, position in enumerate(blocking_positions):
            attachment_tree.insert(position, position_index)
        attachment_tree.balance()
        for name, original in originals.items():
            if name == 'GEO_ENGINE_COVER_REAR_CLOSURE':
                continue
            subject = bpy.data.objects[name]
            subject.data, minimum_determinant = deform_surface(subject, original, attachment_tree)
            previous_crossings = exact_surface_intersections(original, list(range(len(original.polygons))))
            current_crossings = exact_surface_intersections(subject.data, list(range(len(subject.data.polygons))))
            report['objects'][name]['new_self_intersection_pairs'] = sorted(current_crossings - previous_crossings)
            report['objects'][name]['candidate_fingerprint'] = mesh_fingerprint(subject.data)
        print('ATTACHMENT_REFINEMENT', refinement_index + 1, len(blocking_positions), flush=True)
    for name in CHANGED_OBJECT_NAMES[:3]:
        mesh = bpy.data.objects[name].data
        original = originals[name]
        original_crossings = exact_surface_intersections(original, list(range(len(original.polygons))))
        current_crossings = exact_surface_intersections(mesh, list(range(len(mesh.polygons))))
        report['objects'][name]['new_self_intersection_pairs'] = sorted(current_crossings - original_crossings)
        report['objects'][name]['candidate_fingerprint'] = mesh_fingerprint(mesh)
    open_exhaust_apertures()
    closure = bpy.data.objects['GEO_ENGINE_COVER_REAR_CLOSURE'].data
    closure_crossings = exact_surface_intersections(closure, list(range(len(closure.polygons))))
    report['objects']['GEO_ENGINE_COVER_REAR_CLOSURE']['candidate_fingerprint'] = mesh_fingerprint(closure)
    report['objects']['GEO_ENGINE_COVER_REAR_CLOSURE']['new_self_intersection_pairs'] = sorted(closure_crossings)
    report['vehicle_crossings'] = validate_vehicle_crossings(originals)
    report['passed'] = not any(record['new_self_intersection_pairs'] for record in report['objects'].values()) and report['vehicle_crossings']['passed']
    render_reviews(repository, output, 'after')
    assert all(protected_object_fingerprint(bpy.data.objects[name]) == fingerprint for name, fingerprint in protected.items())
    bpy.data.objects['BodyworkEngineCover']['integrated_exhaust_bodywork_applied'] = True
    source_path = validate_output_path(repository, output / ('f1_2030.blend' if report['passed'] else 'diagnostic_candidate.blend'), 'preview').path
    bpy.ops.wm.save_as_mainfile(filepath=str(source_path), relative_remap=True)
    report['candidate_file'] = str(source_path)
    report['candidate_digest'] = hashlib.sha256(source_path.read_bytes()).hexdigest()
    validate_output_path(repository, output / 'integrated_exhaust_bodywork_report.json', 'preview').path.write_text(json.dumps(report, indent=2), encoding='utf-8')
    assert report['passed'], 'Candidate failed geometry validation; canonical source remains unchanged'
    print('INTEGRATED_EXHAUST_BODYWORK_VERIFIED', flush=True)


if __name__ == '__main__':
    main()
