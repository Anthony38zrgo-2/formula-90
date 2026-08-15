from __future__ import annotations

import argparse
import json
import sys
import tempfile
import unittest
from pathlib import Path


PIPELINE_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = PIPELINE_DIR.parents[1]
sys.path.insert(0, str(PIPELINE_DIR))

from generate_vehicle_runtime import build  # noqa: E402
from add_gltf_normals import normal_coverage  # noqa: E402
from validate_assembly_equivalence import derive_source_assembly, validate  # noqa: E402


SOURCE = REPO_ROOT / "assets-lowpoly-python/vehicles/canonical/formula_reference_livery_contract_annotated.glb"
DATUMS = REPO_ROOT / "assets-lowpoly-python/vehicles/canonical/formula_reference_livery_contract_annotated_datums.json"


class VehicleAssemblyEquivalenceTests(unittest.TestCase):
    def test_source_snapshot_contains_required_geometric_anchors(self) -> None:
        snapshot, _ = derive_source_assembly(SOURCE)
        anchors = snapshot["anchors_source_m"]
        for name in (
            "DATUM_NOSE",
            "DATUM_TAIL",
            "DATUM_ORIGIN",
            "DATUM_FRONT_AXLE_CENTER",
            "DATUM_REAR_AXLE_CENTER",
            "DATUM_HUB_FL",
            "DATUM_HUB_FR",
            "DATUM_HUB_RL",
            "DATUM_HUB_RR",
            "ANCHOR_CHASSIS_CENTER",
            "ANCHOR_CHASSIS_FLOOR_FRONT",
            "ANCHOR_CHASSIS_FLOOR_REAR",
            "ANCHOR_SUSPENSION_FL",
            "ANCHOR_SUSPENSION_FR",
            "ANCHOR_SUSPENSION_RL",
            "ANCHOR_SUSPENSION_RR",
        ):
            self.assertIn(name, anchors)

    def test_validator_rejects_independent_wheel_displacement(self) -> None:
        staging_root = REPO_ROOT / ".codex-staging"
        staging_root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="assembly_equivalence_test_", dir=staging_root) as temporary:
            output = Path(temporary) / "jordan_197"
            args = argparse.Namespace(
                source=SOURCE,
                datums=DATUMS,
                output_dir=output,
                candidate_id="jordan_197",
                wheel_prefix=None,
                datum_tolerance=0.002,
                cluster_tolerance=0.15,
                assembly_tolerance_mm=2.0,
            )
            manifest = build(args)
            self.assertEqual("PASS", manifest["assembly_equivalence"]["status"])
            for asset_name in (
                "jordan_197_chassis.glb",
                "jordan_197_wheel_fl.glb",
                "jordan_197_wheel_fr.glb",
                "jordan_197_wheel_rl.glb",
                "jordan_197_wheel_rr.glb",
            ):
                coverage = normal_coverage(output / asset_name)
                self.assertEqual(0, coverage["missing_normals"], asset_name)

            shifted = json.loads((output / "vehicle_runtime_manifest.json").read_text(encoding="utf-8"))
            shifted["wheel_hubs_runtime_m"]["FL"][0] += 0.010
            shifted_path = output / "vehicle_runtime_manifest_shifted_fl.json"
            shifted_path.write_text(json.dumps(shifted, indent=2) + "\n", encoding="utf-8")

            report = validate(SOURCE, output, shifted_path, 2.0, None)
            self.assertEqual("FAIL", report["status"])
            self.assertGreater(report["anchor_errors"]["DATUM_HUB_FL"]["error_mm"], 9.9)
            self.assertGreater(report["maxima"]["node_geometry_error_mm"], 9.9)


if __name__ == "__main__":
    unittest.main()
