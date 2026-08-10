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


if __name__ == "__main__":
    unittest.main()
