from __future__ import annotations

from hashlib import sha256
import json
from pathlib import Path
import struct
import unittest


REPO = Path(__file__).resolve().parents[3]
VARIANT = REPO / "game/assets/models/vehicles/f1_94/variants/f1_2009_fw31"


def glb_json(path: Path) -> dict:
    payload = path.read_bytes()
    json_length, json_kind = struct.unpack_from("<II", payload, 12)
    if json_kind != 0x4E4F534A:
        raise ValueError("first GLB chunk is not JSON")
    return json.loads(payload[20:20 + json_length].decode("utf-8").rstrip("\x00 "))


class RuntimeVariant2009Tests(unittest.TestCase):
    def test_manifest_assets_and_physics_hashes_match(self):
        manifest = json.loads((VARIANT / "manifest.json").read_text(encoding="utf-8"))
        self.assertTrue(manifest["canonical"])
        self.assertEqual(manifest["classification"], "canonical_vehicle")
        runtime = manifest["runtime"]
        for key in ("chassis", "wheel_front", "wheel_rear"):
            path = VARIANT / runtime[key]["path"]
            self.assertTrue(path.is_file())
            self.assertEqual(sha256(path.read_bytes()).hexdigest().upper(), runtime[key]["sha256"])
        physics = REPO / "game" / runtime["physics_profile"].removeprefix("res://")
        self.assertEqual(sha256(physics.read_bytes()).hexdigest().upper(), runtime["physics_sha256"])
        blend = REPO / manifest["authoring"]["blend"]["path"]
        self.assertEqual(
            sha256(blend.read_bytes()).hexdigest().upper(),
            manifest["authoring"]["blend"]["sha256"],
        )

    def test_every_glb_primitive_has_uv_and_embedded_albedo(self):
        manifest = json.loads((VARIANT / "manifest.json").read_text(encoding="utf-8"))
        for key in ("chassis", "wheel_front", "wheel_rear"):
            document = glb_json(VARIANT / manifest["runtime"][key]["path"])
            materials = document.get("materials", [])
            primitives = [
                primitive
                for mesh in document.get("meshes", [])
                for primitive in mesh.get("primitives", [])
            ]
            self.assertTrue(document.get("images"), key)
            self.assertTrue(document.get("textures"), key)
            self.assertTrue(all("TEXCOORD_0" in item.get("attributes", {}) for item in primitives), key)
            self.assertTrue(all("material" in item for item in primitives), key)
            self.assertTrue(all(
                "baseColorTexture" in material.get("pbrMetallicRoughness", {})
                for material in materials
            ), key)
            import_config = (VARIANT / manifest["runtime"][key]["path"]).with_suffix(".glb.import")
            self.assertIn("gltf/embedded_image_handling=2", import_config.read_text(encoding="utf-8"))

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

    def test_scene_and_launcher_promote_2009_and_preserve_1994(self):
        scene = (REPO / "game/scenes/vehicles/f1_94/f1_2009_fw31_rust.tscn").read_text(encoding="utf-8")
        session = (REPO / "game/scenes/runtime/vehicle_test_session_2009.tscn").read_text(encoding="utf-8")
        launcher = (REPO / "scripts/run_f1_94.ps1").read_text(encoding="utf-8-sig")
        self.assertIn('physics_config_path = "res://data/vehicles/f1_94/variants/f1_2009_fw31_physics.json"', scene)
        self.assertIn('position = Vector3(-0.585, 0.235125, -1.55)', scene)
        self.assertIn('position = Vector3(0.865, 0.235125, -1.55)', scene)
        self.assertIn('config_json_path = "res://data/vehicles/f1_94/variants/f1_2009_fw31_physics.json"', session)
        default_session = (REPO / "game/data/race_sessions/default.tres").read_text(encoding="utf-8")
        self.assertIn("[string]$VehicleVariant = '2009'", launcher)
        self.assertIn("$VehicleVariant -eq '2009'", launcher)
        self.assertIn("vehicle_test_session_2009.tscn", launcher)
        self.assertIn("res://data/vehicles/f1_2009_fw31.tres", default_session)
        self.assertIn("F1 1994 legacy", launcher)
        self.assertNotIn("'2021'", launcher)


if __name__ == "__main__":
    unittest.main()
