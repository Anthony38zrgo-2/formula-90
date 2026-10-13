import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Vector
from mathutils.bvhtree import BVHTree

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from common.output_policy import validate_output_path
from separate_formula_one_2030_bodywork import protected_object_fingerprint, mesh_fingerprint


def file_digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def boundary_segments(mesh):
    edge_face_counts = {}
    for polygon in mesh.polygons:
        for edge_key in polygon.edge_keys:
            key = tuple(sorted(edge_key))
            edge_face_counts[key] = edge_face_counts.get(key, 0) + 1
    return {tuple(sorted((tuple(mesh.vertices[first_index].co), tuple(mesh.vertices[second_index].co)))) for (first_index, second_index), count in edge_face_counts.items() if count == 1}


def nonadjacent_overlap_parent_pairs(mesh, polygon_parents):
    mesh.calc_loop_triangles()
    coordinates = [tuple(vertex.co) for vertex in mesh.vertices]
    triangles = [tuple(triangle.vertices) for triangle in mesh.loop_triangles]
    parents = [polygon_parents[triangle.polygon_index] for triangle in mesh.loop_triangles]
    overlap_tree = BVHTree.FromPolygons(coordinates, triangles, all_triangles=True)
    overlaps = set()
    for first_triangle, second_triangle in overlap_tree.overlap(overlap_tree):
        if first_triangle >= second_triangle:
            continue
        if {coordinates[index] for index in triangles[first_triangle]} & {coordinates[index] for index in triangles[second_triangle]}:
            continue
        overlaps.add(tuple(sorted((parents[first_triangle], parents[second_triangle]))))
    return overlaps


def render_review(output_directory, label):
    scene = bpy.data.scenes['FinalVehicleStudio']
    camera_data = bpy.data.cameras.new('EngineCoverCorrectionReviewCamera')
    camera = bpy.data.objects.new('EngineCoverCorrectionReviewCamera', camera_data)
    scene.collection.objects.link(camera)
    camera.location = (1.4, -3.4, 1.05)
    camera.rotation_euler = (Vector((0.95, 0, 0.27)) - camera.location).to_track_quat('-Z', 'Y').to_euler()
    camera_data.type = 'ORTHO'
    camera_data.ortho_scale = 2.25
    scene.camera = camera
    scene.render.engine = 'BLENDER_WORKBENCH'
    scene.display.shading.light = 'MATCAP'
    scene.display.shading.studio_light = 'metal_carpaint.exr'
    scene.display.shading.color_type = 'SINGLE'
    scene.display.shading.single_color = (0.65, 0.65, 0.65)
    scene.display.shading.show_cavity = False
    scene.display.shading.show_shadows = False
    scene.display.shading.show_specular_highlight = True
    scene.render.resolution_x = 1600
    scene.render.resolution_y = 800
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = 'PNG'
    output_path = output_directory / (label + '.png')
    validate_output_path(Path(__file__).resolve().parents[2], output_path, 'preview')
    scene.render.filepath = str(output_path)
    bpy.ops.render.render(write_still=True, scene=scene.name)


def attachment_surfaces():
    names = [
        'GEO_ENGINE_AIR_INTAKE_INLET_SURFACE',
        'GEO_ENGINE_COVER_SIDE_CAMERA_HOUSINGS',
        'GEO_ENGINE_COVER_TOP_CAMERA_HOUSING',
        'BodyworkEngineCoverCentralFin',
        'GEO_ENGINE_COVER_REAR_CLOSURE',
    ]
    surfaces = []
    for name in names:
        scene_object = bpy.data.objects[name]
        vertices = [scene_object.matrix_world @ vertex.co for vertex in scene_object.data.vertices]
        polygons = [tuple(polygon.vertices) for polygon in scene_object.data.polygons]
        surfaces.append(BVHTree.FromPolygons(vertices, polygons))
    return surfaces


def eligible_vertex(vertex, surfaces, original_material_vertex_indices):
    if vertex.is_boundary or vertex.index in original_material_vertex_indices:
        return False
    if not (0.52 < vertex.co.x < 1.72 and vertex.co.z > 0.2 and abs(vertex.co.y) > 0.014):
        return False
    if any(surface.find_nearest(vertex.co)[3] < 0.009 for surface in surfaces if surface.find_nearest(vertex.co)[0] is not None):
        return False
    return True


