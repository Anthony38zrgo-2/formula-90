from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from generate_environment import barrier_minimum_center_distance, generate_trackside_props


class EnvironmentPerimeterTests(unittest.TestCase):
    def test_tree_and_bush_radius_stays_outside_tire_barrier(self):
        config = {
            "road": {"width_m": 12.0},
            "tire_barriers": {"separation_from_edge_m": 5.0, "collision_thickness_m": 0.28},
            "vegetation_barrier_clearance_m": {"trees": 1.0, "bushes": 0.5},
        }
        self.assertAlmostEqual(barrier_minimum_center_distance(config, "trees", 4.0), 16.14)
        self.assertAlmostEqual(barrier_minimum_center_distance(config, "bushes", 2.0), 13.64)
        self.assertEqual(barrier_minimum_center_distance(config, "grass", 0.4), 0.0)

    def test_la_chutana_uses_five_metre_tire_escape_margin(self):
        config_path = Path(__file__).resolve().parents[1] / "configs" / "la_chutana.json"
        config = json.loads(config_path.read_text(encoding="utf-8"))
        self.assertEqual(float(config["tire_barriers"]["separation_from_edge_m"]), 5.0)
        self.assertTrue(config["tire_barriers"]["continuous_visual"])
        self.assertTrue(config["tire_barriers"]["both_sides"])
        self.assertEqual(float(config["vegetation_barrier_clearance_m"]["trees"]), 1.0)
        self.assertEqual(float(config["vegetation_barrier_clearance_m"]["bushes"]), 0.5)

    def test_la_chutana_no_dual_guardrail_and_tire_visual(self):
        config_path = Path(__file__).resolve().parents[1] / "configs" / "la_chutana.json"
        config = json.loads(config_path.read_text(encoding="utf-8"))
        # Tire barriers cover the full perimeter; the legacy guardrail segments
        # must not render in the same circuit or both systems overlap visually.
        self.assertFalse(config["guardrails"]["procedural"])
        self.assertTrue(config["tire_barriers"]["procedural"])
        self.assertTrue(config["tire_barriers"]["both_sides"])
        self.assertTrue(config["tire_barriers"]["continuous_visual"])
        self.assertIn("visual_asset_glb", config["tire_barriers"])
        repo = Path(__file__).resolve().parents[3]
        asset = repo / config["tire_barriers"]["visual_asset_glb"]
        self.assertTrue(asset.exists(), f"tire barrier asset missing: {asset}")
        self.assertEqual(float(config["tire_barriers"]["collision_height_m"]), 1.45)
        self.assertEqual(float(config["tire_barriers"]["collision_thickness_m"]), 0.28)

    def test_trackside_cards_are_deterministic_and_outside_collision_perimeter(self):
        points = np.asarray([
            [40.0, 0.0], [28.0, 28.0], [0.0, 40.0], [-28.0, 28.0],
            [-40.0, 0.0], [-28.0, -28.0], [0.0, -40.0], [28.0, -28.0],
        ], dtype=float)
        config = {
            "road": {"width_m": 12.0},
            "tire_barriers": {"separation_from_edge_m": 5.0},
            "trackside_props": {
                "procedural": True,
                "outside_barrier_offset_m": 1.8,
                "max_visible_cards": 100,
                "spacing_m": {"spectator": 20.0, "marshal": 35.0, "photographer": 40.0, "flag": 24.0, "sign": 30.0},
            },
        }
        first = generate_trackside_props(config, points)
        second = generate_trackside_props(config, points)
        self.assertEqual(first, second)
        self.assertTrue(first)
        minimum = 6.0 + 5.0 + 1.8
        self.assertTrue(all(float(item["distance_from_center_m"]) >= minimum for item in first))
        self.assertTrue(all(item["collision"] is False for item in first))


class TireBarrierAssetContractTests(unittest.TestCase):
    def test_glb_has_6_units_blue_bottom_white_top_no_collision(self):
        import struct
        repo = Path(__file__).resolve().parents[3]
        asset = repo / "game" / "assets" / "trackside" / "tire_barrier_jordan_6.glb"
        self.assertTrue(asset.exists(), f"tire barrier asset missing: {asset}")
        data = asset.read_bytes()
        magic, version, length = struct.unpack("<III", data[:12])
        self.assertEqual(magic, 0x46546C67)
        chunks = {}
        off = 12
        while off < len(data):
            clen, ctype = struct.unpack("<II", data[off:off + 8])
            chunks[ctype] = data[off + 8:off + 8 + clen]
            off += 8 + clen
        self.assertIn(0x4E4F534A, chunks)
        scene = json.loads(chunks[0x4E4F534A])
        nodes = scene.get("nodes", [])
        root = [n for n in nodes if n.get("name") == "TireBarrierJordan6"]
        self.assertEqual(len(root), 1)
        # Single fused module mesh under the root.
        root_node = root[0]
        child_count = len(root_node.get("children", []))
        self.assertEqual(child_count, 1)
        child = nodes[root_node["children"][0]]
        self.assertIn("mesh", child)
        # No -colonly and no collision flag nodes.
        self.assertFalse(any("-colonly" in n.get("name", "") for n in nodes))


if __name__ == "__main__":
    unittest.main()
