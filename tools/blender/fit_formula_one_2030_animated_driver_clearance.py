import argparse
import hashlib
import json
import math
from pathlib import Path
import sys

import bpy
import bmesh
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools'))
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools/blender'))
from common.output_policy import validate_output_path
from fit_f1_2030_driver_seat import mesh_fingerprint


def is_moving_arm_bone(bone_name):
    return 'Arm' in bone_name or 'Hand' in bone_name or bone_name.startswith(('DriverLeft', 'DriverRight'))


def prepare_swept_arm_surface(samples):
    triangles = [triangle for triangle in samples[0]['triangles'] if sum(is_moving_arm_bone(samples[0]['dominant_bones'][index]) for index in triangle) >= 2]
    source_indices = sorted({index for triangle in triangles for index in triangle})
    remapping = {source_index: index for index, source_index in enumerate(source_indices)}
    positions = []
    swept_triangles = []
    for sample in samples:
        vertex_offset = len(positions)
        positions.extend(Vector(sample['vertices'][index]) for index in source_indices)
        swept_triangles.extend([remapping[index] + vertex_offset for index in triangle] for triangle in triangles)
    print('SWEPT_ARM_TRIANGLES=' + str(len(swept_triangles)), flush=True)
    return BVHTree.FromPolygons(positions, swept_triangles, all_triangles=True)


def footprint_offsets(radius):
    return [(0.0, 0.0)] + [(math.cos(index * math.pi / 4.0) * radius, math.sin(index * math.pi / 4.0) * radius) for index in range(8)]


def directional_extent(surface, position, axis, direction_sign, radius):
    remaining_axes = [index for index in range(3) if index != axis]
    direction = Vector((0, 0, 0))
    direction[axis] = -direction_sign
    extent = None
    for first_offset, second_offset in footprint_offsets(radius):
        origin = position.copy()
        origin[axis] = direction_sign * 1.0
        origin[remaining_axes[0]] += first_offset
        origin[remaining_axes[1]] += second_offset
        hit = surface.ray_cast(origin, direction, 2.0)[0]
        if hit is not None:
            signed_extent = hit[axis] * direction_sign
            extent = signed_extent if extent is None else max(extent, signed_extent)
    return extent


def reshape_headrest(scene_object, conversion, swept_surface, clearance):
    transformation = conversion @ scene_object.matrix_world
    inverse_transformation = transformation.inverted()
    seam_indices = set(scene_object.get('cockpit_upper_seam_vertex_indices', []))
    attachment_indices = set(scene_object.get('cockpit_attachment_vertex_indices', []))
    displacements = []
    for vertex in scene_object.data.vertices:
        original_position = transformation @ vertex.co
        if vertex.index in seam_indices or original_position.y < -0.22 or original_position.z > 0.255:
            continue
        extent = directional_extent(swept_surface, original_position, 2, 1, clearance)
        if extent is None:
            continue
        displacement = max(0.0, extent + clearance - original_position.z)
        if displacement <= 0.000001:
            continue
        if displacement > 0.035:
            raise RuntimeError('Headrest fitting exceeds the local displacement budget')
        adjusted = original_position + Vector((0, 0, displacement))
        vertex.co = inverse_transformation @ adjusted
        displacements.append({'vertex_index': vertex.index, 'millimeters': displacement * 1000, 'attachment_vertex': vertex.index in attachment_indices})
    scene_object.data.update()
    return displacements


def preserve_surface_triangulation(scene_object):
    original_mesh = scene_object.data
    original_mesh.calc_loop_triangles()
    triangle_vertices = [tuple(triangle.vertices) for triangle in original_mesh.loop_triangles]
    triangle_loops = [tuple(triangle.loops) for triangle in original_mesh.loop_triangles]
    replacement_mesh = bpy.data.meshes.new(original_mesh.name + 'PreservedSurface')
    replacement_mesh.from_pydata([vertex.co for vertex in original_mesh.vertices], [], triangle_vertices)
    for material in original_mesh.materials:
        replacement_mesh.materials.append(material)
    for polygon, triangle in zip(replacement_mesh.polygons, original_mesh.loop_triangles):
        original_polygon = original_mesh.polygons[triangle.polygon_index]
        polygon.material_index = original_polygon.material_index
        polygon.use_smooth = original_polygon.use_smooth
    for coordinates in original_mesh.uv_layers:
        replacement_coordinates = replacement_mesh.uv_layers.new(name=coordinates.name)
        replacement_coordinates.active_render = coordinates.active_render
        for polygon, original_loops in zip(replacement_mesh.polygons, triangle_loops):
            for new_loop, original_loop in zip(polygon.loop_indices, original_loops):
                replacement_coordinates.data[new_loop].uv = coordinates.data[original_loop].uv
    sharp_vertex_pairs = {tuple(sorted(edge.vertices)) for edge in original_mesh.edges if edge.use_edge_sharp}
    for edge in replacement_mesh.edges:
        edge.use_edge_sharp = tuple(sorted(edge.vertices)) in sharp_vertex_pairs
    scene_object.data = replacement_mesh


