from __future__ import annotations

import sys
import tempfile
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from vehicle_studio.api_server import APIError, OnboardingController


class OnboardingControllerTests(unittest.TestCase):
    def test_only_allowlisted_preset_can_scan(self):
        controller = OnboardingController(Path.cwd())
        with self.assertRaises(APIError) as raised:
            controller.scan_preset("../../arbitrary")
        self.assertEqual(raised.exception.code, "UNKNOWN_PRESET")

    def test_preview_asset_rejects_client_controlled_identifiers(self):
        controller = OnboardingController(Path.cwd())
        with self.assertRaises(APIError) as raised:
            controller.preview_asset("../../arbitrary.glb")
        self.assertEqual(raised.exception.code, "ASSET_NOT_ALLOWED")

    def test_presets_report_missing_sources_without_scanning(self):
        with tempfile.TemporaryDirectory() as directory:
            result = OnboardingController(Path(directory)).presets()
        self.assertFalse(result["presets"][0]["available"])

    def test_2021_profile_respects_declared_width_envelope(self):
        profile = OnboardingController(Path.cwd()).dimensional_profiles()["profiles"][0]
        targets = profile["targets"]
        self.assertEqual(profile["profile_id"], "f1-2021-nominal")
        self.assertAlmostEqual(
            targets["parameter-front-track"] + targets["parameter-front-tire-width"],
            1.925,
        )
        self.assertAlmostEqual(
            targets["parameter-rear-track"] + targets["parameter-rear-tire-width"],
            1.980,
        )
        self.assertLessEqual(
            profile["constraints"]["computed_rear_overall_width_m"],
            profile["constraints"]["maximum_overall_width_m"],
        )

    def test_merge_reassigns_stable_unique_ids_and_keeps_source(self):
        scans = [
            {"source": {"path": "a.glb"}, "marker": "A"},
            {"source": {"path": "b.glb"}, "marker": "B"},
        ]
        def suggestions(scan):
            return {"suggestions": [{
                "suggestion_id": "suggestion-0001",
                "semantic_kind": "component",
                "proposed_role": "nose",
                "source_kind": "mesh",
                "source_name": f"GEO_NOSE_{scan['marker']}",
                "evidence": [],
            }]}
        with patch("vehicle_studio.api_server.suggest_semantics", side_effect=suggestions):
            result = OnboardingController._merge_suggestions(scans)
        self.assertEqual(
            [item["suggestion_id"] for item in result["suggestions"]],
            ["suggestion-0001", "suggestion-0002"],
        )
        self.assertEqual(
            {item["source_path"] for item in result["suggestions"]},
            {"a.glb", "b.glb"},
        )


if __name__ == "__main__":
    unittest.main()
