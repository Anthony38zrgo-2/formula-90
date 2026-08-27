from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path


COMMON_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(COMMON_DIR))

from content_flow import ContentFlowError, preview, promote  # noqa: E402
from output_policy import OutputPolicyError  # noqa: E402


class ContentFlowTests(unittest.TestCase):
    def setUp(self) -> None:
        self._temporary = tempfile.TemporaryDirectory()
        self.repo = Path(self._temporary.name).resolve()
        self.scratch = self.repo / "scratch"
        self.runtime_assets = self.repo / "game" / "assets"
        self.runtime_sounds = self.repo / "game" / "sounds"
        self.sources = self.repo / "source-assets"
        for directory in (self.scratch, self.runtime_assets, self.runtime_sounds, self.sources):
            directory.mkdir(parents=True)

    def tearDown(self) -> None:
        self._temporary.cleanup()

    def _source_file(self, content: str = "candidate") -> Path:
        path = self.sources / "candidate.txt"
        path.write_text(content, encoding="utf-8")
        return path

    def test_preview_copies_file_into_scratch(self) -> None:
        source = self._source_file()
        result = preview(self.repo, source, self.scratch / "audio" / "candidate.txt")
        self.assertEqual((self.scratch / "audio" / "candidate.txt").read_text(), "candidate")
        self.assertFalse(result.planned)
        self.assertFalse(result.replaced)

    def test_preview_copies_directory(self) -> None:
        source = self.sources / "bundle"
        source.mkdir()
        (source / "asset.txt").write_text("bundle", encoding="utf-8")
        preview(self.repo, source, self.scratch / "bundle-preview")
        self.assertEqual((self.scratch / "bundle-preview" / "asset.txt").read_text(), "bundle")

    def test_preview_rejects_runtime_output(self) -> None:
        with self.assertRaises(OutputPolicyError):
            preview(self.repo, self._source_file(), self.runtime_assets / "candidate.txt")

    def test_preview_requires_replace_for_existing_output(self) -> None:
        source = self._source_file("new")
        output = self.scratch / "candidate.txt"
        output.write_text("old", encoding="utf-8")
        with self.assertRaises(ContentFlowError):
            preview(self.repo, source, output)
        result = preview(self.repo, source, output, replace=True)
        self.assertTrue(result.replaced)
        self.assertEqual(output.read_text(), "new")

    def test_preview_plan_does_not_write(self) -> None:
        output = self.scratch / "planned.txt"
        result = preview(self.repo, self._source_file(), output, plan=True)
        self.assertTrue(result.planned)
        self.assertFalse(output.exists())

    def test_promote_requires_source_under_scratch(self) -> None:
        with self.assertRaises(ContentFlowError):
            promote(self.repo, self._source_file(), self.runtime_assets / "candidate.txt")

    def test_promote_installs_file(self) -> None:
        source = self.scratch / "approved.txt"
        source.write_text("approved", encoding="utf-8")
        output = self.runtime_assets / "environment" / "approved.txt"
        result = promote(self.repo, source, output)
        self.assertEqual(output.read_text(), "approved")
        self.assertIsNone(result.backup)

    def test_promote_replacement_preserves_backup(self) -> None:
        source = self.scratch / "approved.txt"
        source.write_text("new", encoding="utf-8")
        output = self.runtime_sounds / "bank" / "engine.txt"
        output.parent.mkdir()
        output.write_text("old", encoding="utf-8")
        result = promote(self.repo, source, output, replace=True, transaction_id="known")
        self.assertEqual(output.read_text(), "new")
        self.assertIsNotNone(result.backup)
        backup = self.repo / result.backup
        self.assertEqual(backup.read_text(), "old")

    def test_promote_plan_does_not_write(self) -> None:
        source = self.scratch / "approved.txt"
        source.write_text("approved", encoding="utf-8")
        output = self.runtime_assets / "planned.txt"
        result = promote(self.repo, source, output, plan=True)
        self.assertTrue(result.planned)
        self.assertFalse(output.exists())

    def test_rejects_overlapping_source_and_preview_output(self) -> None:
        source = self.scratch / "bundle"
        source.mkdir()
        with self.assertRaises(ContentFlowError):
            preview(self.repo, source, source / "nested", replace=True)


if __name__ == "__main__":
    unittest.main()
