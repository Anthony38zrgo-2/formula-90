import copy
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender" / "track_pipeline"))

from safety_barrier_layout import (SafetyBarrierLayoutError, barrier_conflict,
                                   compile_layout, exterior_coverage_gaps, load_safety_barrier_layout,
                                   validate_manifest)


class SafetyBarrierLayoutTests(unittest.TestCase):
    def setUp(self):
        config = json.loads((ROOT / "blender/track_pipeline/configs/la_chutana.json").read_text())
        config["safety_barriers"] = {"manifest": "blender/track_pipeline/manifests/la_chutana_safety_barriers.json"}
        self.manifest = load_safety_barrier_layout(ROOT, config)

    def test_loads_schema_v2_and_wraps_main_straight(self):
        compiled = compile_layout(self.manifest, 2420.0)
        pit = next(item for item in compiled["segments"] if item["id"] == "MainStraightPitWall")
        self.assertAlmostEqual(pit["span_m"], 338.8, places=3)

    def test_rejects_unknown_schema(self):
        bad = copy.deepcopy(self.manifest)
        bad["schema_version"] = 3
        with self.assertRaises(SafetyBarrierLayoutError):
            validate_manifest(bad)

    def test_rejects_duplicate_ids_unknown_prototype_and_bad_side(self):
        for mutate in (
            lambda m: m["segments"].append(copy.deepcopy(m["segments"][0])),
            lambda m: m["segments"][0].update(side="inside"),
            lambda m: m["segments"][0]["system"].update(type="missing"),
        ):
            bad = copy.deepcopy(self.manifest)
            mutate(bad)
            with self.assertRaises(SafetyBarrierLayoutError):
                validate_manifest(bad)

    def test_rejects_incompatible_terminal(self):
        bad = copy.deepcopy(self.manifest)
        bad["segments"][3]["terminal_start"] = "jersey_end"
        with self.assertRaises(SafetyBarrierLayoutError):
            validate_manifest(bad)

    def test_rejects_undeclared_sequence_gap(self):
        bad = copy.deepcopy(self.manifest)
        bad["segments"][2]["system"]["items"] = [{"type": "tire_stack", "length_m": 10.0}]
        with self.assertRaises(SafetyBarrierLayoutError):
            compile_layout(bad, 2420.0)

    def test_sequence_repeat_false_and_module_adjustment(self):
        compiled = compile_layout(self.manifest, 2420.0)
        t1 = [m for m in compiled["modules"] if m["segment_id"] == "T1Outer"]
        self.assertEqual(t1[0]["type"], "tire_stack")
        self.assertIn("tecpro", {m["type"] for m in t1})
        self.assertAlmostEqual(sum(m["length_m"] for m in t1), 326.7, places=3)

    def test_repeat_true_fills_span(self):
        manifest = copy.deepcopy(self.manifest)
        segment = manifest["segments"][2]
        segment["system"] = {"mode": "sequence", "repeat": True, "items": [
            {"type": "tire_stack", "length_m": 10.0},
            {"type": "tecpro", "length_m": 5.0},
        ]}
        compiled = compile_layout(manifest, 2420.0)
        result = next(s for s in compiled["segments"] if s["id"] == "T1Outer")
        self.assertAlmostEqual(result["compiled_length_m"], result["span_m"], places=3)

    def test_corridor_conflict_respects_side(self):
        self.assertTrue(barrier_conflict(self.manifest, "trees", .20, 1, 13.0, 1.0))
        self.assertFalse(barrier_conflict(self.manifest, "trees", .20, -1, 13.0, 1.0))

    def test_exterior_route_has_no_unprotected_fraction(self):
        self.assertEqual(exterior_coverage_gaps(self.manifest), [])


if __name__ == "__main__":
    unittest.main()
