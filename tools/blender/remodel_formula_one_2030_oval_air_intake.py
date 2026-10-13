import argparse
import bisect
import json
import math
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
from smooth_formula_one_2030_engine_cover import boundary_segments, file_digest, nonadjacent_overlap_parent_pairs
from separate_formula_one_2030_bodywork import mesh_fingerprint, protected_object_fingerprint
from common.output_policy import validate_output_path


CHANGED_OBJECT_NAMES = ('BodyworkEngineCover', 'GEO_ENGINE_AIR_INTAKE_INLET_SURFACE', 'GEO_ENGINE_COVER_SIDE_CAMERA_HOUSINGS')
OPENING_CENTER_HEIGHT = (0.548608124256134 + 0.3976673483848572) / 2
OPENING_VERTICAL_RADIUS = (0.548608124256134 - 0.3976673483848572) / 2
OPENING_HORIZONTAL_RADIUS = OPENING_VERTICAL_RADIUS / 1.7
PRECEDING_HORIZONTAL_RADIUS = 0.08834455162286758
INLET_REAR_POSITION = 0.47234052419662476
DUCT_EXTENSION = 0.045


def smooth_transition(value, lower, upper):
    fraction = min(1, max(0, (value - lower) / (upper - lower)))
    return fraction * fraction * (3 - 2 * fraction)


def opening_profile(mesh):
    positions = {}
    for vertex in mesh.vertices:
        if abs(vertex.co.x - INLET_REAR_POSITION) < 0.000001 and vertex.co.z > 0.39:
            key = (round(vertex.co.y, 6), round(vertex.co.z, 6))
            positions.setdefault(key, vertex.co.copy())
    assert len(positions) == 24
    samples = []
    for position in positions.values():
        horizontal = position.y / PRECEDING_HORIZONTAL_RADIUS
        vertical = (position.z - OPENING_CENTER_HEIGHT) / OPENING_VERTICAL_RADIUS
        angle = math.atan2(horizontal, vertical) % (2 * math.pi)
        samples.append((angle, math.hypot(horizontal, vertical)))
    samples.sort()
    return [samples[-1][0] - 2 * math.pi] + [sample[0] for sample in samples] + [samples[0][0] + 2 * math.pi], [samples[-1][1]] + [sample[1] for sample in samples] + [samples[0][1]]


def remodel_position(position, profile):
    remodeled = position.copy()
    longitudinal_weight = 1 - smooth_transition(position.x, 0.58, 1.12)
    height_weight = smooth_transition(position.z, 0.22, 0.395)
    weight = longitudinal_weight * height_weight
    if weight == 0:
        return remodeled
    horizontal = position.y / PRECEDING_HORIZONTAL_RADIUS
    vertical = (position.z - OPENING_CENTER_HEIGHT) / OPENING_VERTICAL_RADIUS
    angle = math.atan2(horizontal, vertical) % (2 * math.pi)
    sample_index = bisect.bisect_right(profile[0], angle) - 1
    fraction = (angle - profile[0][sample_index]) / (profile[0][sample_index + 1] - profile[0][sample_index])
    preceding_radius = profile[1][sample_index] * (1 - fraction) + profile[1][sample_index + 1] * fraction
    relative_radius = math.hypot(horizontal, vertical) / preceding_radius
    oval_weight = 1 - smooth_transition(position.x, 0.49, 0.64)
    ellipse_horizontal = OPENING_HORIZONTAL_RADIUS * relative_radius * math.sin(angle)
    ellipse_vertical = OPENING_CENTER_HEIGHT + OPENING_VERTICAL_RADIUS * relative_radius * math.cos(angle)
    narrowed_horizontal = position.y * OPENING_HORIZONTAL_RADIUS / PRECEDING_HORIZONTAL_RADIUS
    remodeled.y = position.y * (1 - weight) + (narrowed_horizontal * (1 - oval_weight) + ellipse_horizontal * oval_weight) * weight
    remodeled.z = position.z + (ellipse_vertical - position.z) * weight * oval_weight
    return remodeled


