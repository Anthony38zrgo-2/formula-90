from __future__ import annotations

from copy import deepcopy
import json
import math
from pathlib import Path
import sys
import unittest


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from vehicle_studio import Severity, VehicleDocument, VehicleDocumentError
from vehicle_studio.canonical import canonical_json_bytes


FIXTURE = ROOT / "fixtures" / "vehicle_document_valid_minimal.json"


def fixture_data() -> dict:
    return json.loads(FIXTURE.read_text(encoding="utf-8"))


def codes(document: VehicleDocument) -> set[str]:
    return {item.code for item in document.diagnostics() if item.severity >= Severity.ERROR}


class VehicleDocumentTests(unittest.TestCase):
    def test_valid_fixture(self):
        document = VehicleDocument.load(FIXTURE)
        self.assertTrue(document.is_valid())
        self.assertEqual([], document.diagnostics())

    def test_canonical_bytes_are_stable_for_equivalent_order(self):
        first = fixture_data()
        second = deepcopy(first)
        for collection in (
            "component_map",
            "frames",
            "material_regions",
            "view_definitions",
        ):
            second[collection].reverse()
        second["material_regions"][0]["component_ids"].reverse()
        self.assertEqual(
            VehicleDocument.from_dict(first).canonical_bytes(),
            VehicleDocument.from_dict(second).canonical_bytes(),
        )

    def test_canonical_float_format_and_negative_zero(self):
        encoded = canonical_json_bytes({"b": 1.25, "a": -0.0})
        self.assertEqual(b'{"a":0.000000,"b":1.250000}\n', encoded)

    def test_duplicate_global_id_fails(self):
        data = fixture_data()
        data["component_map"][1]["component_id"] = data["component_map"][0]["component_id"]
        self.assertIn("VEHICLE_DOCUMENT_DUPLICATE_ID", codes(VehicleDocument.from_dict(data)))

    def test_missing_required_frame_fails(self):
        data = fixture_data()
        data["frames"] = [
            frame for frame in data["frames"] if frame["semantic_role"] != "wheel_fl"
        ]
        self.assertIn("VEHICLE_DOCUMENT_REQUIRED_FRAME", codes(VehicleDocument.from_dict(data)))

    def test_frame_cycle_fails(self):
        data = fixture_data()
        by_role = {frame["semantic_role"]: frame for frame in data["frames"]}
        by_role["vehicle_origin"]["parent_frame_id"] = by_role["wheel_fl"]["frame_id"]
        self.assertIn("VEHICLE_DOCUMENT_FRAME_CYCLE", codes(VehicleDocument.from_dict(data)))

    def test_axis_convention_fails(self):
        data = fixture_data()
        data["coordinate_system"]["forward_axis"] = "+Z"
        self.assertIn("VEHICLE_DOCUMENT_AXIS_CONVENTION", codes(VehicleDocument.from_dict(data)))

    def test_non_finite_fails_and_blocks_serialization(self):
        data = fixture_data()
        data["dimensions"]["wheelbase_m"] = math.inf
        document = VehicleDocument.from_dict(data)
        self.assertIn("VEHICLE_DOCUMENT_NON_FINITE", codes(document))
        with self.assertRaises(VehicleDocumentError):
            document.canonical_bytes()

    def test_absolute_source_path_fails(self):
        data = fixture_data()
        data["source"]["path"] = r"D:\foreign\source.glb"
        self.assertIn("VEHICLE_DOCUMENT_SOURCE_PATH", codes(VehicleDocument.from_dict(data)))

    def test_unknown_component_reference_fails(self):
        data = fixture_data()
        data["material_regions"][0]["component_ids"].append("component-unknown")
        self.assertIn(
            "VEHICLE_DOCUMENT_COMPONENT_REFERENCE",
            codes(VehicleDocument.from_dict(data)),
        )

    def test_view_axes_must_be_orthogonal(self):
        data = fixture_data()
        data["view_definitions"][0]["depth_axis"] = "+X"
        self.assertIn("VEHICLE_DOCUMENT_VIEW_AXES", codes(VehicleDocument.from_dict(data)))

    def test_policy_constants_fail(self):
        data = fixture_data()
        data["validation_policy"]["topology_changes_allowed"] = True
        self.assertIn("VEHICLE_DOCUMENT_POLICY", codes(VehicleDocument.from_dict(data)))

    def test_parameter_bounds_fail(self):
        data = fixture_data()
        data["parameters"] = [
            {
                "parameter_id": "parameter-wheelbase",
                "kind": "dimension",
                "absolute_value": 3.2,
                "baseline_value": 2.9,
                "percentage": 110.0,
                "unit": "meter",
                "minimum": 2.5,
                "maximum": 3.1,
                "dependency_ids": [],
            }
        ]
        self.assertIn("VEHICLE_DOCUMENT_PARAMETER_BOUNDS", codes(VehicleDocument.from_dict(data)))

    def test_diagnostics_have_deterministic_order(self):
        data = fixture_data()
        data["source"]["path"] = "/absolute.glb"
        data["coordinate_system"]["forward_axis"] = "+Z"
        first = [item.as_dict() for item in VehicleDocument.from_dict(data).diagnostics()]
        second = [item.as_dict() for item in VehicleDocument.from_dict(data).diagnostics()]
        self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()

