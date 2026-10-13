import argparse
import json
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
from separate_formula_one_2030_bodywork import mesh_fingerprint, protected_object_fingerprint
from smooth_formula_one_2030_engine_cover import file_digest
from taper_formula_one_2030_engine_cover_rear import exact_surface_intersections, retain_surface_triangulation
from common.output_policy import validate_output_path


CHANGED_OBJECT_NAMES = ('GEO_FRONT_WING_MAIN_ELEMENT', 'GEO_FRONT_WING_UPPER_ELEMENT')
HALF_SPAN = 0.855910062789917
ENDPLATE_FRONT_POSITION = -2.5323593616485596
CENTER_ADVANCE = 0.065


def leading_profile(mesh, matrix):
    positions = [matrix @ vertex.co for vertex in mesh.vertices]
    lower = min(position.y for position in positions)
    upper = max(position.y for position in positions)
    return {'lower': lower, 'upper': upper, 'edges': [(tuple(positions[edge.vertices[0]]), tuple(positions[edge.vertices[1]])) for edge in mesh.edges], 'cache': {}, 'fallback': min(position.x for position in positions)}


def section_leading_position(lateral_position, profile):
    inset = profile.get('cap_inset', 0)
    position = min(profile['upper'] - inset, max(profile['lower'] + inset, round(lateral_position, 7)))
    if position not in profile['cache']:
        intersections = []
        for first, second in profile['edges']:
            if min(first[1], second[1]) <= position <= max(first[1], second[1]):
                if abs(second[1] - first[1]) < 1e-9:
                    intersections.extend((first[0], second[0]))
                else:
                    fraction = (position - first[1]) / (second[1] - first[1])
                    intersections.append(first[0] * (1 - fraction) + second[0] * fraction)
        profile['cache'][position] = min(intersections, default=profile['fallback'])
    return profile['cache'][position]


def curve_position(position, profile, main_element):
    result = position.copy()
    front = section_leading_position(position.y, profile)
    rear_limit = -2.25 if main_element else -2.18
    if position.x >= rear_limit:
        return result
    fraction = min(1, max(0, (rear_limit - position.x) / max(rear_limit - front, 0.0001)))
    weight = fraction * fraction * (3 - 2 * fraction)
    target = ENDPLATE_FRONT_POSITION - CENTER_ADVANCE * (1 - (position.y / HALF_SPAN) ** 2)
    result.x += min(0, target - front) * weight
    return result


def curve_mesh(scene_object):
    preceding_mesh = scene_object.data
    original_mesh, polygon_parents = retain_surface_triangulation(preceding_mesh, scene_object.name)
    original_mesh.attributes['rear_taper_preceding_polygon_index'].name = 'front_wing_preceding_polygon_index'
    profile = leading_profile(original_mesh, scene_object.matrix_world)
    profile['cap_inset'] = 0.006 if scene_object.name == CHANGED_OBJECT_NAMES[1] else 0.001
    curved_mesh = original_mesh.copy()
    curved_mesh.name = scene_object.name + 'CurvedLeadingEdgeMesh'
    inverse_matrix = scene_object.matrix_world.inverted()
    original_normals = [normal.vector.copy() for normal in original_mesh.corner_normals]
    for vertex in curved_mesh.vertices:
        preceding_position = scene_object.matrix_world @ original_mesh.vertices[vertex.index].co
        vertex.co = inverse_matrix @ curve_position(preceding_position, profile, scene_object.name == CHANGED_OBJECT_NAMES[0])
    curved_mesh.update()
    normals = []
    for loop in original_mesh.loops:
        position = scene_object.matrix_world @ original_mesh.vertices[loop.vertex_index].co
        derivatives = []
        for axis in range(3):
            first = position.copy()
            second = position.copy()
            first[axis] -= 0.0001
            second[axis] += 0.0001
            derivatives.append((curve_position(second, profile, scene_object.name == CHANGED_OBJECT_NAMES[0]) - curve_position(first, profile, scene_object.name == CHANGED_OBJECT_NAMES[0])) / 0.0002)
        derivative_matrix = Matrix(derivatives).transposed()
        assert derivative_matrix.determinant() > 0
        normals.append((derivative_matrix.inverted_safe().transposed() @ original_normals[loop.index]).normalized())
    curved_mesh.normals_split_custom_set(normals)
    curved_mesh.update()
    original_mesh.calc_loop_triangles()
    curved_mesh.calc_loop_triangles()
    assert len(curved_mesh.loop_triangles) == len(original_mesh.loop_triangles)
    assert all(vertex.co.y == original_mesh.vertices[vertex.index].co.y and vertex.co.z == original_mesh.vertices[vertex.index].co.z for vertex in curved_mesh.vertices)
    assert all([tuple(coordinate.uv) for coordinate in layer.data] == [tuple(coordinate.uv) for coordinate in curved_mesh.uv_layers[layer.name].data] for layer in original_mesh.uv_layers)
    assert all(triangle.area > 1e-12 for triangle in curved_mesh.loop_triangles)
    preceding_intersections = exact_surface_intersections(preceding_mesh, list(range(len(preceding_mesh.polygons))))
    curved_intersections = exact_surface_intersections(curved_mesh, polygon_parents)
    assert not curved_intersections - preceding_intersections, scene_object.name + ' ' + str(curved_intersections - preceding_intersections)
    verification_mesh = curved_mesh.copy()
    assert not verification_mesh.validate(verbose=True, clean_customdata=False)
    bpy.data.meshes.remove(verification_mesh)
    return curved_mesh, {'triangles': len(curved_mesh.loop_triangles), 'moved_vertices': sum((vertex.co - original_mesh.vertices[vertex.index].co).length > 1e-7 for vertex in curved_mesh.vertices), 'maximum_forward_displacement_meters': max(original_mesh.vertices[vertex.index].co.x - vertex.co.x for vertex in curved_mesh.vertices), 'new_self_intersection_pairs': 0}


