import argparse
import json
import math
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
from smooth_formula_one_2030_engine_cover import attachment_surfaces, boundary_segments, eligible_vertex, file_digest, nonadjacent_overlap_parent_pairs, render_review
from separate_formula_one_2030_bodywork import mesh_fingerprint, protected_object_fingerprint
from common.output_policy import validate_output_path


def refine_surface(original_mesh):
    editing_mesh = bmesh.new()
    editing_mesh.from_mesh(original_mesh)
    editing_mesh.verts.ensure_lookup_table()
    transition_indices = {vertex.index for face in editing_mesh.faces if face.material_index == 3 for vertex in face.verts}
    surfaces = attachment_surfaces()
    movable_indices = {vertex.index for vertex in editing_mesh.verts if eligible_vertex(vertex, surfaces, transition_indices)}
    extrema = [(min(vertex.co[axis] for vertex in editing_mesh.verts), max(vertex.co[axis] for vertex in editing_mesh.verts)) for axis in range(3)]
    movable_indices -= {vertex.index for vertex in editing_mesh.verts if any(vertex.co[axis] in extrema[axis] for axis in range(3))}
    adjacent_indices = {vertex.index: {edge.other_vert(vertex).index for edge in vertex.link_edges} for vertex in editing_mesh.verts}
    boundary_indices = {vertex.index for vertex in editing_mesh.verts if vertex.is_boundary}
    editing_mesh.free()
    preceding_positions = [vertex.co.copy() for vertex in original_mesh.vertices]
    preceding_normals = [normal.vector.copy() for normal in original_mesh.corner_normals]
    refined_mesh = original_mesh.copy()
    refined_mesh.name = 'BodyworkEngineCoverSecondReviewMesh'
    positions = [position.copy() for position in preceding_positions]
    for iteration in range(20):
        updated_positions = [position.copy() for position in positions]
        factor = 0.35 if iteration % 2 == 0 else -0.36
        for vertex_index in movable_indices:
            neighbors = adjacent_indices[vertex_index]
            weighted_average = Vector((0, 0, 0))
            total_weight = 0
            for neighbor_index in neighbors:
                weight = 1 / max((positions[neighbor_index] - positions[vertex_index]).length, 0.0001)
                weighted_average += positions[neighbor_index] * weight
                total_weight += weight
            displacement = positions[vertex_index] + factor * (weighted_average / total_weight - positions[vertex_index]) - preceding_positions[vertex_index]
            if displacement.length > 0.002:
                displacement *= 0.002 / displacement.length
            updated_positions[vertex_index] = preceding_positions[vertex_index] + displacement
            for axis in range(3):
                updated_positions[vertex_index][axis] = max(extrema[axis][0], min(extrema[axis][1], updated_positions[vertex_index][axis]))
        positions = updated_positions
    for vertex, position in zip(refined_mesh.vertices, positions):
        vertex.co = position
    refined_mesh.update()
    area_normals = [Vector((0, 0, 0)) for vertex in refined_mesh.vertices]
    exterior_indices = set()
    for polygon in refined_mesh.polygons:
        if polygon.material_index == 0:
            for vertex_index in polygon.vertices:
                area_normals[vertex_index] += polygon.normal * polygon.area
                exterior_indices.add(vertex_index)
    area_normals = [normal.normalized() for normal in area_normals]
    original_area_normals = [normal.copy() for normal in area_normals]
    normal_protected_indices = boundary_indices | transition_indices | {vertex.index for vertex in refined_mesh.vertices if any(surface.find_nearest(vertex.co)[0] is not None and surface.find_nearest(vertex.co)[3] < 0.009 for surface in surfaces)}
    normal_movable_indices = movable_indices - normal_protected_indices
    for iteration in range(5):
        updated_normals = [normal.copy() for normal in area_normals]
        for vertex_index in normal_movable_indices:
            weighted_average = Vector((0, 0, 0))
            total_weight = 0
            for neighbor_index in adjacent_indices[vertex_index] & exterior_indices:
                weight = 1 / max((positions[neighbor_index] - positions[vertex_index]).length, 0.0001)
                weighted_average += area_normals[neighbor_index] * weight
                total_weight += weight
            if total_weight:
                updated_normals[vertex_index] = (area_normals[vertex_index] * 0.55 + weighted_average / total_weight * 0.45).normalized()
        area_normals = updated_normals
    corner_normals = []
    for polygon in refined_mesh.polygons:
        for loop_index in polygon.loop_indices:
            vertex_index = refined_mesh.loops[loop_index].vertex_index
            if polygon.material_index != 0 or vertex_index in normal_protected_indices:
                corner_normals.append(preceding_normals[loop_index])
            else:
                corner_normals.append(area_normals[vertex_index])
    refined_mesh.normals_split_custom_set(corner_normals)
    refined_mesh.update()
    refined_mesh.calc_loop_triangles()
    original_mesh.calc_loop_triangles()
    assert len(refined_mesh.loop_triangles) == len(original_mesh.loop_triangles) <= 4000
    assert boundary_segments(refined_mesh) == boundary_segments(original_mesh)
    assert all(tuple(vertex.co) == tuple(preceding_positions[vertex.index]) for vertex in refined_mesh.vertices if vertex.index not in movable_indices)
    assert [tuple(polygon.vertices) for polygon in refined_mesh.polygons] == [tuple(polygon.vertices) for polygon in original_mesh.polygons]
    assert all([tuple(coordinate.uv) for coordinate in layer.data] == [tuple(coordinate.uv) for coordinate in refined_mesh.uv_layers[layer.name].data] for layer in original_mesh.uv_layers)
    assert all(polygon.normal.dot(original_mesh.polygons[polygon.index].normal) > 0 for polygon in refined_mesh.polygons)
    assert all((refined_mesh.vertices[triangle.vertices[1]].co - refined_mesh.vertices[triangle.vertices[0]].co).cross(refined_mesh.vertices[triangle.vertices[2]].co - refined_mesh.vertices[triangle.vertices[0]].co).length > 1e-12 for triangle in refined_mesh.loop_triangles)
    preceding_overlaps = nonadjacent_overlap_parent_pairs(original_mesh, list(range(len(original_mesh.polygons))))
    refined_overlaps = nonadjacent_overlap_parent_pairs(refined_mesh, list(range(len(refined_mesh.polygons))))
    assert not refined_overlaps - preceding_overlaps
    preceding_bounds = [(min(position[axis] for position in preceding_positions), max(position[axis] for position in preceding_positions)) for axis in range(3)]
    refined_bounds = [(min(vertex.co[axis] for vertex in refined_mesh.vertices), max(vertex.co[axis] for vertex in refined_mesh.vertices)) for axis in range(3)]
    assert preceding_bounds == refined_bounds
    assert max((vertex.co - preceding_positions[vertex.index]).length for vertex in refined_mesh.vertices) <= 0.002001
    report = {
        'triangles_before': len(original_mesh.loop_triangles),
        'triangles_after': len(refined_mesh.loop_triangles),
        'triangle_budget': 4000,
        'moved_vertices': sum((vertex.co - preceding_positions[vertex.index]).length > 1e-8 for vertex in refined_mesh.vertices),
        'maximum_additional_displacement_meters': max((vertex.co - preceding_positions[vertex.index]).length for vertex in refined_mesh.vertices),
        'geometry_smoothing_iterations': 20,
        'normal_smoothing_iterations': 5,
        'maximum_exterior_normal_adjustment_degrees': max(math.degrees(original_area_normals[index].angle(area_normals[index], 0)) for index in normal_movable_indices),
        'topology_unchanged': True,
        'all_texture_coordinates_unchanged': True,
        'protected_vertices_unchanged': True,
        'attachment_boundary_segments_unchanged': True,
        'overall_bounds_unchanged': True,
        'inverted_polygons': 0,
        'degenerate_triangles': 0,
        'preceding_nonadjacent_overlap_pairs': len(preceding_overlaps),
        'new_nonadjacent_overlap_pairs': 0,
        'preceding_mesh_fingerprint': mesh_fingerprint(original_mesh),
        'refined_mesh_fingerprint': mesh_fingerprint(refined_mesh),
    }
    return refined_mesh, report


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
    assert cover.get('engine_cover_local_smoothing_applied')
    assert not cover.get('engine_cover_second_review_applied')
    protected_fingerprints = {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object != cover}
    refined_mesh, report = refine_surface(cover.data)
    cover.data = refined_mesh
    cover['engine_cover_second_review_applied'] = True
    report.update({'source_file': str(arguments.source_file), 'source_digest': file_digest(arguments.source_file), 'output_file': str(output_file), 'protected_object_fingerprints': protected_fingerprints})
    bpy.ops.wm.save_as_mainfile(filepath=str(output_file), relative_remap=True)
    bpy.ops.wm.open_mainfile(filepath=str(output_file))
    assert mesh_fingerprint(bpy.data.objects['BodyworkEngineCover'].data) == report['refined_mesh_fingerprint']
    assert {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.name in protected_fingerprints} == protected_fingerprints
    report['output_digest'] = file_digest(output_file)
    (arguments.output_directory / 'engine_cover_second_review_report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    render_review(arguments.output_directory, 'after_second_review')
    bpy.ops.wm.open_mainfile(filepath=str(arguments.source_file))
    render_review(arguments.output_directory, 'before_second_review')
    print(json.dumps({key: value for key, value in report.items() if key != 'protected_object_fingerprints'}))


if __name__ == '__main__':
    main()
