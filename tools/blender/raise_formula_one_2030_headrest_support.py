import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Matrix
from mathutils import Vector
from mathutils.bvhtree import BVHTree

PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools'))
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools/blender'))
from common.output_policy import validate_output_path
from fit_f1_2030_driver_seat import mesh_fingerprint
from generate_formula_one_2030_cockpit_headrest import measure_clearance, measure_cockpit_attachment, measure_head_motion_clearance, object_surface
from validate_formula_one_2030_cockpit_geometry import inspect_mesh_geometry


def smooth_transition(fraction):
    fraction = min(1.0, max(0.0, fraction))
    return fraction * fraction * (3.0 - 2.0 * fraction)


def support_elevation(lateral_position, original_height, base_height, radius):
    lateral_distance = abs(lateral_position)
    transition_start = radius - 0.005
    transition_end = radius + 0.015
    if lateral_distance >= transition_end:
        return 0.0
    if lateral_distance <= transition_start:
        return max(0.0, base_height + math.sqrt(radius * radius - lateral_distance * lateral_distance) - original_height)
    interval = transition_end - transition_start
    fraction = (lateral_distance - transition_start) / interval
    circular_height = math.sqrt(radius * radius - transition_start * transition_start)
    circular_slope = -transition_start / circular_height
    circular_height = (2 * fraction ** 3 - 3 * fraction ** 2 + 1) * circular_height + (fraction ** 3 - 2 * fraction ** 2 + fraction) * interval * circular_slope
    return max(0.0, circular_height + (base_height - original_height) * (1.0 - smooth_transition(fraction)))


def studio_settings():
    return {
        'scenes': {scene.name: [scene.camera.name if scene.camera else None, scene.render.engine, scene.render.resolution_x, scene.render.resolution_y, scene.render.resolution_percentage, scene.cycles.samples, scene.world.name if scene.world else None, scene.view_settings.exposure, scene.view_settings.view_transform, scene.render.filepath] for scene in bpy.data.scenes},
        'texts': {text.name: text.as_string() for text in bpy.data.texts},
        'lights': {light.name: [light.type, light.energy, list(light.color)] for light in bpy.data.lights},
        'cameras': {camera.name: [camera.type, camera.lens, camera.ortho_scale, camera.clip_start, camera.clip_end] for camera in bpy.data.cameras},
    }


