"""Playback metadata stays explicit and independent from mixer policy."""

from __future__ import annotations

import json

from tools.audio.bank_spec import RETIRED_KEYS
from tools.audio.playback_metadata import ENGINE_BANDS, playback_for_key
from tools.audio.render_audio_scenario import BANK_DIR


def _runtime_files() -> dict[str, dict]:
    manifest = json.loads((BANK_DIR / "bank_manifest.json").read_text(encoding="utf-8"))
    return {entry["file"]: entry for entry in manifest["files"]}


def test_engine_band_order_and_sample_metadata_are_explicit():
    assert [band.key for band in ENGINE_BANDS] == [
        "engine_idle",
        "engine_low",
        "engine_mid",
        "engine_high",
        "engine_redline",
    ]
    assert [band.index for band in ENGINE_BANDS] == list(range(5))
    assert [band.center for band in ENGINE_BANDS] == [0.0, 0.25, 0.5, 0.75, 1.0]
    assert {band.width for band in ENGINE_BANDS} == {0.25}


def test_runtime_manifest_excludes_retired_engine_playback_metadata():
    files = _runtime_files()
    for key in RETIRED_KEYS:
        assert f"{key}.wav" not in files


def test_runtime_manifest_declares_retired_keys_explicitly():
    manifest = json.loads((BANK_DIR / "bank_manifest.json").read_text(encoding="utf-8"))
    assert manifest["retired_keys"] == list(RETIRED_KEYS)


def test_non_pitched_entries_do_not_serialize_empty_playback():
    files = _runtime_files()
    assert "playback" not in files["impact_barrier.wav"]
    assert playback_for_key("impact_barrier") == {}


def test_manifest_does_not_absorb_mixer_policy():
    manifest_text = (BANK_DIR / "bank_manifest.json").read_text(encoding="utf-8")
    assert "pitch_min" not in manifest_text
    assert "pitch_max" not in manifest_text
