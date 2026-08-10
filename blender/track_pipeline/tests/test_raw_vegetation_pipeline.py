import json
import unittest
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender" / "track_pipeline"))
from raw_vegetation_common import derive_raw_placements, outer_side


class RawVegetationPipelineTests(unittest.TestCase):
    def test_outer_side_is_deterministic(self):
        self.assertEqual(outer_side([(0, 0), (1, 0), (1, 1), (0, 1)]), 1)
        self.assertEqual(outer_side([(0, 0), (0, 1), (1, 1), (1, 0)]), -1)

    def test_raw_zones_and_props(self):
        root = Path(__file__).resolve().parents[3]
        config = json.loads((root / "blender/track_pipeline/configs/la_chutana_raw.json").read_text())
        center = json.loads((root / "blender/generated/la_chutana/centerline.json").read_text())
        placements = json.loads((root / "blender/generated/la_chutana/placements.json").read_text())
        result = derive_raw_placements({"road_width_m": 12.0, **config}, center, placements, lambda points, fraction: ((0.0, 0.0), (1.0, 0.0), (0.0, -1.0)))
        self.assertTrue(result["placements"])
        self.assertTrue(all(p["category"] != "grass" or p["distance_to_track_m"] <= 23.5 for p in result["placements"]))
        self.assertTrue(all(p["side"] == result["outer_side"] for p in result["trackside_props"]))
        self.assertTrue(all(p["collision"] is False for p in result["trackside_props"]))


if __name__ == "__main__":
    unittest.main()