def transformed_normal(position, normal, profile):
    step = 0.00001
    derivatives = []
    for axis in range(3):
        first_position = position.copy()
        second_position = position.copy()
        first_position[axis] -= step
        second_position[axis] += step
        derivatives.append((remodel_position(second_position, profile) - remodel_position(first_position, profile)) / (2 * step))
    derivative_matrix = Matrix(derivatives).transposed()
    return (derivative_matrix.inverted_safe().transposed() @ normal).normalized()


def unify_front_centerline_normals(mesh):
    mesh.update()
    corner_normals = [normal.vector.copy() for normal in mesh.corner_normals]
    grouped_normals = {}
    grouped_corners = {}
    for polygon in mesh.polygons:
        if polygon.material_index != 0:
            continue
        for loop_index in polygon.loop_indices:
            position = mesh.vertices[mesh.loops[loop_index].vertex_index].co
            if abs(position.y) < 0.000001 and position.z > 0.22 and position.x < 1.12:
                key = tuple(round(component, 6) for component in position)
                grouped_normals.setdefault(key, Vector())
                grouped_normals[key] += corner_normals[loop_index] * polygon.area
                grouped_corners.setdefault(key, []).append(loop_index)
    for key, normal in grouped_normals.items():
        normal.y = 0
        normal.normalize()
        for loop_index in grouped_corners[key]:
            corner_normals[loop_index] = normal
    mesh.normals_split_custom_set(corner_normals)
    mesh.update()


