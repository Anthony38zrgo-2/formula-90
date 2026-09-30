import argparse
import hashlib
import json
import math
import struct
import subprocess
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree


PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / "tools"))
sys.path.insert(0, str(PROJECT_DIRECTORY / "tools/blender"))
from common.output_policy import validate_output_path
from export_f1_2030_v10 import export_chassis


def mesh_fingerprint(scene_object):
    digest = hashlib.sha256()
    digest.update(struct.pack("<16d", *(value for row in scene_object.matrix_world for value in row)))
    for vertex in scene_object.data.vertices:
        digest.update(struct.pack("<3f", *vertex.co))
    for polygon in scene_object.data.polygons:
        digest.update(struct.pack("<" + "I" * len(polygon.vertices), *polygon.vertices))
        digest.update(struct.pack("<I?", polygon.material_index, polygon.use_smooth))
    for texture_coordinates in scene_object.data.uv_layers:
        for coordinate in texture_coordinates.data:
            digest.update(struct.pack("<2f", *coordinate.uv))
    digest.update(str([material.name if material else None for material in scene_object.data.materials]).encode())
    return digest.hexdigest()


def create_driver_surfaces():
    source_snapshot = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=PROJECT_DIRECTORY, text=True).strip()
    if (PROJECT_DIRECTORY / "game/BUILD_SOURCE").read_text().strip() != source_snapshot:
        raise RuntimeError("Driver seat fitting requires runtime BUILD/HEAD parity")
    surface_path = validate_output_path(PROJECT_DIRECTORY, "scratch/driver_seat/runtime_driver_surface.json", "preview").path
    surface_path.parent.mkdir(parents=True, exist_ok=True)
    capture = subprocess.run([
        str(PROJECT_DIRECTORY / ".tools/godot/Godot_v4.7.1-stable_win64_console.exe"),
        "--headless", "--path", str(PROJECT_DIRECTORY / "game"),
        "--script", str(PROJECT_DIRECTORY / "tools/blender/capture_driver_seated_surface.gd"), "--", str(surface_path),
    ], cwd=PROJECT_DIRECTORY, capture_output=True, text=True, timeout=60, creationflags=subprocess.CREATE_NO_WINDOW)
    if capture.returncode != 0:
        raise RuntimeError("Animated driver surface capture failed: " + capture.stdout + capture.stderr)
    samples = json.loads(surface_path.read_text(encoding="utf-8"))["samples"]
    driver_surfaces = [(sample["steering_degrees"], BVHTree.FromPolygons([Vector(position) for position in sample["vertices"]], sample["triangles"], all_triangles=True)) for sample in samples]
    return driver_surfaces


