from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path


COMMON_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(COMMON_DIR))

from architecture_inventory import build_inventory, serialize_inventory  # noqa: E402


class ArchitectureInventoryTests(unittest.TestCase):
    def setUp(self) -> None:
        self._temporary = tempfile.TemporaryDirectory()
        self.repo = Path(self._temporary.name).resolve()
        for directory in ("blender", "tools", "game", "scripts", "tests", "native"):
            (self.repo / directory).mkdir(parents=True)

    def tearDown(self) -> None:
        self._temporary.cleanup()

    def _write(self, relative: str, content: str) -> None:
        path = self.repo / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def test_detects_cross_zone_paths_and_parent_arithmetic(self) -> None:
        self._write(
            "tools/example.py",
            "ROOT = Path(__file__).resolve().parents[2]\n"
            "SOURCE = 'assets-lowpoly-python/sounds'\n",
        )
        inventory = build_inventory(self.repo)
        kinds = [finding["kind"] for finding in inventory["findings"]]
        self.assertIn("cross_zone_literal", kinds)
        self.assertIn("parent_arithmetic", kinds)

    def test_detects_runtime_output_default_and_write_candidate(self) -> None:
        self._write(
            "tools/publisher.py",
            "DEFAULT_OUTPUT = Path('game/assets/environment')\n"
            "DEFAULT_OUTPUT.write_text('value')\n",
        )
        inventory = build_inventory(self.repo)
        findings = inventory["findings"]
        self.assertTrue(any(item["kind"] == "runtime_output_default" for item in findings))
        candidate = next(item for item in findings if item["kind"] == "runtime_write_candidate")
        self.assertEqual(candidate["line"], 2)
        self.assertEqual(candidate["related_line"], 1)

    def test_detects_game_dependency_on_legacy_source(self) -> None:
        self._write("game/runtime.gd", 'const SOURCE = "assets-lowpoly-python/vehicles/car.glb"\n')
        inventory = build_inventory(self.repo)
        self.assertTrue(
            any(item["kind"] == "game_source_dependency" for item in inventory["findings"])
        )

    def test_excludes_vendor_cache_reports_and_binary_files(self) -> None:
        self._write("tools/target/generated.py", "VALUE = 'game/assets/generated'\n")
        self._write("tools/third_party/vendor.py", "VALUE = 'game/assets/vendor'\n")
        self._write("tools/reports/old.py", "VALUE = 'game/assets/report'\n")
        self._write("tools/ignored.bin", "game/assets/binary")
        inventory = build_inventory(self.repo)
        self.assertEqual(inventory["finding_count"], 0)

    def test_serialization_is_stable_and_contains_no_timestamp(self) -> None:
        self._write("scripts/example.ps1", "$Path = 'game/sounds/bank'\n")
        first = serialize_inventory(build_inventory(self.repo))
        second = serialize_inventory(build_inventory(self.repo))
        self.assertEqual(first, second)
        self.assertNotIn("timestamp", first.lower())
        self.assertNotIn(str(self.repo), first)

    def test_findings_have_deterministic_order(self) -> None:
        self._write("tools/z.py", "X = 'game/assets/z'\n")
        self._write("tools/a.py", "X = 'game/assets/a'\n")
        inventory = build_inventory(self.repo)
        paths = [item["path"] for item in inventory["findings"]]
        self.assertEqual(paths, sorted(paths))


if __name__ == "__main__":
    unittest.main()
