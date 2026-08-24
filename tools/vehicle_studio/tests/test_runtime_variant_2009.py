from __future__ import annotations

from hashlib import sha256
import json
from pathlib import Path
import unittest


REPO = Path(__file__).resolve().parents[3]
VARIANT = REPO / "game/assets/models/vehicles/f1_94/variants/f1_2009_fw31"


class RuntimeVariant2009Tests(unittest.TestCase):
    def test_manifest_assets_and_physics_hashes_match(self):
        manifest = json.loads((VARIANT / "manifest.json").read_text(encoding="utf-8"))
        runtime = manifest["runtime"]
        for key in ("chassis", "wheel_front", "wheel_rear"):
            path = VARIANT / runtime[key]["path"]
            self.assertTrue(path.is_file())
            self.assertEqual(sha256(path.read_bytes()).hexdigest().upper(), runtime[key]["sha256"])
        physics = REPO / "game" / runtime["physics_profile"].removeprefix("res://")
        self.assertEqual(sha256(physics.read_bytes()).hexdigest().upper(), runtime["physics_sha256"])

    def test_physics_dimensions_match_fw31_profile(self):
        manifest = json.loads((VARIANT / "manifest.json").read_text(encoding="utf-8"))
        physics = json.loads((
            REPO / "game/data/vehicles/f1_94/variants/f1_2009_fw31_physics.json"
        ).read_text(encoding="utf-8"))
        self.assertEqual(physics["geometry"]["wheelbase"], 3.1)
        self.assertEqual(physics["geometry"]["front_track"], 1.45)
        self.assertEqual(physics["geometry"]["rear_track"], 1.425)
        self.assertEqual(physics["tires"]["front"]["radius"], 0.330)
        self.assertEqual(physics["tires"]["front"]["width"], 0.350)
        self.assertEqual(physics["tires"]["rear"]["radius"], 0.330)
        self.assertEqual(manifest["geometry"]["front_tire_radius_m"], 0.330)
        self.assertEqual(manifest["geometry"]["front_tire_width_m"], 0.350)
        self.assertEqual(manifest["geometry"]["front_overall_width_m"], 1.8)
        self.assertEqual(manifest["geometry"]["rear_overall_width_m"], 1.8)
        self.assertEqual(physics["tires"]["rear"]["width"], 0.375)

    def test_scene_and_launcher_select_variant_without_changing_default(self):
        scene = (REPO / "game/scenes/vehicles/f1_94/f1_2009_fw31_rust.tscn").read_text(encoding="utf-8")
        session = (REPO / "game/scenes/runtime/vehicle_test_session_2009.tscn").read_text(encoding="utf-8")
        launcher = (REPO / "scripts/run_f1_94.ps1").read_text(encoding="utf-8-sig")
        self.assertIn('physics_config_path = "res://data/vehicles/f1_94/variants/f1_2009_fw31_physics.json"', scene)
        self.assertIn('position = Vector3(-0.585, 0.235125, -1.55)', scene)
        self.assertIn('position = Vector3(0.865, 0.235125, -1.55)', scene)
        self.assertIn('config_json_path = "res://data/vehicles/f1_94/variants/f1_2009_fw31_physics.json"', session)
        self.assertIn("[string]$VehicleVariant = '1994'", launcher)
        self.assertIn("$VehicleVariant -eq '2009'", launcher)
        self.assertIn("vehicle_test_session_2009.tscn", launcher)
        self.assertNotIn("'2021'", launcher)


if __name__ == "__main__":
    unittest.main()
