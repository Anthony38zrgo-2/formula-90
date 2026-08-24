


from __future__ import annotations

from hashlib import sha256
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from vehicle_studio.materializer import MaterializationError, materialize_build_ir

GEOMETRY = Path("blender/williams94_wheels_retextured/geometry")
SOURCE_RELATIVE = (GEOMETRY / "F1_94_chassis_geometry.glb").as_posix()
MODULAR_NAMES = (
    "F1_94_chassis_core_geometry.glb", "F1_94_front_wing_nose_geometry.glb",
    "F1_94_rear_wing_geometry.glb", "F1_94_wheel_front_geometry.glb",
    "F1_94_wheel_rear_geometry.glb",
)
BASELINES = {
    "wheelbase": 2.920651025, "front_track": 1.59250099, "rear_track": 1.52461302,
    "front_tire_radius": 0.31695619536451, "rear_tire_radius": 0.32900932535519894,
    "front_tire_width": 0.3002989888191223, "rear_tire_width": 0.36832499504089355,
}


def file_hash(path: Path) -> str:
    return sha256(path.read_bytes()).hexdigest().upper()


def source_spec(relative: str) -> dict:
    return {"path": relative, "sha256": file_hash(REPO / relative)}


def build_ir(target: float | None = None, semantic_role: str = "wheelbase",
             policy: str = "centered") -> dict:
    parameters = []
    for role, baseline in BASELINES.items():
        parameters.append({"parameter_id": f"parameter-{role}", "semantic_role": role,
                           "baseline_value": baseline,
                           "absolute_value": target if role == semantic_role and target is not None else baseline,
                           "unit": "meter"})
    changed_target = target if target is not None else BASELINES[semantic_role]
    return {
        "schema_version": "vehicle-build-ir/v1", "document_sha256": "A" * 64,
        "source_sha256": "B" * 64,
        "policy": {"topology_changes_allowed": False, "source_mutation_allowed": False,
                   "output_scope": "staging_only"},
        "parameters": parameters,
        "width_policies": {"front_tire_width": policy, "rear_tire_width": policy},
        "operations": [
            {"operation_id": "op-0010-assert-source", "order": 10, "kind": "assert_source_hash",
             "depends_on": [], "inputs": {"source_path": "bundle", "source_sha256": "B" * 64,
             "materialization_source": source_spec(SOURCE_RELATIVE), "ground_y_m": -0.322982,
             "materialization_sources": [
                 source_spec((GEOMETRY / name).as_posix()) for name in MODULAR_NAMES]},
             "postconditions": ["source_hash_matches"]},
            {"operation_id": "op-0100-change", "order": 100, "kind": "test_parameter_operation",
             "depends_on": ["op-0010-assert-source"],
             "inputs": {"baseline_value": BASELINES.get(semantic_role, 1.0),
                        "target_value": changed_target, "semantic_role": semantic_role},
             "postconditions": ["parameter_matches"]},
            {"operation_id": "op-0900-validate", "order": 900, "kind": "validate_vehicle",
             "depends_on": ["op-0100-change"], "inputs": {}, "postconditions": ["topology_matches_source"]},
            {"operation_id": "op-0990-stage-export", "order": 990, "kind": "stage_variant_export",
             "depends_on": ["op-0900-validate"], "inputs": {"formats": ["blend", "glb"], "publish": False},
             "postconditions": ["source_unchanged"]},
        ],
    }


class MaterializerTests(unittest.TestCase):
    def test_changed_unsupported_operation_fails_before_staging_write(self):
        with tempfile.TemporaryDirectory() as temporary:
            staging = Path(temporary) / "staging"
            with self.assertRaises(MaterializationError) as raised:
                materialize_build_ir(build_ir(1.2, "nose_length"), repo_root=REPO, staging_root=staging)
            self.assertEqual(raised.exception.code, "MATERIALIZER_UNSUPPORTED_CHANGED_OPERATION")
            self.assertFalse(staging.exists())

    def test_global_and_tire_changes_export_complete_package(self):
        cases = (
            ("wheelbase", BASELINES["wheelbase"] * 1.05, "centered"),
            ("front_tire_radius", BASELINES["front_tire_radius"] * 1.10, "centered"),
            ("rear_tire_width", BASELINES["rear_tire_width"] * 1.15, "inboard_fixed"),
            ("front_tire_width", BASELINES["front_tire_width"] * 0.90, "outboard_fixed"),
        )
        with tempfile.TemporaryDirectory() as temporary:
            for role, target, policy in cases:
                report = materialize_build_ir(build_ir(target, role, policy), repo_root=REPO,
                                              staging_root=Path(temporary) / "staging")
                self.assertAlmostEqual(report["measured_dimensions_m"][role], target)
                self.assertEqual(set(report["artifacts"]),
                                 {"blend", "chassis_glb", "front_wheel_glb", "rear_wheel_glb", "preview_glb"})
                self.assertTrue(all(Path(item["path"]).is_file() for item in report["artifacts"].values()))
                self.assertAlmostEqual(report["ground_contact"]["front_y_m"], -0.322982)
                self.assertEqual(report["ground_contact"]["front_y_m"], report["ground_contact"]["rear_y_m"])
                self.assertEqual(len(report["preview"]["wheel_placements"]), 4)

    def test_source_hash_mismatch_fails_before_staging_write(self):
        plan = build_ir()
        plan["operations"][0]["inputs"]["materialization_source"]["sha256"] = "0" * 64
        with tempfile.TemporaryDirectory() as temporary:
            staging = Path(temporary) / "staging"
            with self.assertRaises(MaterializationError) as raised:
                materialize_build_ir(plan, repo_root=REPO, staging_root=staging)
            self.assertEqual(raised.exception.code, "MATERIALIZER_SOURCE_HASH_MISMATCH")
            self.assertFalse(staging.exists())

    def test_baseline_materialization_is_repeatable_and_source_stays_unchanged(self):
        source = REPO / SOURCE_RELATIVE
        before = file_hash(source)
        reports = []
        with tempfile.TemporaryDirectory() as first, tempfile.TemporaryDirectory() as second:
            for temporary in (first, second):
                report = materialize_build_ir(build_ir(), repo_root=REPO,
                                              staging_root=Path(temporary) / "staging")
                reports.append(report)
        self.assertEqual(before, file_hash(source))
        self.assertEqual(reports[0]["topology_signature"], reports[1]["topology_signature"])
        self.assertEqual(
            {key: value["sha256"] for key, value in reports[0]["artifacts"].items() if key != "blend"},
            {key: value["sha256"] for key, value in reports[1]["artifacts"].items() if key != "blend"},
        )


if __name__ == "__main__":
    unittest.main()
