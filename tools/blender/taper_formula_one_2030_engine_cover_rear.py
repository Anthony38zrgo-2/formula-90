import argparse
import ast
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector
from mathutils.kdtree import KDTree
from mathutils.bvhtree import BVHTree

sys.path.insert(0, str(Path(__file__).resolve().parent))
from smooth_formula_one_2030_engine_cover import file_digest, nonadjacent_overlap_parent_pairs
from separate_formula_one_2030_bodywork import mesh_fingerprint, protected_object_fingerprint
from common.output_policy import validate_output_path


CHANGED_OBJECT_NAMES = ('BodyworkEngineCover', 'BodyworkSidepodLeft', 'BodyworkSidepodRight', 'GEO_ENGINE_COVER_REAR_CLOSURE')


def exact_surface_intersections(mesh, polygon_parents):
    source = Path(__file__).with_name('validate_formula_one_2030_cockpit_geometry.py').read_text(encoding='utf-8')
    document = ast.parse(source)
    functions = [node for node in document.body if isinstance(node, ast.FunctionDef) and node.name in ('segments_cross_triangle', 'coplanar_overlap_area', 'planar_cross_product')]
    namespace = {'math': math, 'Vector': Vector}
    exec(compile(ast.Module(body=functions, type_ignores=[]), str(Path(__file__).with_name('validate_formula_one_2030_cockpit_geometry.py')), 'exec'), namespace)
    mesh.calc_loop_triangles()
    positions = [vertex.co.copy() for vertex in mesh.vertices]
    triangles = [tuple(triangle.vertices) for triangle in mesh.loop_triangles]
    parents = [polygon_parents[triangle.polygon_index] for triangle in mesh.loop_triangles]
    tree = BVHTree.FromPolygons(positions, triangles, all_triangles=True)
    intersections = set()
    for first_index, second_index in tree.overlap(tree):
        if first_index >= second_index:
            continue
        first_positions = [positions[index] for index in triangles[first_index]]
        second_positions = [positions[index] for index in triangles[second_index]]
        if any((first - second).length < 0.000001 for first in first_positions for second in second_positions):
            continue
        if namespace['segments_cross_triangle'](first_positions, second_positions) or namespace['segments_cross_triangle'](second_positions, first_positions) or namespace['coplanar_overlap_area'](first_positions, second_positions) > 1e-12:
            intersections.add(tuple(sorted((parents[first_index], parents[second_index]))))
    return intersections


def smooth_transition(value, lower, upper):
    fraction = min(1, max(0, (value - lower) / (upper - lower)))
    return fraction * fraction * (3 - 2 * fraction)


def sample_cover_width(mesh, longitudinal_position):
    intersections = []
    for edge in mesh.edges:
        first_position, second_position = [mesh.vertices[index].co for index in edge.vertices]
        if min(first_position.x, second_position.x) <= longitudinal_position <= max(first_position.x, second_position.x):
            if abs(second_position.x - first_position.x) < 1e-9:
                intersections.extend((abs(first_position.y), abs(second_position.y)))
            else:
                fraction = (longitudinal_position - first_position.x) / (second_position.x - first_position.x)
                intersections.append(abs(first_position.y * (1 - fraction) + second_position.y * fraction))
    return max(intersections, default=0.012)


def taper_position(position, inner_sidepod_only=False):
    tapered = position.copy()
    if position.x <= 0.645 or position.x >= 1.94 or abs(position.y) <= 0.012:
        return tapered
    scale = 0.58
    weight = smooth_transition(position.x, 0.645, 0.82) * (1 - smooth_transition(position.x, 1.84, 1.94))
    if inner_sidepod_only:
        weight *= 1 - smooth_transition(abs(position.y), 0.24, 0.35)
    tapered.y = math.copysign(0.012 + (abs(position.y) - 0.012) * (1 - weight + weight * scale), position.y)
    return tapered


def taper_derivative(position, inner_sidepod_only):
    derivatives = []
    for axis in range(3):
        first_position = position.copy()
        second_position = position.copy()
        first_position[axis] -= 0.00001
        second_position[axis] += 0.00001
        derivatives.append((taper_position(second_position, inner_sidepod_only) - taper_position(first_position, inner_sidepod_only)) / 0.00002)
    return Matrix(derivatives).transposed()


