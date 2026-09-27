"""Contract tests for the canonical v10-v2 engine bank (schema 1)."""

from __future__ import annotations

import hashlib
import itertools
import json
import math
import subprocess
import sys
import wave
from pathlib import Path

import pytest

from tools.audio import build_canonical_v10_engine_bank as canonical

ROOT = Path(__file__).resolve().parents[3]
BANK_DIR = ROOT / "game/sounds/banks/v10-v2-bank"
VEHICLE_PROFILE = ROOT / "game/data/vehicles/f1_2030/f1_2030_v10_geometric.json"


@pytest.fixture(scope="module")
def manifest() -> dict:
    return json.loads((BANK_DIR / "manifest.json").read_text(encoding="utf-8"))


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


def test_canonical_bank_is_byte_deterministic() -> None:
    first_manifest, first_outputs = canonical.build_manifest(
        canonical.DEFAULT_BANK_DIRECTORY, canonical.EVENT_SOURCE_DIRECTORY
    )
    second_manifest, second_outputs = canonical.build_manifest(
        canonical.DEFAULT_BANK_DIRECTORY, canonical.EVENT_SOURCE_DIRECTORY
    )
    assert first_manifest == second_manifest
    assert sorted(first_outputs) == sorted(second_outputs)
    for filename, payload in first_outputs.items():
        assert payload == second_outputs[filename], filename


