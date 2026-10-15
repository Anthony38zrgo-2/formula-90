"""Contract tests for the AI-only V10 engine bank (schema 1, engine-only)."""

from __future__ import annotations

import hashlib
import json
import math
import wave
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BANK_DIR = ROOT / "game/sounds/banks/v10-bank-ai"
PREVIEW_DIR = ROOT / "scratch/audio/v10-bank-ai"


def bank_directory() -> Path:
    if (BANK_DIR / "manifest.json").is_file():
        return BANK_DIR
    return PREVIEW_DIR


def manifest_path() -> Path:
    candidate = bank_directory()
    if candidate is BANK_DIR:
        return BANK_DIR / "manifest.json"
    return PREVIEW_DIR / "manifest.preview.json"


def load_manifest() -> tuple[Path, dict]:
    directory = bank_directory()
    manifest = json.loads(manifest_path().read_text(encoding="utf-8"))
    return directory, manifest


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_wav_format(path: Path) -> tuple[int, int, int, int]:
    with wave.open(str(path), "rb") as reader:
        return (
            reader.getnchannels(),
            reader.getsampwidth(),
            reader.getframerate(),
            reader.getnframes(),
        )


def test_ai_bank_is_engine_only() -> None:
    _, manifest = load_manifest()
    assert manifest["schema_version"] == 1
    assert manifest["bank_id"] == "v10_ai_engine_bank"
    assert manifest["output_sample_rate"] == 44100
    assert manifest["events"] == []
    assert manifest["event_groups"] == []
    assert "transmission_whine" not in manifest
    policy = manifest["event_trigger_policy"]
    assert 0.0 <= policy["lift_edge"]["throttle_max"] < policy["lift_edge"]["previous_throttle_min"] <= 1.0


def test_ai_bank_covers_full_revolutions_range_without_gaps() -> None:
    _, manifest = load_manifest()
    coverage = manifest["coverage"]
    assert coverage["minimum_revolutions_per_minute"] == 4500.0
    assert coverage["maximum_revolutions_per_minute"] == 18000.0
    loops = manifest["loops"]
    assert 2 <= len(loops) <= 16
    assert len(manifest["transitions"]) == len(loops) - 1
    assert all(asset["role"] == "engine_loop" for asset in loops)
    assert all(len(asset["presence_curve"]) > 0 for asset in loops)
    boundaries: list[float] = []
    for asset in loops:
        points = asset["presence_curve"]
        assert points[0]["revolutions_per_minute"] == asset["active_coverage_revolutions_per_minute"][0]
        assert points[-1]["revolutions_per_minute"] == asset["active_coverage_revolutions_per_minute"][1]
        for point in points:
            assert 0.0 <= point["relative_weight"] <= 1.0
        boundaries.extend(point["revolutions_per_minute"] for point in points)
    for revolutions in sorted(set(boundaries)):
        assert any(
            asset["active_coverage_revolutions_per_minute"][0]
            <= revolutions
            <= asset["active_coverage_revolutions_per_minute"][1]
            for asset in loops
        )


def test_ai_bank_loops_match_declared_wav_assets() -> None:
    directory, manifest = load_manifest()
    for asset in manifest["loops"] + manifest["coast_loops"]:
        filename = asset["derived_filename"]
        path = directory / filename
        assert path.is_file(), f"missing loop asset: {filename}"
        assert read_wav_format(path) == (1, 2, 44100, asset["derived_frames"])
        assert sha256(path) == asset["derived_sha256"]
        assert asset["loop_start_frame"] == 0
        assert asset["loop_end_frame_exclusive"] == asset["derived_frames"]
        assert 0 < asset["loop_crossfade_frames"] * 4 <= asset["derived_frames"]
        expected_reference = (
            asset["integer_cycle_count"] * 44100 / asset["derived_frames"] * 120.0
        )
        assert math.isclose(
            asset["reference_revolutions_per_minute"], expected_reference, rel_tol=1e-9
        )
        assert asset["calibrated_gain"] > 0.0
        assert asset["valid_playback_rate_min"] < asset["valid_playback_rate_max"]


def test_ai_bank_coast_collection_stays_sequential() -> None:
    _, manifest = load_manifest()
    coast = manifest["coast_loops"]
    assert 2 <= len(coast) <= 16
    assert len(manifest["coast_transitions"]) == len(coast) - 1
    zones = [asset["zone_revolutions_per_minute"] for asset in coast]
    assert zones == sorted(zones)
    for asset in coast:
        assert "presence_curve" not in asset
