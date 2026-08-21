"""Determinism & metadata tests for the hybrid v10_vehicle bank."""

from __future__ import annotations

import hashlib
from pathlib import Path

from tools.audio.bank_generator import generate_bank
from tools.audio.bank_manifest import BankManifest

SOURCE = Path("assets-lowpoly-python/sounds")


def test_bank_is_byte_deterministic(tmp_path):
    a = tmp_path / "a"
    b = tmp_path / "b"
    ma = generate_bank(SOURCE, a)
    mb = generate_bank(SOURCE, b)
    assert ma.to_dict() == mb.to_dict()
    for entry in ma.files:
        ha = hashlib.sha256((a / entry.file).read_bytes()).hexdigest()
        hb = hashlib.sha256((b / entry.file).read_bytes()).hexdigest()
        assert ha == hb == entry.sha256, entry.file
    assert (a / "bank_manifest.json").read_bytes() == (b / "bank_manifest.json").read_bytes()


def test_manifest_contains_required_fields():
    bank = Path("game/sounds/banks/v10_vehicle")
    manifest = BankManifest.load(bank / "bank_manifest.json")
    assert manifest.bank_name == "v10_vehicle"
    assert manifest.sample_rate == 44100
    assert manifest.channels == 1
    assert manifest.pcm_bits == 16
    assert len(manifest.files) == 25
    roles = {e.role for e in manifest.files}
    assert {"engine_idle", "engine_low", "engine_mid", "engine_high", "engine_redline"} <= roles
    assert {"shift_up", "shift_down"} <= roles
    assert {"surf_grass", "surf_sand", "surf_rumble"} <= roles
    assert {"impact_barrier", "impact_cone", "impact_hit"} <= roles
    for e in manifest.files:
        if e.synthesis.get("recipe"):
            assert e.synthesis["recipe"] in ("flat_floor_scrape_v1", "flat_floor_scrape_v2")
            assert "procedural synthesis" in e.provenance.lower()
        else:
            assert e.synthesis.get("source_file"), f"{e.file} missing source_file"
            assert e.synthesis.get("source_sha256"), f"{e.file} missing source_sha256"
            assert len(e.synthesis["source_sha256"]) == 64
            assert "derived from original" in e.provenance.lower()
        assert e.sha256 and len(e.sha256) == 64
        assert e.duration_s > 0
        assert e.peak < 0.99
        assert abs(e.dc_offset) < 0.005


def test_manifest_hashes_match_files_on_disk():
    bank = Path("game/sounds/banks/v10_vehicle")
    manifest = BankManifest.load(bank / "bank_manifest.json")
    for e in manifest.files:
        actual = hashlib.sha256((bank / e.file).read_bytes()).hexdigest()
        assert actual == e.sha256, f"hash mismatch {e.file}"


def test_bank_matches_declared_source_spec():
    """Every manifest entry maps to its declared source or synthesis recipe."""
    from tools.audio.bank_manifest import BankManifest
    from tools.audio.bank_spec import BANK_SPEC

    bank = Path("game/sounds/banks/v10_vehicle")
    manifest = BankManifest.load(bank / "bank_manifest.json")
    by_key = {e.key: e for e in BANK_SPEC}
    assert len(manifest.files) == len(BANK_SPEC)
    for entry in manifest.files:
        key = entry.file[:-4]
        spec = by_key[key]
        assert entry.role == spec.role
        assert entry.loop == spec.loop
        if spec.synthesis is not None:
            assert entry.synthesis["recipe"] == spec.synthesis
            continue
        assert entry.synthesis["source_file"] == spec.source, f"{key} source drift"
        assert spec.source is not None
        src = SOURCE / spec.source
        assert src.is_file(), f"source missing {src}"
        assert hashlib.sha256(src.read_bytes()).hexdigest() == entry.synthesis["source_sha256"], f"{key} source hash"