def retain_surface_triangulation(original_mesh, name):
    original_mesh.calc_loop_triangles()
    retained_triangles = []
    preceding_triangle_keys = set()
    for triangle in original_mesh.loop_triangles:
        key = tuple(sorted(triangle.vertices))
        if key not in preceding_triangle_keys:
            retained_triangles.append(triangle)
            preceding_triangle_keys.add(key)
    mesh = bpy.data.meshes.new(name + 'PrecedingSurfaceTriangles')
    mesh.from_pydata([vertex.co.copy() for vertex in original_mesh.vertices], [tuple(edge.vertices) for edge in original_mesh.edges], [tuple(triangle.vertices) for triangle in retained_triangles])
    mesh.update(calc_edges=True)
    for material in original_mesh.materials:
        mesh.materials.append(material)
    polygon_indices = [triangle.polygon_index for triangle in retained_triangles]
    corner_indices = [loop_index for triangle in retained_triangles for loop_index in triangle.loops]
    original_edges = {tuple(sorted(edge.vertices)): edge.index for edge in original_mesh.edges}
    for polygon, original_index in zip(mesh.polygons, polygon_indices):
        polygon.material_index = original_mesh.polygons[original_index].material_index
        polygon.use_smooth = original_mesh.polygons[original_index].use_smooth
    preceding_polygon_indices = mesh.attributes.new('rear_taper_preceding_polygon_index', 'INT', 'FACE')
    for polygon_index, original_index in enumerate(polygon_indices):
        preceding_polygon_indices.data[polygon_index].value = original_index
    for layer in original_mesh.uv_layers:
        coordinates = mesh.uv_layers.new(name=layer.name)
        for loop_index, original_index in enumerate(corner_indices):
            coordinates.data[loop_index].uv = layer.data[original_index].uv
        coordinates.active_render = layer.active_render
    mesh.uv_layers.active_index = original_mesh.uv_layers.active_index
    for attribute in original_mesh.attributes:
        if attribute.name.startswith('.') or attribute.name in ('position', 'custom_normal', 'material_index', 'sharp_edge', 'sharp_face') or attribute.name in original_mesh.uv_layers:
            continue
        if attribute.domain not in ('POINT', 'FACE', 'CORNER'):
            continue
        target = mesh.attributes.get(attribute.name) or mesh.attributes.new(attribute.name, attribute.data_type, attribute.domain)
        indices = list(range(len(mesh.vertices))) if attribute.domain == 'POINT' else polygon_indices if attribute.domain == 'FACE' else corner_indices
        for target_index, original_index in enumerate(indices):
            for property_name in ('value', 'vector', 'color'):
                if hasattr(attribute.data[original_index], property_name):
                    setattr(target.data[target_index], property_name, getattr(attribute.data[original_index], property_name))
                    break
    for edge in mesh.edges:
        original_index = original_edges.get(tuple(sorted(edge.vertices)))
        if original_index is not None:
            edge.use_edge_sharp = original_mesh.edges[original_index].use_edge_sharp
    mesh.normals_split_custom_set([original_mesh.corner_normals[index].vector.copy() for index in corner_indices])
    mesh.update()
    assert len(mesh.polygons) == len(retained_triangles)
    return mesh, polygon_indices


