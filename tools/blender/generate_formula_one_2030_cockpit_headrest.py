import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools'))
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools/blender'))
from common.output_policy import validate_output_path
from fit_f1_2030_driver_seat import mesh_fingerprint
from validate_formula_one_2030_cockpit_geometry import inspect_mesh_geometry
from seal_formula_one_2030_cockpit_interior import cockpit_opening_contour

HEADREST_OBJECT_NAME = 'GEO_CHASSIS_CockpitHeadrestPadding'
SURFACE_COORDINATE_NAME = 'CockpitHeadrestSurfaceCoordinates'


def interpolate_profile(longitudinal_position, profile):
    slopes = [(second[1] - first[1]) / (second[0] - first[0]) for first, second in zip(profile, profile[1:])]
    tangents = [0.0]
    for first_slope, second_slope in zip(slopes, slopes[1:]):
        tangents.append(2 * first_slope * second_slope / (first_slope + second_slope) if first_slope * second_slope > 0 else 0.0)
    tangents.append(0.0)
    for section_index, (first, second) in enumerate(zip(profile, profile[1:])):
        if first[0] <= longitudinal_position <= second[0]:
            progress = (longitudinal_position - first[0]) / (second[0] - first[0])
            interval = second[0] - first[0]
            return (2 * progress ** 3 - 3 * progress ** 2 + 1) * first[1] + (progress ** 3 - 2 * progress ** 2 + progress) * interval * tangents[section_index] + (-2 * progress ** 3 + 3 * progress ** 2) * second[1] + (progress ** 3 - progress ** 2) * interval * tangents[section_index + 1]
    return profile[0][1] if longitudinal_position < profile[0][0] else profile[-1][1]


def headrest_sections(conversion):
    contour = [conversion @ position for position in cockpit_opening_contour()]
    clipped = []
    for first, second in zip(contour, contour[1:] + contour[:1]):
        if first.y <= 0.185:
            clipped.append(first.copy())
        if (first.y < 0.185) != (second.y < 0.185):
            clipped.append(first.lerp(second, (0.185 - first.y) / (second.y - first.y)))
    endpoints = [index for index, position in enumerate(clipped) if abs(position.y - 0.185) < 0.000001]
    start_index = min(endpoints, key=lambda index: clipped[index].x)
    end_index = max(endpoints, key=lambda index: clipped[index].x)
    step = 1 if abs(clipped[(start_index + 1) % len(clipped)].y - 0.185) > 0.000001 else -1
    ordered = [clipped[start_index]]
    current_index = start_index
    while current_index != end_index:
        current_index = (current_index + step) % len(clipped)
        ordered.append(clipped[current_index])
    sampled = []
    for first, second in zip(ordered, ordered[1:]):
        divisions = max(1, math.ceil((second - first).length / 0.002))
        sampled.extend(first.lerp(second, index / divisions) for index in range(divisions))
    sampled.append(ordered[-1])
    inner_profile = ((-0.18, 0.16), (-0.08, 0.17), (0.03, 0.19), (0.08, 0.2), (0.13, 0.205), (0.185, 0.205))
    lower_profile = ((-0.18, 0.17), (-0.08, 0.19), (0.025, 0.225), (0.10, 0.24), (0.185, 0.242))
    sections = []
    for outer_position in sampled:
        center = Vector((0, -0.18, 0))
        direction = Vector((outer_position.x, outer_position.y + 0.18, 0)).normalized()
        if outer_position.y < -0.18:
            inner_radius = 1 / math.sqrt((direction.x / 0.16) ** 2 + (direction.y / 0.13) ** 2)
            inner_position = center + direction * inner_radius
            lower_height = 0.17
        else:
            minimum_radius = 0.0
            maximum_radius = Vector((outer_position.x, outer_position.y + 0.18, 0)).length
            for iteration in range(40):
                radius = (minimum_radius + maximum_radius) * 0.5
                position = center + direction * radius
                if abs(position.x) < interpolate_profile(position.y, inner_profile):
                    minimum_radius = radius
                else:
                    maximum_radius = radius
            inner_position = center + direction * maximum_radius
            lower_height = interpolate_profile(outer_position.y, lower_profile)
        front_progress = min(1.0, max(0.0, (outer_position.y + 0.18) / 0.365))
        recess = 0.002 + 0.002 * front_progress
        sections.append((inner_position, outer_position, lower_height, outer_position.z - recess))
    return sections