def correct_cover(original_mesh, maximum_triangles):
    editing_mesh = bmesh.new()
    editing_mesh.from_mesh(original_mesh)
    editing_mesh.verts.ensure_lookup_table()
    editing_mesh.edges.ensure_lookup_table()
    editing_mesh.faces.ensure_lookup_table()
    editing_mesh.normal_update()
    original_vertex_layer = editing_mesh.verts.layers.int.new('engine_cover_preceding_vertex_index')
    original_face_layer = editing_mesh.faces.layers.int.new('engine_cover_preceding_polygon_index')
    for vertex in editing_mesh.verts:
        vertex[original_vertex_layer] = vertex.index
    for face in editing_mesh.faces:
        face[original_face_layer] = face.index
    transition_vertices = {vertex.index for face in editing_mesh.faces if face.material_index == 3 for vertex in face.verts}
    surfaces = attachment_surfaces()
    eligible_vertices = {vertex for vertex in editing_mesh.verts if eligible_vertex(vertex, surfaces, transition_vertices)}
    eligible_original_indices = {vertex.index for vertex in eligible_vertices}
    protected_indices = {vertex.index for vertex in editing_mesh.verts if vertex not in eligible_vertices}
    candidate_edges = []
    for edge in editing_mesh.edges:
        if len(edge.link_faces) != 2 or not all(vertex in eligible_vertices for vertex in edge.verts):
            continue
        if any(face.material_index != 0 for face in edge.link_faces):
            continue
        angle = math.degrees(edge.calc_face_angle())
        if 2 < angle < 65 and edge.calc_length() > 0.018:
            candidate_edges.append((edge.calc_length() * min(angle, 30), edge.index))
    candidate_edges.sort(reverse=True)
    selected_edge_indices = [edge_index for score, edge_index in candidate_edges[:220]]
    selected_edges = [editing_mesh.edges[index] for index in selected_edge_indices]
    old_locations = {vertex.index: vertex.co.copy() for vertex in editing_mesh.verts}
    coordinate_indices = {}
    for original_index, original_location in old_locations.items():
        coordinate_indices.setdefault(tuple(original_location), []).append(original_index)
    if selected_edges:
        bmesh.ops.subdivide_edges(editing_mesh, edges=selected_edges, cuts=1, use_grid_fill=True)
    new_vertices = set()
    for vertex in editing_mesh.verts:
        preceding_index = vertex[original_vertex_layer]
        if preceding_index in old_locations and tuple(vertex.co) == tuple(old_locations[preceding_index]):
            continue
        if tuple(vertex.co) in coordinate_indices:
            vertex[original_vertex_layer] = coordinate_indices[tuple(vertex.co)][0]
        else:
            vertex[original_vertex_layer] = -1
            new_vertices.add(vertex)
    retained_indices = {vertex[original_vertex_layer] for vertex in editing_mesh.verts if vertex[original_vertex_layer] >= 0}
    assert retained_indices == set(old_locations), 'Preceding vertices lost during subdivision'
    movement_origins = {vertex: old_locations[vertex[original_vertex_layer]].copy() if vertex[original_vertex_layer] >= 0 else vertex.co.copy() for vertex in editing_mesh.verts}
    movable_vertices = {vertex for vertex in editing_mesh.verts if vertex[original_vertex_layer] in eligible_original_indices} | new_vertices
    movement_limit = 0.004
    smoothing_iterations = 12
    for iteration in range(smoothing_iterations):
        for relaxation_factor in (0.33, -0.34):
            vertex_updates = {}
            for vertex in movable_vertices:
                neighbors = [edge.other_vert(vertex) for edge in vertex.link_edges]
                if not neighbors:
                    continue
                weighted_center = Vector()
                total_weight = 0
                for neighbor in neighbors:
                    weight = 1 / max((neighbor.co - vertex.co).length, 0.0001)
                    weighted_center += neighbor.co * weight
                    total_weight += weight
                proposed_location = vertex.co + relaxation_factor * (weighted_center / total_weight - vertex.co)
                displacement = proposed_location - movement_origins[vertex]
                if displacement.length > movement_limit:
                    proposed_location = movement_origins[vertex] + displacement.normalized() * movement_limit
                vertex_updates[vertex] = proposed_location
            for vertex, proposed_location in vertex_updates.items():
                vertex.co = proposed_location
    editing_mesh.normal_update()
    edited_mesh = bpy.data.meshes.new('BodyworkEngineCoverLocallySmoothedMesh')
    for material in original_mesh.materials:
        edited_mesh.materials.append(material)
    editing_mesh.to_mesh(edited_mesh)
    edited_mesh.update()
    if edited_mesh.attributes.get('custom_normal') is not None:
        edited_mesh.attributes.remove(edited_mesh.attributes['custom_normal'])
    if edited_mesh.attributes.get('bodywork_preserved_corner_normal') is not None:
        edited_mesh.attributes.remove(edited_mesh.attributes['bodywork_preserved_corner_normal'])
    source_vertex_layer = editing_mesh.verts.layers.int.get('bodywork_source_vertex_index')
    if source_vertex_layer is not None:
        for vertex in new_vertices:
            vertex[source_vertex_layer] = -1
        editing_mesh.to_mesh(edited_mesh)
        if edited_mesh.attributes.get('custom_normal') is not None:
            edited_mesh.attributes.remove(edited_mesh.attributes['custom_normal'])
        if edited_mesh.attributes.get('bodywork_preserved_corner_normal') is not None:
            edited_mesh.attributes.remove(edited_mesh.attributes['bodywork_preserved_corner_normal'])
    sharp_edges_before = sum(edge.use_edge_sharp for edge in original_mesh.edges)
    edge_faces = {tuple(sorted(edge.vertices)): [] for edge in edited_mesh.edges}
    for polygon in edited_mesh.polygons:
        for edge_key in polygon.edge_keys:
            edge_faces[tuple(sorted(edge_key))].append(polygon.index)
    provenance = edited_mesh.attributes['engine_cover_preceding_vertex_index']
    polygon_provenance = edited_mesh.attributes['engine_cover_preceding_polygon_index']
    maximum_retained_texture_coordinate_difference = 0
    for coordinates in original_mesh.uv_layers:
        original_corner_values = {(polygon.index, original_mesh.loops[loop_index].vertex_index): coordinates.data[loop_index].uv.copy() for polygon in original_mesh.polygons for loop_index in polygon.loop_indices}
        edited_coordinates = edited_mesh.uv_layers[coordinates.name]
        for polygon in edited_mesh.polygons:
            preceding_polygon_index = polygon_provenance.data[polygon.index].value
            for loop_index in polygon.loop_indices:
                preceding_vertex_index = provenance.data[edited_mesh.loops[loop_index].vertex_index].value
                if preceding_vertex_index >= 0:
                    difference = (edited_coordinates.data[loop_index].uv - original_corner_values[(preceding_polygon_index, preceding_vertex_index)]).length
                    maximum_retained_texture_coordinate_difference = max(maximum_retained_texture_coordinate_difference, difference)
    assert maximum_retained_texture_coordinate_difference < 0.000001, 'Preceding texture coordinates changed'
    for edge in edited_mesh.edges:
        face_indices = edge_faces[tuple(sorted(edge.vertices))]
        if len(face_indices) == 2 and all(edited_mesh.polygons[index].material_index == 0 for index in face_indices):
            edge.use_edge_sharp = False
    edited_mesh.update()
    corner_normals = [normal.vector.copy() for normal in edited_mesh.corner_normals]
    weighted_vertex_normals = [Vector() for vertex in edited_mesh.vertices]
    for polygon in edited_mesh.polygons:
        if polygon.material_index == 0:
            for vertex_index in polygon.vertices:
                weighted_vertex_normals[vertex_index] += polygon.normal * polygon.area
    normal_protected_indices = {vertex.index for vertex in original_mesh.vertices if vertex.index in transition_vertices}
    original_edge_face_counts = {}
    for polygon in original_mesh.polygons:
        for edge_key in polygon.edge_keys:
            key = tuple(sorted(edge_key))
            original_edge_face_counts[key] = original_edge_face_counts.get(key, 0) + 1
    for edge in original_mesh.edges:
        if original_edge_face_counts[tuple(sorted(edge.vertices))] == 1:
            normal_protected_indices.update(edge.vertices)
    for vertex in original_mesh.vertices:
        if any(surface.find_nearest(vertex.co)[3] < 0.009 for surface in surfaces if surface.find_nearest(vertex.co)[0] is not None):
            normal_protected_indices.add(vertex.index)
    preceding_normals = {}
    for polygon in original_mesh.polygons:
        for loop_index in polygon.loop_indices:
            vertex_index = original_mesh.loops[loop_index].vertex_index
            preceding_normals.setdefault(vertex_index, []).append((polygon.normal.copy(), original_mesh.corner_normals[loop_index].vector.copy()))
    for polygon in edited_mesh.polygons:
        for loop_index in polygon.loop_indices:
            vertex_index = edited_mesh.loops[loop_index].vertex_index
            original_index = provenance.data[vertex_index].value
            if polygon.material_index == 0 and weighted_vertex_normals[vertex_index].length:
                corner_normals[loop_index] = weighted_vertex_normals[vertex_index].normalized()
            if original_index in normal_protected_indices:
                candidates = preceding_normals[original_index]
                corner_normals[loop_index] = max(candidates, key=lambda pair: polygon.normal.dot(pair[0]))[1]
    edited_mesh.normals_split_custom_set(corner_normals)
    for original_coordinates in original_mesh.uv_layers:
        edited_mesh.uv_layers[original_coordinates.name].active_render = original_coordinates.active_render
    edited_mesh.uv_layers.active_index = original_mesh.uv_layers.active_index
    edited_mesh.calc_loop_triangles()
    if len(edited_mesh.loop_triangles) > maximum_triangles:
        raise ValueError(f'Triangle budget exceeded: {len(edited_mesh.loop_triangles)} > {maximum_triangles}')
    changed_vertices = []
    maximum_displacement = 0
    for vertex in edited_mesh.vertices:
        original_index = provenance.data[vertex.index].value
        if original_index >= 0:
            displacement = (vertex.co - original_mesh.vertices[original_index].co).length
            maximum_displacement = max(maximum_displacement, displacement)
            if original_index in protected_indices:
                assert displacement == 0, 'Protected attachment vertex moved'
            if displacement > 0.0000001:
                changed_vertices.append(original_index)
    assert maximum_displacement <= movement_limit + 0.000001
    original_mesh.calc_loop_triangles()
    original_zero_area = sum(triangle.area < 0.000000000001 for triangle in original_mesh.loop_triangles)
    edited_zero_area = sum(triangle.area < 0.000000000001 for triangle in edited_mesh.loop_triangles)
    assert edited_zero_area <= original_zero_area, 'New degenerate triangles'
    assert boundary_segments(original_mesh) == boundary_segments(edited_mesh), 'Attachment boundary changed'
    for polygon in edited_mesh.polygons:
        assert polygon.normal.dot(original_mesh.polygons[polygon_provenance.data[polygon.index].value].normal) >= 0, 'Inverted polygon'
    original_overlap_pairs = nonadjacent_overlap_parent_pairs(original_mesh, list(range(len(original_mesh.polygons))))
    edited_overlap_pairs = nonadjacent_overlap_parent_pairs(edited_mesh, [entry.value for entry in polygon_provenance.data])
    assert not edited_overlap_pairs - original_overlap_pairs, 'New nonadjacent self intersection'
    original_bounds = [(min(vertex.co[axis] for vertex in original_mesh.vertices), max(vertex.co[axis] for vertex in original_mesh.vertices)) for axis in range(3)]
    edited_bounds = [(min(vertex.co[axis] for vertex in edited_mesh.vertices), max(vertex.co[axis] for vertex in edited_mesh.vertices)) for axis in range(3)]
    assert original_bounds == edited_bounds, 'Overall silhouette bounds changed'
    report = {
        'triangles_before': len(original_mesh.loop_triangles),
        'triangles_after': len(edited_mesh.loop_triangles),
        'triangle_budget': maximum_triangles,
        'subdivided_edges': len(selected_edges),
        'new_vertices': len(new_vertices),
        'moved_preceding_vertices': len(changed_vertices),
        'maximum_preceding_vertex_displacement_meters': maximum_displacement,
        'maximum_allowed_displacement_meters': movement_limit,
        'protected_vertex_count': len(protected_indices),
        'protected_vertices_unchanged': True,
        'sharp_edges_before': sharp_edges_before,
        'sharp_edges_after': sum(edge.use_edge_sharp for edge in edited_mesh.edges),
        'texture_coordinate_layer_names': [coordinates.name for coordinates in edited_mesh.uv_layers],
        'maximum_retained_texture_coordinate_difference': maximum_retained_texture_coordinate_difference,
        'degenerate_triangles_before': original_zero_area,
        'degenerate_triangles_after': edited_zero_area,
        'smoothing_iterations': smoothing_iterations,
        'attachment_boundary_segments_unchanged': True,
        'overall_bounds_unchanged': True,
        'inverted_polygons': 0,
        'preceding_nonadjacent_overlap_parent_pairs': len(original_overlap_pairs),
        'new_nonadjacent_overlap_parent_pairs': 0,
        'corner_normal_policy': 'Area weighted exterior normals with preceding normals retained on attachment boundaries, accessory contacts and the lower transition material.',
    }
    editing_mesh.free()
    return edited_mesh, report


