from __future__ import annotations

import hashlib
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from vehicle_studio.blend_scan import DEFAULT_BLENDER_EXE, BlendScanError, scan_blend, scan_blend_bytes


CREATOR = ROOT / "tests" / "create_blend_fixture.py"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest().upper()


@unittest.skipUnless(DEFAULT_BLENDER_EXE.is_file(), "Blender 5.2 is not installed")
class BlendScanIntegrationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="formula90_blend_fixture_")
        self.project = Path(self.temporary.name)
        self.fixture = self.project / "fixture.blend"
        completed = subprocess.run(
            [
                str(DEFAULT_BLENDER_EXE),
                "--background",
                "--factory-startup",
                "--python",
                str(CREATOR),
                "--",
                str(self.fixture),
            ],
            capture_output=True,
            text=True,
            check=False,
            timeout=120,
        )
        if completed.returncode != 0:
            self.fail(completed.stdout + completed.stderr)

    def tearDown(self):
        self.temporary.cleanup()

    def test_scan_is_deterministic_and_source_preserving(self):
        before = sha256(self.fixture)
        first = scan_blend_bytes(
            self.fixture,
            repo_root=REPO,
            project_root=self.project,
        )
        second = scan_blend_bytes(
            self.fixture,
            repo_root=REPO,
            project_root=self.project,
        )
        self.assertEqual(first, second)
        self.assertEqual(before, sha256(self.fixture))
        self.assertNotIn(str(self.project), first.decode("utf-8"))

    def test_probe_reports_evaluated_mesh_uv_modifier_material_and_properties(self):
        report = scan_blend(
            self.fixture,
            repo_root=REPO,
            project_root=self.project,
        )
        self.assertEqual("fixture.blend", report["source"]["path"])
        obj = report["evidence"]["objects"][0]
        self.assertEqual("GEO_TEST_CHASSIS", obj["name"])
        self.assertEqual("chassis", obj["custom_properties"]["semantic_role"])
        self.assertEqual("MIRROR", obj["modifiers"][0]["type"])
        self.assertEqual("UVMap", obj["mesh"]["uv_layers"][0]["name"])
        self.assertEqual("MAT_TEST_PAINT", obj["mesh"]["material_slots"][0])


class BlendScanPreconditionTests(unittest.TestCase):
    def test_non_blend_extension_is_rejected(self):
        with self.assertRaisesRegex(BlendScanError, r"\.blend extension"):
            scan_blend(REPO / "README.md", repo_root=REPO)

    def test_missing_blender_is_rejected_after_valid_source(self):
        source = REPO / "assets-lowpoly-python" / "vehicles" / "blender" / "Untitled.blend"
        if not source.is_file():
            self.skipTest("repository Blend source is unavailable")
        with self.assertRaisesRegex(BlendScanError, "Blender executable not found"):
            scan_blend(
                source,
                repo_root=REPO,
                blender_executable=REPO / "missing-blender.exe",
            )


if __name__ == "__main__":
    unittest.main()

