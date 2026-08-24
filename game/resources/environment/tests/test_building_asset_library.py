from __future__ import annotations

import hashlib
import json
import unittest
from pathlib import Path

import numpy as np
import trimesh


ROOT = Path(__file__).resolve().parents[4]


class BuildingAssetLibraryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest_path = ROOT / "game/resources/environment/assets/buildings/building_manifest.json"
        cls.manifest = json.loads(cls.manifest_path.read_text(encoding="utf-8"))

    def test_manifest_and_catalog(self) -> None:
        self.assertEqual(self.manifest["schema_version"], 1)
        self.assertEqual(self.manifest["generator"], "formula90_building_manifest_v1")
        self.assertEqual(len(self.manifest["assets"]), 2)
        catalog = ROOT / self.manifest["human_gate_catalog"]
        self.assertEqual(hashlib.sha256(catalog.read_bytes()).hexdigest(),
                         self.manifest["human_gate_catalog_sha256"])

    def test_assets_are_grounded_finite_and_within_budget(self) -> None:
        for asset in self.manifest["assets"]:
            path = ROOT / asset["glb"]
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), asset["glb_sha256"])
            scene = trimesh.load(path, force="scene")
            self.assertTrue(np.isfinite(scene.bounds).all(), asset["id"])
            self.assertAlmostEqual(float(scene.bounds[0][1]), 0.0, places=3)
            self.assertLessEqual(asset["geometry"]["triangles"], asset["triangle_budget"])
            self.assertTrue(asset["geometry"]["winding_consistent"])
            self.assertFalse(asset["collision"])
            self.assertTrue((ROOT / asset["preview"]).is_file())


if __name__ == "__main__":
    unittest.main()
