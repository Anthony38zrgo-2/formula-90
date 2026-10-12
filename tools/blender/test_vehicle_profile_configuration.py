import json
from pathlib import Path
import shutil
import tempfile
import unittest

from vehicle_profile_configuration import (
    GEOMETRIC_PROFILE_PATH, REPOSITORY_DIRECTORY,
    load_vehicle_configuration, resolve_vehicle_profile, vehicle_profile_asset_contract,
)


class VehicleProfileConfigurationTests(unittest.TestCase):
    def test_versioned_profile_matches_legacy_geometry_and_tire_dimensions(self):
        resolved = load_vehicle_configuration(GEOMETRIC_PROFILE_PATH)
        legacy = load_vehicle_configuration(REPOSITORY_DIRECTORY / "game/data/vehicles/f1_2030/f1_2030_v10_geometric.json")
        for name, value in legacy["geometry"].items():
            self.assertEqual(resolved["geometry"][name], value)
        self.assertIsNone(resolved["geometry"].get("wheel_hubs"))
        for axle in ["front", "rear"]:
            for dimension in ["radius", "width"]:
                self.assertEqual(resolved["tires"][axle][dimension], legacy["tires"][axle][dimension])

    def test_asset_contract_uses_loader_provenance(self):
        contract = vehicle_profile_asset_contract(GEOMETRIC_PROFILE_PATH)
        resolved = resolve_vehicle_profile(GEOMETRIC_PROFILE_PATH)
        self.assertEqual(contract["physics_sha256"].lower(), resolved["provenance"]["global_digest_sha256"])
        self.assertEqual(contract["physics_digest_contract"], "vehicle_profile_provenance_v1")

    def test_dependency_bytes_change_digest_without_changing_effective_configuration(self):
        with tempfile.TemporaryDirectory(dir=REPOSITORY_DIRECTORY / "scratch") as temporary_directory:
            destination = Path(temporary_directory)
            manifest = json.loads(GEOMETRIC_PROFILE_PATH.read_text())
            for filename in ["vehicle.json", *manifest["domains"].values()]:
                shutil.copy2(GEOMETRIC_PROFILE_PATH.parent / filename, destination / filename)
            initial = resolve_vehicle_profile(destination / "vehicle.json")
            chassis = destination / manifest["domains"]["chassis"]
            chassis.write_bytes(chassis.read_bytes() + b"\n")
            changed = resolve_vehicle_profile(destination / "vehicle.json")
            self.assertEqual(initial["configuration"], changed["configuration"])
            self.assertNotEqual(initial["provenance"]["global_digest_sha256"], changed["provenance"]["global_digest_sha256"])


if __name__ == "__main__":
    unittest.main()
