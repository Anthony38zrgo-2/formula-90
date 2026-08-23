from __future__ import annotations

import json
import unittest
from copy import deepcopy
from pathlib import Path

from curb_manifest import CurbManifestError, material_for_longitudinal, validate_curb_assignments, validate_curb_manifest


ROOT = Path(__file__).resolve().parents[1]


class CurbManifestTests(unittest.TestCase):
    def setUp(self) -> None:
        self.manifest = json.loads((ROOT / "manifests" / "curb_profiles.json").read_text(encoding="utf-8"))
        self.config = json.loads((ROOT / "configs" / "la_chutana.json").read_text(encoding="utf-8"))

    def test_catalog_and_track_assignments_are_valid(self) -> None:
        validate_curb_manifest(self.manifest)
        validate_curb_assignments(self.config["curb"]["segments"], self.manifest)

    def test_all_profiles_are_thirty_percent_wider(self) -> None:
        for profile in self.manifest["profiles"].values():
            self.assertAlmostEqual(float(profile["width_m"]), 1.508)
            self.assertAlmostEqual(float(profile["points"][-1][0]), 1.508)

    def test_pattern_alternates_transversely_along_curb(self) -> None:
        pattern = self.manifest["patterns"]["white_navy_alternating"]
        self.assertEqual(material_for_longitudinal(pattern, 0.5), "white")
        self.assertEqual(material_for_longitudinal(pattern, 2.5), "navy")
        self.assertEqual(material_for_longitudinal(pattern, 4.5), "white")

    def test_rejects_same_side_overlap(self) -> None:
        segments = deepcopy(self.config["curb"]["segments"])
        segments[1]["side"] = segments[0]["side"]
        segments[1]["start_fraction"] = segments[0]["start_fraction"]
        with self.assertRaises(CurbManifestError):
            validate_curb_assignments(segments, self.manifest)


if __name__ == "__main__":
    unittest.main()
