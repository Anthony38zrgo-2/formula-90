import unittest
from pathlib import Path
import hashlib
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "blender" / "track_pipeline"))

from svg_sanitizer import SVGSanitizeError, canonical_xml, sanitize_svg
from svg_normalizer import NormalizeError, normalized_json_bytes, normalize

FIXTURE = ROOT / "blender" / "track_pipeline" / "tests" / "fixtures" / "minimal_track.svg"


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


class SVGProfileSanitizerTests(unittest.TestCase):
    def test_valid_fixture_sanitizes(self):
        root = sanitize_svg(FIXTURE.read_bytes())
        self.assertEqual(root.get("data-track-id"), "fixture_minimal")
        self.assertEqual(root.get("viewBox"), "0 0 100 60")

    def test_script_element_rejected(self):
        malicious = b'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><script>alert(1)</script></svg>'
        with self.assertRaises(SVGSanitizeError) as ctx:
            sanitize_svg(malicious)
        self.assertTrue(any("script" in d.lower() for d in ctx.exception.diagnostics))

    def test_event_handler_rejected(self):
        malicious = b'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1" onload="evil()"></svg>'
        with self.assertRaises(SVGSanitizeError) as ctx:
            sanitize_svg(malicious)
        self.assertTrue(any("onload" in d for d in ctx.exception.diagnostics))

    def test_external_reference_rejected(self):
        malicious = b'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 1 1"><image xlink:href="https://evil.example/x"/></svg>'
        with self.assertRaises(SVGSanitizeError):
            sanitize_svg(malicious)

    def test_transform_rejected_to_avoid_ambiguity(self):
        malicious = b'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><g transform="translate(5,5)"></g></svg>'
        with self.assertRaises(SVGSanitizeError) as ctx:
            sanitize_svg(malicious)
        self.assertTrue(any("transform" in d for d in ctx.exception.diagnostics))

    def test_canonical_serialization_is_deterministic(self):
        root_a = sanitize_svg(FIXTURE.read_bytes())
        root_b = sanitize_svg(FIXTURE.read_bytes())
        self.assertEqual(canonical_xml(root_a), canonical_xml(root_b))

    def test_no_disallowed_element_survives(self):
        root = sanitize_svg(FIXTURE.read_bytes())
        allowed = {"svg", "metadata", "g", "path", "polygon", "polyline", "line", "circle", "rect", "author"}
        for element in root.iter():
            self.assertIn(element.tag.rsplit("}", 1)[-1], allowed)


class SVGProfileNormalizerTests(unittest.TestCase):
    def _canonical(self):
        return sanitize_svg(FIXTURE.read_bytes())

    def test_normalized_json_byte_identical_across_two_runs(self):
        first = normalized_json_bytes(self._canonical())
        second = normalized_json_bytes(self._canonical())
        self.assertEqual(_sha256(first), _sha256(second))
        self.assertEqual(first, second)

    def test_centerline_closed_and_sampled(self):
        result = normalize(self._canonical())
        centerline = result["centerline"]
        self.assertTrue(centerline["closed"])
        self.assertEqual(centerline["sample_spacing_m"], 2.0)
        self.assertGreater(centerline["length_m"], 100.0)
        self.assertGreater(centerline["point_count"], 10)

    def test_banking_stored_against_centreline_distance(self):
        result = normalize(self._canonical())
        banking = result["banking"]
        self.assertEqual(len(banking), 3)
        for zone in banking:
            self.assertIn("s_m", zone)
            self.assertIn("degrees", zone)
            self.assertIn("center_fraction", zone)
            self.assertTrue(0.0 <= zone["center_fraction"] < 1.0)

    def test_asset_instance_stable_id(self):
        result = normalize(self._canonical())
        self.assertEqual(len(result["assets"]), 1)
        self.assertEqual(result["assets"][0]["instance_id"], "asset_0000")
        self.assertEqual(result["assets"][0]["asset_id"], "tree_v2_01")

    def test_collision_invariants_preserved(self):
        result = normalize(self._canonical())
        checks = result["collision_validation"]
        self.assertTrue(checks["finite_vertices"])
        self.assertTrue(checks["nondegenerate_triangles"])
        self.assertTrue(checks["blender_winding_upward"])
        self.assertTrue(checks["continuous_collision_grid"])
        self.assertLessEqual(checks["max_collision_seam_error_m"], 1e-6)
        self.assertTrue(checks["safety_floor_valid"])

    def test_missing_required_concept_rejected(self):
        root = self._canonical()

        def strip(parent, condition):
            for child in list(parent):
                if condition(child):
                    parent.remove(child)
                else:
                    strip(child, condition)

        strip(root, lambda element: element.get("data-role") == "banking")
        with self.assertRaises(NormalizeError) as ctx:
            normalize(root)
        self.assertIn("banking", str(ctx.exception))


if __name__ == "__main__":
    unittest.main()
