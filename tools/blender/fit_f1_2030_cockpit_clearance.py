import argparse
import hashlib
import json
import math
import re as regular_expressions
import shutil
import subprocess
import sys
from functools import lru_cache
from pathlib import Path

import bpy
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree


PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / "tools"))
sys.path.insert(0, str(PROJECT_DIRECTORY / "tools/blender"))
from common.output_policy import validate_output_path
from export_f1_2030_v10 import export_chassis
from fit_f1_2030_driver_seat import mesh_fingerprint

ANIMATION_FILE_PATHS = (
    "game/scripts/vehicle/driver_visual_controller.gd",
    "game/scripts/vehicle/driver_arm_inverse_kinematics_modifier.gd",
    "game/scripts/vehicle/steering_wheel_visual_controller.gd",
    "game/scripts/runtime/wheel_gun_arm_inverse_kinematics_modifier.gd",
    "game/scenes/vehicles/f1_2030_v10/f1_2030_v10_rust.tscn",
)


def driver_capture_provenance():
    driver_path = PROJECT_DIRECTORY / "game/assets/models/drivers/driver.glb"
    return {
        "driver_sha256": hashlib.sha256(driver_path.read_bytes()).hexdigest(),
        "animation_sha256": {relative_path: hashlib.sha256((PROJECT_DIRECTORY / relative_path).read_bytes()).hexdigest() for relative_path in ANIMATION_FILE_PATHS},
        "capture_sha256": {relative_path: hashlib.sha256((PROJECT_DIRECTORY / relative_path).read_bytes()).hexdigest() for relative_path in ("tools/blender/capture_driver_cockpit_surfaces.gd", "tools/blender/capture_driver_seated_surface.gd")},
    }


def capture_driver_surfaces(chassis_path, seated_position):
    provenance = driver_capture_provenance()
    source_commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=PROJECT_DIRECTORY, text=True).strip()
    capture_directory = validate_output_path(PROJECT_DIRECTORY, "scratch/cockpit_clearance_capture/" + source_commit, "preview").path
    capture_directory.mkdir(parents=True, exist_ok=True)
    copies = {
        "game/assets/models/drivers/driver.glb": "assets/driver.glb",
        "tools/blender/capture_driver_seated_surface.gd": "capture_driver_seated_surface.gd",
        "tools/blender/capture_driver_cockpit_surfaces.gd": "capture_driver_cockpit_surfaces.gd",
    }
    copies.update({relative_path: relative_path.removeprefix("game/") for relative_path in ANIMATION_FILE_PATHS if relative_path.endswith(".gd")})
    for source_relative_path, output_relative_path in copies.items():
        copied_path = validate_output_path(PROJECT_DIRECTORY, capture_directory / output_relative_path, "preview").path
        copied_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(PROJECT_DIRECTORY / source_relative_path, copied_path)
    shutil.copyfile(chassis_path, validate_output_path(PROJECT_DIRECTORY, capture_directory / "assets/chassis.glb", "preview").path)
    project_path = validate_output_path(PROJECT_DIRECTORY, capture_directory / "project.godot", "preview").path
    project_path.write_text('config_version=5\n[application]\nconfig/name="Cockpit clearance capture"\n[rendering]\nrenderer/rendering_method="gl_compatibility"\n', encoding="utf-8")
    executable_path = PROJECT_DIRECTORY / ".tools/godot/Godot_v4.7.1-stable_win64_console.exe"
    output_path = validate_output_path(PROJECT_DIRECTORY, capture_directory / "driver_surfaces.json", "preview").path
    commands = [
        [str(executable_path), "--headless", "--path", str(capture_directory), "--editor", "--import"],
        [str(executable_path), "--headless", "--path", str(capture_directory), "--fixed-fps", "60", "--script", "res://capture_driver_cockpit_surfaces.gd", "--", str(output_path), *[str(coordinate) for coordinate in seated_position]],
    ]
    for command in commands:
        result = subprocess.run(command, cwd=PROJECT_DIRECTORY, capture_output=True, text=True, timeout=180, creationflags=subprocess.CREATE_NO_WINDOW)
        if result.returncode != 0 or "SCRIPT ERROR" in result.stdout + result.stderr:
            raise RuntimeError("Isolated driver capture failed: " + result.stdout + result.stderr)
    provenance_path = validate_output_path(PROJECT_DIRECTORY, output_path.with_suffix(".provenance.json"), "preview").path
    if provenance != driver_capture_provenance():
        raise RuntimeError("Driver capture sources changed while capturing the animation")
    provenance["surfaces_sha256"] = hashlib.sha256(output_path.read_bytes()).hexdigest()
    provenance["capture_chassis_sha256"] = hashlib.sha256(chassis_path.read_bytes()).hexdigest()
    provenance["seated_position"] = list(seated_position)
    provenance_path.write_text(json.dumps(provenance, indent=2) + "\n", encoding="utf-8")
    return output_path