def test_canonical_bank_check_mode_accepts_shipped_artifacts() -> None:
    completed = subprocess.run(
        [sys.executable, "-m", "tools.audio.build_canonical_v10_engine_bank", "--check"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert completed.returncode == 0, completed.stderr


def test_canonical_bank_declares_format_and_mid_loops(manifest: dict) -> None:
    assert manifest["bank_id"] == "v10_v2_engine_bank"
    assert manifest["schema_version"] == 1
    assert manifest["output_sample_rate"] == 44100
    assert manifest["coverage"] == {
        "minimum_revolutions_per_minute": 4500.0,
        "maximum_revolutions_per_minute": 18000.0,
    }
    powered_ids = [asset["id"] for asset in manifest["loops"]]
    assert powered_ids == [
        "engine_idle_loop",
        "engine_low_on_loop",
        "engine_mid_on_loop",
        "engine_high_on_loop",
    ]
    coast_ids = [asset["id"] for asset in manifest["coast_loops"]]
    assert coast_ids == [
        "engine_low_off_loop",
        "engine_mid_off_loop",
        "engine_high_off_loop",
    ]
    assert len(manifest["transitions"]) == len(powered_ids) - 1
    assert len(manifest["coast_transitions"]) == len(coast_ids) - 1
    mid_transitions = [
        (item["from_loop_id"], item["to_loop_id"]) for item in manifest["transitions"]
    ]
    assert ("engine_low_on_loop", "engine_mid_on_loop") in mid_transitions
    assert ("engine_mid_on_loop", "engine_high_on_loop") in mid_transitions
    coast_mid_transitions = [
        (item["from_loop_id"], item["to_loop_id"]) for item in manifest["coast_transitions"]
    ]
    assert ("engine_low_off_loop", "engine_mid_off_loop") in coast_mid_transitions
    assert ("engine_mid_off_loop", "engine_high_off_loop") in coast_mid_transitions


def test_canonical_loop_zones_and_transitions_are_ordered(manifest: dict) -> None:
    for collection, transitions in (
        (manifest["loops"], manifest["transitions"]),
        (manifest["coast_loops"], manifest["coast_transitions"]),
    ):
        zones = [asset["zone_revolutions_per_minute"] for asset in collection]
        assert zones == sorted(zones)
        assert len(set(zones)) == len(zones)
        windows = [
            (item["start_revolutions_per_minute"], item["end_revolutions_per_minute"])
            for item in transitions
        ]
        for start, end in windows:
            assert start < end
            assert start >= manifest["coverage"]["minimum_revolutions_per_minute"]
            assert end <= manifest["coverage"]["maximum_revolutions_per_minute"]
        for previous, following in itertools.pairwise(windows):
            assert following[0] >= previous[1]
        for index, asset in enumerate(collection):
            active = asset["active_coverage_revolutions_per_minute"]
            reference = asset["reference_revolutions_per_minute"]
            assert math.isclose(
                active[0] / reference,
                asset["valid_playback_rate_min"],
                rel_tol=1e-4,
            )
            assert math.isclose(
                active[1] / reference,
                asset["valid_playback_rate_max"],
                rel_tol=1e-4,
            )
            assert 0.0 < asset["calibrated_gain"] <= 1.0
            assert 0 < asset["loop_crossfade_frames"] * 4 <= asset["derived_frames"]
            cycle_count = reference / 120.0 * asset["derived_frames"] / 44100.0
            assert abs(cycle_count - round(cycle_count)) < 1e-8


def test_declared_references_match_measured_engine_pitch(manifest: dict) -> None:
    for asset in manifest["loops"] + manifest["coast_loops"]:
        reference_hertz = (
            asset["reference_revolutions_per_minute"] / canonical.REVOLUTIONS_PER_MINUTE_PER_HERTZ
        )
        sample_rate, samples = canonical.read_wave_mono(BANK_DIR / asset["derived_filename"])
        assert sample_rate == 44100
        measurement = canonical.measure_cepstral_cycle_frequency(samples, reference_hertz)
        assert measurement is not None, asset["id"]
        measured_hertz, _, prominence = measurement
        assert (
            prominence >= canonical.REFERENCE_MEASUREMENT_MINIMUM_PROMINENCE_RATIO
        ), f"{asset['id']} pitch prominence {prominence}"
        error_ratio = abs(measured_hertz - reference_hertz) / reference_hertz
        assert (
            error_ratio <= canonical.REFERENCE_MEASUREMENT_TOLERANCE_RATIO
        ), f"{asset['id']} declared {reference_hertz} Hz vs measured {measured_hertz} Hz"


def test_canonical_assets_are_mono16_and_seam_safe(manifest: dict) -> None:
    for asset in manifest["loops"] + manifest["coast_loops"] + manifest["events"]:
        path = BANK_DIR / asset["derived_filename"]
        assert path.is_file(), asset["derived_filename"]
        assert sha256(path) == asset["derived_sha256"], asset["derived_filename"]
        channels, sample_width, sample_rate, frames = read_wav_format(path)
        assert channels == 1
        assert sample_width == 2
        assert sample_rate == 44100
        assert frames == asset["derived_frames"]
    for asset in manifest["loops"] + manifest["coast_loops"]:
        with wave.open(str(BANK_DIR / asset["derived_filename"]), "rb") as reader:
            raw = reader.readframes(reader.getnframes())
        pcm = [
            int.from_bytes(raw[index : index + 2], "little", signed=True)
            for index in range(0, len(raw), 2)
        ]
        deltas = sorted(abs(pcm[index + 1] - pcm[index]) for index in range(len(pcm) - 1))
        local_slope = deltas[int(len(deltas) * 0.99)]
        wrap_step = abs(pcm[0] - pcm[-1])
        assert wrap_step <= max(2 * local_slope, 64), asset["id"]


def test_canonical_event_groups_cover_every_event(manifest: dict) -> None:
    groups = {group["id"]: group for group in manifest["event_groups"]}
    assert set(groups) == {"upshift", "downshift", "lift_backfire", "limiter"}
    variant_ids = {
        variant["asset_id"] for group in manifest["event_groups"] for variant in group["variants"]
    }
    event_ids = {asset["id"] for asset in manifest["events"]}
    assert variant_ids == event_ids
    for event in manifest["events"]:
        source_name = event["source_filename"]
        assert (BANK_DIR / source_name).is_file()
        assert sha256(BANK_DIR / source_name) == event["source_sha256"]


def test_canonical_source_inventory_hash_matches(manifest: dict) -> None:
    inventory_path = BANK_DIR / "source_inventory.json"
    payload = inventory_path.read_bytes()
    assert hashlib.sha256(payload).hexdigest() == manifest["source_inventory_sha256"]
    inventory = json.loads(payload.decode("utf-8"))
    assert inventory["bank_id"] == manifest["bank_id"]
    assert len(inventory["engine_sources"]) == len(manifest["loops"] + manifest["coast_loops"])
    runtime_hashes = {asset["id"]: asset["derived_sha256"] for asset in manifest["loops"] + manifest["coast_loops"]}
    for entry in inventory["engine_sources"]:
        assert entry["runtime_sha256"] == runtime_hashes[entry["runtime_asset_id"]]


def test_vehicle_profile_points_at_the_canonical_manifest() -> None:
    profile = json.loads(VEHICLE_PROFILE.read_text(encoding="utf-8"))
    manifest_relative = profile["audio"]["grand_prix_sampler"]["manifest"]
    assert manifest_relative == "sounds/banks/v10-v2-bank/manifest.json"
    assert (ROOT / "game" / manifest_relative).is_file()
