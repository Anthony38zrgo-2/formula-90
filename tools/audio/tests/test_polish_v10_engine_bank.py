"""Contract tests for the applied V10 engine polish ledger."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

import numpy as np
import pytest

from tools.audio import build_canonical_v10_engine_bank as canonical
from tools.audio import polish_v10_engine_bank as polish

ROOT = Path(__file__).resolve().parents[3]
BANK_DIR = ROOT / "game/sounds/banks/v10-v2-bank"


@pytest.fixture(scope="module")
def documents() -> tuple[dict, dict]:
    manifest = json.loads((BANK_DIR / "manifest.json").read_text(encoding="utf-8"))
    inventory = json.loads((BANK_DIR / "source_inventory.json").read_text(encoding="utf-8"))
    return manifest, inventory


def test_polish_cli_check_passes() -> None:
    completed = subprocess.run(
        [sys.executable, "-m", "tools.audio.polish_v10_engine_bank", "--check"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert completed.returncode == 0, completed.stdout + completed.stderr


def test_polish_ledger_matches_the_frozen_chains(documents: tuple[dict, dict]) -> None:
    manifest, inventory = documents
    ledger = inventory["engine_polish"]
    assert ledger["tool"] == polish.TOOL_NAME
    assert ledger["tool_revision"] == polish.TOOL_REVISION
    assert {entry["asset_id"] for entry in ledger["assets"]} == set(polish.APPROVED_POLISH_CHAINS)
    for entry in ledger["assets"]:
        delivered = [
            {key: value for key, value in setting.items() if key != "measured"}
            for setting in entry["settings"]
        ]
        assert delivered == polish.APPROVED_POLISH_CHAINS[entry["asset_id"]]


def test_polish_rewrites_manifest_and_inventory_hashes(documents: tuple[dict, dict]) -> None:
    manifest, inventory = documents
    assert manifest["polish_tool"] == polish.TOOL_NAME
    assert manifest["polish_tool_revision"] == polish.TOOL_REVISION
    ledger_files = {entry["runtime_filename"]: entry["post_sha256"] for entry in inventory["engine_polish"]["assets"]}
    for asset in manifest["loops"] + manifest["coast_loops"]:
        if asset["id"] in ledger_files:
            payload = (BANK_DIR / asset["derived_filename"]).read_bytes()
            assert hashlib.sha256(payload).hexdigest() == ledger_files[asset["id"]]
            assert asset["derived_sha256"] == ledger_files[asset["id"]]
    for record in inventory["engine_sources"]:
        if record["runtime_asset_id"] in ledger_files:
            assert record["runtime_sha256"] == ledger_files[record["runtime_asset_id"]]
    for record in manifest["engine_source_selection"]:
        if record["runtime_asset_id"] in ledger_files:
            assert record["runtime_sha256"] == ledger_files[record["runtime_asset_id"]]
    inventory_digest = hashlib.sha256((BANK_DIR / "source_inventory.json").read_bytes()).hexdigest()
    assert inventory_digest == manifest["source_inventory_sha256"]


def test_polished_loops_keep_level_reference_and_expectations(documents: tuple[dict, dict]) -> None:
    _, inventory = documents
    from tools.audio import evaluate_v10_engine_bank as evaluator

    for entry in inventory["engine_polish"]["assets"]:
        path = BANK_DIR / entry["runtime_filename"]
        samples = canonical.read_wave_mono(path)[1]
        direct_current = float(np.mean(samples))
        assert abs(direct_current) <= evaluator.DIRECT_CURRENT_LIMIT, entry["asset_id"]
        assert -6.0 < entry["post_peak_dbfs"] <= -0.3
        assert -22.0 < entry["post_rms_dbfs"] < -10.0


def test_events_and_transmission_whine_survive_polish(documents: tuple[dict, dict]) -> None:
    manifest, _ = documents
    event_ids = {asset["id"] for asset in manifest["events"]}
    assert event_ids == {
        "backfire_burst_3", "backfire_burst_4", "backfire_burst_5",
        "backfire_burst_6", "backfire_burst_7",
        "downshift_event", "upshift_event", "limiter_event",
    }
    whine = manifest["transmission_whine"]
    payload = (BANK_DIR / whine["derived_filename"]).read_bytes()
    assert hashlib.sha256(payload).hexdigest() == whine["derived_sha256"]
    for asset in manifest["events"]:
        payload = (BANK_DIR / asset["derived_filename"]).read_bytes()
        assert hashlib.sha256(payload).hexdigest() == asset["derived_sha256"]