def smooth_interval(value, lower, upper):
    fraction = min(max((value - lower) / (upper - lower), 0.0), 1.0)
    return fraction * fraction * (3.0 - 2.0 * fraction)


def interval_influence(value, outer_lower, inner_lower, inner_upper, outer_upper):
    return smooth_interval(value, outer_lower, inner_lower) * (1.0 - smooth_interval(value, inner_upper, outer_upper))


def mesh_surface(scene_object, conversion):
    return BVHTree.FromPolygons(
        [conversion @ scene_object.matrix_world @ vertex.co for vertex in scene_object.data.vertices],
        [list(polygon.vertices) for polygon in scene_object.data.polygons],
    )


def move_driving_assembly(conversion, rearward_distance):
    moved_object_names = ("GEO_CHASSIS_SEAT", "GEO_CHASSIS_STEER", "GEO_CHASSIS_SCREEN", "GEO_CHASSIS_SHIFTERBRAK", "GEO_CHASSIS_SHIFTERTHRO", "GEO_CHASSIS_CYLINDER_001")
    for object_name in moved_object_names:
        scene_object = bpy.data.objects[object_name]
        source_to_runtime = conversion @ scene_object.matrix_world
        runtime_to_source = source_to_runtime.inverted()
        for vertex in scene_object.data.vertices:
            vertex.co = runtime_to_source @ (source_to_runtime @ vertex.co - Vector((0.0, rearward_distance, 0.0)))
        scene_object.data.update()
    steering_column = bpy.data.objects["GEO_CHASSIS_STEERCOLUM"]
    source_to_runtime = conversion @ steering_column.matrix_world
    runtime_to_source = source_to_runtime.inverted()
    column_positions = [source_to_runtime @ vertex.co for vertex in steering_column.data.vertices]
    column_front = max(position.y for position in column_positions)
    column_rear = min(position.y for position in column_positions)
    for vertex, position in zip(steering_column.data.vertices, column_positions):
        extension_fraction = (column_front - position.y) / (column_front - column_rear)
        position.y -= rearward_distance * extension_fraction
        vertex.co = runtime_to_source @ position
    steering_column.data.update()
    return {"rearward_distance_meters": rearward_distance, "moved_objects": list(moved_object_names), "column_original_length_meters": column_front - column_rear, "column_final_length_meters": column_front - column_rear + rearward_distance, "column_front_anchor_meters": column_front}


def reshape_body(body, conversion, rearward_distance):
    source_to_runtime = conversion @ body.matrix_world
    runtime_to_source = source_to_runtime.inverted()
    moved_vertices = []
    for vertex in body.data.vertices:
        original_position = source_to_runtime @ vertex.co
        position = original_position.copy()
        longitudinal_influence = interval_influence(position.y + rearward_distance, -0.12, 0.10, 0.34, 0.55)
        lateral_influence = interval_influence(abs(position.x), 0.13, 0.18, 0.24, 0.42)
        height_influence = interval_influence(position.z, 0.19, 0.245, 0.31, 0.39)
        side_sign = -1.0 if position.x < 0.0 else 1.0
        position.x += side_sign * 0.050 * longitudinal_influence * lateral_influence * height_influence
        footwell_influence = interval_influence(position.y + rearward_distance, 1.00, 1.15, 1.45, 1.65)
        footwell_influence *= 1.0 - smooth_interval(abs(position.x), 0.15, 0.28)
        footwell_influence *= interval_influence(position.z, -0.22, -0.15, -0.04, 0.00)
        position.z -= 0.035 * footwell_influence
        if position != original_position:
            moved_vertices.append({"vertex": vertex.index, "before": list(original_position), "after": list(position)})
            vertex.co = runtime_to_source @ position
    body.data.update()
    return moved_vertices


