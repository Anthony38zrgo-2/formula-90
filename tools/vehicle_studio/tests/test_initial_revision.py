from __future__ import annotations

from copy import deepcopy
import sys
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from vehicle_studio.initial_revision import compile_initial_revision
from vehicle_studio.onboarding import OnboardingError


def accepted_mapping() -> dict:
    components = []
    for role in ("chassis", "nose", "front_wing", "rear_wing", "engine_cover", "sidepods", "wheel_fl", "wheel_fr", "wheel_rl", "wheel_rr"):
        components.append({
            "semantic_role": role, "source_kind": "mesh", "source_name": f"GEO_{role}",
            "provenance": {"source_path": f"geometry/{role}.glb", "evidence": []},
        })
    translations = {
        "vehicle_origin": [0, 0, 0],
        "front_axle_center": [0, 0, -1.45],
        "rear_axle_center": [0, 0, 1.45],
        "wheel_fl_anchor": [-0.8, 0, -1.45],
        "wheel_fr_anchor": [0.8, 0, -1.45],
        "wheel_rl_anchor": [-0.75, 0, 1.45],
        "wheel_rr_anchor": [0.75, 0, 1.45],
        "front_wing_mount": [0, 0, -1.9],
        "rear_wing_mount": [0, 0.2, 1.6],
    }
    frames = [{
        "semantic_role": role, "source_kind": "node", "source_name": f"DATUM_{role}",
        "provenance": {"source_path": "geometry/chassis.glb", "evidence": [{"translation_m": value}]},
    } for role, value in translations.items()]
    return {
        "ready_to_compile": True,
        "missing_requirements": [],
        "revision": 27,
        "source": {"path": "geometry", "sha256": "A" * 64, "files": [{"size_bytes": 10}]},
        "axes": {"right_axis": "+X", "up_axis": "+Y", "forward_axis": "-Z"},
        "ground_y_m": 0.0,
        "symmetry_plane_x_m": 0.0,
        "components": components,
        "frames": frames,
        "optional_roles": {"cockpit": "absent", "driver": "absent"},
    }


class InitialRevisionTests(unittest.TestCase):
    def test_compiled_revision_validates_and_references_source_hash(self):
        document = compile_initial_revision(accepted_mapping(), project_id="generic-car")
        self.assertTrue(document.is_valid(), document.diagnostics())
        self.assertEqual(document.data["source"]["sha256"], "A" * 64)
        self.assertEqual(document.data["dimensions"]["wheelbase_m"], 2.9)

    def test_command_revision_and_input_order_do_not_affect_bytes(self):
        first = accepted_mapping()
        second = deepcopy(first)
        second["revision"] = 999
        second["components"].reverse()
        second["frames"].reverse()
        self.assertEqual(
            compile_initial_revision(first, project_id="generic-car").canonical_bytes(),
            compile_initial_revision(second, project_id="generic-car").canonical_bytes(),
        )

    def test_incomplete_mapping_is_rejected(self):
        mapping = accepted_mapping()
        mapping["ready_to_compile"] = False
        with self.assertRaises(OnboardingError):
            compile_initial_revision(mapping, project_id="generic-car")

    def test_wheelbase_range_supports_2021_nominal_profile(self):
        document = compile_initial_revision(accepted_mapping(), project_id="generic-car")
        wheelbase = next(
            item for item in document.data["parameters"]
            if item["semantic_role"] == "wheelbase"
        )
        self.assertGreaterEqual(wheelbase["maximum"], 3.640)


if __name__ == "__main__":
    unittest.main()