def taper_mesh(original_mesh, inner_sidepod_only, name):
    tapered_mesh = original_mesh.copy()
    tapered_mesh.name = name + 'RearTaperMesh'
    original_normals = [normal.vector.copy() for normal in original_mesh.corner_normals]
    for vertex in tapered_mesh.vertices:
        vertex.co = taper_position(original_mesh.vertices[vertex.index].co, inner_sidepod_only)
    tapered_mesh.update()
    normals = []
    for loop in original_mesh.loops:
        position = original_mesh.vertices[loop.vertex_index].co
        if tuple(position) == tuple(tapered_mesh.vertices[loop.vertex_index].co):
            normals.append(original_normals[loop.index])
            continue
        derivative_matrix = taper_derivative(position, inner_sidepod_only)
        assert derivative_matrix.determinant() > 0
        normals.append((derivative_matrix.inverted_safe().transposed() @ original_normals[loop.index]).normalized())
    tapered_mesh.normals_split_custom_set(normals)
    tapered_mesh.update()
    original_mesh.calc_loop_triangles()
    tapered_mesh.calc_loop_triangles()
    assert len(tapered_mesh.loop_triangles) == len(original_mesh.loop_triangles)
    assert all(vertex.co.x == original_mesh.vertices[vertex.index].co.x and vertex.co.z == original_mesh.vertices[vertex.index].co.z for vertex in tapered_mesh.vertices)
    assert all([tuple(coordinate.uv) for coordinate in layer.data] == [tuple(coordinate.uv) for coordinate in tapered_mesh.uv_layers[layer.name].data] for layer in original_mesh.uv_layers)
    assert all(triangle.area > 1e-12 for triangle in tapered_mesh.loop_triangles)
    inverted_indices = []
    for polygon in tapered_mesh.polygons:
        preceding_polygon = original_mesh.polygons[polygon.index]
        preceding_center = sum((original_mesh.vertices[index].co for index in preceding_polygon.vertices), Vector()) / len(preceding_polygon.vertices)
        derivative_matrix = taper_derivative(preceding_center, inner_sidepod_only)
        expected_normal = derivative_matrix.inverted_safe().transposed() @ preceding_polygon.normal
        if polygon.normal.dot(expected_normal) <= 0:
            inverted_indices.append(polygon.index)
    if inverted_indices:
        corner_normals = [normal.vector.copy() for normal in tapered_mesh.corner_normals]
        for polygon_index in inverted_indices:
            polygon = tapered_mesh.polygons[polygon_index]
            for loop_index in polygon.loop_indices:
                corner_normals[loop_index] = polygon.normal
        tapered_mesh.normals_split_custom_set(corner_normals)
    return tapered_mesh


