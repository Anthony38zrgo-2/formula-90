#!/usr/bin/env python3
"""Generate R24-inspired deterministic gearbox events for the Grand Prix sampler."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import shutil
import wave
from pathlib import Path

import numpy as np
from scipy.signal import butter, resample_poly, sosfiltfilt


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BANK_DIRECTORY = REPOSITORY_ROOT / "game/sounds/banks/v10-gp3"
DEFAULT_SOURCE_DIRECTORY = REPOSITORY_ROOT / "scripts/audio/source_assets/r24_gear_events"
DEFAULT_REPORT_PATH = (
    REPOSITORY_ROOT
    / "game/sounds/banks/v10-gp3/tobe/R24_GEAR_SYNTHESIS_GENERATION.json"
)
SAMPLE_RATE = 44_100
OUTPUT_DURATION_SECONDS = 0.320
TRUE_PEAK_CEILING_DECIBELS_FULL_SCALE = -4.0
RANDOM_SEED = 24_1994

EVENT_DEFINITIONS = {
    "upshift": {
        "original_filename": "upshift_original_body.wav",
        "output_filename": "gearup.wav",
        "body_gain_decibels": -8.0,
        "body_low_frequency_gain_decibels": -12.0,
        "attack_times_seconds": [0.007, 0.031],
        "attack_gains": [0.65, 0.35],
        "resonances": [
            (1_350.0, 0.060, 0.350),
            (2_250.0, 0.038, 0.300),
            (3_650.0, 0.024, 0.220),
            (5_450.0, 0.014, 0.120),
        ],
        "structural_resonances": [(285.0, 0.110, 0.120), (445.0, 0.075, 0.100)],
    },
    "downshift": {
        "original_filename": "downshift_original_body.wav",
        "output_filename": "geardn.wav",
        "body_gain_decibels": -9.0,
        "body_low_frequency_gain_decibels": -14.0,
        "attack_times_seconds": [0.006, 0.044],
        "attack_gains": [0.72, 0.40],
        "resonances": [
            (920.0, 0.080, 0.400),
            (1_620.0, 0.055, 0.350),
            (2_750.0, 0.034, 0.280),
            (4_250.0, 0.021, 0.180),
            (6_100.0, 0.012, 0.100),
        ],
        "structural_resonances": [(340.0, 0.095, 0.140), (610.0, 0.060, 0.120)],
    },
}


def amplitude_from_decibels(decibels: float) -> float:
    return 10.0 ** (decibels / 20.0)


def decibels_full_scale(amplitude: float) -> float:
    return 20.0 * math.log10(max(float(amplitude), 1e-12))


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_mono_pcm16(path: Path) -> tuple[np.ndarray, int]:
    with wave.open(str(path), "rb") as handle:
        sample_rate = handle.getframerate()
        channel_count = handle.getnchannels()
        sample_width = handle.getsampwidth()
        frame_count = handle.getnframes()
        payload = handle.readframes(frame_count)
    if sample_width != 2:
        raise ValueError(f"Expected PCM16 source: {path}")
    samples = np.frombuffer(payload, dtype="<i2").astype(np.float64)
    samples = samples.reshape(-1, channel_count).mean(axis=1) / 32768.0
    return samples, sample_rate


def write_mono_pcm16(path: Path, samples: np.ndarray, sample_rate: int, seed: int) -> None:
    generator = np.random.default_rng(seed)
    dither = (generator.random(samples.size) - generator.random(samples.size)) / 65536.0
    quantized = np.clip(np.round((samples + dither) * 32767.0), -32768, 32767).astype("<i2")
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(sample_rate)
        handle.writeframes(quantized.tobytes())


def filtered(samples: np.ndarray, sample_rate: int, filter_kind: str, cutoff) -> np.ndarray:
    sections = butter(4, cutoff, btype=filter_kind, fs=sample_rate, output="sos")
    return sosfiltfilt(sections, samples)


def place_signal(destination: np.ndarray, source: np.ndarray, start_frame: int) -> None:
    if start_frame >= destination.size:
        return
    frame_count = min(source.size, destination.size - start_frame)
    destination[start_frame:start_frame + frame_count] += source[:frame_count]


def synthesize_noise_attack(
    sample_rate: int,
    duration_seconds: float,
    gain: float,
    generator: np.random.Generator,
) -> np.ndarray:
    frame_count = max(1, round(duration_seconds * sample_rate))
    time_seconds = np.arange(frame_count, dtype=np.float64) / sample_rate
    noise = generator.standard_normal(frame_count)
    noise = filtered(noise, sample_rate, "bandpass", [1_150.0, 8_000.0])
    noise /= max(float(np.max(np.abs(noise))), 1e-12)
    envelope = (1.0 - np.exp(-time_seconds / 0.0008)) * np.exp(-time_seconds / 0.010)
    return noise * envelope * gain


def synthesize_resonance(
    sample_rate: int,
    frequency_hertz: float,
    decay_seconds: float,
    gain: float,
    phase_radians: float,
    duration_seconds: float,
) -> np.ndarray:
    frame_count = max(1, round(duration_seconds * sample_rate))
    time_seconds = np.arange(frame_count, dtype=np.float64) / sample_rate
    envelope = (1.0 - np.exp(-time_seconds / 0.0007)) * np.exp(-time_seconds / decay_seconds)
    carrier = np.sin(2.0 * math.pi * frequency_hertz * time_seconds + phase_radians)
    return gain * envelope * carrier


def true_peak_amplitude(samples: np.ndarray) -> float:
    reconstructed = resample_poly(samples, 8, 1, window=("kaiser", 12.0), padtype="line")
    return float(np.max(np.abs(reconstructed)))


def spectrum_metrics(samples: np.ndarray, sample_rate: int) -> dict:
    size = 1 << int(math.ceil(math.log2(samples.size)))
    padded = np.pad(samples - float(np.mean(samples)), (0, size - samples.size))
    power = np.abs(np.fft.rfft(padded * np.hanning(size))) ** 2
    frequencies = np.fft.rfftfreq(size, 1.0 / sample_rate)
    audible = (frequencies >= 20.0) & (frequencies <= 18_000.0)
    total = float(np.sum(power[audible])) + 1e-30
    bands = {}
    for low_hertz, high_hertz in [
        (20, 160),
        (160, 315),
        (315, 630),
        (630, 1_250),
        (1_250, 2_500),
        (2_500, 5_000),
        (5_000, 8_000),
        (8_000, 12_000),
    ]:
        selection = (frequencies >= low_hertz) & (frequencies < high_hertz)
        bands[f"{low_hertz}-{high_hertz}"] = 100.0 * float(np.sum(power[selection])) / total
    centroid = float(np.sum(frequencies[audible] * power[audible]) / total)
    cumulative = np.cumsum(power[audible])
    rolloff = float(frequencies[audible][np.searchsorted(cumulative, 0.85 * cumulative[-1])])
    return {
        "root_mean_square_decibels_full_scale": decibels_full_scale(
            float(np.sqrt(np.mean(samples * samples)))
        ),
        "sample_peak_decibels_full_scale": decibels_full_scale(float(np.max(np.abs(samples)))),
        "estimated_true_peak_decibels_full_scale": decibels_full_scale(
            true_peak_amplitude(samples)
        ),
        "spectral_centroid_hertz": centroid,
        "spectral_rolloff_85_percent_hertz": rolloff,
        "band_energy_percent": bands,
    }


def synthesize_event(
    original: np.ndarray,
    sample_rate: int,
    definition: dict,
    generator: np.random.Generator,
) -> np.ndarray:
    output_frame_count = round(OUTPUT_DURATION_SECONDS * sample_rate)
    original = original - float(np.mean(original))
    original = np.pad(original, (0, max(0, output_frame_count - original.size)))[:output_frame_count]
    low_body = filtered(original, sample_rate, "lowpass", 630.0)
    upper_body = original - low_body
    output = low_body * amplitude_from_decibels(definition["body_low_frequency_gain_decibels"])
    output += upper_body * amplitude_from_decibels(definition["body_gain_decibels"])

    for attack_time_seconds, attack_gain in zip(
        definition["attack_times_seconds"], definition["attack_gains"]
    ):
        attack = synthesize_noise_attack(sample_rate, 0.065, attack_gain, generator)
        place_signal(output, attack, round(attack_time_seconds * sample_rate))

    for frequency_hertz, decay_seconds, gain in definition["resonances"]:
        resonance = synthesize_resonance(
            sample_rate,
            frequency_hertz,
            decay_seconds,
            gain,
            generator.uniform(0.0, 2.0 * math.pi),
            min(0.22, decay_seconds * 5.0),
        )
        place_signal(output, resonance, round(definition["attack_times_seconds"][0] * sample_rate))

    for frequency_hertz, decay_seconds, gain in definition["structural_resonances"]:
        resonance = synthesize_resonance(
            sample_rate,
            frequency_hertz,
            decay_seconds,
            gain,
            generator.uniform(0.0, 2.0 * math.pi),
            min(0.28, decay_seconds * 5.0),
        )
        place_signal(output, resonance, round(definition["attack_times_seconds"][1] * sample_rate))

    fade_frame_count = round(0.008 * sample_rate)
    output[-fade_frame_count:] *= np.linspace(1.0, 0.0, fade_frame_count, endpoint=True)
    ceiling_amplitude = amplitude_from_decibels(TRUE_PEAK_CEILING_DECIBELS_FULL_SCALE)
    measured_true_peak = true_peak_amplitude(output)
    if measured_true_peak > ceiling_amplitude:
        output *= ceiling_amplitude / measured_true_peak
    return output


def initialize_original_sources(bank_directory: Path, source_directory: Path) -> None:
    source_directory.mkdir(parents=True, exist_ok=True)
    for definition in EVENT_DEFINITIONS.values():
        source_path = source_directory / definition["original_filename"]
        if source_path.exists():
            continue
        shutil.copyfile(bank_directory / definition["output_filename"], source_path)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bank-directory", type=Path, default=DEFAULT_BANK_DIRECTORY)
    parser.add_argument("--source-directory", type=Path, default=DEFAULT_SOURCE_DIRECTORY)
    parser.add_argument("--report-path", type=Path, default=DEFAULT_REPORT_PATH)
    arguments = parser.parse_args()
    bank_directory = arguments.bank_directory.resolve()
    source_directory = arguments.source_directory.resolve()
    report_path = arguments.report_path.resolve()
    initialize_original_sources(bank_directory, source_directory)
    generator = np.random.default_rng(RANDOM_SEED)
    records = []
    for event_name, definition in EVENT_DEFINITIONS.items():
        original_path = source_directory / definition["original_filename"]
        output_path = bank_directory / definition["output_filename"]
        original, sample_rate = read_mono_pcm16(original_path)
        if sample_rate != SAMPLE_RATE:
            original = resample_poly(original, SAMPLE_RATE, sample_rate)
            sample_rate = SAMPLE_RATE
        synthesized = synthesize_event(original, sample_rate, definition, generator)
        event_seed = RANDOM_SEED + len(records) + 1
        write_mono_pcm16(output_path, synthesized, sample_rate, event_seed)
        rendered, rendered_rate = read_mono_pcm16(output_path)
        records.append(
            {
                "event": event_name,
                "source_path": str(original_path.relative_to(REPOSITORY_ROOT)).replace("\\", "/"),
                "source_sha256": sha256_file(original_path),
                "output_path": str(output_path.relative_to(REPOSITORY_ROOT)).replace("\\", "/"),
                "output_sha256": sha256_file(output_path),
                "sample_rate_hertz": rendered_rate,
                "frames": int(rendered.size),
                "duration_seconds": rendered.size / rendered_rate,
                "random_seed": event_seed,
                "metrics": spectrum_metrics(rendered, rendered_rate),
            }
        )
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(records, indent=2) + "\n", encoding="utf-8")
    for record in records:
        print(f"{record['event']}: {record['output_sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