def remodel_cover(original_mesh, profile):
    original_editing_mesh = bmesh.new()
    original_editing_mesh.from_mesh(original_mesh)
    protected_boundary_indices = {vertex.index for vertex in original_editing_mesh.verts if vertex.is_boundary and not (abs(vertex.co.x - INLET_REAR_POSITION) < 0.000001 and vertex.co.z > 0.39)}
    original_editing_mesh.free()
    remodeled_mesh = original_mesh.copy()
    remodeled_mesh.name = 'BodyworkEngineCoverOvalIntakeMesh'
    original_normals = [normal.vector.copy() for normal in original_mesh.corner_normals]
    for vertex in remodeled_mesh.vertices:
        if vertex.index not in protected_boundary_indices:
            vertex.co = remodel_position(original_mesh.vertices[vertex.index].co, profile)
    remodeled_mesh.update()
    transformed_normals = [transformed_normal(original_mesh.vertices[loop.vertex_index].co, original_normals[loop.index], profile) for loop in original_mesh.loops]
    area_normals = [Vector() for vertex in remodeled_mesh.vertices]
    neighbors = [set() for vertex in remodeled_mesh.vertices]
    for edge in remodeled_mesh.edges:
        first_index, second_index = edge.vertices
        neighbors[first_index].add(second_index)
        neighbors[second_index].add(first_index)
    for polygon in remodeled_mesh.polygons:
        if polygon.material_index == 0:
            for vertex_index in polygon.vertices:
                area_normals[vertex_index] += polygon.normal * polygon.area
    area_normals = [normal.normalized() for normal in area_normals]
    for iteration in range(6):
        updated_normals = [normal.copy() for normal in area_normals]
        for vertex in remodeled_mesh.vertices:
            preceding_position = original_mesh.vertices[vertex.index].co
            if preceding_position.x >= 1.12 or preceding_position.z <= 0.22 or not area_normals[vertex.index].length:
                continue
            neighbor_normals = [area_normals[index] for index in neighbors[vertex.index] if area_normals[index].length and area_normals[index].dot(area_normals[vertex.index]) > 0.7]
            if neighbor_normals:
                updated_normals[vertex.index] = (area_normals[vertex.index] * 0.55 + sum(neighbor_normals, Vector()) / len(neighbor_normals) * 0.45).normalized()
        area_normals = updated_normals
    for polygon in remodeled_mesh.polygons:
        if polygon.material_index != 0:
            continue
        for loop_index in polygon.loop_indices:
            vertex_index = remodeled_mesh.loops[loop_index].vertex_index
            preceding_position = original_mesh.vertices[vertex_index].co
            weight = (1 - smooth_transition(preceding_position.x, 0.92, 1.12)) * smooth_transition(preceding_position.z, 0.22, 0.39)
            if weight and area_normals[vertex_index].length:
                transformed_normals[loop_index] = (transformed_normals[loop_index] * (1 - weight) + area_normals[vertex_index] * weight).normalized()
            if vertex_index in protected_boundary_indices and preceding_position.z < 0.35:
                transformed_normals[loop_index] = original_normals[loop_index]
    remodeled_mesh.normals_split_custom_set(transformed_normals)
    editing_mesh = bmesh.new()
    editing_mesh.from_mesh(remodeled_mesh)
    editing_mesh.verts.ensure_lookup_table()
    opening_vertices = {}
    for vertex in editing_mesh.verts:
        if abs(vertex.co.x - INLET_REAR_POSITION) < 0.000001 and vertex.co.z > 0.39:
            opening_vertices.setdefault((round(vertex.co.y, 6), round(vertex.co.z, 6)), vertex)
    front_ring = sorted(opening_vertices.values(), key=lambda vertex: math.atan2(vertex.co.y / OPENING_HORIZONTAL_RADIUS, (vertex.co.z - OPENING_CENTER_HEIGHT) / OPENING_VERTICAL_RADIUS))
    assert len(front_ring) == 24
    rear_ring = []
    for vertex in front_ring:
        position = vertex.co.copy()
        position.x += DUCT_EXTENSION
        position.y *= 0.98
        position.z = OPENING_CENTER_HEIGHT + (position.z - OPENING_CENTER_HEIGHT) * 0.98
        rear_vertex = editing_mesh.verts.new(position)
        for layer_name in ('bodywork_source_vertex_index', 'engine_cover_preceding_vertex_index'):
            layer = editing_mesh.verts.layers.int.get(layer_name)
            if layer is not None:
                rear_vertex[layer] = -1
        rear_ring.append(rear_vertex)
    for vertex_index in range(24):
        following_index = (vertex_index + 1) % 24
        face = editing_mesh.faces.new((front_ring[vertex_index], front_ring[following_index], rear_ring[following_index], rear_ring[vertex_index]))
        face.material_index = 1
        face.smooth = True
        for layer in editing_mesh.loops.layers.uv.values():
            for loop_index, loop in enumerate(face.loops):
                loop[layer].uv = (vertex_index / 24 if loop_index in (0, 3) else (vertex_index + 1) / 24, 1 if loop_index in (2, 3) else 0)
    rear_positions = [vertex.co.copy() for vertex in rear_ring]
    editing_mesh.normal_update()
    editing_mesh.to_mesh(remodeled_mesh)
    editing_mesh.free()
    remodeled_mesh.update()
    assert [tuple(polygon.vertices) for polygon in remodeled_mesh.polygons[:len(original_mesh.polygons)]] == [tuple(polygon.vertices) for polygon in original_mesh.polygons]
    assert all([tuple(coordinate.uv) for coordinate in layer.data] == [tuple(coordinate.uv) for coordinate in remodeled_mesh.uv_layers[layer.name].data[:len(layer.data)]] for layer in original_mesh.uv_layers)
    normals = [normal.vector.copy() for normal in remodeled_mesh.corner_normals]
    normals[:len(transformed_normals)] = transformed_normals
    remodeled_mesh.normals_split_custom_set(normals)
    unify_front_centerline_normals(remodeled_mesh)
    remodeled_mesh.calc_loop_triangles()
    assert len(remodeled_mesh.loop_triangles) == 3808
    assert all(triangle.area > 1e-12 for triangle in remodeled_mesh.loop_triangles)
    assert all(polygon.normal.dot(original_mesh.polygons[polygon.index].normal) > 0 for polygon in remodeled_mesh.polygons[:len(original_mesh.polygons)])
    assert all(tuple(remodeled_mesh.vertices[index].co) == tuple(original_mesh.vertices[index].co) for index in protected_boundary_indices)
    return remodeled_mesh, rear_positions


