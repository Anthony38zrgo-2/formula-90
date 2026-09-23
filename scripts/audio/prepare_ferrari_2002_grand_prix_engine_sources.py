from __future__ import annotations

import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path

import numpy as np
import soundfile
from scipy.signal import resample_poly
from scipy.optimize import minimize_scalar


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
ORIGINAL_DIRECTORY = Path(r"D:\ASETS\f12002")
SOURCE_BANK_DIRECTORY = REPOSITORY_ROOT / "game/sounds/banks/v10-gp3"
ENGINE_SOURCE_MAPPING = {
    "idle.wav": "98_int_idle.wav",
    "low_on.wav": "98_int_low.wav",
    "hi_on.wav": "98_int_med.wav",
    "hi_max_on.wav": "98_int_high_1.wav",
    "max_on.wav": "98_int_max_5.wav",
    "low_off.wav": "engine_coast_low.wav",
    "hi_off.wav": "engine_coast_high.wav",
    "max_off.wav": "engine_coast_maximum.wav",
}
ORIGINAL_SOURCE_SHA256 = {
    "idle.wav": "a2af839a00955f414b29e27fcae00837a00ca8b4ad1633dac75880a82cc00101",
    "low_on.wav": "9dccccec035b1553824ae815761b9893891126f06afb9133a7abf3add6faaae4",
    "hi_on.wav": "ccf087f6b41b4dc4f73ded9535688926b184ffeb6842b4315645c4dfe0ca2c31",
    "hi_max_on.wav": "638485f34e50b1f4df46a406acf548eb22448ce208cf40540408cd466b6f638b",
    "max_on.wav": "77a311a25e4e015004171e1f148721b17db95f1206d159f5a295adcb50f21e96",
    "low_off.wav": "a3ae253ddeaf19872af315305a2c54b7cc6025dba7a8efad7440bf4acecd9c55",
    "hi_off.wav": "4142964333dba9aae06a20d76827bc01e0956b8401b984179a23a347ec4d3c3c",
    "max_off.wav": "441e9231f1377ba9d7aa51e0c1630f21691cc0769623f9e70bc0bf5c109e3269",
}
SAMPLE_RATE = 44100
MAXIMUM_SEGMENT_SECONDS = 2.5
TARGET_TRUE_PEAK_DECIBELS_FULL_SCALE = -4.0
ENGINE_CYCLE_FREQUENCY_ESTIMATES = {
    "idle.wav": 37.8,
    "low_on.wav": 76.2,
    "hi_on.wav": 141.1,
    "hi_max_on.wav": 140.5,
    "max_on.wav": 149.0,
    "low_off.wav": 66.0,
    "hi_off.wav": 103.6,
    "max_off.wav": 137.8,
}


def load_bank_builder():
    builder_path = REPOSITORY_ROOT / "scripts/audio/build_grand_prix_sampler_bank.py"
    builder_specification = importlib.util.spec_from_file_location(
        "grand_prix_bank_builder", builder_path
    )
    if builder_specification is None or builder_specification.loader is None:
        raise RuntimeError(f"Cannot load {builder_path}")
    builder = importlib.util.module_from_spec(builder_specification)
    builder_specification.loader.exec_module(builder)
    return builder


