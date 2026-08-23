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

    def test_loads_schema_v2_and_compiles_current_chicane(self):
        compiled = compile_layout(self.manifest, 2420.0)
        self.assertEqual({item["id"] for item in compiled["segments"]}, {"ChicaneLeft", "ChicaneRight"})

    def test_rejects_unknown_schema(self):
        bad = copy.deepcopy(self.manifest)
        bad["schema_version"] = 3
        with self.assertRaises(SafetyBarrierLayoutError):
            validate_manifest(bad)

    def test_rejects_duplicate_ids_unknown_prototype_and_bad_side(self):
        for mutate in (
            lambda m: m["segments"].append(copy.deepcopy(m["segments"][0])),
            lambda m: m["segments"][0].update(side="inside"),
            lambda m: m["segments"][0]["system"]["items"][0].update(type="missing"),
        ):
            bad = copy.deepcopy(self.manifest)
            mutate(bad)
            with self.assertRaises(SafetyBarrierLayoutError):
                validate_manifest(bad)

    def test_rejects_incompatible_terminal(self):
        bad = copy.deepcopy(self.manifest)
        bad["segments"][0]["terminal_start"] = "jersey_end"
        with self.assertRaises(SafetyBarrierLayoutError):
            validate_manifest(bad)

    def test_rejects_undeclared_sequence_gap(self):
        bad = copy.deepcopy(self.manifest)
        bad["segments"][0]["system"]["items"] = [{"type": "tire_stack", "length_m": 10.0}]
        with self.assertRaises(SafetyBarrierLayoutError):
            compile_layout(bad, 2420.0)

    def test_sequence_repeat_false_and_module_adjustment(self):
        compiled = compile_layout(self.manifest, 2420.0)
        t1 = [m for m in compiled["modules"] if m["segment_id"] == "ChicaneLeft"]
        self.assertEqual(t1[0]["type"], "tecpro")
        self.assertIn("tire_stack", {m["type"] for m in t1})
        self.assertAlmostEqual(sum(m["length_m"] for m in t1), 266.2, places=3)

    def test_repeat_true_fills_span(self):
        manifest = copy.deepcopy(self.manifest)
        segment = manifest["segments"][0]
        segment["system"] = {"mode": "sequence", "repeat": True, "items": [
            {"type": "tire_stack", "length_m": 10.0},
            {"type": "tecpro", "length_m": 5.0},
        ]}
        compiled = compile_layout(manifest, 2420.0)
        result = next(s for s in compiled["segments"] if s["id"] == "ChicaneLeft")
        self.assertAlmostEqual(result["compiled_length_m"], result["span_m"], places=3)

    def test_chicane_corridors_cover_both_sides(self):
        self.assertTrue(barrier_conflict(self.manifest, "trees", .57, -1, 14.0, 1.0))
        self.assertTrue(barrier_conflict(self.manifest, "trees", .57, 1, 14.0, 1.0))

    def test_chicane_manifest_is_not_misreported_as_full_perimeter(self):
        self.assertGreater(len(exterior_coverage_gaps(self.manifest)), 0)


if __name__ == "__main__":
    unittest.main()