def reshape_seat(seat, conversion, driver_surfaces):
    longitudinal_stations = (-0.225, -0.195, -0.165, -0.135, -0.100, -0.060, -0.020, 0.020, 0.060, 0.100, 0.140, 0.180, 0.220, 0.260, 0.300, 0.340, 0.380, 0.420, 0.460, 0.500)
    lateral_fractions = (-1.0, -0.96, -0.90, -0.80, -0.60, -0.40, -0.20, 0.0, 0.20, 0.40, 0.60, 0.80, 0.90, 0.96, 1.0)
    vertices = []
    faces = []
    seat_to_runtime = conversion @ seat.matrix_world
    runtime_to_seat = seat_to_runtime.inverted()
    for longitudinal_position in longitudinal_stations:
        center_hits = [surface.ray_cast(Vector((0.0, longitudinal_position, -1.0)), Vector((0.0, 0.0, 1.0)), 2.0)[0] for steering_degrees, surface in driver_surfaces]
        center_hit = min((hit for hit in center_hits if hit), key=lambda hit: hit.z, default=None)
        if longitudinal_position < -0.165:
            center_height = 0.055 - (longitudinal_position + 0.225) / 0.060 * 0.085
        else:
            center_height = center_hit.z - 0.022 if center_hit else -0.075
        if longitudinal_position >= 0.380:
            thigh_hits = [surface.ray_cast(Vector((lateral_position, longitudinal_position, -1.0)), Vector((0.0, 0.0, 1.0)), 2.0)[0] for lateral_position in (-0.110, -0.080, -0.050, 0.050, 0.080, 0.110) for steering_degrees, surface in driver_surfaces]
            lowest_thigh_hit = min((hit for hit in thigh_hits if hit), key=lambda hit: hit.z, default=None)
            if lowest_thigh_hit:
                center_height = min(center_height, lowest_thigh_hit.z - 0.022)
        half_width = 0.160
        if longitudinal_position < -0.135:
            half_width = 0.110 + 0.050 * (longitudinal_position + 0.225) / 0.090
        for lateral_fraction in lateral_fractions:
            lateral_position = lateral_fraction * half_width
            side_height = center_height + abs(lateral_fraction) ** 3 * 0.040
            lateral_hits = [surface.ray_cast(Vector((lateral_position, longitudinal_position, -1.0)), Vector((0.0, 0.0, 1.0)), 2.0)[0] for steering_degrees, surface in driver_surfaces]
            hit = min((hit for hit in lateral_hits if hit), key=lambda hit: hit.z, default=None)
            surface_height = min(side_height, hit.z - 0.022) if hit else side_height
            surface_height = min(surface_height, 0.065)
            vertices.append(Vector((lateral_position, longitudinal_position, surface_height)))
    row_width = len(lateral_fractions)
    inner_vertex_count = len(vertices)
    vertices.extend([position - Vector((0.0, 0.0, 0.006)) for position in vertices.copy()])
    for row_index in range(len(longitudinal_stations) - 1):
        for lateral_index in range(row_width - 1):
            first = row_index * row_width + lateral_index
            second = first + 1
            third = first + row_width + 1
            fourth = first + row_width
            faces.append((first, second, third, fourth))
            faces.append((fourth + inner_vertex_count, third + inner_vertex_count, second + inner_vertex_count, first + inner_vertex_count))
    perimeter = list(range(row_width))
    perimeter.extend(row_index * row_width + row_width - 1 for row_index in range(1, len(longitudinal_stations)))
    perimeter.extend(range(inner_vertex_count - 2, inner_vertex_count - row_width - 1, -1))
    perimeter.extend(row_index * row_width for row_index in range(len(longitudinal_stations) - 2, 0, -1))
    for perimeter_index, first in enumerate(perimeter):
        second = perimeter[(perimeter_index + 1) % len(perimeter)]
        faces.append((second, first, first + inner_vertex_count, second + inner_vertex_count))
    new_surface = BVHTree.FromPolygons(vertices, faces, all_triangles=False)
    intersection_measurements = []
    for steering_degrees, driver_surface in driver_surfaces:
        intersections = new_surface.overlap(driver_surface)
        intersection_measurements.append({"steering_degrees": steering_degrees, "intersection_pairs": len(intersections)})
        if intersections:
            intersection_centers = [sum((vertices[index] for index in faces[seat_face_index]), Vector()) / len(faces[seat_face_index]) for seat_face_index, driver_face_index in intersections]
            print("SEAT_INTERSECTION_CENTERS=" + str([tuple(round(value, 4) for value in position) for position in intersection_centers[:24]]))
            raise RuntimeError("Fitted seat intersects the animated driver at " + str(steering_degrees) + " degrees: " + str(len(intersections)) + " triangle pairs")
    original_mesh = seat.data
    fitted_mesh = bpy.data.meshes.new("GEO_CHASSIS_SEAT_MESH")
    fitted_mesh.from_pydata([runtime_to_seat @ position for position in vertices], [], faces)
    for material in original_mesh.materials:
        fitted_mesh.materials.append(material)
    texture_coordinates = fitted_mesh.uv_layers.new(name="SeatSurfaceCoordinates")
    for polygon in fitted_mesh.polygons:
        for loop_index in polygon.loop_indices:
            position = vertices[fitted_mesh.loops[loop_index].vertex_index]
            texture_coordinates.data[loop_index].uv = ((position.x + 0.16) / 0.32, (position.y + 0.225) / 0.725)
    fitted_mesh.update()
    seat.data = fitted_mesh
    if original_mesh.users == 0:
        original_name = original_mesh.name
        bpy.data.meshes.remove(original_mesh)
        fitted_mesh.name = original_name
    return {
        "vertex_count": len(vertices), "polygon_count": len(faces), "driver_intersection_pairs": intersection_measurements,
        "upper_edge_height_meters": max(position.z for position in vertices),
        "minimum_height_meters": min(position.z for position in vertices),
        "surface_clearance_meters": 0.022, "shell_thickness_meters": 0.006,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-directory", required=True)
    parser.add_argument("--mode", required=True, choices=("preview", "promote"))
    options = parser.parse_args(sys.argv[sys.argv.index("--") + 1:])
    destination = validate_output_path(PROJECT_DIRECTORY, options.output_directory, options.mode).path
    source_path = PROJECT_DIRECTORY / "game/assets/models/vehicles/f1-2030/source/f1_2030.blend"
    driver_path = PROJECT_DIRECTORY / "game/assets/models/drivers/driver.glb"
    program_directory = source_path.parents[1]
    manifest = json.loads((program_directory / "manifest.json").read_text(encoding="utf-8"))
    export_report = json.loads((program_directory / "export_report.json").read_text(encoding="utf-8"))
    source_sha256 = hashlib.sha256(source_path.read_bytes()).hexdigest()
    bpy.ops.wm.open_mainfile(filepath=str(source_path))
    seat = bpy.data.objects["GEO_CHASSIS_SEAT"]
    protected_meshes = {scene_object.name: mesh_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.type == "MESH" and scene_object != seat}
    conversion = Matrix.Translation((0.0, 0.0, manifest["authored_alignment"]["chassis_vertical_offset"])) @ Matrix.Rotation(-math.pi / 2, 4, "Z")
    original_upper_edge_height = max((conversion @ seat.matrix_world @ vertex.co).z for vertex in seat.data.vertices)
    driver_surfaces = create_driver_surfaces()
    measurements = reshape_seat(seat, conversion, driver_surfaces)
    for object_name, fingerprint in protected_meshes.items():
        if mesh_fingerprint(bpy.data.objects[object_name]) != fingerprint:
            raise RuntimeError("Seat fitting changed protected geometry: " + object_name)
    destination.mkdir(parents=True, exist_ok=True)
    output_source = destination / "source/f1_2030.blend" if options.mode == "promote" else destination / "f1_2030_driver_seat.blend"
    validate_output_path(PROJECT_DIRECTORY, output_source, options.mode)
    output_source.parent.mkdir(parents=True, exist_ok=True)
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(output_source))
    chassis_path = destination / "f1_2030_v10_chassis.glb"
    validate_output_path(PROJECT_DIRECTORY, chassis_path, options.mode)
    mesh_statistics = export_chassis(chassis_path, manifest["authored_alignment"]["chassis_vertical_offset"])
    chassis_record = {"path": chassis_path.name, "sha256": hashlib.sha256(chassis_path.read_bytes()).hexdigest().upper(), "size_bytes": chassis_path.stat().st_size}
    manifest["geometry_assets"]["chassis"] = chassis_record
    export_report["files"]["chassis"] = chassis_record
    export_report["mesh_stats"]["chassis"] = mesh_statistics
    report = {
        "operation": "Fit and lower GEO_CHASSIS_SEAT around the original F1 2030 driver", "passed": True,
        "source_before_sha256": source_sha256,
        "source_after_sha256": hashlib.sha256(output_source.read_bytes()).hexdigest(),
        "driver_sha256": hashlib.sha256(driver_path.read_bytes()).hexdigest(),
        "protected_mesh_count": len(protected_meshes), "protected_meshes_unchanged": True,
        "original_upper_edge_height_meters": original_upper_edge_height,
        "upper_edge_trim_meters": original_upper_edge_height - measurements["upper_edge_height_meters"],
        "measurements": measurements, "chassis": chassis_record,
    }
    for output_name, contents in (("manifest.json", manifest), ("export_report.json", export_report), ("source/driver_seat_fit_report.json", report)):
        output_path = destination / output_name
        validate_output_path(PROJECT_DIRECTORY, output_path, options.mode)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(json.dumps(contents, indent=2) + "\n", encoding="utf-8", newline="\n")
    print("DRIVER_SEAT_FIT=" + json.dumps(report))


if __name__ == "__main__":
    main()