def original_chassis_head_motion_surfaces(conversion, captured_head_motion, headrest):
    protected_surfaces = [object_surface(scene_object, conversion)[0] for scene_object in bpy.data.objects if scene_object.type == 'MESH' and scene_object.name.startswith('GEO_CHASSIS_') and scene_object != headrest]
    usable_samples = []
    baseline_intersections = []
    for sample in captured_head_motion['samples']:
        surface = BVHTree.FromPolygons([Vector(position) for position in sample['vertices']], sample['triangles'], all_triangles=True)
        if any(surface.overlap(protected_surface) for protected_surface in protected_surfaces):
            baseline_intersections.append({name: value for name, value in sample.items() if name not in ('vertices', 'triangles')})
        else:
            usable_samples.append((sample, surface))
    return usable_samples, baseline_intersections


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--output-directory', type=Path, required=True)
    parser.add_argument('--driver-surfaces', type=Path, required=True)
    parser.add_argument('--head-motion-surfaces', type=Path, required=True)
    parser.add_argument('--radius-millimeters', type=float, default=75.0)
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:])
    output_directory = validate_output_path(PROJECT_DIRECTORY, options.output_directory, 'preview').path
    output_directory.mkdir(parents=True, exist_ok=True)
    source_digest = hashlib.sha256(options.source_file.read_bytes()).hexdigest()
    bpy.ops.wm.open_mainfile(filepath=str(options.source_file), load_ui=False, use_scripts=False)
    headrest = bpy.data.objects['GEO_CHASSIS_CockpitHeadrestPadding']
    if 'raised_rear_support_radius_meters' in headrest:
        raise RuntimeError('Use the source before the upper support was raised')
    protected_fingerprints = {scene_object.name: mesh_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.type == 'MESH' and scene_object != headrest}
    original_studio = studio_settings()
    original_objects = sorted(bpy.data.objects.keys())
    manifest = json.loads((PROJECT_DIRECTORY / 'game/assets/models/vehicles/f1-2030/manifest.json').read_text())
    conversion = Matrix.Translation((0, 0, manifest['authored_alignment']['chassis_vertical_offset'])) @ Matrix.Rotation(-math.pi / 2, 4, 'Z')
    inverse_conversion = conversion.inverted()
    attachment_vertices = set(headrest['cockpit_attachment_vertex_indices'])
    original_positions = [conversion @ vertex.co for vertex in headrest.data.vertices]
    base_height = max(position.z for index, position in enumerate(original_positions) if index not in attachment_vertices and abs(position.x) < 0.0005 and position.y > -0.315 and position.y < -0.313)
    radius = options.radius_millimeters / 1000.0
    changed_vertices = []
    for vertex, original_position, rim_height in zip(headrest.data.vertices, original_positions, headrest['cockpit_rim_height_limits']):
        if vertex.index in attachment_vertices or original_position.y >= -0.25 or original_position.z <= 0.24:
            continue
        elevation = support_elevation(original_position.x, rim_height - 0.004, base_height, radius)
        if elevation <= 0:
            continue
        position = original_position.copy()
        upper_height = rim_height - 0.004
        top_progress = (rim_height - original_position.z) / 0.004
        if 0.2499 <= top_progress <= 1.0001:
            original_inner_position = -0.18 - 0.13 * math.sqrt(max(0.0, 1.0 - (original_position.x / 0.16) ** 2))
            activation = smooth_transition(elevation / 0.025)
            if top_progress <= 0.5:
                target_longitudinal = original_inner_position - 0.0115 + 0.003 * max(0.0, (top_progress - 0.25) / 0.25)
            else:
                target_longitudinal = (original_inner_position - 0.0085) * (2.0 - 2.0 * top_progress) + (original_inner_position + 0.002) * (2.0 * top_progress - 1.0)
            position.y += (target_longitudinal - position.y) * activation
            bevel_recess = 0.003 * max(0.0, (0.5 - top_progress) / 0.25)
            position.z += elevation + (upper_height - bevel_recess - position.z) * activation
        else:
            vertical_weight = smooth_transition((original_position.z - 0.24) / max(0.001, upper_height - 0.004 - 0.24))
            position.z += elevation * vertical_weight
            position.y += 0.006 * smooth_transition(elevation / 0.025) * vertical_weight
        vertex.co = inverse_conversion @ position
        changed_vertices.append(vertex.index)
    topology = bmesh.new()
    topology.from_mesh(headrest.data)
    bmesh.ops.recalc_face_normals(topology, faces=list(topology.faces))
    topology.to_mesh(headrest.data)
    topology.free()
    headrest.data.update()
    headrest['raised_rear_support_radius_meters'] = radius
    headrest['raised_rear_support_base_height_meters'] = base_height
    headrest['raised_rear_support_vertex_indices'] = changed_vertices
    headrest['raised_rear_support_generator'] = 'tools/blender/raise_formula_one_2030_headrest_support.py'
    headrest['height_priority'] = 'Central rear support has a raised circular crown; original cockpit attachments and recessed front ends remain fixed'
    candidate_path = validate_output_path(PROJECT_DIRECTORY, output_directory / 'f1_2030.blend', 'preview').path
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(candidate_path))
    print('Candidate saved; inspecting closed topology', flush=True)
    topology_report = inspect_mesh_geometry(headrest)
    print('Topology inspection complete; inspecting attachment and animated clearance', flush=True)
    attachment_report = measure_cockpit_attachment(headrest, conversion)
    positions = [conversion @ vertex.co for vertex in headrest.data.vertices]
    changed_vertex_set = set(changed_vertices)
    unchanged_exposed_vertices = [index for index in range(len(positions)) if index not in attachment_vertices and index not in changed_vertex_set]
    minimum_unchanged_recess = min(headrest['cockpit_rim_height_limits'][index] - positions[index].z for index in unchanged_exposed_vertices)
    attachment_report['minimum_exposed_padding_recess_below_rim_meters'] = minimum_unchanged_recess
    attachment_report['central_raised_support_exemption_vertices'] = len(changed_vertices)
    attachment_unchanged = all((positions[index] - original_positions[index]).length <= 0.0000001 for index in attachment_vertices)
    attachment_report['original_attachment_vertices_unchanged'] = attachment_unchanged
    attachment_report['passed'] = attachment_unchanged and attachment_report['maximum_attachment_edge_gap_meters'] <= 0.001 and attachment_report['maximum_visible_seam_gap_meters'] <= 0.00002 and minimum_unchanged_recess >= 0.00099 and attachment_report['minimum_exposed_front_padding_recess_below_rim_meters'] >= 0.0014 and not attachment_report['uncovered_support_vertices']
    driver_capture_path = options.driver_surfaces
    captured_driver = json.loads(driver_capture_path.read_text())
    clearance_report = measure_clearance(headrest, conversion, captured_driver)
    head_capture_path = options.head_motion_surfaces
    usable_samples, baseline_intersections = original_chassis_head_motion_surfaces(conversion, json.loads(head_capture_path.read_text()), headrest)
    head_motion_report = measure_head_motion_clearance(headrest, conversion, usable_samples, baseline_intersections)
    driver_accepted = all(measurement['maximum_sampled_penetration_meters'] <= (0.012 if all('Head' in bone or 'Neck' in bone for bone in measurement['bones']) else 0.004) for measurement in clearance_report['driver_intersections'])
    protected_unchanged = all(mesh_fingerprint(bpy.data.objects[name]) == fingerprint for name, fingerprint in protected_fingerprints.items())
    report = {
        'source_path': str(options.source_file), 'source_before_sha256': source_digest,
        'candidate_path': str(candidate_path), 'candidate_sha256': hashlib.sha256(candidate_path.read_bytes()).hexdigest(),
        'headrest_object': headrest.name, 'headrest_mesh_fingerprint': mesh_fingerprint(headrest),
        'protected_mesh_fingerprints': protected_fingerprints, 'protected_geometry_unchanged': protected_unchanged,
        'original_studio_settings': original_studio, 'original_object_names': original_objects,
        'studio_settings_preserved': studio_settings() == original_studio,
        'raised_rear_support': {'radius_meters': radius, 'diameter_meters': 2 * radius, 'base_height_meters': base_height, 'crown_height_meters': base_height + radius, 'nominal_rear_top_depth_meters': 0.0175, 'lateral_blend_extension_meters': 0.015, 'changed_vertex_count': len(changed_vertices)},
        'topology': topology_report, 'cockpit_attachment': attachment_report,
        'clearance': clearance_report, 'head_motion_validation': head_motion_report,
        'driver_capture_sha256': hashlib.sha256(driver_capture_path.read_bytes()).hexdigest(),
        'head_motion_capture_sha256': hashlib.sha256(head_capture_path.read_bytes()).hexdigest(),
        'runtime_exported': False,
    }
    report['passed'] = topology_report['passed'] and attachment_report['passed'] and protected_unchanged and report['studio_settings_preserved'] and not clearance_report['chassis_intersections'] and driver_accepted and head_motion_report['passed']
    (output_directory / 'upper_support_report.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'passed': report['passed'], 'topology_passed': topology_report['passed'], 'attachment_passed': attachment_report['passed'], 'chassis_intersections': clearance_report['chassis_intersections'], 'maximum_head_penetration_meters': head_motion_report['maximum_configured_pose_penetration_meters'], 'raised_rear_support': report['raised_rear_support']}, indent=2), flush=True)
    if not report['passed']:
        raise RuntimeError('The raised support validation did not pass')


if __name__ == '__main__':
    main()
