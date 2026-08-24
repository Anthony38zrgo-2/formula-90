from __future__ import annotations

import hashlib
import json
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class LaChutanaSurfaceInvariantTests(unittest.TestCase):
    def setUp(self):
        self.config = json.loads((ROOT / "blender/track_pipeline/configs/la_chutana.json").read_text())

    def test_authored_asphalt_and_curb_catalog_are_frozen(self):
        self.assertEqual(self.config["materials"]["asphalt_source_sha256"],
                         "506a0a3b99e1d62144cd64836afaed3bd614cc0d3abfe928159541214269d88e")
        self.assertEqual(sha256(ROOT / self.config["curb"]["manifest"]),
                         "029b9044df15fefdbf9d05c50c60f148b81c9e9a2a5e6b32616a44d6bde3483b")

    def test_terrain_and_curb_graphics_safety_parameters_are_frozen(self):
        expected = {
            "grid_cell_m": 6.0, "collision_underlay_drop_m": 0.12,
            "collision_underlay_blend_m": 1.0, "visual_under_road_drop_m": 0.22,
            "visual_under_road_blend_m": 1.4, "visual_edge_blend_m": 1.25,
            "visual_sink_m": 0.003, "roadside_collision_width_m": 2.0,
        }
        self.assertEqual({key: self.config["terrain"][key] for key in expected}, expected)
        self.assertEqual(self.config["road"]["surface_elevation_m"], 0.025)
        self.assertEqual(self.config["terrain_ground_cover"]["base_source_sha256"],
                         "e5d6009f035b3d9b003962e87c2019c7e240b72a8c36e795a91389f4a933a8ab")

    def test_canonical_svg_remains_environment_authority(self):
        semantic = self.config["semantic_environment"]
        self.assertTrue(semantic["enabled"])
        root = ET.parse(ROOT / semantic["source_svg"]).getroot()
        instances = [node for node in root.iter() if node.get("data-role") == "asset-instance"]
        counts = {
            category: sum(node.get("data-category") == category for node in instances)
            for category in ("trees", "bushes", "grass")
        }
        self.assertEqual(counts, {"trees": 130, "bushes": 110, "grass": 9320})
        self.assertEqual(sum(node.get("data-category") is None for node in instances), 124)
        self.assertTrue(self.config["procedural_environment"]["grass_cards"]["enabled"])


if __name__ == "__main__":
    unittest.main()
