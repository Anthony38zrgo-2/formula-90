"""Determinism and metadata tests for the procedural-engine vehicle bank."""

from __future__ import annotations

import hashlib
from pathlib import Path

from tools.audio.bank_generator import generate_bank
from tools.audio.bank_manifest import BankManifest
from tools.audio.bank_spec import RETIRED_KEYS
from tools.audio.promote_f1_2030_backfire_sounds import LIMITER_SOURCES
from tools.audio.promote_f1_2030_backfire_sounds import SOURCES as BACKFIRE_SOURCES

SOURCE = Path("source-assets/audio/legacy-f1-1998")
BACKFIRE_BANK_FILES = set(BACKFIRE_SOURCES)
LIMITER_BANK_FILES = set(LIMITER_SOURCES)
COMMONS_FILE_COUNT = 11


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
    bank = Path("game/sounds/banks/commons")
    manifest = BankManifest.load(bank / "bank_manifest.json")
    assert manifest.bank_name == "commons"
    assert manifest.sample_rate == 44100
    assert manifest.channels == 1
    assert manifest.pcm_bits == 16
    assert len(manifest.files) == COMMONS_FILE_COUNT
    roles = {e.role for e in manifest.files}
    assert not set(RETIRED_KEYS) & roles
    assert {"surf_grass", "surf_sand", "surf_rumble"} <= roles
    assert {"impact_barrier", "impact_cone", "impact_hit", "impact_scrape"} <= roles
    assert "tyre_scrub" in roles
    # Known provenance families; every entry must declare one of them with its
    # required source/recipe evidence.
    source_derived = (
        "derived from original samples (source-assets/audio/legacy-f1-1998)",
        "derived from original user-supplied replacement samples",
        "derived from curated v10 alternative gear samples",
    )
    for e in manifest.files:
        if e.file in BACKFIRE_BANK_FILES:
            assert e.role == "engine_backfire"
            assert not e.loop
            assert e.synthesis.get("promotion_recipe") == "f1_2030_sfx_backfire_overlay_v1"
        if e.file in LIMITER_BANK_FILES:
            assert e.role in ("limiter_hit", "tc_cut")
            assert not e.loop
            assert e.synthesis.get("promotion_recipe") == "f1_2030_sfx_limiter_tc_overlay_v1"
        assert e.synthesis.get("source_file") or e.synthesis.get("recipe"), f"{e.file} missing evidence"
        if e.synthesis.get("recipe"):
            assert e.synthesis["recipe"] in (
                "flat_floor_scrape_v1",
                "flat_floor_scrape_v2",
                "exhaust_mic_v1",
            )
            assert "procedural synthesis" in e.provenance.lower()
        else:
            assert e.synthesis.get("source_file"), f"{e.file} missing source_file"
            assert e.synthesis.get("source_sha256"), f"{e.file} missing source_sha256"
            assert len(e.synthesis["source_sha256"]) == 64
            assert any(family in e.provenance.lower() for family in source_derived), e.provenance
        assert e.sha256 and len(e.sha256) == 64
        assert e.duration_s > 0
        assert e.peak < 0.99
        assert abs(e.dc_offset) < 0.005


def test_manifest_hashes_match_files_on_disk():
    bank = Path("game/sounds/banks/commons")
    manifest = BankManifest.load(bank / "bank_manifest.json")
    for e in manifest.files:
        actual = hashlib.sha256((bank / e.file).read_bytes()).hexdigest()
        assert actual == e.sha256, f"hash mismatch {e.file}"


def test_bank_matches_declared_source_spec():
    """Every manifest entry maps to its declared source or synthesis recipe."""
    from tools.audio.bank_manifest import BankManifest
    from tools.audio.bank_spec import BANK_SPEC
    from tools.audio.promote_vehicle_replacement_sounds import REPLACEMENTS

    bank = Path("game/sounds/banks/commons")
    manifest = BankManifest.load(bank / "bank_manifest.json")
    by_key = {e.key: e for e in BANK_SPEC}
    assert len(manifest.files) == COMMONS_FILE_COUNT
    for entry in manifest.files:
        if entry.file in BACKFIRE_BANK_FILES or entry.file in LIMITER_BANK_FILES:
            src = Path(entry.synthesis["source_file"])
            assert src.is_file(), f"overlay source missing {src}"
            assert hashlib.sha256(src.read_bytes()).hexdigest() == entry.synthesis["source_sha256"]
            continue
        key = entry.file[:-4]
        if entry.synthesis.get("promotion_recipe") == "f1_2026_2008_replacement_overlay_v1":
            role, loop, _category = REPLACEMENTS[entry.file]
            assert entry.role == role
            assert entry.loop == loop
            src = Path(entry.synthesis["source_file"])
            assert src.is_file(), f"replacement source missing {src}"
            assert hashlib.sha256(src.read_bytes()).hexdigest() == entry.synthesis["source_sha256"]
            continue
        if entry.synthesis.get("promotion_recipe") == "v10_curated_alt_shift_v1":
            src = Path(entry.synthesis["source_file"])
            assert src.is_file(), f"curated shift source missing {src}"
            assert hashlib.sha256(src.read_bytes()).hexdigest() == entry.synthesis["source_sha256"]
            continue
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