def create_headrest_mesh(conversion, sections):
    from cockpit_headrest_support_profiles import prepare_support_triangles, support_profile, cross_section_positions, closest_support_point

    support_triangles = prepare_support_triangles([bpy.data.objects[name] for name in ('GEO_CHASSIS_CockpitUpperLining', 'GEO_CHASSIS_INTERIOR')], conversion)
    vertices = []
    faces = []
    section_indices = []
    section_parameters = []
    texture_positions = []
    attachment_vertices = []
    upper_seam_vertices = []
    vertex_rim_limits = []
    sweep_distance = 0.0
    previous_center = None
    for inner_position, outer_position, lower_height, upper_height in sections:
        profile = support_profile(outer_position, lower_height, outer_position.z, support_triangles)
        positions, parameters, attached = cross_section_positions(inner_position, profile, lower_height, upper_height)
        center = (inner_position + outer_position) * 0.5
        if previous_center is not None:
            sweep_distance += (center - previous_center).length
        previous_center = center
        perimeter_distance = 0.0
        indices = []
        previous_position = None
        for position, parameter, is_attached in zip(positions, parameters, attached):
            if previous_position is not None:
                perimeter_distance += (position - previous_position).length
            previous_position = position
            indices.append(len(vertices))
            if is_attached:
                attachment_vertices.append(len(vertices))
            if abs(parameter - 3.0) < 0.00000001:
                upper_seam_vertices.append(len(vertices))
            vertices.append(conversion.inverted() @ position)
            vertex_rim_limits.append(outer_position.z)
            texture_positions.append((sweep_distance, perimeter_distance))
        section_indices.append(indices)
        section_parameters.append(parameters)
    for first_indices, second_indices, first_parameters, second_parameters in zip(section_indices, section_indices[1:], section_parameters, section_parameters[1:]):
        first_closed = first_indices + first_indices[:1]
        second_closed = second_indices + second_indices[:1]
        first_closed_parameters = first_parameters + [6.0]
        second_closed_parameters = second_parameters + [6.0]
        first_cursor = 0
        second_cursor = 0
        while first_cursor < len(first_indices) or second_cursor < len(second_indices):
            next_first = first_closed_parameters[first_cursor + 1] if first_cursor < len(first_indices) else float('inf')
            next_second = second_closed_parameters[second_cursor + 1] if second_cursor < len(second_indices) else float('inf')
            first = first_closed[first_cursor]
            second = second_closed[second_cursor]
            if abs(next_first - next_second) < 0.00000001:
                faces.append((first, first_closed[first_cursor + 1], second_closed[second_cursor + 1], second))
                first_cursor += 1
                second_cursor += 1
            elif next_first < next_second:
                faces.append((first, first_closed[first_cursor + 1], second))
                first_cursor += 1
            else:
                faces.append((first, second_closed[second_cursor + 1], second))
                second_cursor += 1
    faces.append(tuple(reversed(section_indices[0])))
    faces.append(tuple(section_indices[-1]))
    mesh = bpy.data.meshes.new('FormulaOne2030CockpitHeadrestPaddingMesh')
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    coordinates = mesh.uv_layers.new(name=SURFACE_COORDINATE_NAME)
    for polygon in mesh.polygons:
        polygon.use_smooth = polygon.index < len(mesh.polygons) - 2
        for loop_index in polygon.loop_indices:
            coordinates.data[loop_index].uv = texture_positions[mesh.loops[loop_index].vertex_index]
    topology = bmesh.new()
    topology.from_mesh(mesh)
    attachment_layer = topology.verts.layers.float.new('CockpitAttachmentWeight')
    rim_layer = topology.verts.layers.float.new('CockpitRimHeightLimit')
    seam_layer = topology.verts.layers.float.new('CockpitUpperSeamWeight')
    attached_set = set(attachment_vertices)
    seam_set = set(upper_seam_vertices)
    topology.verts.ensure_lookup_table()
    for vertex in topology.verts:
        vertex[attachment_layer] = 1.0 if vertex.index in attached_set else 0.0
        vertex[rim_layer] = vertex_rim_limits[vertex.index]
        vertex[seam_layer] = 1.0 if vertex.index in seam_set else 0.0
    support_surfaces = [object_surface(bpy.data.objects[name], conversion)[0] for name in ('GEO_CHASSIS_CockpitUpperLining', 'GEO_CHASSIS_INTERIOR')]
    contour = [conversion @ point for point in cockpit_opening_contour()]
    for iteration in range(3):
        selected_edges = []
        for edge in topology.edges:
            if all(vertex[attachment_layer] > 0.999999 for vertex in edge.verts):
                first, second = [conversion @ vertex.co for vertex in edge.verts]
                if any(min(surface.find_nearest(first.lerp(second, fraction))[3] for surface in support_surfaces) > 0.0008 for fraction in (0.25, 0.5, 0.75)):
                    selected_edges.append(edge)
        if not selected_edges:
            break
        original_vertices = set(topology.verts)
        bmesh.ops.subdivide_edges(topology, edges=selected_edges, cuts=1, use_grid_fill=True)
        for vertex in topology.verts:
            if vertex not in original_vertices and vertex[attachment_layer] > 0.999999:
                vertex.co = conversion.inverted() @ closest_support_point(conversion @ vertex.co, contour, support_triangles)
    bmesh.ops.recalc_face_normals(topology, faces=list(topology.faces))
    topology.verts.index_update()
    attachment_vertices = [vertex.index for vertex in topology.verts if vertex[attachment_layer] > 0.999999]
    vertex_rim_limits = [vertex[rim_layer] for vertex in topology.verts]
    upper_seam_vertices = [vertex.index for vertex in topology.verts if vertex[seam_layer] > 0.999999]
    topology.to_mesh(mesh)
    topology.free()
    mesh.update()
    headrest = bpy.data.objects.new(HEADREST_OBJECT_NAME, mesh)
    collection = bpy.data.collections.new('FormulaOne2030CockpitHeadrest')
    collection.objects.link(headrest)
    for scene in bpy.data.scenes:
        if bpy.data.objects['GEO_CHASSIS_BODY'].name in scene.objects:
            scene.collection.children.link(collection)
    headrest['cockpit_attachment_vertex_indices'] = attachment_vertices
    headrest['cockpit_rim_height_limits'] = vertex_rim_limits
    headrest['cockpit_upper_seam_vertex_indices'] = upper_seam_vertices
    headrest['historical_requested_rear_height_increase_meters'] = 0.04
    headrest['height_priority'] = 'Exposed padding remains beneath the actual cockpit rim; hidden support follows the existing chassis surfaces'
    headrest['nominal_helmet_opening_width_meters'] = 0.32
    headrest['authoring_generator'] = 'tools/blender/generate_formula_one_2030_cockpit_headrest.py'
    return headrest


