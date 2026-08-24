from __future__ import annotations

import unittest

from vehicle_studio.semantic_suggestions import (
    semantic_suggestions_bytes,
    suggest_semantics,
)


class SemanticSuggestionTests(unittest.TestCase):
    def _scan(self):
        return {
            "adapter": "glb-scan/v1",
            "source": {"path": "car/model.glb", "sha256": "a" * 64},
            "evidence": {
                "nodes_detail": [
                    {"name": "DATUM_FL"},
                    {"name": "GEO_NOSE"},
                    {"name": "unclassified_part"},
                ],
                "primitive_attribute_coverage": [
                    {"mesh_name": "GEO_AERO_FRONT_WING"},
                    {"mesh_name": "TYRE_RR"},
                ],
            },
        }

    def test_explicit_tokens_produce_auditable_suggestions(self):
        result = suggest_semantics(self._scan())
        roles = {item["proposed_role"] for item in result["suggestions"]}
        self.assertEqual(
            roles,
            {"front_wing", "nose", "wheel_fl_anchor", "wheel_rr"},
        )
        self.assertEqual(result["authority"], "suggestion_only")
        for item in result["suggestions"]:
            self.assertTrue(item["requires_human_confirmation"])
            self.assertEqual(item["evidence"][0]["code"], "explicit_name_token")

    def test_unknown_names_are_not_guessed(self):
        result = suggest_semantics(
            {"adapter": "blend-scan/v1", "source": {}, "evidence": {"objects": [{"name": "Part.001"}]}}
        )
        self.assertEqual(result["suggestions"], [])

    def test_blend_object_names_are_supported(self):
        result = suggest_semantics(
            {
                "adapter": "blend-scan/v1",
                "source": {"path": "car.blend", "sha256": "b" * 64},
                "evidence": {"objects": [{"name": "GEO_ENGINE_COVER"}, {"name": "WHEEL_FR"}]},
            }
        )
        self.assertEqual(
            {item["proposed_role"] for item in result["suggestions"]},
            {"engine_cover", "wheel_fr"},
        )

    def test_serialization_is_deterministic_across_input_order(self):
        first = self._scan()
        second = self._scan()
        second["evidence"]["nodes_detail"].reverse()
        second["evidence"]["primitive_attribute_coverage"].reverse()
        self.assertEqual(semantic_suggestions_bytes(first), semantic_suggestions_bytes(second))

    def test_rules_are_model_agnostic(self):
        scan = self._scan()
        scan["source"]["path"] = "generic-open-wheel/custom-car.glb"
        roles = {item["proposed_role"] for item in suggest_semantics(scan)["suggestions"]}
        self.assertIn("nose", roles)
        self.assertIn("front_wing", roles)


if __name__ == "__main__":
    unittest.main()
