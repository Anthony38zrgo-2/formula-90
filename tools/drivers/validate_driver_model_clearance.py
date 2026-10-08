import argparse
import hashlib
import json
import sys
from pathlib import Path

import bpy
from mathutils import Vector
from mathutils.bvhtree import BVHTree


PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / "tools"))
from common.output_policy import validate_output_path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver-surfaces", type=Path, required=True)
    parser.add_argument("--driver-model", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    options = parser.parse_args(sys.argv[sys.argv.index("--") + 1:])
    output_path = validate_output_path(PROJECT_DIRECTORY, options.output, "preview").path
    captured_surfaces = json.loads(options.driver_surfaces.read_text(encoding="utf-8-sig"))
    chassis_path = PROJECT_DIRECTORY / "game/assets/models/vehicles/f1-2030/f1_2030_v10_chassis.glb"
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(chassis_path))
    protected_names = ("GEO_CHASSIS_BODY", "GEO_CHASSIS_INTERIOR", "GEO_CHASSIS_FLOOR", "GEO_CHASSIS_SEAT", "GEO_CHASSIS_CockpitUpperLining", "GEO_CHASSIS_STEERCOLUM")
    cockpit_surfaces = {}
    for object_name in protected_names:
        character_object = bpy.data.objects.get(object_name)
        if character_object is None:
            raise RuntimeError("Cockpit clearance requires " + object_name)
        cockpit_surfaces[object_name] = BVHTree.FromPolygons([character_object.matrix_world @ vertex.co for vertex in character_object.data.vertices], [list(polygon.vertices) for polygon in character_object.data.polygons])
    intersections = {object_name: [] for object_name in protected_names}
    minimum_separations = {object_name: float("inf") for object_name in protected_names}
    for sample in captured_surfaces["samples"]:
        positions = [Vector(position) for position in sample["vertices"]]
        driver_surface = BVHTree.FromPolygons(positions, sample["triangles"], all_triangles=True)
        for object_name, cockpit_surface in cockpit_surfaces.items():
            overlap_pairs = cockpit_surface.overlap(driver_surface)
            if overlap_pairs:
                implicated_vertex_indices = {vertex_index for _, driver_triangle_index in overlap_pairs for vertex_index in sample["triangles"][driver_triangle_index]}
                implicated_bones = sorted({sample["dominant_bones"][vertex_index] for vertex_index in implicated_vertex_indices})
                implicated_materials = sorted({sample["triangle_materials"][driver_triangle_index] for _, driver_triangle_index in overlap_pairs}) if "triangle_materials" in sample else []
                intersections[object_name].append({"steering_degrees": sample["steering_degrees"], "triangle_pairs": len(overlap_pairs), "bones": implicated_bones, "materials": implicated_materials, "example_positions": [list(positions[index]) for index in sorted(implicated_vertex_indices)[:12]]})
            for position in positions:
                nearest_surface_point = cockpit_surface.find_nearest(position)
                if nearest_surface_point and nearest_surface_point[0] is not None:
                    minimum_separations[object_name] = min(minimum_separations[object_name], nearest_surface_point[3])
    report = {"passed": not any(intersections.values()), "driver_sha256": hashlib.sha256(options.driver_model.read_bytes()).hexdigest(), "chassis_sha256": hashlib.sha256(chassis_path.read_bytes()).hexdigest(), "surface_capture_sha256": hashlib.sha256(options.driver_surfaces.read_bytes()).hexdigest(), "sample_count": len(captured_surfaces["samples"]), "intersections": intersections, "minimum_vertex_separation_meters": minimum_separations}
    output_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print("DRIVER_CLEARANCE_PASSED=" + str(report["passed"]))
    print("DRIVER_CLEARANCE_INTERSECTIONS=" + json.dumps({name: len(samples) for name, samples in intersections.items()}))
    if not report["passed"]:
        raise RuntimeError("Driver model intersects the existing cockpit")


if __name__ == "__main__":
    main()