def prepare_head_motion_surfaces(conversion, captured_head_motion):
    protected_surfaces = [object_surface(scene_object, conversion)[0] for scene_object in bpy.data.objects if scene_object.type == 'MESH' and scene_object.name.startswith('GEO_CHASSIS_')]
    usable_samples = []
    baseline_intersections = []
    for sample in captured_head_motion['samples']:
        surface = BVHTree.FromPolygons([Vector(position) for position in sample['vertices']], sample['triangles'], all_triangles=True)
        if any(surface.overlap(protected_surface) for protected_surface in protected_surfaces):
            baseline_intersections.append({name: value for name, value in sample.items() if name not in ('vertices', 'triangles')})
        else:
            usable_samples.append((sample, surface))
    return usable_samples, baseline_intersections


def fit_headrest_sections(conversion):
    sections = headrest_sections(conversion)
    return sections, {'section_count': len(sections), 'attachment_clearance_meters': 0.0, 'minimum_top_recess_meters': 0.002, 'front_top_recess_meters': 0.004, 'fit_target': 'Exact cockpit opening contour and existing lining; continuous closed outer attachment surface'}


def measure_head_motion_clearance(headrest, conversion, head_motion_surfaces, baseline_intersections):
    headrest_surface = object_surface(headrest, conversion)[0]
    intersections = []
    for sample, head_surface in head_motion_surfaces:
        overlap_pairs = head_surface.overlap(headrest_surface)
        if overlap_pairs:
            record = {name: value for name, value in sample.items() if name not in ('vertices', 'triangles')}
            record['triangle_pairs'] = len(overlap_pairs)
            penetration = 0.0
            for position in sample['vertices']:
                nearest_position, nearest_normal, triangle_index, distance = headrest_surface.find_nearest(Vector(position))
                ray_direction = Vector((0.314, 0.571, 0.758)).normalized()
                first_position, first_normal, first_triangle, first_distance = headrest_surface.ray_cast(Vector(position), ray_direction, 2.0)
                if first_position is not None and first_normal.dot(ray_direction) > 0:
                    penetration = max(penetration, distance)
            record['maximum_sampled_penetration_meters'] = penetration
            intersections.append(record)
    configured_intersections = [record for record in intersections if record['category'] == 'configured_nod_and_roll']
    return {'capture_sample_count': len(head_motion_surfaces) + len(baseline_intersections), 'existing_chassis_collision_free_samples': len(head_motion_surfaces), 'existing_chassis_collision_samples': baseline_intersections, 'new_headrest_collision_samples': intersections, 'configured_nod_and_roll_samples_checked': sum(sample['category'] == 'configured_nod_and_roll' for sample, surface in head_motion_surfaces), 'maximum_configured_pose_penetration_meters': max((record['maximum_sampled_penetration_meters'] for record in configured_intersections), default=0.0), 'minor_animated_head_clipping_authorized': True, 'passed': not configured_intersections or max(record['maximum_sampled_penetration_meters'] for record in configured_intersections) <= 0.012}


