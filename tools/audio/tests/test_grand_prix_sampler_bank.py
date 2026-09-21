"""Contract tests for the standalone Grand Prix sampler bank (schema 1)."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import math
import wave
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[3]
BUILDER_PATH = ROOT / "scripts/audio/build_grand_prix_sampler_bank.py"
SOURCE_DIR = ROOT / "game/sounds/banks/v10-gp3"
BANK_DIR = ROOT / "game/audio/formula_one_2030_grand_prix_sampler"
REPORTS_DIR = ROOT / "reports/audio-v10/grand-prix-sampler"


def load_builder():
    spec = importlib.util.spec_from_file_location("build_grand_prix_sampler_bank", BUILDER_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def builder():
    return load_builder()


@pytest.fixture(scope="module")
def manifest():
    return json.loads((BANK_DIR / "manifest.json").read_text(encoding="utf-8"))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_wav(path: Path) -> tuple[list[int], int]:
    import struct

    with wave.open(str(path), "rb") as handle:
        assert handle.getnchannels() == 1
        assert handle.getsampwidth() == 2
        rate = handle.getframerate()
        pcm = struct.unpack(f"<{handle.getnframes()}h", handle.readframes(handle.getnframes()))
    return list(pcm), rate


def test_grand_prix_bank_is_byte_deterministic(tmp_path, builder):
    if not SOURCE_DIR.is_dir():
        pytest.skip("implementation/sfx sources unavailable")
    first = tmp_path / "first"
    second = tmp_path / "second"
    reports_first = tmp_path / "reports-first"
    reports_second = tmp_path / "reports-second"
    assert builder.build_manifest(SOURCE_DIR, first, reports_first, False) == 0
    assert builder.build_manifest(SOURCE_DIR, second, reports_second, False) == 0
    first_files = sorted(path.name for path in first.iterdir())
    second_files = sorted(path.name for path in second.iterdir())
    assert first_files == second_files
    for name in first_files:
        assert (first / name).read_bytes() == (second / name).read_bytes(), name
    for name in ("source_inventory.json", "calibration.json", "seam_evidence.json"):
        assert (reports_first / name).read_bytes() == (reports_second / name).read_bytes(), name


def test_grand_prix_shipped_manifest_hashes_match_files(manifest):
    assert manifest["bank_id"] == "formula_one_2030_grand_prix_sampler"
    assert manifest["schema_version"] == 1
    assert manifest["preparation_tool_revision"] >= 1
    assert manifest["output_sample_rate"] == 44100
    assets = manifest["loops"] + manifest["events"]
    assert len(manifest["loops"]) == 5
    assert len(manifest["events"]) == 8
    for asset in assets:
        path = BANK_DIR / asset["derived_filename"]
        assert path.is_file(), asset["derived_filename"]
        assert sha256(path) == asset["derived_sha256"], asset["derived_filename"]
        pcm, rate = read_wav(path)
        assert rate == 44100
        assert len(pcm) == asset["derived_frames"]
        assert max(abs(sample) for sample in pcm) <= 32768


def test_grand_prix_source_inventory_is_accounted(manifest):
    inventory = json.loads((REPORTS_DIR / "source_inventory.json").read_text(encoding="utf-8"))
    source_hashes = {entry["sha256"] for entry in inventory["sources"]}
    assert len(inventory["sources"]) == 13
    assert not any(entry["filename"].startswith("01_Crackle") for entry in inventory["sources"])
    sources_by_role = {}
    for entry in inventory["sources"]:
        sources_by_role.setdefault(entry["role"], []).append(entry["filename"])
    assert sources_by_role["gearbox_upshift"] == ["gearup.wav"]
    assert sources_by_role["gearbox_downshift"] == ["geardn.wav"]
    assert sources_by_role["limiter_event"] == ["500_limiter.wav"]
    declared = {
        asset["source_sha256"] for asset in manifest["loops"] + manifest["events"]
    }
    assert declared == source_hashes
    inventory_payload = json.dumps(inventory, sort_keys=True, indent=2).encode("utf-8")
    assert hashlib.sha256(inventory_payload).hexdigest() == manifest["source_inventory_sha256"]


def test_grand_prix_reference_ladder_covers_physical_range(manifest):
    loops = sorted(manifest["loops"], key=lambda asset: asset["reference_revolutions_per_minute"])
    references = [asset["reference_revolutions_per_minute"] for asset in loops]
    assert references == sorted(references)
    assert all(reference > 0.0 and math.isfinite(reference) for reference in references)
    coverage = manifest["coverage"]
    assert coverage["minimum_revolutions_per_minute"] == 4500.0
    assert coverage["maximum_revolutions_per_minute"] == 18000.0
    transitions = manifest["transitions"]
    assert len(transitions) == len(loops) - 1
    assert transitions[0]["start_revolutions_per_minute"] >= coverage["minimum_revolutions_per_minute"]
    assert transitions[-1]["end_revolutions_per_minute"] <= coverage["maximum_revolutions_per_minute"]
    for previous, following in zip(transitions, transitions[1:]):
        assert following["start_revolutions_per_minute"] >= previous["end_revolutions_per_minute"]
    for index, loop in enumerate(loops):
        start = (
            coverage["minimum_revolutions_per_minute"]
            if index == 0
            else transitions[index - 1]["start_revolutions_per_minute"]
        )
        end = (
            coverage["maximum_revolutions_per_minute"]
            if index == len(loops) - 1
            else transitions[index]["end_revolutions_per_minute"]
        )
        reference = loop["reference_revolutions_per_minute"]
        low_rate = min(start / reference, end / reference)
        high_rate = max(start / reference, end / reference)
        assert loop["valid_playback_rate_min"] <= low_rate + 1e-6
        assert loop["valid_playback_rate_max"] >= high_rate - 1e-6
        assert math.isfinite(loop["calibrated_gain"])
        assert 0.0 < loop["calibrated_gain"] <= 1.0
        assert loop["loop_start_frame"] == 0
        assert loop["loop_end_frame_exclusive"] == loop["derived_frames"]
        assert 0 < loop["loop_crossfade_frames"] < loop["derived_frames"] // 4


def test_grand_prix_loops_are_seamless(manifest):
    for loop in manifest["loops"]:
        pcm, _ = read_wav(BANK_DIR / loop["derived_filename"])
        peak = max(abs(sample) for sample in pcm)
        assert peak > 1000
        deltas = [abs(pcm[index + 1] - pcm[index]) for index in range(len(pcm) - 1)]
        deltas.sort()
        local_step = deltas[int(len(deltas) * 0.99)]
        wrap_step = abs(pcm[0] - pcm[-1])
        assert wrap_step <= max(2 * local_step, 64), (
            f"{loop['id']} wrap step {wrap_step} exceeds local slope {local_step}"
        )


def test_grand_prix_event_groups_are_complete(manifest):
    groups = {group["id"]: group for group in manifest["event_groups"]}
    assert set(groups) == {"upshift", "downshift", "lift_backfire", "limiter"}
    variant_ids = {
        variant["asset_id"]
        for group in manifest["event_groups"]
        for variant in group["variants"]
    }
    event_ids = {asset["id"] for asset in manifest["events"]}
    assert variant_ids == event_ids
    backfire_variants = [variant["asset_id"] for variant in groups["lift_backfire"]["variants"]]
    assert backfire_variants == [
        "backfire_burst_3",
        "backfire_burst_4",
        "backfire_burst_5",
        "backfire_burst_6",
        "backfire_burst_7",
    ]
    downshift_variants = [variant["asset_id"] for variant in groups["downshift"]["variants"]]
    assert downshift_variants == ["downshift_event"]
    assert groups["downshift"]["selection"] == "single_variant"
    for asset in manifest["events"]:
        if asset["role"] == "gearbox_upshift":
            assert asset["source_filename"] == "gearup.wav"
            assert asset["preparation_recipe"] == "native_44100_copy"
        if asset["role"] == "gearbox_downshift":
            assert asset["source_filename"] == "geardn.wav"
            assert asset["preparation_recipe"] == "native_44100_copy"
        if asset["role"] == "lift_backfire":
            assert asset["source_filename"].startswith("500_backfire")
            assert asset["preparation_recipe"] == "backfire_overlay_recipe_v1"
            assert asset["derived_peak_dbfs"] <= 20.0 * math.log10(0.89) + 1e-3
    for group in manifest["event_groups"]:
        assert group["voice_limit"] >= 1
        assert group["minimum_retrigger_seconds"] > 0.0
        assert group["selection"] in (
            "single_variant",
            "seeded_cycle",
            "seeded_no_immediate_repeat",
        )
    policy = manifest["event_trigger_policy"]
    assert policy["lift_edge"]["previous_throttle_min"] == 0.80
    assert policy["lift_edge"]["throttle_max"] == 0.15
    assert policy["lift_edge"]["revolutions_per_minute_min"] == 13500.0
    assert policy["lift_edge"]["cooldown_seconds"] == 1.0
    assert policy["limiter_entry_edge"]["cooldown_seconds"] == 0.250


def test_grand_prix_limiter_event_is_the_adopted_accent(manifest):
    limiter = next(asset for asset in manifest["events"] if asset["id"] == "limiter_event")
    assert limiter["source_filename"] == "500_limiter.wav"
    assert limiter["preparation_recipe"] == "limiter_overlay_recipe_v1"
    pcm, rate = read_wav(BANK_DIR / limiter["derived_filename"])
    assert rate == 44100
    assert limiter["derived_frames"] == 15939
    assert abs(limiter["duration_seconds"] - 0.3614) < 1e-3
    assert len(pcm) == 15939
    assert limiter["derived_peak_dbfs"] <= 20.0 * math.log10(0.89) + 1e-3


def test_grand_prix_bank_check_mode_detects_drift(tmp_path, builder):
    if not SOURCE_DIR.is_dir():
        pytest.skip("implementation/sfx sources unavailable")
    bank = tmp_path / "bank"
    reports = tmp_path / "reports"
    assert builder.build_manifest(SOURCE_DIR, bank, reports, False) == 0
    assert builder.build_manifest(SOURCE_DIR, bank, reports, True) == 0
    (bank / "engine_idle_loop.wav").write_bytes((bank / "engine_idle_loop.wav").read_bytes() + b"\x00\x00")
    assert builder.build_manifest(SOURCE_DIR, bank, reports, True) == 1
