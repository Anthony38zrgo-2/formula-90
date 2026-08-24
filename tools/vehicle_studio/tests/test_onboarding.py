from __future__ import annotations

import unittest

from vehicle_studio.onboarding import (
    OnboardingError, OnboardingSession, OPTIONAL_COMPONENT_ROLES,
    REQUIRED_COMPONENT_ROLES, REQUIRED_FRAME_ROLES,
)


class OnboardingTests(unittest.TestCase):
    def setUp(self):
        self.session = OnboardingSession(
            {"path": "car.glb", "sha256": "A" * 64},
            {"suggestions": [{
                "suggestion_id": "suggestion-0001",
                "semantic_kind": "component",
                "proposed_role": "nose",
                "source_kind": "mesh",
                "source_name": "GEO_NOSE",
                "evidence": [{"code": "explicit_name_token"}],
            }]},
        )

    def test_batch_accepts_unique_roles_in_one_revision(self):
        self.session.suggestions["suggestion-0002"] = {
            "suggestion_id": "suggestion-0002",
            "semantic_kind": "component",
            "proposed_role": "chassis",
            "source_kind": "mesh",
            "source_name": "GEO_CHASSIS",
            "evidence": [{"code": "explicit_name_token"}],
        }
        result = self.session.apply(0, {
            "type": "accept_suggestions",
            "suggestion_ids": ["suggestion-0001", "suggestion-0002"],
        })
        self.assertEqual(result["revision"], 1)
        self.assertEqual({item["semantic_role"] for item in result["components"]}, {"nose", "chassis"})

    def test_batch_rejects_duplicate_roles_before_mutation(self):
        duplicate = dict(self.session.suggestions["suggestion-0001"])
        duplicate["suggestion_id"] = "suggestion-0002"
        self.session.suggestions["suggestion-0002"] = duplicate
        with self.assertRaises(OnboardingError) as raised:
            self.session.apply(0, {
                "type": "accept_suggestions",
                "suggestion_ids": ["suggestion-0001", "suggestion-0002"],
            })
        self.assertEqual(raised.exception.code, "AMBIGUOUS_SUGGESTION_BATCH")
        self.assertEqual(self.session.snapshot()["components"], [])
        self.assertEqual(self.session.revision, 0)

    def test_incomplete_mapping_cannot_compile(self):
        self.assertFalse(self.session.snapshot()["ready_to_compile"])
        with self.assertRaises(OnboardingError) as raised:
            self.session.accepted_mapping_bytes()
        self.assertEqual(raised.exception.code, "ONBOARDING_INCOMPLETE")

    def test_accepted_suggestion_preserves_evidence(self):
        result = self.session.apply(0, {"type": "accept_suggestion", "suggestion_id": "suggestion-0001"})
        nose = next(item for item in result["components"] if item["semantic_role"] == "nose")
        self.assertEqual(nose["provenance"]["evidence"][0]["code"], "explicit_name_token")

    def test_confirmed_optional_component_becomes_explicitly_mapped(self):
        self.session.apply(0, {
            "type": "map_role",
            "semantic_kind": "component",
            "semantic_role": "cockpit",
            "source_kind": "mesh",
            "source_name": "GEO_COCKPIT",
        })
        self.assertEqual(self.session.snapshot()["optional_roles"]["cockpit"], "mapped")

    def test_optional_roles_must_be_explicit(self):
        for role in OPTIONAL_COMPONENT_ROLES:
            self.session.apply(
                self.session.revision,
                {"type": "set_optional_role", "semantic_role": role, "status": "absent"},
            )
        missing = self.session.snapshot()["missing_requirements"]
        self.assertFalse(any(item.startswith("optional:") for item in missing))

    def test_complete_manual_mapping_is_byte_stable(self):
        for role in sorted(REQUIRED_COMPONENT_ROLES):
            self.session.apply(self.session.revision, {
                "type": "map_role", "semantic_kind": "component",
                "semantic_role": role, "source_kind": "mesh", "source_name": f"GEO_{role}",
            })
        for role in sorted(REQUIRED_FRAME_ROLES):
            self.session.apply(self.session.revision, {
                "type": "map_role", "semantic_kind": "frame",
                "semantic_role": role, "source_kind": "node", "source_name": f"DATUM_{role}",
            })
        self.session.apply(self.session.revision, {
            "type": "set_axes", "right_axis": "+X", "up_axis": "+Y", "forward_axis": "-Z",
        })
        self.session.apply(self.session.revision, {"type": "set_ground", "ground_y_m": 0.0})
        self.session.apply(self.session.revision, {"type": "set_symmetry", "symmetry_plane_x_m": 0.0})
        for role in sorted(OPTIONAL_COMPONENT_ROLES):
            self.session.apply(self.session.revision, {
                "type": "set_optional_role", "semantic_role": role, "status": "absent",
            })
        self.assertTrue(self.session.snapshot()["ready_to_compile"])
        self.assertEqual(self.session.accepted_mapping_bytes(), self.session.accepted_mapping_bytes())

    def test_revision_conflict_and_unknown_commands_are_rejected(self):
        self.session.apply(0, {"type": "set_ground", "ground_y_m": 0})
        with self.assertRaises(OnboardingError) as conflict:
            self.session.apply(0, {"type": "set_ground", "ground_y_m": 0})
        self.assertEqual(conflict.exception.code, "REVISION_CONFLICT")
        with self.assertRaises(OnboardingError) as unknown:
            self.session.apply(1, {"type": "execute", "value": "anything"})
        self.assertEqual(unknown.exception.code, "COMMAND_NOT_ALLOWED")


if __name__ == "__main__":
    unittest.main()




