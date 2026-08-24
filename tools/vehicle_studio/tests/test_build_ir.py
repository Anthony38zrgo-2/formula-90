from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from vehicle_studio.build_ir import BuildIRError, build_ir_bytes, compile_build_ir, validate_build_ir

FIXTURE = ROOT / "fixtures" / "vehicle_document_valid_minimal.json"


def document():
    data = json.loads(FIXTURE.read_text(encoding="utf-8"))
    data["parameters"] = [{
        "parameter_id": "parameter-wheelbase",
        "semantic_role": "wheelbase",
        "unit": "meter",
        "minimum": 2.4,
        "maximum": 3.4,
        "baseline_value": 2.9,
        "absolute_value": 2.9,
        "dependency_ids": [],
        "build_operations": [{
            "kind": "translate_axle_frames",
            "inputs": {"ownership": "front_and_rear_axles"},
            "postconditions": ["wheelbase_matches", "wheel_anchors_match"],
        }, {
            "kind": "piecewise_body_deform",
            "inputs": {"protected_frames": ["front_wing_mount", "rear_wing_mount"]},
            "postconditions": ["body_continuity_matches", "topology_matches_source"],
        }],
    }]
    return data


class BuildIRTests(unittest.TestCase):
    def test_compiler_emits_explicit_ordered_operations_and_postconditions(self):
        result = compile_build_ir(document(), parameter_values={"parameter-wheelbase": 3.1})
        self.assertEqual([item["order"] for item in result["operations"]], [10, 100, 110, 900, 990])
        self.assertEqual(result["operations"][1]["kind"], "translate_axle_frames")
        self.assertEqual(result["operations"][-1]["inputs"]["formats"], ["blend", "glb"])
        self.assertFalse(result["operations"][-1]["inputs"]["publish"])

    def test_canonical_bytes_ignore_override_dict_order(self):
        first = compile_build_ir(document(), parameter_values={"parameter-wheelbase": 3.1})
        second = compile_build_ir(deepcopy(document()), parameter_values={"parameter-wheelbase": 3.1})
        self.assertEqual(build_ir_bytes(first), build_ir_bytes(second))

    def test_unknown_and_out_of_bounds_values_are_rejected(self):
        with self.assertRaises(BuildIRError) as unknown:
            compile_build_ir(document(), parameter_values={"parameter-unknown": 1})
        self.assertEqual(unknown.exception.code, "BUILD_IR_UNKNOWN_PARAMETER")
        with self.assertRaises(BuildIRError) as bounds:
            compile_build_ir(document(), parameter_values={"parameter-wheelbase": 8})
        self.assertEqual(bounds.exception.code, "BUILD_IR_PARAMETER_BOUNDS")

    def test_dependency_must_precede_consumer(self):
        result = compile_build_ir(document())
        result["operations"][1]["depends_on"] = ["op-0990-stage-export"]
        with self.assertRaises(BuildIRError) as raised:
            validate_build_ir(result)
        self.assertEqual(raised.exception.code, "BUILD_IR_DEPENDENCY_ORDER")


if __name__ == "__main__":
    unittest.main()
