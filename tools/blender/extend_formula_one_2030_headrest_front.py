import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Matrix, Vector

project_directory = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(project_directory / 'tools'))
sys.path.insert(0, str(project_directory / 'tools/blender'))
from common.output_policy import validate_output_path
from fit_f1_2030_driver_seat import mesh_fingerprint
from seal_formula_one_2030_cockpit_interior import cockpit_opening_contour
from raise_formula_one_2030_headrest_support import studio_settings
from validate_formula_one_2030_cockpit_geometry import inspect_mesh_geometry


def trim_restored_headrest(headrest, conversion, cut_center, cut_slope, minimum_longitudinal):
    original_positions = [vertex.co.copy() for vertex in headrest.data.vertices]
    attachment_positions = {tuple(original_positions[index]) for index in headrest['cockpit_attachment_vertex_indices']}
    seam_positions = {tuple(original_positions[index]) for index in headrest['cockpit_upper_seam_vertex_indices']}
    topology = bmesh.new()
    topology.from_mesh(headrest.data)
    height_layer = topology.verts.layers.float.new('CockpitRimHeightLimit')
    original_index_layer = topology.verts.layers.int.new('OriginalHeadrestVertexIndex')
    for vertex, height in zip(topology.verts, headrest['cockpit_rim_height_limits']):
        vertex[height_layer] = height
        vertex[original_index_layer] = vertex.index
        vertex.co = conversion @ vertex.co
    for side_sign in (-1, 1):
        selected = [vertex for vertex in topology.verts if vertex.co.x * side_sign >= -0.000001 and vertex.co.y > minimum_longitudinal]
        selected += [edge for edge in topology.edges if any(vertex.co.x * side_sign >= -0.000001 and vertex.co.y > minimum_longitudinal for vertex in edge.verts)]
        selected += [face for face in topology.faces if any(vertex.co.x * side_sign >= -0.000001 and vertex.co.y > minimum_longitudinal for vertex in face.verts)]
        result = bmesh.ops.bisect_plane(topology, geom=selected, dist=0.0000001, plane_co=Vector((0, cut_center, 0)), plane_no=Vector((-side_sign * cut_slope, 1, 0)), clear_outer=True, clear_inner=False)
        boundary = [element for element in result['geom_cut'] if isinstance(element, bmesh.types.BMEdge) and element.is_boundary]
        caps = bmesh.ops.holes_fill(topology, edges=boundary, sides=0)['faces']
        coordinates = topology.loops.layers.uv.get('CockpitHeadrestSurfaceCoordinates')
        for cap in caps:
            cap.material_index = 0
            cap.smooth = False
            if coordinates:
                for loop in cap.loops:
                    loop[coordinates].uv = (loop.vert.co.x, loop.vert.co.z)
    bmesh.ops.recalc_face_normals(topology, faces=list(topology.faces))
    height_limits = [vertex[height_layer] for vertex in topology.verts]
    inverse_conversion = conversion.inverted()
    for vertex in topology.verts:
        vertex.co = inverse_conversion @ vertex.co
        original_index = vertex[original_index_layer]
        if 0 <= original_index < len(original_positions) and (vertex.co - original_positions[original_index]).length < 0.0000001:
            vertex.co = original_positions[original_index]
    topology.to_mesh(headrest.data)
    topology.free()
    headrest.data.update()
    headrest['cockpit_rim_height_limits'] = height_limits
    headrest['cockpit_attachment_vertex_indices'] = [vertex.index for vertex in headrest.data.vertices if tuple(vertex.co) in attachment_positions]
    headrest['cockpit_upper_seam_vertex_indices'] = [vertex.index for vertex in headrest.data.vertices if tuple(vertex.co) in seam_positions]
    headrest['front_trimmed'] = True
    headrest['front_cut_center_longitudinal_meters'] = cut_center
    headrest['front_cut_lateral_slope'] = cut_slope
    geometry = inspect_mesh_geometry(headrest)
    if not geometry['passed']:
        raise RuntimeError('The front transition failed geometry validation')
    return geometry


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--untrimmed-source-file', type=Path, required=True)
    parser.add_argument('--output-directory', type=Path, required=True)
    parser.add_argument('--taper-front-ends', action='store_true')
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    options.source_file = options.source_file.resolve()
    options.untrimmed_source_file = options.untrimmed_source_file.resolve()
    output_directory = validate_output_path(project_directory, options.output_directory, 'preview').path
    output_directory.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=str(options.source_file), load_ui=False, use_scripts=False)
    headrest = bpy.data.objects['GEO_CHASSIS_CockpitHeadrestPadding']
    if headrest.get('front_tapered_to_chassis') or (headrest.get('front_extended_to_cockpit_middle') and not options.taper_front_ends):
        raise RuntimeError('Use the source preceding the front extension')
    protected = {character.name: mesh_fingerprint(character) for character in bpy.data.objects if character.type == 'MESH' and character != headrest}
    original_settings = studio_settings()
    manifest = json.loads((project_directory / 'game/assets/models/vehicles/f1-2030/manifest.json').read_text())
    conversion = Matrix.Translation((0, 0, manifest['authored_alignment']['chassis_vertical_offset'])) @ Matrix.Rotation(-math.pi / 2, 4, 'Z')
    contour = [conversion @ position for position in cockpit_opening_contour()]
    original_positions = [conversion @ headrest.matrix_world @ vertex.co for vertex in headrest.data.vertices]
    original_rear_positions = {tuple(vertex.co) for vertex, position in zip(headrest.data.vertices, original_positions) if position.y < -0.12}
    opening_middle = (min(position.y for position in contour) + max(position.y for position in contour)) * 0.5
    extension_length = opening_middle - max(position.y for position in original_positions)
    cut_center = headrest.get('front_cut_center_longitudinal_meters', -0.175) + extension_length
    cut_slope = headrest.get('front_cut_lateral_slope', 0.38)
    transition_end = 0.185
    terminal_width = 0.002
    if options.taper_front_ends:
        contour_intersections = []
        for first, second in zip(contour, contour[1:] + contour[:1]):
            if abs(second.y - first.y) > 0.00000001:
                fraction = (transition_end - first.y) / (second.y - first.y)
                if 0 <= fraction <= 1:
                    position = first.lerp(second, fraction)
                    if position.x > 0:
                        contour_intersections.append(position)
        outer_position = max(contour_intersections, key=lambda position: position.x)
        cut_slope = (transition_end - 0.04) / (outer_position.x - terminal_width - 0.19)
        cut_center = 0.04 - cut_slope * 0.19
        extension_length = transition_end - max(position.y for position in original_positions)
    with bpy.data.libraries.load(str(options.untrimmed_source_file), link=False) as (available, requested):
        requested.objects = ['GEO_CHASSIS_CockpitHeadrestPadding']
    restored_headrest = requested.objects[0]
    if restored_headrest is None or restored_headrest.get('front_trimmed'):
        raise RuntimeError('The restoration source must contain the untrimmed headrest')
    restored_positions = {tuple(vertex.co) for vertex in restored_headrest.data.vertices}
    if not original_rear_positions.issubset(restored_positions):
        raise RuntimeError('The restoration source does not preserve the existing rear crown')
    headrest.data = restored_headrest.data
    for key in list(headrest.keys()):
        del headrest[key]
    for key in restored_headrest.keys():
        headrest[key] = restored_headrest[key]
    bpy.data.objects.remove(restored_headrest, do_unlink=True)
    for image in bpy.data.images:
        if image.filepath and not image.packed_file:
            image.filepath = bpy.path.abspath(image.filepath)
    bpy.context.preferences.filepaths.save_version = 0
    restored_path = validate_output_path(project_directory, output_directory / 'source_with_restored_front.blend', 'preview').path
    bpy.ops.wm.save_as_mainfile(filepath=str(restored_path))
    geometry = trim_restored_headrest(headrest, conversion, cut_center, cut_slope, -0.12 if options.taper_front_ends else float('-inf'))
    headrest = bpy.data.objects['GEO_CHASSIS_CockpitHeadrestPadding']
    rear_preserved = original_rear_positions.issubset({tuple(vertex.co) for vertex in headrest.data.vertices})
    protected_unchanged = all(mesh_fingerprint(bpy.data.objects[name]) == fingerprint for name, fingerprint in protected.items())
    settings_preserved = studio_settings() == original_settings
    if not rear_preserved or not protected_unchanged or not settings_preserved:
        raise RuntimeError('Protected source geometry or studio settings changed')
    headrest['front_extended_to_cockpit_middle'] = True
    if options.taper_front_ends:
        headrest['front_tapered_to_chassis'] = True
        headrest['front_transition_end_longitudinal_meters'] = transition_end
        headrest['front_terminal_width_meters'] = terminal_width
    headrest['front_extension_length_meters'] = extension_length
    headrest['front_extension_generator'] = 'tools/blender/extend_formula_one_2030_headrest_front.py'
    source_path = validate_output_path(project_directory, output_directory / 'f1_2030.blend', 'preview').path
    bpy.ops.wm.save_as_mainfile(filepath=str(source_path))
    report = {'source_before_sha256': hashlib.sha256(options.source_file.read_bytes()).hexdigest(), 'source_after_sha256': hashlib.sha256(source_path.read_bytes()).hexdigest(), 'restoration_source_sha256': hashlib.sha256(options.untrimmed_source_file.read_bytes()).hexdigest(), 'extension_length_millimeters': extension_length * 1000, 'cockpit_middle_longitudinal_meters': opening_middle, 'rear_crown_vertices_preserved': rear_preserved, 'protected_meshes_unchanged': protected_unchanged, 'studio_settings_preserved': settings_preserved, 'geometry': geometry}
    report.update({'front_tapered_to_chassis': options.taper_front_ends, 'front_transition_end_longitudinal_meters': transition_end if options.taper_front_ends else None, 'front_terminal_width_millimeters': terminal_width * 1000 if options.taper_front_ends else None, 'cut_center_longitudinal_meters': cut_center, 'cut_lateral_slope': cut_slope})
    (output_directory / 'headrest_front_extension_report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report), flush=True)


if __name__ == '__main__':
    main()
