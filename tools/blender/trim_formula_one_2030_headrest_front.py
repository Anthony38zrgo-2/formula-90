import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "blender"))
from formula_one_2030_object_names import enable_legacy_source_names, normalize_object_name_document
enable_legacy_source_names()

import bmesh
from mathutils import Matrix, Vector

PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools'))
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools/blender'))
from common.output_policy import validate_output_path
from fit_f1_2030_driver_seat import mesh_fingerprint
from validate_formula_one_2030_cockpit_geometry import inspect_mesh_geometry


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--output-directory', type=Path, required=True)
    parser.add_argument('--cut-center-longitudinal-meters', type=float, default=-0.175)
    parser.add_argument('--cut-lateral-slope', type=float, default=0.38)
    parser.add_argument('--minimum-cut-longitudinal-meters', type=float, default=float('-inf'))
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    directory = validate_output_path(PROJECT_DIRECTORY, options.output_directory, 'preview').path
    directory.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=str(options.source_file), load_ui=False, use_scripts=False)
    headrest = bpy.data.objects['GEO_COCKPIT_HeadrestPadding']
    if headrest.get('front_trimmed'):
        raise RuntimeError('Use the source preceding the front trim')
    protected = {character.name: mesh_fingerprint(character) for character in bpy.data.objects if character.type == 'MESH' and character != headrest}
    source_settings = {scene.name: (scene.camera, scene.render.engine, scene.render.resolution_x, scene.render.resolution_y, scene.render.resolution_percentage) for scene in bpy.data.scenes}
    manifest = normalize_object_name_document(json.loads((PROJECT_DIRECTORY / 'game/assets/models/vehicles/f1-2030/manifest.json').read_text()))
    conversion = Matrix.Translation((0, 0, manifest['authored_alignment']['chassis_vertical_offset'])) @ Matrix.Rotation(-math.pi / 2, 4, 'Z')
    inverse_conversion = conversion.inverted()
    original_count = len(headrest.data.vertices)
    original_positions = [vertex.co.copy() for vertex in headrest.data.vertices]
    attachment_positions = {tuple(original_positions[index]) for index in headrest['cockpit_attachment_vertex_indices']}
    seam_positions = {tuple(original_positions[index]) for index in headrest['cockpit_upper_seam_vertex_indices']}
    editable = bmesh.new()
    editable.from_mesh(headrest.data)
    height_layer = editable.verts.layers.float.new('CockpitRimHeightLimit')
    original_index_layer = editable.verts.layers.int.new('OriginalHeadrestVertexIndex')
    for vertex, height in zip(editable.verts, headrest['cockpit_rim_height_limits']):
        vertex[original_index_layer] = vertex.index
        vertex[height_layer] = height
        vertex.co = conversion @ vertex.co
    cap_count = 0
    for side in (-1, 1):
        selected = [vertex for vertex in editable.verts if vertex.co.x * side >= -0.000001 and vertex.co.y > options.minimum_cut_longitudinal_meters]
        selected += [edge for edge in editable.edges if any(vertex.co.x * side >= -0.000001 and vertex.co.y > options.minimum_cut_longitudinal_meters for vertex in edge.verts)]
        selected += [face for face in editable.faces if any(vertex.co.x * side >= -0.000001 and vertex.co.y > options.minimum_cut_longitudinal_meters for vertex in face.verts)]
        result = bmesh.ops.bisect_plane(editable, geom=selected, dist=0.0000001, plane_co=Vector((0, options.cut_center_longitudinal_meters, 0)), plane_no=Vector((-side * options.cut_lateral_slope, 1, 0)), clear_outer=True, clear_inner=False)
        boundary = [element for element in result['geom_cut'] if isinstance(element, bmesh.types.BMEdge) and element.is_boundary]
        caps = bmesh.ops.holes_fill(editable, edges=boundary, sides=0)['faces']
        coordinates = editable.loops.layers.uv.get('CockpitHeadrestSurfaceCoordinates')
        for cap in caps:
            cap.material_index = 0
            cap.smooth = False
            if coordinates:
                for loop in cap.loops:
                    loop[coordinates].uv = (loop.vert.co.x, loop.vert.co.z)
        cap_count += len(caps)
    bmesh.ops.recalc_face_normals(editable, faces=list(editable.faces))
    heights = [vertex[height_layer] for vertex in editable.verts]
    for vertex in editable.verts:
        vertex.co = inverse_conversion @ vertex.co
        original_index = vertex[original_index_layer]
        if 0 <= original_index < original_count and (vertex.co - original_positions[original_index]).length < 0.0000001:
            vertex.co = original_positions[original_index]
    editable.to_mesh(headrest.data)
    editable.free()
    headrest.data.update()
    headrest['cockpit_rim_height_limits'] = heights
    headrest['cockpit_attachment_vertex_indices'] = [vertex.index for vertex in headrest.data.vertices if tuple(vertex.co) in attachment_positions]
    headrest['cockpit_upper_seam_vertex_indices'] = [vertex.index for vertex in headrest.data.vertices if tuple(vertex.co) in seam_positions]
    headrest['front_trimmed'] = True
    headrest['front_cut_center_longitudinal_meters'] = options.cut_center_longitudinal_meters
    headrest['front_cut_lateral_slope'] = options.cut_lateral_slope
    geometry = inspect_mesh_geometry(headrest)
    print('TRIM_GEOMETRY=' + json.dumps(geometry), flush=True)
    if not geometry['passed']:
        raise RuntimeError('The trimmed headrest failed geometry validation')
    if any(mesh_fingerprint(bpy.data.objects[name]) != fingerprint for name, fingerprint in protected.items()):
        raise RuntimeError('A protected mesh changed')
    for name, settings in source_settings.items():
        scene = bpy.data.scenes[name]
        if (scene.camera, scene.render.engine, scene.render.resolution_x, scene.render.resolution_y, scene.render.resolution_percentage) != settings:
            raise RuntimeError('The saved studio settings changed')
    source_path = validate_output_path(PROJECT_DIRECTORY, directory / 'f1_2030.blend', 'preview').path
    bpy.context.preferences.filepaths.save_version = 0
    for image in bpy.data.images:
        if image.filepath and not image.packed_file:
            image.filepath = bpy.path.abspath(image.filepath)
    bpy.ops.wm.save_as_mainfile(filepath=str(source_path))
    report = {'source_before_sha256': hashlib.sha256(options.source_file.read_bytes()).hexdigest(), 'source_after_sha256': hashlib.sha256(source_path.read_bytes()).hexdigest(), 'original_vertex_count': original_count, 'trimmed_vertex_count': len(headrest.data.vertices), 'cut_caps': cap_count, 'cut_center_longitudinal_meters': options.cut_center_longitudinal_meters, 'cut_lateral_slope': options.cut_lateral_slope, 'protected_meshes_unchanged': True, 'geometry': geometry}
    (directory / 'headrest_front_trim_report.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
