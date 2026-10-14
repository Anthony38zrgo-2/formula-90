"""Contract tests for the historical GP3 provisioning of the canonical bank."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

import pytest

from tools.audio import build_canonical_v10_engine_bank as canonical
from tools.audio import generate_v10_engine_intermediates as generator

ROOT = Path(__file__).resolve().parents[3]
BANK_DIR = ROOT / "game/sounds/banks/v10-v2-bank"
HISTORICAL_BANK_DIR = ROOT / "game/sounds/banks/v10-gp3"


@pytest.fixture(scope="module")
def generated() -> tuple[dict, dict[str, bytes]]:
    return generator.build_intermediates(BANK_DIR, HISTORICAL_BANK_DIR)


def test_historical_provisioning_is_deterministic(
    generated: tuple[dict, dict[str, bytes]],
) -> None:
    metadata, outputs = generated
    second_metadata, second_outputs = generator.build_intermediates(
        BANK_DIR, HISTORICAL_BANK_DIR
    )
    assert second_metadata == metadata
    assert second_outputs == outputs


def test_historical_provisioning_checks_the_shipped_bank() -> None:
    completed = subprocess.run(
        [sys.executable, "-m", "tools.audio.generate_v10_engine_intermediates", "--check"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert completed.returncode == 0, completed.stdout + completed.stderr


def test_canonical_maximum_copies_the_historical_source() -> None:
    historical_source = HISTORICAL_BANK_DIR / "engine_coast_maximum.wav"
    manifest = json.loads((BANK_DIR / "manifest.json").read_text(encoding="utf-8"))
    maximum = next(
        record
        for record in manifest["engine_source_selection"]
        if record["runtime_asset_id"] == "engine_off_maximum_loop"
    )
    assert maximum["source_sha256"] == hashlib.sha256(historical_source.read_bytes()).hexdigest()


def test_retired_fields_no_longer_exist() -> None:
    assert hasattr(canonical, "POWERED_ENGINE_PRESENCE_CURVES")
    assert not hasattr(generator, "SPECTRAL_MORPHS")
    assert not (BANK_DIR / generator.METADATA_FILENAME).exists()