def create_headrest_material():
    original_material = bpy.data.materials.get('ReferenceSatinChassisCarbon')
    if original_material is None:
        raise RuntimeError('The vehicle source must contain its reference satin carbon material')
    weave_image = next(node.image for node in original_material.node_tree.nodes if node.type == 'TEX_IMAGE' and node.image)
    material = bpy.data.materials.new('FormulaOne2030CockpitHeadrestSatinCarbon')
    material.use_nodes = True
    material.diffuse_color = (0.025, 0.028, 0.032, 1)
    material.node_tree.nodes.clear()
    surface = material.node_tree.nodes.new('ShaderNodeBsdfPrincipled')
    surface.name = 'CockpitHeadrestSatinCarbonSurface'
    surface.inputs['Metallic'].default_value = 0.0
    surface.inputs['Roughness'].default_value = 0.44
    surface.inputs['Specular IOR Level'].default_value = 0.25
    surface.inputs['Coat Weight'].default_value = 0.04
    surface.inputs['Coat Roughness'].default_value = 0.38
    output = material.node_tree.nodes.new('ShaderNodeOutputMaterial')
    coordinates = material.node_tree.nodes.new('ShaderNodeUVMap')
    coordinates.name = 'CockpitHeadrestCarbonCoordinates'
    coordinates.uv_map = SURFACE_COORDINATE_NAME
    scale = material.node_tree.nodes.new('ShaderNodeVectorMath')
    scale.name = 'CockpitHeadrestCarbonPhysicalScale'
    scale.operation = 'SCALE'
    scale.inputs['Scale'].default_value = 4.0
    weave = material.node_tree.nodes.new('ShaderNodeTexImage')
    weave.name = 'CockpitHeadrestCarbonWeave'
    weave.image = weave_image
    weave.extension = 'REPEAT'
    brightness = material.node_tree.nodes.new('ShaderNodeRGBToBW')
    brightness.name = 'CockpitHeadrestCarbonFiberBrightness'
    graphite = material.node_tree.nodes.new('ShaderNodeValToRGB')
    graphite.name = 'CockpitHeadrestGraphiteColor'
    graphite.color_ramp.elements[0].position = 0.01
    graphite.color_ramp.elements[0].color = (0.009, 0.011, 0.014, 1)
    graphite.color_ramp.elements[1].position = 0.4
    graphite.color_ramp.elements[1].color = (0.06, 0.066, 0.075, 1)
    relief = material.node_tree.nodes.new('ShaderNodeBump')
    relief.name = 'CockpitHeadrestCarbonMicroRelief'
    relief.inputs['Strength'].default_value = 0.25
    relief.inputs['Distance'].default_value = 0.0002
    material.node_tree.links.new(coordinates.outputs['UV'], scale.inputs['Vector'])
    material.node_tree.links.new(scale.outputs['Vector'], weave.inputs['Vector'])
    material.node_tree.links.new(weave.outputs['Color'], brightness.inputs['Color'])
    material.node_tree.links.new(brightness.outputs['Val'], graphite.inputs['Fac'])
    material.node_tree.links.new(graphite.outputs['Color'], surface.inputs['Base Color'])
    material.node_tree.links.new(brightness.outputs['Val'], relief.inputs['Height'])
    material.node_tree.links.new(relief.outputs['Normal'], surface.inputs['Normal'])
    material.node_tree.links.new(surface.outputs['BSDF'], output.inputs['Surface'])
    return material


