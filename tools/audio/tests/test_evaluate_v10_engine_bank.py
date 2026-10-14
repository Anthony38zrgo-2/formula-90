"""Contract tests for the V10 engine bank sonic evaluation tool."""

from __future__ import annotations

import shutil
import subprocess
import sys
import wave
from pathlib import Path

import pytest

from tools.audio import evaluate_v10_engine_bank as evaluator

ROOT = Path(__file__).resolve().parents[3]
BANK_DIR = ROOT / "game/sounds/banks/v10-v2-bank"

EXPECTED_ASSET_IDS = {
    "engine_idle_loop",
    "engine_powered_low_loop",
    "engine_mid_on_loop",
    "engine_powered_medium_high_loop",
    "engine_powered_high_loop",
    "engine_powered_maximum_loop",
    "engine_low_off_loop",
    "engine_mid_off_loop",
    "engine_off_maximum_loop",
}
EXPECTED_POLISH_KINDS = {"highpass", "high_shelf", "bell"}


@pytest.fixture(scope="module")
def scratch_root() -> Path:
    directory = ROOT / "scratch/audio/evaluate-v10-engine-bank-selftest"
    shutil.rmtree(directory, ignore_errors=True)
    directory.mkdir(parents=True, exist_ok=True)
    yield directory
    shutil.rmtree(directory, ignore_errors=True)


def test_report_covers_every_engine_asset_and_only_checks_settings(scratch_root: Path) -> None:
    report = evaluator.build_report(BANK_DIR)
    assert report["bank_path"] == "game/sounds/banks/v10-v2-bank"
    assert {asset["asset_id"] for asset in report["assets"]} == EXPECTED_ASSET_IDS
    for asset in report["assets"]:
        assert asset["duration_seconds"] > 1.5
        assert asset["peak_dbfs"] <= -0.3
        assert asset["crest_db"] < 15.0
        for setting in asset["polish_settings"]:
            assert setting["kind"] in EXPECTED_POLISH_KINDS
            assert -12.0 < setting.get("gain_db", 0.0) <= 0.0
            assert 20.0 < setting.get("center_hz", 1000.0) < 12000.0


def test_report_is_deterministic(scratch_root: Path) -> None:
    first_directory = scratch_root / "first"
    second_directory = scratch_root / "second"
    first_bytes = evaluator.write_report_outputs(
        evaluator.build_report(BANK_DIR), first_directory
    )[0].read_bytes()
    second_bytes = evaluator.write_report_outputs(
        evaluator.build_report(BANK_DIR), second_directory
    )[0].read_bytes()
    assert first_bytes == second_bytes
    assert (first_directory / "report.md").read_text(encoding="utf-8") == (
        second_directory / "report.md"
    ).read_text(encoding="utf-8")


def test_render_outputs_are_deterministic_and_riff(scratch_root: Path) -> None:
    first_directory = scratch_root / "render-first"
    second_directory = scratch_root / "render-second"
    first_written = evaluator.write_render_outputs(
        evaluator.build_report(BANK_DIR), BANK_DIR, first_directory, ["engine_low_off_loop"]
    )
    second_written = evaluator.write_render_outputs(
        evaluator.build_report(BANK_DIR), BANK_DIR, second_directory, ["engine_low_off_loop"]
    )
    assert [path.name for path in first_written] == [
        "engine_low_off_loop_solo.wav",
        "engine_low_off_loop_sweep.wav",
        "engine_low_off_loop_polished.wav",
    ]
    for first_path, second_path in zip(first_written, second_written):
        assert first_path.read_bytes() == second_path.read_bytes(), first_path.name
    for path in first_written:
        with wave.open(str(path), "rb") as reader:
            assert reader.getnchannels() == 1
            assert reader.getsampwidth() == 2
            assert reader.getframerate() == evaluator.SAMPLE_RATE
            expected_minimum_frames = {
                "engine_low_off_loop_solo.wav": 440_000,
                "engine_low_off_loop_sweep.wav": 520_000,
                "engine_low_off_loop_polished.wav": 88_100,
            }[path.name]
            assert reader.getnframes() >= expected_minimum_frames, path.name


def test_bare_module_run_reports_help() -> None:
    completed = subprocess.run(
        [sys.executable, "-m", "tools.audio.evaluate_v10_engine_bank", "--help"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert completed.returncode == 0, completed.stderr
    assert "--analyze" in completed.stdout
    assert "--render" in completed.stdout
