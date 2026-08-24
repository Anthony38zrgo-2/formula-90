from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from safety_barrier_layout import barrier_envelope_at


ROOT = Path(__file__).resolve().parents[3]


class TracksideSafetyIntegrationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest = json.loads((
            ROOT / "blender/track_pipeline/manifests/la_chutana_safety_barriers.json"
        ).read_text(encoding="utf-8"))
        cls.lap_length = 2420.0

    def test_chicane_sequence_resolves_actual_prototype(self):
        first = barrier_envelope_at(self.manifest, .53, 1, self.lap_length)
        second = barrier_envelope_at(self.manifest, .56, 1, self.lap_length)
        self.assertEqual(first["prototype_id"], "tire_navy_single")
        self.assertEqual(second["prototype_id"], "plastic")
        self.assertAlmostEqual(first["outer_face_distance_m"], 14.325)
        self.assertAlmostEqual(second["outer_face_distance_m"], 14.6)

    def test_sector_distances_are_not_legacy_constant(self):
        t2 = barrier_envelope_at(self.manifest, .25, 1, self.lap_length)
        t4 = barrier_envelope_at(self.manifest, .40, 1, self.lap_length)
        self.assertAlmostEqual(t2["outer_face_distance_m"], 11.15)
        self.assertAlmostEqual(t4["outer_face_distance_m"], 13.6)
        self.assertNotEqual(t2["outer_face_distance_m"], t4["outer_face_distance_m"])


if __name__ == "__main__":
    unittest.main()
