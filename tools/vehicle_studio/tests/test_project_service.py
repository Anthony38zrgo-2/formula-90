from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

from vehicle_studio.project_service import LocalProjectService, ServiceError

FIXTURE = Path(__file__).parents[1] / "fixtures" / "vehicle_document_valid_minimal.json"


class LocalProjectServiceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.document = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.document["parameters"].append({
            "parameter_id": "parameter-wheelbase",
            "semantic_role": "wheelbase",
            "unit": "meter",
            "minimum": 2.0,
            "maximum": 4.0,
            "baseline_value": 2.9,
            "absolute_value": 2.9,
            "dependency_ids": [],
        })
        source = self.root / self.document["source"]["path"]
        source.parent.mkdir(parents=True)
        source.touch()
        self.service = LocalProjectService(self.root)

    def test_snapshot_is_canonical_and_does_not_build(self):
        snapshot = self.service.create_project("car-a", self.document)
        self.assertEqual(snapshot["revision"], 0)
        self.assertEqual(snapshot["diagnostics"], [])
        self.assertEqual(self.service.build_request_count, 0)

    def test_parameter_command_is_allowlisted_and_revision_guarded(self):
        self.service.create_project("car-a", self.document)
        parameter = self.document["parameters"][0]
        result = self.service.apply_command(
            "car-a", 0, {
                "type": "set_parameter",
                "parameter_id": parameter["parameter_id"],
                "absolute_value": 3.1,
            }
        )
        self.assertEqual(result["revision"], 1)
        self.assertEqual(result["document"]["parameters"][0]["absolute_value"], 3.1)
        with self.assertRaisesRegex(ServiceError, "revision changed") as conflict:
            self.service.apply_command(
                "car-a", 0, {
                    "type": "set_parameter",
                    "parameter_id": parameter["parameter_id"],
                    "absolute_value": 3.0,
                }
            )
        self.assertEqual(conflict.exception.code, "REVISION_CONFLICT")

    def test_arbitrary_command_or_process_string_is_rejected(self):
        self.service.create_project("car-a", self.document)
        with self.assertRaises(ServiceError) as raised:
            self.service.apply_command(
                "car-a", 0, {"type": "run", "command": "blender --python evil.py"}
            )
        self.assertEqual(raised.exception.code, "COMMAND_NOT_ALLOWED")

    def test_malformed_payload_is_rejected(self):
        self.service.create_project("car-a", self.document)
        with self.assertRaises(ServiceError) as raised:
            self.service.apply_command(
                "car-a", 0, {"type": "set_parameter", "parameter_id": 42}
            )
        self.assertEqual(raised.exception.code, "MALFORMED_COMMAND")

    def test_invalid_project_ids_are_rejected(self):
        for project_id in ("../escape", "nested/project", "C:\\escape"):
            with self.subTest(project_id=project_id):
                with self.assertRaises(ServiceError):
                    self.service.create_project(project_id, self.document)

    def test_save_command_does_not_exist_and_build_is_explicit(self):
        self.service.create_project("car-a", self.document)
        self.assertEqual(self.service.build_request_count, 0)
        build = self.service.request_build("car-a", 0)
        self.assertEqual(build["status"], "queued")
        self.assertEqual(self.service.build_request_count, 1)

    def test_input_and_snapshots_are_defensive_copies(self):
        created = self.service.create_project("car-a", self.document)
        created["document"]["schema_version"] = 999
        self.document["schema_version"] = 999
        snapshot = self.service.snapshot("car-a")
        self.assertEqual(snapshot["document"]["schema_version"], 1)
        self.assertEqual(
            snapshot["document"]["parameters"][0]["parameter_id"],
            "parameter-wheelbase",
        )


if __name__ == "__main__":
    unittest.main()