def reshape_floor(floor, conversion, seat_surface, rearward_distance):
    source_to_runtime = conversion @ floor.matrix_world
    runtime_to_source = source_to_runtime.inverted()
    moved_vertices = []
    for vertex in floor.data.vertices:
        original_position = source_to_runtime @ vertex.co
        position = original_position.copy()
        longitudinal_influence = interval_influence(position.y + rearward_distance, 0.22, 0.32, 0.54, 0.72)
        lateral_influence = 1.0 - smooth_interval(abs(position.x), 0.13, 0.22)
        upper_panel_influence = smooth_interval(position.z, -0.24, -0.16)
        influence = longitudinal_influence * lateral_influence * upper_panel_influence
        if influence > 0.0:
            seat_hit = seat_surface.ray_cast(Vector((position.x, position.y, -1.0)), Vector((0.0, 0.0, 1.0)), 2.0)[0]
            desired_height = min(position.z - 0.025, seat_hit.z - 0.010) if seat_hit else position.z - 0.025
            position.z += (desired_height - position.z) * influence
        if position != original_position:
            moved_vertices.append({"vertex": vertex.index, "before": list(original_position), "after": list(position)})
            vertex.co = runtime_to_source @ position
    floor.data.update()
    return moved_vertices


def rebuild_interior(interior, conversion, seat_surface, driver_surfaces, rearward_distance):
    source_to_runtime = conversion @ interior.matrix_world
    runtime_to_source = source_to_runtime.inverted()
    original_surface = mesh_surface(interior, conversion)
    @lru_cache(maxsize=None)
    def wall_extent(longitudinal_position, height, side_sign):
        extent = 0.0
        for driver_surface in driver_surfaces[::2]:
            for longitudinal_offset in (-0.025, 0.0, 0.025):
                for height_offset in (-0.030, 0.0, 0.030):
                    hit = driver_surface.ray_cast(Vector((side_sign * 0.8, longitudinal_position + longitudinal_offset, height + height_offset)), Vector((-side_sign, 0.0, 0.0)), 1.6)[0]
                    if hit:
                        extent = max(extent, hit.x * side_sign + 0.012)
        return extent

    longitudinal_stations = [-0.30 - rearward_distance + station_index * 0.025 for station_index in range(71)]
    vertices = []
    faces = []
    for longitudinal_position in longitudinal_stations:
        driver_floor_hits = [driver_surface.ray_cast(Vector((lateral_position, longitudinal_position + longitudinal_offset, -1.0)), Vector((0.0, 0.0, 1.0)), 2.0)[0] for driver_surface in driver_surfaces[::16] for longitudinal_offset in (-0.025, 0.0, 0.025) for lateral_position in (-0.30, -0.24, -0.18, -0.12, -0.06, 0.0, 0.06, 0.12, 0.18, 0.24, 0.30)]
        minimum_height = min((hit.z for hit in driver_floor_hits if hit), default=-0.075) - 0.015
        seat_hits = [seat_surface.ray_cast(Vector((lateral_position, longitudinal_position + longitudinal_offset, -1.0)), Vector((0.0, 0.0, 1.0)), 2.0)[0] for longitudinal_offset in (-0.025, 0.0, 0.025) for lateral_position in (-0.15, -0.10, -0.05, 0.0, 0.05, 0.10, 0.15)]
        minimum_height = min(minimum_height, min((hit.z for hit in seat_hits if hit), default=minimum_height) - 0.010)
        original_wall_hit = original_surface.ray_cast(Vector((0.0, longitudinal_position, 0.15)), Vector((1.0, 0.0, 0.0)), 0.8)[0]
        original_half_width = original_wall_hit.x if original_wall_hit else 0.15
        top_width = original_half_width + 0.050 * interval_influence(longitudinal_position + rearward_distance, -0.12, 0.10, 0.34, 0.55)
        top_height = 0.265 - 0.055 * smooth_interval(longitudinal_position + rearward_distance, 0.30, 0.55) - 0.070 * smooth_interval(longitudinal_position + rearward_distance, 1.20, 1.45)
        top_height += 0.025 * (1.0 - smooth_interval(longitudinal_position + rearward_distance, -0.25, -0.10))
        bottom_width = min(top_width, 0.165)
        left_wall = []
        right_wall = []
        for wall_level in range(13):
            fraction = wall_level / 12.0
            height = minimum_height + (top_height - minimum_height) * fraction
            base_width = bottom_width + (top_width - bottom_width) * smooth_interval(fraction, 0.0, 1.0)
            for side_sign, wall_positions in [(-1.0, left_wall), (1.0, right_wall)]:
                width = max(base_width, wall_extent(longitudinal_position, height, side_sign))
                wall_positions.append(Vector((side_sign * width, longitudinal_position, height)))
        floor_positions = [Vector((bottom_width * fraction, longitudinal_position, minimum_height)) for fraction in (-0.75, -0.50, -0.25, 0.0, 0.25, 0.50, 0.75)]
        vertices.extend(list(reversed(left_wall)) + floor_positions + right_wall)
    row_width = 33
    inner_vertex_count = len(vertices)
    for position_index, position in enumerate(vertices.copy()):
        column = position_index % row_width
        outer_position = position.copy()
        if position_index < row_width:
            outer_position.y -= 0.006
        elif position_index >= inner_vertex_count - row_width:
            outer_position.y += 0.006
        if column < 13:
            outer_position.x -= 0.006
        elif column >= 20:
            outer_position.x += 0.006
        else:
            outer_position.z -= 0.006
        vertices.append(outer_position)
    for row_index in range(len(longitudinal_stations) - 1):
        for column in range(row_width - 1):
            first = row_index * row_width + column
            second = first + 1
            third = first + row_width + 1
            fourth = first + row_width
            faces.append((first, second, third, fourth))
            faces.append((fourth + inner_vertex_count, third + inner_vertex_count, second + inner_vertex_count, first + inner_vertex_count))
    for row_index in range(len(longitudinal_stations) - 1):
        for column in (0, row_width - 1):
            first = row_index * row_width + column
            second = first + row_width
            faces.append((second, first, first + inner_vertex_count, second + inner_vertex_count))
    rear_cross_section = list(range(row_width))
    front_cross_section = list(range(inner_vertex_count - row_width, inner_vertex_count))
    faces.append(tuple(reversed(rear_cross_section)))
    faces.append(tuple(vertex_index + inner_vertex_count for vertex_index in rear_cross_section))
    faces.append(tuple(front_cross_section))
    faces.append(tuple(vertex_index + inner_vertex_count for vertex_index in reversed(front_cross_section)))
    for cross_section in (rear_cross_section, front_cross_section):
        first = cross_section[0]
        second = cross_section[-1]
        faces.append((first, second, second + inner_vertex_count, first + inner_vertex_count))
    original_mesh = interior.data
    rebuilt_mesh = bpy.data.meshes.new("CockpitInteriorSurface")
    rebuilt_mesh.from_pydata([runtime_to_source @ position for position in vertices], [], faces)
    for material in original_mesh.materials:
        rebuilt_mesh.materials.append(material)
    texture_coordinates = rebuilt_mesh.uv_layers.new(name="CockpitSurfaceCoordinates")
    for polygon in rebuilt_mesh.polygons:
        polygon.use_smooth = True
        for loop_index in polygon.loop_indices:
            vertex_index = rebuilt_mesh.loops[loop_index].vertex_index
            texture_coordinates.data[loop_index].uv = (vertex_index % row_width / (row_width - 1), (vertices[vertex_index].y + 0.30 + rearward_distance) / 1.75)
    rebuilt_mesh.update()
    interior.data = rebuilt_mesh
    if original_mesh.users == 0:
        original_name = original_mesh.name
        bpy.data.meshes.remove(original_mesh)
        rebuilt_mesh.name = original_name
    return {"vertices": len(vertices), "polygons": len(faces), "wall_thickness_meters": 0.006}