def main():
    arguments = sys.argv[sys.argv.index('--') + 1:]
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--output-directory', type=Path, required=True)
    parser.add_argument('--maximum-triangles', type=int, default=4000)
    options = parser.parse_args(arguments)
    repository = Path(__file__).resolve().parents[2]
    validate_output_path(repository, options.output_directory, 'preview')
    options.output_directory.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=str(options.source_file))
    source_cover = bpy.data.objects['BodyworkEngineCover']
    if source_cover.get('engine_cover_local_smoothing_applied'):
        raise ValueError('Source already has this correction')
    protected_objects = {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object != source_cover}
    original_mesh = source_cover.data
    original_cover_fingerprint = mesh_fingerprint(original_mesh)
    corrected_mesh, report = correct_cover(original_mesh, options.maximum_triangles)
    source_cover.data = corrected_mesh
    source_cover['engine_cover_local_smoothing_applied'] = True
    source_cover['engine_cover_preceding_source_digest'] = file_digest(options.source_file)
    assert {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.name in protected_objects} == protected_objects
    output_source = options.output_directory / 'f1_2030.blend'
    validate_output_path(repository, output_source, 'preview')
    bpy.ops.wm.save_as_mainfile(filepath=str(output_source), relative_remap=True)
    report.update({'source_file': str(options.source_file), 'source_digest': file_digest(options.source_file), 'output_file': str(output_source), 'output_digest': file_digest(output_source), 'original_cover_mesh_fingerprint': original_cover_fingerprint, 'corrected_cover_mesh_fingerprint': mesh_fingerprint(corrected_mesh), 'protected_object_fingerprints': protected_objects, 'protected_objects_preserved': True})
    bpy.ops.wm.open_mainfile(filepath=str(output_source))
    assert {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.name in protected_objects} == protected_objects
    assert mesh_fingerprint(bpy.data.objects['BodyworkEngineCover'].data) == report['corrected_cover_mesh_fingerprint']
    report['saved_source_reopened_and_protected_objects_verified'] = True
    report_path = options.output_directory / 'engine_cover_smoothing_report.json'
    validate_output_path(repository, report_path, 'preview')
    report_path.write_text(json.dumps(report, indent=2), encoding='utf-8')
    render_review(options.output_directory, 'after_engine_cover_correction')
    bpy.ops.wm.open_mainfile(filepath=str(options.source_file))
    render_review(options.output_directory, 'before_engine_cover_correction')
    print(json.dumps({key: value for key, value in report.items() if key != 'protected_object_fingerprints'}, indent=2))


if __name__ == '__main__':
    main()
