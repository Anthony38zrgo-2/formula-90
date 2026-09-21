#!/usr/bin/env python3
"""Render level-matched original-then-composite gearbox review files."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path

import numpy as np
import soundfile


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE_DIRECTORY = REPOSITORY_ROOT / "game/sounds/banks/v10-gp3"
DEFAULT_CANDIDATE_BANK_DIRECTORY = (
    REPOSITORY_ROOT
    / "reports/audio-v10/grand-prix-sampler/candidates/gearbox-fusion-v1/bank"
)
DEFAULT_OUTPUT_DIRECTORY = (
    REPOSITORY_ROOT
    / "reports/audio-v10/grand-prix-sampler/candidates/gearbox-fusion-v1/listening-review"
)
EVENT_DEFINITIONS = {
    "upshift": ("gearup.wav", "upshift_event.wav"),
    "downshift": ("geardn.wav", "downshift_event.wav"),
}


def root_mean_square(samples: np.ndarray) -> float:
    return float(np.sqrt(np.mean(np.square(samples, dtype=np.float64))))


def decibels_full_scale(amplitude: float) -> float:
    return 20.0 * math.log10(max(amplitude, 1e-12))


def read_mono(path: Path) -> tuple[np.ndarray, int]:
    samples, sample_rate = soundfile.read(path, dtype="float64", always_2d=True)
    return np.mean(samples, axis=1), int(sample_rate)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def render_review(
    label: str,
    source_path: Path,
    composite_path: Path,
    output_directory: Path,
) -> dict:
    original, original_sample_rate = read_mono(source_path)
    composite, composite_sample_rate = read_mono(composite_path)
    if original_sample_rate != composite_sample_rate:
        raise ValueError(f"sample rate mismatch for {label}")
    original_level = root_mean_square(original)
    composite_level = root_mean_square(composite)
    matched_level = min(original_level, composite_level)
    original_gain = matched_level / max(original_level, 1e-12)
    composite_gain = matched_level / max(composite_level, 1e-12)
    silence = np.zeros(round(0.25 * original_sample_rate), dtype=np.float64)
    review_audio = np.concatenate(
        [
            original * original_gain,
            silence,
            composite * composite_gain,
        ]
    )
    output_directory.mkdir(parents=True, exist_ok=True)
    output_path = output_directory / f"{label}_original_then_fused.wav"
    soundfile.write(output_path, review_audio, original_sample_rate, subtype="PCM_16")
    return {
        "label": label,
        "order": ["original", "silence_250_milliseconds", "fused"],
        "output_path": str(output_path.relative_to(REPOSITORY_ROOT)).replace("\\", "/"),
        "output_sha256": sha256_file(output_path),
        "sample_rate_hz": original_sample_rate,
        "original_source_filename": source_path.name,
        "composite_filename": composite_path.name,
        "original_duration_seconds": original.size / original_sample_rate,
        "composite_duration_seconds": composite.size / composite_sample_rate,
        "original_peak_decibels_full_scale": decibels_full_scale(
            float(np.max(np.abs(original)))
        ),
        "composite_peak_decibels_full_scale": decibels_full_scale(
            float(np.max(np.abs(composite)))
        ),
        "original_rms_decibels_full_scale": decibels_full_scale(original_level),
        "composite_rms_decibels_full_scale": decibels_full_scale(composite_level),
        "original_gain_decibels": decibels_full_scale(original_gain),
        "composite_gain_decibels": decibels_full_scale(composite_gain),
        "matched_rms_decibels_full_scale": decibels_full_scale(matched_level),
        "integrated_loudness_lufs": None,
        "integrated_loudness_reason": (
            "events are shorter than the 400 millisecond BS.1770 gating block"
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-directory", type=Path, default=DEFAULT_SOURCE_DIRECTORY)
    parser.add_argument(
        "--candidate-bank-directory",
        type=Path,
        default=DEFAULT_CANDIDATE_BANK_DIRECTORY,
    )
    parser.add_argument("--output-directory", type=Path, default=DEFAULT_OUTPUT_DIRECTORY)
    arguments = parser.parse_args()
    source_directory = arguments.source_directory.resolve()
    candidate_bank_directory = arguments.candidate_bank_directory.resolve()
    output_directory = arguments.output_directory.resolve()
    records = []
    for label, (source_filename, composite_filename) in EVENT_DEFINITIONS.items():
        records.append(
            render_review(
                label,
                source_directory / source_filename,
                candidate_bank_directory / composite_filename,
                output_directory,
            )
        )
    (output_directory / "review_manifest.json").write_text(
        json.dumps(records, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(f"listening review: {output_directory}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
