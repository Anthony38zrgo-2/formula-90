"""Contract tests shared by audio bank producers and consumers."""

from __future__ import annotations

import copy
import json
from pathlib import Path

from tools.audio.bank_contract import CONTRACT_PATH, load_contract, validate_manifest_contract
from tools.audio.bank_spec import RETIRED_KEYS
from tools.audio.render_audio_scenario import BANK_DIR

RUNTIME_MANIFEST = BANK_DIR / "bank_manifest.json"


def _runtime_manifest() -> dict:
    return json.loads(RUNTIME_MANIFEST.read_text(encoding="utf-8"))


def test_contract_is_repository_owned_json_schema():
    contract = load_contract()
    assert CONTRACT_PATH.resolve() == Path("formats/audio_bank/manifest.schema.json").resolve()
    assert contract["$schema"].endswith("2020-12/schema")
    assert contract["properties"]["schema_version"]["const"] == 1
    assert contract["additionalProperties"] is True
    assert contract["$defs"]["file"]["additionalProperties"] is True


def test_runtime_manifest_satisfies_minimal_contract():
    assert validate_manifest_contract(_runtime_manifest()) == []


def test_contract_rejects_missing_or_mistyped_core_field():
    manifest = _runtime_manifest()
    del manifest["sample_rate"]
    manifest["schema_version"] = True
    issues = validate_manifest_contract(manifest)
    assert any(issue.path == "sample_rate" and issue.code == "required" for issue in issues)
    assert any(issue.path == "schema_version" and issue.code == "type" for issue in issues)


def test_contract_rejects_path_traversal_and_duplicate_files():
    manifest = _runtime_manifest()
    duplicate = copy.deepcopy(manifest["files"][0])
    manifest["files"].extend([duplicate, copy.deepcopy(duplicate)])
    manifest["files"][0]["file"] = "../escape.wav"
    issues = validate_manifest_contract(manifest)
    assert any(issue.path == "files[0].file" and issue.code == "pattern" for issue in issues)
    assert any(issue.code == "duplicate" for issue in issues)


def test_contract_rejects_invalid_hash_and_loop_bounds():
    manifest = _runtime_manifest()
    entry = manifest["files"][0]
    entry["sha256"] = "not-a-hash"
    entry["loop_start_s"] = entry["duration_s"]
    entry["loop_end_s"] = 0.0
    issues = validate_manifest_contract(manifest)
    assert any(issue.path == "files[0].sha256" and issue.code == "pattern" for issue in issues)
    assert any(issue.path == "files[0]" and issue.code == "loop_bounds" for issue in issues)


def test_contract_rejects_invalid_playback_metadata():
    manifest = _runtime_manifest()
    entry = copy.deepcopy(manifest["files"][0])
    entry["file"] = "synthetic_probe.wav"
    entry["playback"] = {"native_rpm": 6000, "engine_band": {"index": 0, "center": 0.0, "width": 0.25}}
    manifest["files"].append(entry)
    playback = manifest["files"][-1]["playback"]
    playback["native_rpm"] = 0
    playback["engine_band"]["width"] = 0
    issues = validate_manifest_contract(manifest)
    assert any(issue.path.endswith("playback.native_rpm") for issue in issues)
    assert any(issue.path.endswith("playback.engine_band.width") for issue in issues)


def test_retired_keys_are_absent():
    manifest = _runtime_manifest()
    files = {e["file"] for e in manifest["files"]}
    for key in RETIRED_KEYS:
        assert f"{key}.wav" not in files, f"retired key {key} still in manifest files"
    assert manifest.get("retired_keys") == list(RETIRED_KEYS)