def render_review(directory, label):
    scene = bpy.data.scenes['FinalVehicleStudio']
    camera_data = bpy.data.cameras.new('FrontWingArcReviewCamera')
    camera = bpy.data.objects.new('FrontWingArcReviewCamera', camera_data)
    scene.collection.objects.link(camera)
    camera.location = (-2.18, 0, 4)
    camera.rotation_euler = (Vector((-2.18, 0, 0)) - camera.location).to_track_quat('-Z', 'Y').to_euler()
    camera_data.type = 'ORTHO'
    camera_data.ortho_scale = 2.05
    scene.camera = camera
    scene.render.engine = 'BLENDER_WORKBENCH'
    scene.display.shading.light = 'STUDIO'
    scene.display.shading.color_type = 'MATERIAL'
    scene.display.shading.show_shadows = False
    scene.display.shading.show_cavity = False
    scene.render.resolution_x = 1200
    scene.render.resolution_y = 1100
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = 'PNG'
    scene.render.filepath = str(directory / (label + '_top.png'))
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
    assert not bpy.data.objects[CHANGED_OBJECT_NAMES[0]].get('front_wing_leading_arc_applied')
    protected = {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.name not in CHANGED_OBJECT_NAMES}
    original_fingerprints = {name: mesh_fingerprint(bpy.data.objects[name].data) for name in CHANGED_OBJECT_NAMES}
    measurements = {}
    for name in CHANGED_OBJECT_NAMES:
        scene_object = bpy.data.objects[name]
        scene_object.data, measurements[name] = curve_mesh(scene_object)
    bpy.data.objects[CHANGED_OBJECT_NAMES[0]]['front_wing_leading_arc_applied'] = True
    report = {'source_file': str(arguments.source_file), 'source_digest': file_digest(arguments.source_file), 'output_file': str(output_file), 'original_mesh_fingerprints': original_fingerprints, 'curved_mesh_fingerprints': {name: mesh_fingerprint(bpy.data.objects[name].data) for name in CHANGED_OBJECT_NAMES}, 'protected_object_fingerprints': protected, 'component_measurements': measurements, 'center_ahead_of_endplate_front_meters': CENTER_ADVANCE, 'widths_and_heights_unchanged': True, 'texture_coordinates_unchanged': True, 'endplates_nose_and_mounting_pylons_unchanged': True, 'runtime_vehicle_assets_promoted': False}
    bpy.ops.wm.save_as_mainfile(filepath=str(output_file), relative_remap=True)
    bpy.ops.wm.open_mainfile(filepath=str(output_file))
    assert {name: mesh_fingerprint(bpy.data.objects[name].data) for name in CHANGED_OBJECT_NAMES} == report['curved_mesh_fingerprints']
    assert {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.name in protected} == protected
    report['output_digest'] = file_digest(output_file)
    (arguments.output_directory / 'front_wing_leading_arc_report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    render_review(arguments.output_directory, 'after')
    bpy.ops.wm.open_mainfile(filepath=str(arguments.source_file))
    render_review(arguments.output_directory, 'before')
    print(json.dumps(measurements))


if __name__ == '__main__':
    main()