def validate_driver_clearance(conversion, driver_surfaces, driver_samples):
    intersections = {}
    for object_name in ("GEO_CHASSIS_BODY", "GEO_CHASSIS_INTERIOR", "GEO_CHASSIS_FLOOR", "GEO_CHASSIS_SEAT"):
        surface = mesh_surface(bpy.data.objects[object_name], conversion)
        failures = []
        for sample_index, driver_surface in enumerate(driver_surfaces):
            overlap_pairs = surface.overlap(driver_surface)
            if overlap_pairs:
                failures.append({"sample": sample_index, "steering_degrees": driver_samples[sample_index]["steering_degrees"], "triangle_pairs": len(overlap_pairs)})
        intersections[object_name] = failures
    return intersections


def validate_steering_clearance(conversion, driver_surfaces, driver_samples):
    steering_wheel = bpy.data.objects["GEO_CHASSIS_STEER"]
    steering_column = bpy.data.objects["GEO_CHASSIS_STEERCOLUM"]
    wheel_positions = [conversion @ steering_wheel.matrix_world @ vertex.co for vertex in steering_wheel.data.vertices]
    column_positions = [conversion @ steering_column.matrix_world @ vertex.co for vertex in steering_column.data.vertices]
    pivot = Vector(((min(position.x for position in column_positions) + max(position.x for position in column_positions)) * 0.5, (min(position.y for position in wheel_positions) + max(position.y for position in wheel_positions)) * 0.5, (min(position.z for position in column_positions) + max(position.z for position in column_positions)) * 0.5))
    intersections = {}
    for object_name in ("GEO_CHASSIS_STEER", "GEO_CHASSIS_SCREEN", "GEO_CHASSIS_SHIFTERBRAK", "GEO_CHASSIS_SHIFTERTHRO", "GEO_CHASSIS_STEERCOLUM", "GEO_CHASSIS_CYLINDER_001"):
        scene_object = bpy.data.objects[object_name]
        original_positions = [conversion @ scene_object.matrix_world @ vertex.co for vertex in scene_object.data.vertices]
        polygons = [list(polygon.vertices) for polygon in scene_object.data.polygons]
        rotating = object_name not in ("GEO_CHASSIS_STEERCOLUM", "GEO_CHASSIS_CYLINDER_001")
        @lru_cache(maxsize=None)
        def rotated_surface(steering_degrees):
            rotation = Matrix.Rotation(math.radians(-steering_degrees), 4, "Y")
            positions = [pivot + rotation @ (position - pivot) for position in original_positions] if rotating else original_positions
            return BVHTree.FromPolygons(positions, polygons)
        failures = []
        for sample_index, (driver_sample, driver_surface) in enumerate(zip(driver_samples, driver_surfaces)):
            overlap_pairs = rotated_surface(driver_sample["steering_degrees"]).overlap(driver_surface)
            if overlap_pairs:
                failures.append({"sample": sample_index, "steering_degrees": driver_sample["steering_degrees"], "triangle_pairs": len(overlap_pairs)})
        intersections[object_name] = failures
    return intersections


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver-surfaces", type=Path)
    parser.add_argument("--source-file", type=Path)
    parser.add_argument("--rearward-distance", type=float, default=0.100)
    parser.add_argument("--output-directory", required=True)
    parser.add_argument("--mode", choices=("preview", "promote"), required=True)
    options = parser.parse_args(sys.argv[sys.argv.index("--") + 1:])
    source_branch = subprocess.check_output(["git", "branch", "--show-current"], cwd=PROJECT_DIRECTORY, text=True).strip()
    source_commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=PROJECT_DIRECTORY, text=True).strip()
    source_status = subprocess.check_output(["git", "status", "--short"], cwd=PROJECT_DIRECTORY, text=True).splitlines()
    destination = validate_output_path(PROJECT_DIRECTORY, options.output_directory, options.mode).path
    destination.mkdir(parents=True, exist_ok=True)
    canonical_source_path = PROJECT_DIRECTORY / "game/assets/models/vehicles/f1-2030/source/f1_2030.blend"
    source_path = options.source_file.resolve() if options.source_file else canonical_source_path
    program_directory = canonical_source_path.parents[1]
    manifest = json.loads((program_directory / "manifest.json").read_text())
    export_report = json.loads((program_directory / "export_report.json").read_text())
    existing_report_path = program_directory / "source/cockpit_clearance_report.json"
    if existing_report_path.exists() and not options.source_file:
        existing_report = json.loads(existing_report_path.read_text())
        if existing_report.get("source_after_sha256") == hashlib.sha256(source_path.read_bytes()).hexdigest():
            raise RuntimeError("Cockpit is already fitted; supply --source-file with the recorded original source to regenerate")
    bpy.ops.wm.open_mainfile(filepath=str(source_path))
    changed_object_names = {"GEO_CHASSIS_BODY", "GEO_CHASSIS_INTERIOR", "GEO_CHASSIS_FLOOR", "GEO_CHASSIS_SEAT", "GEO_CHASSIS_STEER", "GEO_CHASSIS_SCREEN", "GEO_CHASSIS_SHIFTERBRAK", "GEO_CHASSIS_SHIFTERTHRO", "GEO_CHASSIS_CYLINDER_001", "GEO_CHASSIS_STEERCOLUM"}
    protected_meshes = {scene_object.name: mesh_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.type == "MESH" and scene_object.name not in changed_object_names}
    conversion = Matrix.Translation((0.0, 0.0, manifest["authored_alignment"]["chassis_vertical_offset"])) @ Matrix.Rotation(-math.pi / 2.0, 4, "Z")
    driving_assembly = move_driving_assembly(conversion, options.rearward_distance)
    scene_source = (PROJECT_DIRECTORY / "game/scenes/vehicles/f1_2030_v10/f1_2030_v10_rust.tscn").read_text()
    seated_position = tuple(float(coordinate) for coordinate in regular_expressions.search(r"^seated_position = Vector3\(([^)]+)\)", scene_source, regular_expressions.MULTILINE).group(1).split(","))
    if abs(seated_position[2] - (-0.34 + options.rearward_distance)) > 0.000001:
        raise RuntimeError("Driver scene position does not match the requested rearward displacement")
    if options.driver_surfaces:
        driver_surfaces_path = options.driver_surfaces.resolve()
    else:
        capture_chassis_path = validate_output_path(PROJECT_DIRECTORY, "scratch/cockpit_correction/capture_rearward_chassis.glb", "preview").path
        capture_chassis_path.parent.mkdir(parents=True, exist_ok=True)
        export_chassis(capture_chassis_path, manifest["authored_alignment"]["chassis_vertical_offset"])
        driver_surfaces_path = capture_driver_surfaces(capture_chassis_path, seated_position)
    captured_provenance = json.loads(driver_surfaces_path.with_suffix(".provenance.json").read_text())
    for field_name, expected_value in driver_capture_provenance().items():
        if captured_provenance.get(field_name) != expected_value:
            raise RuntimeError("Driver surface capture is stale: " + field_name)
    if captured_provenance.get("surfaces_sha256") != hashlib.sha256(driver_surfaces_path.read_bytes()).hexdigest():
        raise RuntimeError("Driver surface capture does not match its recorded hash")
    if captured_provenance.get("seated_position") != list(seated_position):
        raise RuntimeError("Driver surface capture does not match the driver scene position")
    captured_driver = json.loads(driver_surfaces_path.read_text())
    driver_samples = captured_driver["samples"]
    required_glove_bones = {"Driver" + side + finger + segment for side in ("Left", "Right") for finger in ("Index", "Middle", "Ring", "Little", "Thumb") for segment in (("Metacarpal", "Proximal", "Distal") if finger == "Thumb" else ("Proximal", "Middle", "Distal"))}
    if any(not required_glove_bones.issubset(sample["dominant_bones"]) for sample in driver_samples):
        raise RuntimeError("Driver surface capture does not include both complete articulated gloves")
    driver_surfaces = [BVHTree.FromPolygons([Vector(position) for position in sample["vertices"]], sample["triangles"], all_triangles=True) for sample in driver_samples]
    seat_surface = mesh_surface(bpy.data.objects["GEO_CHASSIS_SEAT"], conversion)
    body_changes = reshape_body(bpy.data.objects["GEO_CHASSIS_BODY"], conversion, options.rearward_distance)
    floor_changes = reshape_floor(bpy.data.objects["GEO_CHASSIS_FLOOR"], conversion, seat_surface, options.rearward_distance)
    interior_measurements = rebuild_interior(bpy.data.objects["GEO_CHASSIS_INTERIOR"], conversion, seat_surface, driver_surfaces, options.rearward_distance)
    for object_name, original_fingerprint in protected_meshes.items():
        if mesh_fingerprint(bpy.data.objects[object_name]) != original_fingerprint:
            raise RuntimeError("Cockpit fitting changed protected mesh: " + object_name)
    intersections = validate_driver_clearance(conversion, driver_surfaces, driver_samples)
    steering_intersections = validate_steering_clearance(conversion, driver_surfaces, driver_samples)
    seat_intersections = {object_name: len(seat_surface.overlap(mesh_surface(bpy.data.objects[object_name], conversion))) for object_name in ("GEO_CHASSIS_BODY", "GEO_CHASSIS_INTERIOR", "GEO_CHASSIS_FLOOR")}
    passed = not any(intersections.values()) and not any(seat_intersections.values()) and not any(steering_intersections.values())
    report = {
        "operation": "Fit F1 2030 cockpit around the animated driver",
        "passed": passed,
        "source_branch": source_branch,
        "source_commit": source_commit,
        "source_status_before_operation": source_status,
        "source_before_sha256": hashlib.sha256(source_path.read_bytes()).hexdigest(),
        "driver_surfaces_sha256": hashlib.sha256(driver_surfaces_path.read_bytes()).hexdigest(),
        "driver_capture_provenance": captured_provenance,
        "driver_surface_samples": len(driver_samples),
        "driver_gloves_included": True,
        "articulated_glove_bones": sorted(required_glove_bones),
        "validation_sequences": captured_driver["validation_sequences"],
        "driving_assembly": driving_assembly,
        "seated_position": list(seated_position),
        "protected_mesh_count": len(protected_meshes),
        "protected_meshes_unchanged": True,
        "body_vertex_changes": body_changes,
        "floor_vertex_changes": floor_changes,
        "interior": interior_measurements,
        "driver_intersections": intersections,
        "seat_intersections": seat_intersections,
        "steering_intersections": steering_intersections,
    }
    if options.mode == "promote" and not passed:
        raise RuntimeError("Cockpit still intersects the animated driver; promotion rejected")
    report_relative_path = "source/cockpit_clearance_report.json" if options.mode == "promote" else "cockpit_clearance_report.json"
    report_path = validate_output_path(PROJECT_DIRECTORY, destination / report_relative_path, options.mode).path
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    output_source = destination / "source/f1_2030.blend" if options.mode == "promote" else destination / "f1_2030_cockpit.blend"
    validate_output_path(PROJECT_DIRECTORY, output_source, options.mode)
    output_source.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(output_source))
    chassis_path = validate_output_path(PROJECT_DIRECTORY, destination / "f1_2030_v10_chassis.glb", options.mode).path
    mesh_statistics = export_chassis(chassis_path, manifest["authored_alignment"]["chassis_vertical_offset"])
    chassis_record = {"path": chassis_path.name, "sha256": hashlib.sha256(chassis_path.read_bytes()).hexdigest().upper(), "size_bytes": chassis_path.stat().st_size}
    manifest["geometry_assets"]["chassis"] = chassis_record
    export_report["files"]["chassis"] = chassis_record
    export_report["mesh_stats"]["chassis"] = mesh_statistics
    report["source_after_sha256"] = hashlib.sha256(output_source.read_bytes()).hexdigest()
    report["chassis"] = chassis_record
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    for output_name, contents in (("manifest.json", manifest), ("export_report.json", export_report)):
        output_path = validate_output_path(PROJECT_DIRECTORY, destination / output_name, options.mode).path
        output_path.write_text(json.dumps(contents, indent=2) + "\n", encoding="utf-8")
    print("COCKPIT_CLEARANCE=" + json.dumps({"passed": passed, "intersections": {name: len(samples) for name, samples in intersections.items()}, "body_vertices": len(body_changes), "floor_vertices": len(floor_changes)}), flush=True)


if __name__ == "__main__":
    main()