def carve_cockpit_surfaces(samples, conversion, clearance):
    point_groups = {}
    triangle_groups = {}
    for triangle in samples[0]['triangles']:
        moving_names = [samples[0]['dominant_bones'][index] for index in triangle if is_moving_arm_bone(samples[0]['dominant_bones'][index])]
        if len(moving_names) < 2:
            continue
        side = 'Left' if sum('Left' in name for name in moving_names) >= 2 else 'Right'
        segment = 'UpperArm' if sum(name.endswith(side + 'Arm') for name in moving_names) >= 2 else 'ForearmAndHand'
        triangle_groups.setdefault(side + segment, set()).update(triangle)
    for sample in samples:
        for group_name, vertex_indices in triangle_groups.items():
            for vertex_index in vertex_indices:
                position = sample['vertices'][vertex_index]
                key = tuple(round(coordinate / 0.0005) for coordinate in position)
                point_groups.setdefault(group_name, {}).setdefault(key, position)
    clearance_cutters = []
    for name, points in point_groups.items():
        topology = bmesh.new()
        for position in points.values():
            topology.verts.new(position)
        hull = bmesh.ops.convex_hull(topology, input=list(topology.verts), use_existing_faces=False)
        unused = list({element for element in hull['geom_interior'] + hull['geom_unused'] if isinstance(element, bmesh.types.BMVert)})
        if unused:
            bmesh.ops.delete(topology, geom=unused, context='VERTS')
        hull_positions = [vertex.co.copy() for vertex in topology.verts]
        topology.free()
        expanded = bmesh.new()
        for position in hull_positions:
            for first_sign in (-1, 1):
                for second_sign in (-1, 1):
                    for third_sign in (-1, 1):
                        expanded.verts.new(position + Vector((first_sign * clearance, second_sign * clearance, third_sign * clearance)))
        expanded_hull = bmesh.ops.convex_hull(expanded, input=list(expanded.verts), use_existing_faces=False)
        unused = list({element for element in expanded_hull['geom_interior'] + expanded_hull['geom_unused'] if isinstance(element, bmesh.types.BMVert)})
        if unused:
            bmesh.ops.delete(expanded, geom=unused, context='VERTS')
        bmesh.ops.recalc_face_normals(expanded, faces=list(expanded.faces))
        for vertex in expanded.verts:
            vertex.co = conversion.inverted() @ vertex.co
        mesh = bpy.data.meshes.new('AnimatedArmClearanceVolume')
        expanded.to_mesh(mesh)
        expanded.free()
        cutter = bpy.data.objects.new('AnimatedArmClearanceVolume', mesh)
        bpy.context.scene.collection.objects.link(cutter)
        clearance_cutters.append(cutter)
        print('CUTTER=' + name + ' source_points=' + str(len(points)) + ' hull_vertices=' + str(len(mesh.vertices)), flush=True)

    changes = {}
    cutting_collection = bpy.data.collections.new('AnimatedArmClearanceVolumes')
    bpy.context.scene.collection.children.link(cutting_collection)
    for cutter in clearance_cutters:
        for collection in list(cutter.users_collection):
            collection.objects.unlink(cutter)
        cutting_collection.objects.link(cutter)
    for name in ['GEO_CHASSIS_INTERIOR', 'GEO_CHASSIS_SEAT']:
        scene_object = bpy.data.objects[name]
        original_count = len(scene_object.data.vertices)
        preserve_surface_triangulation(scene_object)
        modifier = scene_object.modifiers.new('AnimatedArmClearanceCavity', 'BOOLEAN')
        modifier.operation = 'DIFFERENCE'
        modifier.solver = 'MANIFOLD'
        modifier.operand_type = 'COLLECTION'
        modifier.collection = cutting_collection
        modifier.use_self = True
        bpy.context.view_layer.objects.active = scene_object
        bpy.ops.object.modifier_apply(modifier=modifier.name)
        topology = bmesh.new()
        topology.from_mesh(scene_object.data)
        bmesh.ops.remove_doubles(topology, verts=list(topology.verts), dist=0.0000001)
        bmesh.ops.dissolve_degenerate(topology, dist=0.00000005, edges=list(topology.edges))
        bmesh.ops.recalc_face_normals(topology, faces=list(topology.faces))
        topology.to_mesh(scene_object.data)
        topology.free()
        scene_object.data.update()
        changes[name] = {'original_vertices': original_count, 'candidate_vertices': len(scene_object.data.vertices), 'operation': 'Closed local difference against padded convex swept upper-arm and forearm volumes; no original surface translated'}
    for cutter in clearance_cutters:
        bpy.data.objects.remove(cutter, do_unlink=True)
    bpy.data.collections.remove(cutting_collection)
    return changes


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--baseline-chassis-file', type=Path, required=True)
    parser.add_argument('--baseline-manifest-file', type=Path, required=True)
    parser.add_argument('--driver-surfaces', type=Path, required=True)
    parser.add_argument('--capture-provenance', type=Path, required=True)
    parser.add_argument('--output-directory', type=Path, required=True)
    parser.add_argument('--clearance-millimeters', type=float, default=8.0)
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    if options.clearance_millimeters < 8.0:
        raise RuntimeError('The verified construction allowance must be at least eight millimeters')
    output_directory = validate_output_path(PROJECT_DIRECTORY, options.output_directory, 'preview').path
    output_directory.mkdir(parents=True, exist_ok=True)
    provenance = json.loads(options.capture_provenance.read_text())
    baseline_paths = {
        'game/assets/models/vehicles/f1-2030/source/f1_2030.blend': options.source_file,
        'game/assets/models/vehicles/f1-2030/f1_2030_v10_chassis.glb': options.baseline_chassis_file,
    }
    for relative_path, expected_digest in provenance['sha256'].items():
        current_path = baseline_paths.get(relative_path, PROJECT_DIRECTORY / relative_path)
        if hashlib.sha256(current_path.read_bytes()).hexdigest() != expected_digest:
            raise RuntimeError('Capture provenance mismatch: ' + relative_path)
    bpy.ops.wm.open_mainfile(filepath=str(options.source_file.resolve()), load_ui=False, use_scripts=False)
    bpy.ops.file.make_paths_absolute()
    if bpy.data.objects['GEO_CHASSIS_CockpitHeadrestPadding'].get('animated_driver_clearance_fitted', False):
        raise RuntimeError('Refusing to apply driver clearance fitting twice')
    affected_names = {'GEO_CHASSIS_CockpitHeadrestPadding', 'GEO_CHASSIS_INTERIOR', 'GEO_CHASSIS_SEAT'}
    protected_fingerprints = {scene_object.name: mesh_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.type == 'MESH' and scene_object.name not in affected_names}
    manifest = json.loads(options.baseline_manifest_file.read_text())
    conversion = Matrix.Translation((0, 0, manifest['authored_alignment']['chassis_vertical_offset'])) @ Matrix.Rotation(-math.pi / 2, 4, 'Z')
    samples = json.loads(options.driver_surfaces.read_text())['samples']
    swept_surface = prepare_swept_arm_surface(samples)
    headrest_displacements = reshape_headrest(bpy.data.objects['GEO_CHASSIS_CockpitHeadrestPadding'], conversion, swept_surface, 0.010)
    del swept_surface
    if any(record['attachment_vertex'] for record in headrest_displacements):
        raise RuntimeError('A protected headrest attachment was displaced')
    changes = carve_cockpit_surfaces(samples, conversion, options.clearance_millimeters / 1000)
    for name, digest in protected_fingerprints.items():
        if mesh_fingerprint(bpy.data.objects[name]) != digest:
            raise RuntimeError('Protected mesh changed: ' + name)
    bpy.data.objects['GEO_CHASSIS_CockpitHeadrestPadding']['animated_driver_clearance_fitted'] = True
    report = {
        'source_before_sha256': hashlib.sha256(options.source_file.read_bytes()).hexdigest(),
        'driver_surface_sha256': hashlib.sha256(options.driver_surfaces.read_bytes()).hexdigest(),
        'capture_provenance': provenance,
        'minimum_required_clearance_millimeters': 5,
        'convex_cutter_padding_millimeters': options.clearance_millimeters,
        'source_point_deduplication_cell_millimeters': 0.5,
        'headrest_construction_allowance_millimeters': 10,
        'headrest_moved_vertices': len(headrest_displacements),
        'headrest_maximum_displacement_millimeters': max(record['millimeters'] for record in headrest_displacements),
        'headrest_attachments_preserved': True,
        'rear_crown_preserved': True,
        'protected_meshes_unchanged': True,
        'changes': changes,
    }
    output_source = validate_output_path(PROJECT_DIRECTORY, output_directory / 'f1_2030.blend', 'preview').path
    bpy.ops.wm.save_as_mainfile(filepath=str(output_source))
    report['source_after_sha256'] = hashlib.sha256(output_source.read_bytes()).hexdigest()
    output_report = validate_output_path(PROJECT_DIRECTORY, output_directory / 'animated_driver_clearance_fit_report.json', 'preview').path
    output_report.write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps(report), flush=True)


if __name__ == '__main__':
    main()
