import unittest
from pathlib import Path
import sys

import numpy as np

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender" / "track_pipeline"))

from semantic_layout_common import WorldRasterTransform, _asset_repo_path, extract_markers, marker_rgb, nearest_centerline


class SemanticLayoutTests(unittest.TestCase):
    def test_asset_paths_accept_v2_repo_paths_without_legacy_prefix(self):
        path = "blender/generated/la_chutana/raw_vegetation/assets_v2/glb/tree_v2_01.glb"
        self.assertEqual(_asset_repo_path(path), path)
        self.assertEqual(_asset_repo_path("trees/example.glb"), "blender/assets/vegetation/trees/example.glb")

    def test_world_pixel_round_trip(self):
        transform = WorldRasterTransform(101, 201, -10.0, -20.0, 40.0, 80.0)
        for point in ((-10.0, -20.0), (40.0, 80.0), (15.0, 30.0)):
            pixel = transform.world_to_pixel(*point)
            resolved = transform.pixel_to_world(*pixel)
            self.assertAlmostEqual(point[0], resolved[0], delta=0.51)
            self.assertAlmostEqual(point[1], resolved[1], delta=0.51)

    def test_marker_indices_are_exact_not_ocr(self):
        rgba = np.zeros((32, 32, 4), dtype=np.uint8)
        rgba[4:9, 5:10, :3] = marker_rgb(2)
        rgba[4:9, 5:10, 3] = 255
        rgba[20:25, 21:26, :3] = marker_rgb(7)
        rgba[20:25, 21:26, 3] = 255
        catalog = {"002": {}, "007": {}}
        markers = extract_markers(rgba, catalog)
        self.assertEqual([item["catalog_index"] for item in markers], ["002", "007"])
        self.assertEqual([item["area_px"] for item in markers], [25, 25])

    def test_nearest_centerline_reports_side_and_fraction(self):
        points = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        result = nearest_centerline((5.0, -3.0), points)
        self.assertEqual(result["side"], 1)
        self.assertAlmostEqual(result["distance_from_center_m"], 3.0)
        self.assertAlmostEqual(result["track_fraction"], 0.125)


if __name__ == "__main__":
    unittest.main()