def remodel_inlet(original_mesh, rear_positions):
    remodeled_mesh = bpy.data.meshes.new('EngineAirIntakeRecessedOvalBackingMesh')
    positions = [position.copy() for position in rear_positions]
    center = sum(positions, Vector()) / len(positions)
    positions.append(center)
    faces = [(index, (index + 1) % len(rear_positions), len(rear_positions)) for index in range(len(rear_positions))]
    remodeled_mesh.from_pydata(positions, [], faces)
    remodeled_mesh.update()
    for material in original_mesh.materials:
        remodeled_mesh.materials.append(material)
    for coordinates in original_mesh.uv_layers:
        layer = remodeled_mesh.uv_layers.new(name=coordinates.name)
        for loop in remodeled_mesh.loops:
            position = remodeled_mesh.vertices[loop.vertex_index].co
            layer.data[loop.index].uv = (0.5 + position.y / (2 * OPENING_HORIZONTAL_RADIUS), 0.5 + (position.z - OPENING_CENTER_HEIGHT) / (2 * OPENING_VERTICAL_RADIUS))
    assert all(polygon.normal.x < -0.99 for polygon in remodeled_mesh.polygons)
    return remodeled_mesh


def render_review(output_directory, label):
    scene = bpy.data.scenes['FinalVehicleStudio']
    camera_data = bpy.data.cameras.new('OvalAirIntakeReviewCamera')
    camera = bpy.data.objects.new('OvalAirIntakeReviewCamera', camera_data)
    scene.collection.objects.link(camera)
    scene.camera = camera
    camera_data.type = 'ORTHO'
    scene.render.engine = 'BLENDER_WORKBENCH'
    scene.display.shading.light = 'STUDIO'
    scene.display.shading.color_type = 'MATERIAL'
    scene.display.shading.show_cavity = False
    scene.display.shading.show_shadows = True
    scene.render.resolution_x = 1200
    scene.render.resolution_y = 1000
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = 'PNG'
    for angle_name, location, target, scale in [('front', (-2, 0, 0.49), (0.49, 0, 0.46), 0.58), ('three_quarter', (-1.1, -1.4, 1.0), (0.55, 0, 0.43), 0.68), ('side', (0.9, -3, 0.8), (0.85, 0, 0.32), 1.7)]:
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
    assert not cover.get('oval_air_intake_remodel_applied')
    protected = {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.name not in CHANGED_OBJECT_NAMES}
    original_fingerprints = {name: mesh_fingerprint(bpy.data.objects[name].data) for name in CHANGED_OBJECT_NAMES}
    original_mesh = cover.data
    profile = opening_profile(original_mesh)
    cover.data, rear_positions = remodel_cover(original_mesh, profile)
    inlet = bpy.data.objects['GEO_ENGINE_AIR_INTAKE_INLET_SURFACE']
    inlet.data = remodel_inlet(inlet.data, rear_positions)
    cameras = bpy.data.objects['GEO_ENGINE_COVER_SIDE_CAMERA_HOUSINGS']
    camera_mesh = cameras.data.copy()
    camera_mesh.name = 'EngineCoverNarrowNeckSideCameraMesh'
    preceding_camera_normals = [normal.vector.copy() for normal in cameras.data.corner_normals]
    for vertex in camera_mesh.vertices:
        anchor = vertex.co.copy()
        anchor.y = math.copysign(0.10269683599472046, anchor.y)
        vertex.co.y += remodel_position(anchor, profile).y - anchor.y
    camera_mesh.update()
    camera_mesh.normals_split_custom_set(preceding_camera_normals)
    cameras.data = camera_mesh
    cover['oval_air_intake_remodel_applied'] = True
    assert {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.name in protected} == protected
    lower_indices = [vertex.index for vertex in original_mesh.vertices if vertex.co.z <= 0.22 or vertex.co.x >= 1.12]
    assert all(tuple(original_mesh.vertices[index].co) == tuple(cover.data.vertices[index].co) for index in lower_indices)
    preceding_boundary = boundary_segments(original_mesh)
    retained_boundary = {segment for segment in preceding_boundary if all(position[2] <= 0.22 or position[0] >= 1.12 for position in segment)}
    assert retained_boundary <= boundary_segments(cover.data)
    preceding_overlaps = nonadjacent_overlap_parent_pairs(original_mesh, list(range(len(original_mesh.polygons))))
    preceding_polygon_mesh = cover.data.copy()
    editing_mesh = bmesh.new()
    editing_mesh.from_mesh(preceding_polygon_mesh)
    editing_mesh.faces.ensure_lookup_table()
    bmesh.ops.delete(editing_mesh, geom=list(editing_mesh.faces)[len(original_mesh.polygons):], context='FACES_ONLY')
    editing_mesh.to_mesh(preceding_polygon_mesh)
    editing_mesh.free()
    remodeled_overlaps = nonadjacent_overlap_parent_pairs(preceding_polygon_mesh, list(range(len(original_mesh.polygons))))
    new_overlaps = remodeled_overlaps - preceding_overlaps
    complete_overlaps = nonadjacent_overlap_parent_pairs(cover.data, list(range(len(cover.data.polygons))))
    sleeve_overlaps = {pair for pair in complete_overlaps if max(pair) >= len(original_mesh.polygons)}
    assert not sleeve_overlaps, str(sleeve_overlaps)
    report = {'source_file': str(arguments.source_file), 'source_digest': file_digest(arguments.source_file), 'output_file': str(output_file), 'opening_height_meters': OPENING_VERTICAL_RADIUS * 2, 'opening_width_meters': OPENING_HORIZONTAL_RADIUS * 2, 'opening_height_width_ratio': 1.7, 'preceding_opening_width_meters': PRECEDING_HORIZONTAL_RADIUS * 2, 'duct_extension_meters': DUCT_EXTENSION, 'triangles_after': len(cover.data.loop_triangles), 'triangle_budget': 4000, 'protected_lower_and_rear_vertices': len(lower_indices), 'protected_lower_and_rear_vertices_unchanged': True, 'retained_original_texture_coordinates_unchanged': True, 'new_original_surface_overlap_pairs': len(new_overlaps), 'preceding_original_surface_overlap_pairs': len(preceding_overlaps), 'original_mesh_fingerprints': original_fingerprints, 'remodeled_mesh_fingerprints': {name: mesh_fingerprint(bpy.data.objects[name].data) for name in CHANGED_OBJECT_NAMES}, 'protected_object_fingerprints': protected, 'runtime_vehicle_assets_promoted': False}
    assert not new_overlaps, str(new_overlaps)
    report['new_duct_self_intersection_pairs'] = len(sleeve_overlaps)
    bpy.data.meshes.remove(preceding_polygon_mesh)
    bpy.ops.wm.save_as_mainfile(filepath=str(output_file), relative_remap=True)
    bpy.ops.wm.open_mainfile(filepath=str(output_file))
    assert {name: mesh_fingerprint(bpy.data.objects[name].data) for name in CHANGED_OBJECT_NAMES} == report['remodeled_mesh_fingerprints']
    assert {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.name in protected} == protected
    report['output_digest'] = file_digest(output_file)
    (arguments.output_directory / 'oval_air_intake_remodel_report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    render_review(arguments.output_directory, 'after')
    bpy.ops.wm.open_mainfile(filepath=str(arguments.source_file))
    render_review(arguments.output_directory, 'before')
    print(json.dumps({key: value for key, value in report.items() if 'fingerprints' not in key}))


if __name__ == '__main__':
    main()
