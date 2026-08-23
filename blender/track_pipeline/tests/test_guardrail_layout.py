from __future__ import annotations

import json
import unittest
from pathlib import Path

from guardrail_layout import guardrail_conflict, load_guardrail_layout


ROOT = Path(__file__).resolve().parents[3]


class GuardrailLayoutTests(unittest.TestCase):
    def setUp(self) -> None:
        self.config = json.loads((ROOT / "blender/track_pipeline/configs/la_chutana.json").read_text(encoding="utf-8"))
        self.layout = load_guardrail_layout(ROOT, self.config)

    def test_layout_has_expected_risk_segments(self) -> None:
        self.assertEqual(len(self.layout["segments"]), 7)
        self.assertEqual({int(item["rail_count"]) for item in self.layout["segments"]}, {2, 3})

    def test_detects_same_side_vegetation_conflict(self) -> None:
        self.assertTrue(guardrail_conflict(self.layout, "trees", 0.10, -1, 14.0, 2.0))
        self.assertFalse(guardrail_conflict(self.layout, "trees", 0.10, 1, 14.0, 2.0))


if __name__ == "__main__":
    unittest.main()
