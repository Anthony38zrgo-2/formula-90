from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path


COMMON_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(COMMON_DIR))

from output_policy import OutputMode, OutputPolicyError, validate_output_path  # noqa: E402


class OutputPolicyTests(unittest.TestCase):
    def setUp(self) -> None:
        self._temporary = tempfile.TemporaryDirectory()
        self.repo = Path(self._temporary.name).resolve()
        (self.repo / "scratch").mkdir()
        (self.repo / "game" / "assets").mkdir(parents=True)
        (self.repo / "game" / "sounds").mkdir(parents=True)
        (self.repo / "source-assets").mkdir()

    def tearDown(self) -> None:
        self._temporary.cleanup()

    def test_preview_accepts_relative_child_of_scratch(self) -> None:
        decision = validate_output_path(self.repo, "scratch/audio/candidate.wav", "preview")
        self.assertEqual(decision.mode, OutputMode.PREVIEW)
        self.assertEqual(decision.path, self.repo / "scratch" / "audio" / "candidate.wav")
        self.assertEqual(decision.allowed_root, self.repo / "scratch")

    def test_preview_accepts_absolute_child_of_scratch(self) -> None:
        output = self.repo / "scratch" / "vehicle" / "preview.glb"
        self.assertEqual(validate_output_path(self.repo, output, "preview").path, output)

    def test_preview_rejects_runtime_content(self) -> None:
        with self.assertRaises(OutputPolicyError):
            validate_output_path(self.repo, "game/assets/preview.glb", "preview")

    def test_preview_rejects_source_assets(self) -> None:
        with self.assertRaises(OutputPolicyError):
            validate_output_path(self.repo, "source-assets/audio/source.wav", "preview")

    def test_preview_rejects_traversal_out_of_scratch(self) -> None:
        with self.assertRaises(OutputPolicyError):
            validate_output_path(self.repo, "scratch/../game/assets/escape.glb", "preview")

    def test_preview_rejects_scratch_root(self) -> None:
        with self.assertRaises(OutputPolicyError):
            validate_output_path(self.repo, "scratch", "preview")

    def test_promote_accepts_asset_child(self) -> None:
        output = self.repo / "game" / "assets" / "environment" / "barrier.glb"
        decision = validate_output_path(self.repo, output, "promote")
        self.assertEqual(decision.mode, OutputMode.PROMOTE)
        self.assertEqual(decision.allowed_root, self.repo / "game" / "assets")

    def test_promote_accepts_sound_child(self) -> None:
        output = self.repo / "game" / "sounds" / "banks" / "v10" / "engine.wav"
        self.assertEqual(validate_output_path(self.repo, output, "promote").path, output)

    def test_promote_rejects_scratch(self) -> None:
        with self.assertRaises(OutputPolicyError):
            validate_output_path(self.repo, "scratch/audio/final.wav", "promote")

    def test_promote_rejects_runtime_roots(self) -> None:
        for output in ("game/assets", "game/sounds"):
            with self.subTest(output=output), self.assertRaises(OutputPolicyError):
                validate_output_path(self.repo, output, "promote")

    def test_rejects_unknown_mode(self) -> None:
        with self.assertRaises(OutputPolicyError):
            validate_output_path(self.repo, "scratch/example", "publish")

    def test_rejects_repository_root(self) -> None:
        with self.assertRaises(OutputPolicyError):
            validate_output_path(self.repo, self.repo, "preview")


if __name__ == "__main__":
    unittest.main()