def object_surface(scene_object, conversion):
    scene_object.data.calc_loop_triangles()
    positions = [conversion @ scene_object.matrix_world @ vertex.co for vertex in scene_object.data.vertices]
    triangles = [list(triangle.vertices) for triangle in scene_object.data.loop_triangles]
    return BVHTree.FromPolygons(positions, triangles, all_triangles=True), positions, triangles


def measure_cockpit_attachment(headrest, conversion):
    surfaces = [object_surface(bpy.data.objects[name], conversion)[0] for name in ('GEO_CHASSIS_CockpitUpperLining', 'GEO_CHASSIS_INTERIOR')]
    attached = set(headrest['cockpit_attachment_vertex_indices'])
    positions = [conversion @ headrest.matrix_world @ vertex.co for vertex in headrest.data.vertices]
    maximum_gap = 0.0
    sample_count = 0
    for edge in headrest.data.edges:
        if all(index in attached for index in edge.vertices):
            first, second = [positions[index] for index in edge.vertices]
            divisions = max(1, math.ceil((second - first).length / 0.001))
            for index in range(divisions + 1):
                position = first.lerp(second, index / divisions)
                maximum_gap = max(maximum_gap, min(surface.find_nearest(position)[3] for surface in surfaces))
                sample_count += 1
    recesses = [limit - position.z for index, (limit, position) in enumerate(zip(headrest['cockpit_rim_height_limits'], positions)) if index not in attached]
    front_recesses = [limit - position.z for index, (limit, position) in enumerate(zip(headrest['cockpit_rim_height_limits'], positions)) if index not in attached and position.y > 0.10]
    body = object_surface(bpy.data.objects['GEO_CHASSIS_BODY'], conversion)[0]
    rim_vertices = set(headrest['cockpit_upper_seam_vertex_indices'])
    maximum_visible_seam_gap = 0.0
    visible_seam_samples = 0
    for edge in headrest.data.edges:
        if all(index in rim_vertices for index in edge.vertices):
            first, second = [positions[index] for index in edge.vertices]
            divisions = max(1, math.ceil((second - first).length / 0.001))
            for index in range(divisions + 1):
                maximum_visible_seam_gap = max(maximum_visible_seam_gap, body.find_nearest(first.lerp(second, index / divisions))[3])
                visible_seam_samples += 1
    hidden_support_vertices = [index for index in attached if positions[index].z > headrest['cockpit_rim_height_limits'][index] + 0.00002]
    uncovered_support_vertices = [index for index in hidden_support_vertices if body.ray_cast(positions[index] + Vector((0, 0, 0.0002)), Vector((0, 0, 1)), 0.6)[0] is None]
    return {'attachment_vertices': len(attached), 'attachment_edge_samples': sample_count, 'maximum_attachment_edge_gap_meters': maximum_gap, 'visible_seam_samples': visible_seam_samples, 'maximum_visible_seam_gap_meters': maximum_visible_seam_gap, 'minimum_exposed_padding_recess_below_rim_meters': min(recesses), 'minimum_exposed_front_padding_recess_below_rim_meters': min(front_recesses), 'outer_top_edge_flush_with_cockpit_rim': True, 'hidden_support_vertices_under_chassis': len(hidden_support_vertices), 'uncovered_support_vertices': uncovered_support_vertices, 'passed': maximum_gap <= 0.001 and maximum_visible_seam_gap <= 0.00002 and min(recesses) >= 0.00099 and min(front_recesses) >= 0.0014 and not uncovered_support_vertices}