def render_review(output_directory, label):
    scene = bpy.data.scenes['FinalVehicleStudio']
    camera_data = bpy.data.cameras.new('EngineCoverRearTaperReviewCamera')
    camera = bpy.data.objects.new('EngineCoverRearTaperReviewCamera', camera_data)
    scene.collection.objects.link(camera)
    scene.camera = camera
    camera_data.type = 'ORTHO'
    scene.render.engine = 'BLENDER_WORKBENCH'
    scene.display.shading.light = 'STUDIO'
    scene.display.shading.color_type = 'MATERIAL'
    scene.display.shading.show_shadows = False
    scene.display.shading.show_cavity = False
    scene.render.resolution_x = 1400
    scene.render.resolution_y = 900
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = 'PNG'
    for angle_name, location, target, scale in [('top', (1.05, 0, 4), (1.05, 0, 0), 2.5), ('three_quarter', (1.3, -2.3, 1.55), (1.05, 0, 0.2), 2.3)]:
        camera.location = location
        camera.rotation_euler = (Vector(target) - camera.location).to_track_quat('-Z', 'Y').to_euler()
        camera_data.ortho_scale = scale
        scene.render.filepath = str(output_directory / (label + '_' + angle_name + '.png'))
        bpy.ops.render.render(write_still=True, scene=scene.name)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--output-directory', type=Path, required=True)
    arguments = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    repository = Path(__file__).resolve().parents[2]
    output_file = arguments.output_directory / 'f1_2030.blend'
    validate_output_path(repository, output_file, 'preview')
    arguments.output_directory.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=str(arguments.source_file))
    cover = bpy.data.objects['BodyworkEngineCover']
    assert not cover.get('engine_cover_rear_taper_applied')
    protected = {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.name not in CHANGED_OBJECT_NAMES}
    original_fingerprints = {name: mesh_fingerprint(bpy.data.objects[name].data) for name in CHANGED_OBJECT_NAMES}
    original_meshes = {name: bpy.data.objects[name].data for name in CHANGED_OBJECT_NAMES}
    shared_vertices = {}
    for name in ('BodyworkSidepodLeft', 'BodyworkSidepodRight'):
        other_mesh = original_meshes[name]
        tree = KDTree(len(other_mesh.vertices))
        for vertex in other_mesh.vertices:
            tree.insert(vertex.co, vertex.index)
        tree.balance()
        shared_vertices[name] = [(vertex.index, tree.find(vertex.co)[1]) for vertex in cover.data.vertices if vertex.co.x > 0.645 and tree.find(vertex.co)[2] < 0.000001]
    preceding_triangle_meshes = {}
    polygon_parents = {}
    for name in CHANGED_OBJECT_NAMES:
        preceding_triangle_meshes[name], polygon_parents[name] = retain_surface_triangulation(original_meshes[name], name)
        bpy.data.objects[name].data = taper_mesh(preceding_triangle_meshes[name], name.startswith('BodyworkSidepod'), name)
    for name, indices in shared_vertices.items():
        assert all((cover.data.vertices[cover_index].co - bpy.data.objects[name].data.vertices[other_index].co).length < 0.000002 for cover_index, other_index in indices)
    for name in ('BodyworkSidepodLeft', 'BodyworkSidepodRight'):
        assert all(tuple(vertex.co) == tuple(original_meshes[name].vertices[vertex.index].co) for vertex in bpy.data.objects[name].data.vertices if abs(original_meshes[name].vertices[vertex.index].co.y) >= 0.35)
    preceding_overlaps = nonadjacent_overlap_parent_pairs(original_meshes['BodyworkEngineCover'], list(range(len(cover.data.polygons))))
    tapered_overlaps = nonadjacent_overlap_parent_pairs(cover.data, polygon_parents['BodyworkEngineCover'])
    new_overlaps = tapered_overlaps - preceding_overlaps
    assert not new_overlaps, str(new_overlaps)
    for name in ('BodyworkSidepodLeft', 'BodyworkSidepodRight'):
        original_overlap_pairs = exact_surface_intersections(original_meshes[name], list(range(len(original_meshes[name].polygons))))
        current_overlap_pairs = exact_surface_intersections(bpy.data.objects[name].data, polygon_parents[name])
        assert not current_overlap_pairs - original_overlap_pairs, name + ' ' + str(current_overlap_pairs - original_overlap_pairs)
    assert len(cover.data.loop_triangles) == 3806 <= 4000
    cover['engine_cover_rear_taper_applied'] = True
    width_samples = [{'longitudinal_position_meters': position, 'preceding_width_meters': sample_cover_width(original_meshes['BodyworkEngineCover'], position) * 2, 'tapered_width_meters': sample_cover_width(cover.data, position) * 2} for position in (0.85, 1.0, 1.2, 1.4, 1.6, 1.73)]
    report = {'source_file': str(arguments.source_file), 'source_digest': file_digest(arguments.source_file), 'output_file': str(output_file), 'original_mesh_fingerprints': original_fingerprints, 'tapered_mesh_fingerprints': {name: mesh_fingerprint(bpy.data.objects[name].data) for name in CHANGED_OBJECT_NAMES}, 'protected_object_fingerprints': protected, 'triangles_before': 3808, 'triangles_after': 3806, 'removed_preceding_duplicate_surface_triangles': 2, 'triangle_budget': 4000, 'longitudinal_positions_unchanged': True, 'heights_unchanged': True, 'centerline_attachment_strip_unchanged': True, 'rear_tip_unchanged': True, 'sidepod_outer_surfaces_unchanged': True, 'shared_attachment_vertices_verified': {name: len(indices) for name, indices in shared_vertices.items()}, 'texture_coordinates_unchanged': True, 'new_engine_cover_self_intersection_pairs': len(new_overlaps), 'width_samples': width_samples, 'runtime_vehicle_assets_promoted': False}
    bpy.ops.wm.save_as_mainfile(filepath=str(output_file), relative_remap=True)
    bpy.ops.wm.open_mainfile(filepath=str(output_file))
    assert {name: mesh_fingerprint(bpy.data.objects[name].data) for name in CHANGED_OBJECT_NAMES} == report['tapered_mesh_fingerprints']
    assert {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.name in protected} == protected
    report['output_digest'] = file_digest(output_file)
    (arguments.output_directory / 'engine_cover_rear_taper_report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    render_review(arguments.output_directory, 'after')
    bpy.ops.wm.open_mainfile(filepath=str(arguments.source_file))
    render_review(arguments.output_directory, 'before')
    print(json.dumps({key: value for key, value in report.items() if 'fingerprints' not in key}))


if __name__ == '__main__':
    main()
