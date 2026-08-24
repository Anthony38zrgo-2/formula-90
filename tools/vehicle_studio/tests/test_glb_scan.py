from __future__ import annotations

import hashlib
from pathlib import Path
import sys
import unittest


ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from vehicle_studio.glb_scan import GLBScanError, load_existing_inspector, scan_glb, scan_glb_bytes


FIXTURE = REPO / "blender" / "track_pipeline" / "tests" / "fixtures" / "assets" / "valid_tree.glb"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest().upper()


class GLBScanTests(unittest.TestCase):
    def test_adapter_reuses_existing_inspector_evidence(self):
        report = scan_glb(FIXTURE, repo_root=REPO, full=True)
        direct = load_existing_inspector(REPO).inspect(str(FIXTURE), full=True)
        self.assertEqual(direct["sha256"], report["evidence"]["sha256"])
        self.assertEqual(direct["total_positions"], report["evidence"]["total_positions"])
        self.assertEqual(direct["total_triangles"], report["evidence"]["total_triangles"])
        self.assertEqual(
            "blender/vehicle_pipeline/inspect_glb.py",
            report["adapter"]["reuse_owner"],
        )

    def test_report_contains_no_absolute_source_path(self):
        report = scan_glb(FIXTURE, repo_root=REPO)
        expected = FIXTURE.relative_to(REPO).as_posix()
        self.assertEqual(expected, report["source"]["path"])
        self.assertEqual(expected, report["evidence"]["file"])
        self.assertNotIn(str(REPO), scan_glb_bytes(FIXTURE, repo_root=REPO).decode("utf-8"))

    def test_scan_is_byte_deterministic_and_source_preserving(self):
        before = sha256(FIXTURE)
        first = scan_glb_bytes(FIXTURE, repo_root=REPO)
        second = scan_glb_bytes(FIXTURE, repo_root=REPO)
        self.assertEqual(first, second)
        self.assertEqual(before, sha256(FIXTURE))

    def test_relative_source_is_supported(self):
        relative = FIXTURE.relative_to(REPO)
        report = scan_glb(relative, repo_root=REPO)
        self.assertEqual(relative.as_posix(), report["source"]["path"])

    def test_outside_repository_is_rejected(self):
        with self.assertRaisesRegex(GLBScanError, "inside the repository"):
            scan_glb(Path(REPO.anchor) / "outside.glb", repo_root=REPO)

    def test_non_glb_extension_is_rejected(self):
        with self.assertRaisesRegex(GLBScanError, r"\.glb extension"):
            scan_glb(REPO / "README.md", repo_root=REPO)

    def test_negative_normal_tolerance_is_rejected(self):
        with self.assertRaisesRegex(GLBScanError, "non-negative"):
            scan_glb(FIXTURE, repo_root=REPO, normal_tolerance=-0.1)


if __name__ == "__main__":
    unittest.main()
