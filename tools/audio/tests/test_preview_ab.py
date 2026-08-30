"""Tests for the legacy sample-bank preview boundary."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

import pytest

from tools.audio.preview_ab import DEFAULT_SCENARIO, build_ab_preview, main


def test_preview_cli_rejects_runtime_output():
    unsafe_output = Path("game") / "sounds" / "unsafe-ab"
    with pytest.raises(SystemExit) as error:
        main(["--output", str(unsafe_output)])
    assert "preview output must be a child" in str(error.value)


def test_builds_current_and_candidate_for_same_scenario():
    scratch = Path("scratch").resolve()
    with tempfile.TemporaryDirectory(prefix="audio-ab-test-", dir=scratch) as temp:
        output = Path(temp) / "result"
        summary = build_ab_preview(Path("."), output_dir=output)
        assert summary["scenario"] == DEFAULT_SCENARIO
        assert (output / "current" / f"{DEFAULT_SCENARIO}.wav").is_file()
        assert (output / "candidate" / f"{DEFAULT_SCENARIO}.wav").is_file()
        assert (output / "candidate-bank" / "bank_manifest.json").is_file()
        stored = json.loads((output / "comparison.json").read_text(encoding="utf-8"))
        assert stored == summary
        # Continuous engine keys are retired from both banks. Rebuilding the
        # sample bank must not silently reintroduce a sampled-engine fallback,
        # so this legacy sample-only preview is intentionally identical.
        assert summary["same_render"]
