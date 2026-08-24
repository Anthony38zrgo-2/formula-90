from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender/track_pipeline"))

from safety_barrier_layout import coverage_gaps_by_side, overlaps_by_side, validate_manifest  # noqa: E402


class FullBarrierMigrationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.config = json.loads((ROOT / "blender/track_pipeline/configs/la_chutana.json").read_text(encoding="utf-8"))
        cls.layout = json.loads((ROOT / "blender/track_pipeline/manifests/la_chutana_safety_barriers.json").read_text(encoding="utf-8"))
        cls.assets = json.loads((ROOT / "game/resources/environment/assets/barriers/barrier_manifest.json").read_text(encoding="utf-8"))

    def test_full_circuit_has_one_authority(self) -> None:
        self.assertEqual(self.layout["authority"], "full_circuit_asset_library")
        self.assertEqual(self.config["safety_barriers"]["scope"], "full_circuit")
        self.assertTrue(self.config["safety_barriers"]["legacy_guardrails_disabled"])
        self.assertTrue(self.config["safety_barriers"]["legacy_tire_barriers_disabled"])
        self.assertFalse(self.config["tire_barriers"]["procedural"])

    def test_both_sides_have_exact_coverage(self) -> None:
        validate_manifest(self.layout)
        self.assertEqual(coverage_gaps_by_side(self.layout), {"left": [], "right": []})
        for values in overlaps_by_side(self.layout).values():
            self.assertLessEqual(len(values), len(self.layout["segments"]) // 2)

    def test_every_runtime_prototype_uses_a_known_asset(self) -> None:
        known = {asset["id"] for asset in self.assets["assets"]}
        used = {prototype["asset_id"] for prototype in self.layout["prototypes"].values()}
        self.assertEqual(used, known)
        self.assertNotIn("red", json.dumps(self.layout).lower())


if __name__ == "__main__":
    unittest.main()