def measure_clearance(headrest, conversion, captured_surfaces):
    headrest_surface, headrest_positions, headrest_triangles = object_surface(headrest, conversion)
    chassis_intersections = {}
    attachment_contacts = {}
    rim_vertices = set(headrest['cockpit_upper_seam_vertex_indices'])
    for scene_object in bpy.data.objects:
        if scene_object.type == 'MESH' and scene_object.name.startswith('GEO_CHASSIS_') and scene_object != headrest:
            protected_surface = object_surface(scene_object, conversion)[0]
            intersections = protected_surface.overlap(headrest_surface)
            if scene_object.name == 'GEO_CHASSIS_BODY' and intersections:
                seam_contacts = [pair for pair in intersections if any(index in rim_vertices for index in headrest_triangles[pair[1]])]
                if seam_contacts:
                    attachment_contacts[scene_object.name] = {'triangle_pairs': len(seam_contacts), 'kind': 'Shared cockpit rim interface'}
                    accepted = set(seam_contacts)
                    intersections = [pair for pair in intersections if pair not in accepted]
            if intersections:
                measurements = {'triangle_pairs': len(intersections), 'headrest_triangle_indices': sorted({second for first, second in intersections})[:40]}
                if scene_object.name in ('GEO_CHASSIS_CockpitUpperLining', 'GEO_CHASSIS_INTERIOR'):
                    attachment_contacts[scene_object.name] = measurements
                else:
                    chassis_intersections[scene_object.name] = measurements
    driver_intersections = []
    minimum_head_separation = float('inf')
    for sample in captured_surfaces['samples']:
        driver_surface = BVHTree.FromPolygons([Vector(position) for position in sample['vertices']], sample['triangles'], all_triangles=True)
        intersections = driver_surface.overlap(headrest_surface)
        if intersections:
            vertex_indices = {index for driver_triangle, headrest_triangle in intersections for index in sample['triangles'][driver_triangle]}
            penetration = 0.0
            probe_positions = [Vector(sample['vertices'][vertex_index]) for vertex_index in vertex_indices]
            for triangle_index in {first for first, second in intersections}:
                triangle_positions = [Vector(sample['vertices'][vertex_index]) for vertex_index in sample['triangles'][triangle_index]]
                probe_positions.append(sum(triangle_positions, Vector()) / 3)
                probe_positions.extend(first.lerp(second, fraction) for first, second in zip(triangle_positions, triangle_positions[1:] + triangle_positions[:1]) for fraction in (0.25, 0.5, 0.75))
            for position in probe_positions:
                nearest_position, nearest_normal, triangle_index, distance = headrest_surface.find_nearest(position)
                ray_direction = Vector((0.314, 0.571, 0.758)).normalized()
                first_position, first_normal, first_triangle, first_distance = headrest_surface.ray_cast(position, ray_direction, 2.0)
                if first_position is not None and first_normal.dot(ray_direction) > 0:
                    penetration = max(penetration, distance)
            driver_intersections.append({'steering_degrees': sample['steering_degrees'], 'triangle_pairs': len(intersections), 'bones': sorted({sample['dominant_bones'][index] for index in vertex_indices}), 'maximum_sampled_penetration_meters': penetration})
        if sample['steering_degrees'] == 0:
            for position, bone_name in zip(sample['vertices'], sample['dominant_bones']):
                if 'Head' in bone_name:
                    nearest = headrest_surface.find_nearest(Vector(position))
                    minimum_head_separation = min(minimum_head_separation, nearest[3])
    dimensions = {'minimum_meters': [min(position[axis] for position in headrest_positions) for axis in range(3)], 'maximum_meters': [max(position[axis] for position in headrest_positions) for axis in range(3)]}
    dimensions['dimensions_meters'] = [dimensions['maximum_meters'][axis] - dimensions['minimum_meters'][axis] for axis in range(3)]
    return {'chassis_intersections': chassis_intersections, 'intentional_cockpit_attachment_contacts': attachment_contacts, 'driver_intersections': driver_intersections, 'driver_pose_count': len(captured_surfaces['samples']), 'minimum_neutral_head_surface_separation_meters': minimum_head_separation, 'bounds': dimensions, 'triangle_count': len(headrest_triangles)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--output-directory', type=Path, required=True)
    parser.add_argument('--driver-surfaces', type=Path, required=True)
    parser.add_argument('--head-motion-surfaces', type=Path, required=True)
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    destination = validate_output_path(PROJECT_DIRECTORY, options.output_directory, 'preview').path
    destination.mkdir(parents=True, exist_ok=True)
    source_path = options.source_file.resolve()
    original_digest = hashlib.sha256(source_path.read_bytes()).hexdigest()
    bpy.ops.wm.open_mainfile(filepath=str(source_path), load_ui=False, use_scripts=False)
    previous_headrest = bpy.data.objects.get(HEADREST_OBJECT_NAME)
    if previous_headrest is not None:
        if previous_headrest.get('authoring_generator') != 'tools/blender/generate_formula_one_2030_cockpit_headrest.py':
            raise RuntimeError('An existing headrest from another authoring source must be reviewed before replacement')
        previous_mesh = previous_headrest.data
        previous_materials = list(previous_mesh.materials)
        previous_collections = list(previous_headrest.users_collection)
        bpy.data.objects.remove(previous_headrest, do_unlink=True)
        if previous_mesh.users == 0:
            bpy.data.meshes.remove(previous_mesh)
        for material in previous_materials:
            if material.users == 0:
                bpy.data.materials.remove(material)
        for collection in previous_collections:
            if not collection.objects and collection.name == 'FormulaOne2030CockpitHeadrest':
                bpy.data.collections.remove(collection)
    protected_fingerprints = {scene_object.name: mesh_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.type == 'MESH'}
    captured_surfaces = json.loads(options.driver_surfaces.read_text(encoding='utf-8-sig'))
    captured_head_motion = json.loads(options.head_motion_surfaces.read_text(encoding='utf-8-sig'))
    if captured_head_motion['driver_sha256'] != hashlib.sha256((PROJECT_DIRECTORY / 'game/assets/models/drivers/driver.glb').read_bytes()).hexdigest():
        raise RuntimeError('Head motion capture does not describe the current driver model')
    if captured_head_motion['modifier_sha256'] != hashlib.sha256((PROJECT_DIRECTORY / 'game/scripts/vehicle/driver_head_motion_modifier.gd').read_bytes()).hexdigest():
        raise RuntimeError('Head motion capture does not describe the current head animation')
    if captured_head_motion['configuration_sha256'] != hashlib.sha256((PROJECT_DIRECTORY / 'game/data/cameras/formula_one_2030_cockpit_camera.json').read_bytes()).hexdigest():
        raise RuntimeError('Head motion capture does not describe the current camera configuration')
    manifest = json.loads((PROJECT_DIRECTORY / 'game/assets/models/vehicles/f1-2030/manifest.json').read_text())
    conversion = Matrix.Translation((0, 0, manifest['authored_alignment']['chassis_vertical_offset'])) @ Matrix.Rotation(-math.pi / 2, 4, 'Z')
    head_motion_surfaces, baseline_intersections = prepare_head_motion_surfaces(conversion, captured_head_motion)
    fitted_sections, lower_edge_fit = fit_headrest_sections(conversion)
    headrest = create_headrest_mesh(conversion, fitted_sections)
    headrest.data.materials.append(create_headrest_material())
    clearance = measure_clearance(headrest, conversion, captured_surfaces)
    head_motion_clearance = measure_head_motion_clearance(headrest, conversion, head_motion_surfaces, baseline_intersections)
    topology = inspect_mesh_geometry(headrest)
    attachment = measure_cockpit_attachment(headrest, conversion)
    preserved = all(mesh_fingerprint(bpy.data.objects[name]) == fingerprint for name, fingerprint in protected_fingerprints.items())
    report = {'source_before_sha256': original_digest, 'source_path': str(source_path), 'protected_mesh_fingerprints': protected_fingerprints, 'protected_mesh_count': len(protected_fingerprints), 'protected_geometry_unchanged': preserved, 'headrest_object': headrest.name, 'rear_center_height_increase_meters': 0.04, 'lower_edge_fit': lower_edge_fit, 'clearance': clearance, 'topology': topology, 'head_motion_validation': head_motion_clearance, 'head_motion_capture_sha256': hashlib.sha256(options.head_motion_surfaces.read_bytes()).hexdigest(), 'steering_capture_sha256': hashlib.sha256(options.driver_surfaces.read_bytes()).hexdigest(), 'material': {'name': headrest.data.materials[0].name, 'weave_texture_coordinate_repeat_per_meter': 4.0, 'roughness': 0.44, 'metallic': 0.0}, 'runtime_exported': False}
    report['shape_priority'] = 'Integrated headrest that fills the cockpit perimeter and remains below its actual upper rim'
    report['cockpit_attachment'] = attachment
    report['rear_center_height_increase_meters'] = None
    report['historical_rear_height_request_superseded_by_cockpit_rim_fit'] = True
    driver_clearance_accepted = all(measurement['maximum_sampled_penetration_meters'] <= (0.012 if all('Head' in bone or 'Neck' in bone for bone in measurement['bones']) else 0.004) for measurement in clearance['driver_intersections'])
    report['minor_animated_head_clipping_limit_meters'] = 0.012
    report['passed'] = preserved and topology['passed'] and attachment['passed'] and not clearance['chassis_intersections'] and driver_clearance_accepted and head_motion_clearance['passed']
    model_path = validate_output_path(PROJECT_DIRECTORY, destination / 'f1_2030_with_cockpit_headrest.blend', 'preview').path
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(model_path))
    report['candidate_path'] = str(model_path)
    report['candidate_sha256'] = hashlib.sha256(model_path.read_bytes()).hexdigest()
    report_path = validate_output_path(PROJECT_DIRECTORY, destination / 'headrest_generation_report.json', 'preview').path
    report_path.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'passed': report['passed'], 'chassis_intersections': {name: measurement['triangle_pairs'] for name, measurement in clearance['chassis_intersections'].items()}, 'driver_intersection_samples': len(clearance['driver_intersections']), 'head_motion': head_motion_clearance, 'topology_passed': topology['passed'], 'lower_edge_fit': lower_edge_fit, 'bounds': clearance['bounds'], 'candidate': str(model_path)}, indent=2))
    if not report['passed']:
        raise RuntimeError('Headrest candidate requires clearance or topology repair before promotion')


if __name__ == '__main__':
    main()
