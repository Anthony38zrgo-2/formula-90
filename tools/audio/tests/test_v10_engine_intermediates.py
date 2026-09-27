"""Contract tests for the generated v10 engine intermediate sources."""

from __future__ import annotations

import hashlib
import wave
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


def test_generated_intermediates_match_shipped_bank(
    generated: tuple[dict, dict[str, bytes]],
) -> None:
    metadata, outputs = generated
    for filename, payload in outputs.items():
        path = BANK_DIR / filename
        assert path.is_file(), filename
        assert path.read_bytes() == payload, filename
    metadata_path = BANK_DIR / generator.METADATA_FILENAME
    assert metadata_path.read_bytes() == canonical.serialize_json(metadata)


def test_generated_intermediates_are_deterministic(
    generated: tuple[dict, dict[str, bytes]],
) -> None:
    metadata, outputs = generated
    second_metadata, second_outputs = generator.build_intermediates(
        BANK_DIR, HISTORICAL_BANK_DIR
    )
    assert second_metadata == metadata
    assert sorted(second_outputs) == sorted(outputs)
    for filename, payload in outputs.items():
        assert payload == second_outputs[filename], filename


def test_generated_intermediates_use_integer_cycle_grids(
    generated: tuple[dict, dict[str, bytes]],
) -> None:
    metadata, _ = generated
    for record in metadata["intermediates"]:
        if record["recipe"] != "spectral_magnitude_morph_integer_cycles":
            continue
        frames = record["integer_cycle_count"] * record["samples_per_cycle"]
        assert record["frames"] == frames
        assert record["cycle_frequency_hertz"] == pytest.approx(
            generator.SAMPLE_RATE / record["samples_per_cycle"], rel=1e-12
        )
        assert record["reference_revolutions_per_minute"] == pytest.approx(
            record["cycle_frequency_hertz"]
            * canonical.REVOLUTIONS_PER_MINUTE_PER_HERTZ,
            rel=1e-12,
        )
        with wave.open(str(BANK_DIR / record["output_filename"]), "rb") as reader:
            assert reader.getnchannels() == 1
            assert reader.getsampwidth() == 3
            assert reader.getframerate() == generator.SAMPLE_RATE
            assert reader.getnframes() == frames


def test_generated_intermediates_are_pitch_consistent(
    generated: tuple[dict, dict[str, bytes]],
) -> None:
    metadata, _ = generated
    for record in metadata["intermediates"]:
        sample_rate, samples = canonical.read_wave_mono(
            BANK_DIR / record["output_filename"]
        )
        assert sample_rate == generator.SAMPLE_RATE
        reference_hertz = (
            record["reference_revolutions_per_minute"]
            / canonical.REVOLUTIONS_PER_MINUTE_PER_HERTZ
        )
        measurement = canonical.measure_cepstral_cycle_frequency(
            samples, reference_hertz
        )
        assert measurement is not None, record["output_filename"]
        measured_hertz, _, prominence = measurement
        assert (
            prominence >= canonical.REFERENCE_MEASUREMENT_MINIMUM_PROMINENCE_RATIO
        ), record["output_filename"]
        error_ratio = abs(measured_hertz - reference_hertz) / reference_hertz
        assert (
            error_ratio <= canonical.REFERENCE_MEASUREMENT_TOLERANCE_RATIO
        ), record["output_filename"]


def test_imported_maximum_matches_historical_source(
    generated: tuple[dict, dict[str, bytes]],
) -> None:
    metadata, outputs = generated
    for record in metadata["historical_sources"]:
        source_path = ROOT / record["path"]
        assert hashlib.sha256(source_path.read_bytes()).hexdigest() == record["sha256"]
    assert outputs["engine_off_maximum.wav"] == (
        HISTORICAL_BANK_DIR / "engine_coast_maximum.wav"
    ).read_bytes()