def prepare_source(original_path: Path, output_path: Path, builder) -> dict:
    original_digest = hashlib.sha256(original_path.read_bytes()).hexdigest()
    if original_digest != ORIGINAL_SOURCE_SHA256[original_path.name]:
        raise ValueError(f"Unexpected source content: {original_path}")
    original_audio, original_sample_rate = soundfile.read(
        original_path, dtype="float64", always_2d=True
    )
    if original_sample_rate != SAMPLE_RATE or original_audio.shape[1] != 2:
        raise ValueError(f"Expected 44100 Hz stereo PCM source: {original_path}")
    segment_frames = min(
        len(original_audio), round(MAXIMUM_SEGMENT_SECONDS * SAMPLE_RATE)
    )
    segment_start = max(0, (len(original_audio) - segment_frames) // 2)
    segment = original_audio[segment_start : segment_start + segment_frames].mean(axis=1)
    segment -= float(np.mean(segment))
    sample_times = np.arange(len(segment)) / SAMPLE_RATE
    windowed_segment = segment * np.hanning(len(segment))
    estimated_frequency = ENGINE_CYCLE_FREQUENCY_ESTIMATES[original_path.name] * 5.0
    candidate_frequencies = np.linspace(estimated_frequency * 0.98, estimated_frequency * 1.02, 161)
    def harmonic_energy(frequency):
        return abs(np.dot(windowed_segment, np.exp(-2j * np.pi * frequency * sample_times))) ** 2
    strongest_index = max(range(len(candidate_frequencies)), key=lambda index: harmonic_energy(candidate_frequencies[index]))
    frequency_step = candidate_frequencies[1] - candidate_frequencies[0]
    strongest_frequency = candidate_frequencies[strongest_index]
    measured_frequency = minimize_scalar(
        lambda frequency: -harmonic_energy(frequency),
        bounds=(strongest_frequency - frequency_step, strongest_frequency + frequency_step),
        method="bounded",
    ).x / 5.0
    cycle_count = round(measured_frequency * 2.0)
    loop_frames = round(cycle_count * SAMPLE_RATE / measured_frequency)
    crossfade_frames = round(4.0 * SAMPLE_RATE / measured_frequency)
    loop_start = (len(segment) - loop_frames) // 2
    prepared_loop = segment[loop_start : loop_start + loop_frames].copy()
    crossfade_position = np.linspace(0.0, 1.0, crossfade_frames, endpoint=False)
    crossfade_weight = crossfade_position ** 2 * (3.0 - 2.0 * crossfade_position)
    prepared_loop[-crossfade_frames:] = (
        prepared_loop[-crossfade_frames:] * (1.0 - crossfade_weight)
        + segment[loop_start - crossfade_frames : loop_start] * crossfade_weight
    )
    spectrum = np.fft.rfft(prepared_loop)
    dominant_phase = np.angle(spectrum[cycle_count * 5])
    phase_shift_frames = -dominant_phase * loop_frames / (2.0 * np.pi * cycle_count * 5)
    spectrum *= np.exp(2j * np.pi * np.arange(len(spectrum)) * phase_shift_frames / loop_frames)
    prepared_loop = np.fft.irfft(spectrum, n=loop_frames)
    prepared_loop -= float(np.mean(prepared_loop))
    true_peak = builder.estimate_true_peak_amplitude(prepared_loop)
    if true_peak <= 0.0:
        raise ValueError(f"Silent source: {original_path}")
    target_peak = 10.0 ** (TARGET_TRUE_PEAK_DECIBELS_FULL_SCALE / 20.0)
    prepared_loop *= target_peak / true_peak
    temporary_output_path = output_path.with_suffix(".prepared.wav")
    temporary_output_path.write_bytes(builder.wav_bytes(prepared_loop, SAMPLE_RATE))
    os.replace(temporary_output_path, output_path)
    output_audio, output_sample_rate = soundfile.read(output_path, dtype="float64")
    measured_peak = float(
        np.max(
            np.abs(
                resample_poly(
                    output_audio, 8, 1, window=("kaiser", 12.0), padtype="line"
                )
            )
        )
    )
    print(
        f"{original_path.name} -> {output_path.name} "
        f"{output_sample_rate} Hz {len(output_audio)} frames "
        f"true_peak={20.0 * math.log10(measured_peak):.3f} dBFS "
        f"cycle_frequency={cycle_count * SAMPLE_RATE / loop_frames:.5f} Hz "
        f"sha256={hashlib.sha256(output_path.read_bytes()).hexdigest()}"
    )
    return {
        "original_filename": original_path.name,
        "original_sha256": original_digest,
        "sha256": hashlib.sha256(output_path.read_bytes()).hexdigest(),
        "measured_cycle_frequency_hertz": float(measured_frequency),
        "cycle_count": cycle_count,
        "frames": loop_frames,
        "reference_revolutions_per_minute": 120.0 * cycle_count * SAMPLE_RATE / loop_frames,
        "loop_crossfade_frames": crossfade_frames,
        "dominant_harmonic_phase_radians": float(np.angle(np.fft.rfft(output_audio)[cycle_count * 5])),
        "preparation_recipe": "integer_engine_cycles_dominant_harmonic_phase_aligned_v2",
    }


def main() -> None:
    builder = load_bank_builder()
    SOURCE_BANK_DIRECTORY.mkdir(parents=True, exist_ok=True)
    metadata = {}
    for original_filename, output_filename in ENGINE_SOURCE_MAPPING.items():
        metadata[output_filename] = prepare_source(
            ORIGINAL_DIRECTORY / original_filename,
            SOURCE_BANK_DIRECTORY / output_filename,
            builder,
        )
    (SOURCE_BANK_DIRECTORY / "engine_preparation.json").write_text(
        json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
