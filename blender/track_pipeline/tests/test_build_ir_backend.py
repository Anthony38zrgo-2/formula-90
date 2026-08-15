"""Tests for the BuildIR Blender materialization backend (pure, no bpy)."""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender" / "track_pipeline"))

from blender_backend.build_ir_loader import (
    BuildIR,
    ExplicitAssetInstance,
    RoadSample,
    TerrainCell,
    VegetationInstance,
    materialization_prerequisites,
    materialization_ready,
)
from blender_backend.object_builder import object_card_geometry
from blender_backend.road_builder import road_mesh_geometry
from blender_backend.terrain_builder import terrain_grid


def _sample(station: float, width_left: float = 6.0, width_right: float = 6.0) -> RoadSample:
    return RoadSample(
        station=station,
        position=(0.0, 0.0),
        tangent=(1.0, 0.0),
        normal=(0.0, 1.0),
        width_left=width_left,
        width_right=width_right,
        elevation=0.0,
        bank=0.0,
    )


class BuildIrLoaderTests(unittest.TestCase):
    def test_round_trip_load_and_ready(self) -> None:
        ir = BuildIR(
            build_ir_version=1,
            compiler_version="track-build-0.1.0",
            source_project_hash="a" * 64,
            road_samples=[_sample(0.0), _sample(10.0), _sample(20.0)],
        )
        self.assertTrue(materialization_ready(ir))

    def test_empty_road_samples_not_ready(self) -> None:
        ir = BuildIR(
            build_ir_version=1,
            compiler_version="track-build-0.1.0",
            source_project_hash="a" * 64,
        )
        self.assertFalse(materialization_ready(ir))
        self.assertIn("road_samples are empty", materialization_prerequisites(ir))

    def test_non_positive_width_not_ready(self) -> None:
        ir = BuildIR(
            build_ir_version=1,
            compiler_version="track-build-0.1.0",
            source_project_hash="a" * 64,
            road_samples=[_sample(0.0, width_left=-1.0), _sample(10.0)],
        )
        self.assertFalse(materialization_ready(ir))
        self.assertTrue(any("half width" in message for message in materialization_prerequisites(ir)))

    def test_load_from_json_file(self) -> None:
        data = {
            "build_ir_version": 1,
            "compiler_version": "track-build-0.1.0",
            "source_project_hash": "b" * 64,
            "road_samples": [
                {
                    "station": 0.0,
                    "position": [0.0, 0.0],
                    "tangent": [1.0, 0.0],
                    "normal": [0.0, 1.0],
                    "width_left": 5.0,
                    "width_right": 6.0,
                    "elevation": 0.0,
                    "bank": 0.0,
                }
            ],
            "asset_instances": [],
        }
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "track.build.json")
            with open(path, "w", encoding="utf-8") as handle:
                json.dump(data, handle)
            with open(path, encoding="utf-8") as handle:
                loaded = BuildIR.from_dict(json.load(handle))
        self.assertEqual(loaded.build_ir_version, 1)
        self.assertEqual(loaded.road_samples[0].width_left, 5.0)
        self.assertTrue(materialization_ready(loaded))


class RoadMeshGeometryTests(unittest.TestCase):
    def test_ribbon_vertex_and_face_counts(self) -> None:
        samples = [_sample(0.0), _sample(10.0), _sample(20.0), _sample(30.0)]
        vertices, faces = road_mesh_geometry(samples)
        self.assertEqual(len(vertices), len(samples) * 2)
        # 3 interior quads + 1 closing quad.
        self.assertEqual(len(faces), len(samples))

    def test_left_right_edges_follow_normal(self) -> None:
        samples = [_sample(0.0), _sample(10.0)]
        vertices, _ = road_mesh_geometry(samples)
        # normal = (0,1); left = position - normal*width = (0,-6); right = (0,+6).
        self.assertEqual(vertices[0], (0.0, -6.0))
        self.assertEqual(vertices[1], (0.0, 6.0))


class ObjectCardGeometryTests(unittest.TestCase):
    def test_card_centered_on_position_with_scale(self) -> None:
        instance = ExplicitAssetInstance(
            instance_id="tree_a",
            asset_id="tree_v2_01",
            position=(10.0, 20.0),
            yaw_rad=0.0,
            scale=2.0,
        )
        vertices, faces = object_card_geometry(instance)
        self.assertEqual(len(vertices), 4)
        self.assertEqual(faces, [(0, 1, 2, 3)])
        xs = [vertex[0] for vertex in vertices]
        zs = [vertex[2] for vertex in vertices]
        # The card is anchored at the base: min Z is the instance position, and
        # X is centered on it.
        self.assertAlmostEqual(min(zs), 20.0, places=6)
        self.assertAlmostEqual((min(xs) + max(xs)) / 2.0, 10.0, places=6)
        # Scale doubles the half width (0.8 -> 1.6) and the height (1.8 -> 3.6).
        self.assertAlmostEqual(max(xs) - min(xs), 1.6, places=6)
        self.assertAlmostEqual(max(zs) - min(zs), 3.6, places=6)


class TerrainGridTests(unittest.TestCase):
    def test_grid_vertex_and_face_counts_with_heights(self) -> None:
        cells = [
            TerrainCell(x=0.0, z=0.0, height=0.0),
            TerrainCell(x=6.0, z=0.0, height=1.0),
            TerrainCell(x=0.0, z=6.0, height=2.0),
            TerrainCell(x=6.0, z=6.0, height=3.0),
        ]
        vertices, faces = terrain_grid(cells)
        self.assertEqual(len(vertices), 4)
        self.assertEqual(len(faces), 1)
        # Y is the height of the highest cell.
        ys = [vertex[1] for vertex in vertices]
        self.assertEqual(max(ys), 3.0)

    def test_empty_cells_produce_empty_geometry(self) -> None:
        self.assertEqual(terrain_grid([]), ([], []))


class BuildIrTerrainVegetationParseTests(unittest.TestCase):
    def test_parse_terrain_and_vegetation_from_dict(self) -> None:
        data = {
            "build_ir_version": 1,
            "compiler_version": "track-build-0.1.0",
            "source_project_hash": "c" * 64,
            "terrain_cell_m": 6.0,
            "terrain_heightfield": [
                {"x": 0.0, "z": 0.0, "height": 1.5},
            ],
            "vegetation_instances": [
                {
                    "instance_id": "trees_000_0",
                    "asset_id": "tree_v2_01",
                    "position": [2.0, 3.0],
                    "yaw_rad": 0.5,
                    "scale": 1.1,
                }
            ],
        }
        loaded = BuildIR.from_dict(data)
        self.assertEqual(loaded.terrain_cell_m, 6.0)
        self.assertEqual(loaded.terrain_heightfield[0].height, 1.5)
        self.assertEqual(loaded.vegetation_instances[0].asset_id, "tree_v2_01")
        self.assertEqual(loaded.vegetation_instances[0].position, (2.0, 3.0))


if __name__ == "__main__":
    unittest.main()
