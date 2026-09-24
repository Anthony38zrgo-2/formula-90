#!/usr/bin/env python3
"""Approximate the R24 V10 timbre on the Grand Prix engine loops.

Builds a layered excitation from the measured harmonic comb: a band-limited
sawtooth, a resonant impulse train and a modulated formant texture, processed
through asymmetric saturation and slow roughness modulation. The layer is
spectrally balanced against an optionally low-shelf-attenuated bed, written as
deterministic PCM16 candidates with measurements. Nothing in the runtime bank
is modified unless --promote is requested.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import math
import sys
from pathlib import Path

import numpy as np
from scipy.signal import iirpeak, lfilter, resample_poly, welch

EXISTING_SCRIPT = Path(__file__).with_name("synthesize_engine_odd_harmonic_layer.py")
EXISTING_SPECIFICATION = importlib.util.spec_from_file_location(
    "odd_harmonic_layer", EXISTING_SCRIPT
)
odd = importlib.util.module_from_spec(EXISTING_SPECIFICATION)
sys.modules[EXISTING_SPECIFICATION.name] = odd
EXISTING_SPECIFICATION.loader.exec_module(odd)

DEFAULT_CANDIDATE_ROOT = (
    odd.REPOSITORY_ROOT
    / "reports/audio-v10/grand-prix-sampler/candidates/odd-harmonic-v2"
)
DEFAULT_BANK_DIRECTORY = odd.REPOSITORY_ROOT / "game/sounds/banks/v10-gp3"

SAW_LOWEST_HERTZ = 250.0
SAW_FREQUENCY_LIMIT_RATIO = 0.45
SAW_PHASE_RATIO = 0.6180339887498949

COMPONENT_WEIGHTS = {
    "sawtooth": 0.55,
    "resonant_impulse": 0.30,
    "formant_texture": 0.15,
}

RESONATORS = (
    (1_250.0, 6.0, 1.0),
    (2_500.0, 6.0, 0.70),
    (5_000.0, 5.0, 0.40),
)
IMPULSE_ENVELOPE_BAND_HERTZ = (300.0, 6_000.0)

TEXTURE_BANDS = (
    (1_000.0, 1_500.0, 1.0),
    (4_000.0, 7_000.0, 0.60),
)
TEXTURE_MODULATION_HERTZ = 4.5
TEXTURE_MODULATION_DEPTH = 0.40

ROUGHNESS_COMPONENTS = (
    (4.5, 0.030),
    (7.3, 0.020),
)

SATURATION_DRIVE = 4.0
SATURATION_BIAS = 0.20
SATURATION_OVERSAMPLING = 8

BED_GRAVE_CURVES = {
    "98_int_max_5.wav": (
        (20.0, 0.0),
        (60.0, 0.0),
        (90.0, -4.0),
        (120.0, -10.0),
        (200.0, -6.0),
        (315.0, -4.0),
        (450.0, -3.0),
        (630.0, -1.5),
        (900.0, -1.5),
        (1_250.0, 0.0),
        (20_000.0, 0.0),
    ),
    "98_int_med.wav": (
        (20.0, 0.0),
        (60.0, 0.0),
        (90.0, -1.5),
        (120.0, -4.0),
        (200.0, -2.5),
        (315.0, -1.5),
        (450.0, -0.75),
        (630.0, 0.0),
        (20_000.0, 0.0),
    ),
}

SPECTRUM_TARGET_BANDS = (
    (630.0, 1_250.0, 2.0),
    (1_250.0, 2_500.0, 3.5),
    (2_500.0, 5_000.0, 3.5),
    (5_000.0, 8_000.0, 3.0),
    (8_000.0, 12_000.0, 2.0),
)
SPECTRUM_TARGET_ITERATIONS = 8
SPECTRUM_TARGET_LOOP_GAIN = 0.6
WELCH_SEGMENT_FRAMES = 1 << 14


def spectrum_target_key(low: float, high: float) -> str:
    return f"{int(low)}_{int(high)}"


def spectral_grave_equalization(
    samples: np.ndarray, sample_rate: int, curve: tuple
) -> np.ndarray:
    spectrum = np.fft.rfft(samples)
    frequencies = np.fft.rfftfreq(samples.size, 1.0 / sample_rate)
    node_frequencies = np.asarray([point[0] for point in curve])
    node_amplitudes = np.asarray(
        [10.0 ** (point[1] / 20.0) for point in curve]
    )
    mask = np.interp(
        np.log(np.maximum(frequencies, node_frequencies[0])),
        np.log(node_frequencies),
        node_amplitudes,
    )
    return np.fft.irfft(spectrum * mask, n=samples.size)


def build_bed(original: np.ndarray, filename: str, sample_rate: int) -> np.ndarray:
    bed = original * odd.amplitude_from_decibels(odd.BED_ATTENUATION_DECIBELS)
    curve = BED_GRAVE_CURVES.get(filename)
    if curve:
        bed = spectral_grave_equalization(bed, sample_rate, curve)
    return bed


def band_limited_sawtooth(
    frame_count: int, sample_rate: int, fundamental_hertz: float
) -> np.ndarray:
    time_seconds = np.arange(frame_count, dtype=np.float64) / sample_rate
    sawtooth = np.zeros(frame_count, dtype=np.float64)
    harmonic_index = 1
    frequency_limit = SAW_FREQUENCY_LIMIT_RATIO * sample_rate
    while True:
        frequency_hertz = harmonic_index * fundamental_hertz
        if frequency_hertz > frequency_limit:
            break
        if frequency_hertz >= SAW_LOWEST_HERTZ:
            phase_radians = 2.0 * math.pi * (
                (harmonic_index * SAW_PHASE_RATIO) % 1.0
            )
            sawtooth += (
                np.cos(2.0 * math.pi * frequency_hertz * time_seconds + phase_radians)
                / harmonic_index
            )
        harmonic_index += 1
    return sawtooth


def resonant_impulse_excitation(
    frame_count: int,
    sample_rate: int,
    fundamental_hertz: float,
    envelope: np.ndarray,
) -> np.ndarray:
    period_frames = max(1, round(sample_rate / fundamental_hertz))
    excitation = np.zeros(frame_count, dtype=np.float64)
    excitation[::period_frames] = 1.0
    excitation *= envelope
    resonant = np.zeros(frame_count, dtype=np.float64)
    for center_hertz, quality, weight in RESONATORS:
        numerator, denominator = iirpeak(center_hertz / (sample_rate / 2.0), quality)
        resonant += weight * lfilter(numerator, denominator, excitation)
    return resonant


def formant_texture(
    frame_count: int,
    sample_rate: int,
    envelope: np.ndarray,
    generator: np.random.Generator,
) -> np.ndarray:
    noise = generator.standard_normal(frame_count)
    texture = np.zeros(frame_count, dtype=np.float64)
    for low_hertz, high_hertz, weight in TEXTURE_BANDS:
        texture += weight * odd.bandpass_filter(noise, sample_rate, low_hertz, high_hertz)
    time_seconds = np.arange(frame_count, dtype=np.float64) / sample_rate
    modulation = 1.0 + TEXTURE_MODULATION_DEPTH * np.sin(
        2.0 * math.pi * TEXTURE_MODULATION_HERTZ * time_seconds
    )
    texture *= envelope * modulation
    return texture


def normalize_root_mean_square(samples: np.ndarray) -> np.ndarray:
    magnitude = odd.root_mean_square(samples)
    if magnitude <= 1e-12:
        return samples
    return samples / magnitude


def asymmetric_saturation(samples: np.ndarray) -> np.ndarray:
    oversampled = resample_poly(
        samples, SATURATION_OVERSAMPLING, 1, window=("kaiser", 12.0)
    )
    shaped = np.tanh(SATURATION_DRIVE * oversampled + SATURATION_BIAS) - math.tanh(
        SATURATION_BIAS
    )
    restored = resample_poly(
        shaped, 1, SATURATION_OVERSAMPLING, window=("kaiser", 12.0)
    )
    restored -= float(np.mean(restored))
    return normalize_root_mean_square(restored)


def crest_factor(samples: np.ndarray) -> float:
    magnitude = odd.root_mean_square(samples)
    if magnitude <= 1e-12:
        return 0.0
    return float(np.max(np.abs(samples)) / magnitude)


def apply_roughness_modulation(samples: np.ndarray, sample_rate: int) -> np.ndarray:
    time_seconds = np.arange(samples.size, dtype=np.float64) / sample_rate
    modulation = np.ones(samples.size, dtype=np.float64)
    for index, (frequency_hertz, depth) in enumerate(ROUGHNESS_COMPONENTS):
        modulation += depth * np.sin(
            2.0 * math.pi * frequency_hertz * time_seconds + 0.7 * index
        )
    return samples * modulation


def apply_spectrum_mask(
    layer: np.ndarray, sample_rate: int, band_amplitudes: dict[str, float]
) -> np.ndarray:
    spectrum = np.fft.rfft(layer)
    frequencies = np.fft.rfftfreq(layer.size, 1.0 / sample_rate)
    node_frequencies = [0.0, 600.0]
    node_values = [0.0, 0.0]
    for low_hertz, high_hertz, _ in SPECTRUM_TARGET_BANDS:
        key = spectrum_target_key(low_hertz, high_hertz)
        node_frequencies.append(math.sqrt(low_hertz * high_hertz))
        node_values.append(band_amplitudes[key])
    node_frequencies.extend([16_000.0, sample_rate / 2.0])
    node_values.extend([0.0, 0.0])
    mask = np.interp(frequencies, node_frequencies, node_values)
    return np.fft.irfft(spectrum * mask, n=layer.size)


def welch_band_powers(samples: np.ndarray, sample_rate: int) -> tuple[np.ndarray, np.ndarray]:
    frequencies, spectrum = welch(
        samples, fs=sample_rate, nperseg=min(samples.size, WELCH_SEGMENT_FRAMES)
    )
    return frequencies, spectrum


def apply_peak_budget(layer: np.ndarray, peak_budget: float) -> np.ndarray:
    peak = float(np.max(np.abs(layer))) if layer.size else 0.0
    if peak <= peak_budget or peak <= 1e-12:
        return layer
    return layer * (peak_budget / peak)


def balance_layer_to_targets(
    base_layer: np.ndarray, bed: np.ndarray, sample_rate: int, peak_budget: float
) -> tuple[np.ndarray, dict[str, float], dict[str, float]]:
    band_amplitudes = {
        spectrum_target_key(low, high): 1.0 for low, high, _ in SPECTRUM_TARGET_BANDS
    }
    frequencies, bed_spectrum = welch_band_powers(bed, sample_rate)
    masked_layer = base_layer
    achieved_decibels = {}
    for _ in range(SPECTRUM_TARGET_ITERATIONS):
        masked_layer = apply_spectrum_mask(base_layer, sample_rate, band_amplitudes)
        masked_layer = apply_peak_budget(masked_layer, peak_budget)
        _, mixed_spectrum = welch_band_powers(bed + masked_layer, sample_rate)
        achieved_decibels = {}
        for low_hertz, high_hertz, target_decibels in SPECTRUM_TARGET_BANDS:
            key = spectrum_target_key(low_hertz, high_hertz)
            selection = (frequencies >= low_hertz) & (frequencies < high_hertz)
            bed_power = float(np.sum(bed_spectrum[selection])) + 1e-30
            mixed_power = float(np.sum(mixed_spectrum[selection])) + 1e-30
            achieved = 10.0 * math.log10(mixed_power / bed_power)
            achieved_decibels[key] = achieved
            error_decibels = target_decibels - achieved
            correction = 10.0 ** (
                SPECTRUM_TARGET_LOOP_GAIN * error_decibels / 20.0
            )
            band_amplitudes[key] = float(
                min(max(band_amplitudes[key] * correction, 0.0), 64.0)
            )
    masked_layer = apply_spectrum_mask(base_layer, sample_rate, band_amplitudes)
    masked_layer = apply_peak_budget(masked_layer, peak_budget)
    return masked_layer, band_amplitudes, achieved_decibels


def build_layer(
    original: np.ndarray,
    bed: np.ndarray,
    filename: str,
    fundamental_hertz: float,
    sample_rate: int,
    seed: int,
) -> tuple[np.ndarray, dict]:
    frame_count = original.size
    envelope = odd.upper_band_envelope(original, sample_rate)
    sawtooth = normalize_root_mean_square(
        band_limited_sawtooth(frame_count, sample_rate, fundamental_hertz)
    )
    resonators = normalize_root_mean_square(
        resonant_impulse_excitation(frame_count, sample_rate, fundamental_hertz, envelope)
    )
    texture = normalize_root_mean_square(
        formant_texture(frame_count, sample_rate, envelope, np.random.default_rng(seed))
    )
    layer = (
        COMPONENT_WEIGHTS["sawtooth"] * sawtooth
        + COMPONENT_WEIGHTS["resonant_impulse"] * resonators
        + COMPONENT_WEIGHTS["formant_texture"] * texture
    )
    crest_after_mix = crest_factor(layer)
    layer = asymmetric_saturation(layer)
    layer = apply_roughness_modulation(layer, sample_rate)
    crest_after_saturation = crest_factor(layer)
    layer = layer * envelope
    bed_peak = float(np.max(np.abs(bed))) if bed.size else 0.0
    peak_budget = max(0.0, odd.PEAK_CEILING - bed_peak) * 0.9
    layer, band_amplitudes, achieved_decibels = balance_layer_to_targets(
        layer, bed, sample_rate, peak_budget
    )
    shift_frames, polarity, aligned_peak = odd.choose_layer_shift(
        bed, layer, sample_rate, fundamental_hertz
    )
    aligned_layer = polarity * np.roll(layer, shift_frames)
    diagnostics = {
        "filename": filename,
        "band_amplitudes": band_amplitudes,
        "balanced_band_decibels": achieved_decibels,
        "layer_shift_frames": int(shift_frames),
        "layer_polarity": float(polarity),
        "aligned_bed_sample_peak": float(aligned_peak),
        "layer_crest_factor": crest_factor(aligned_layer),
        "crest_after_mix": crest_after_mix,
        "crest_after_saturation": crest_after_saturation,
        "peak_budget": peak_budget,
        "bed_peak": bed_peak,
        "grave_equalization_curve": [
            {"frequency_hertz": frequency, "gain_decibels": gain}
            for frequency, gain in BED_GRAVE_CURVES.get(filename, ())
        ],
    }
    return aligned_layer, diagnostics


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-directory", type=Path, default=odd.DEFAULT_SOURCE_DIRECTORY)
    parser.add_argument("--candidate-root", type=Path, default=DEFAULT_CANDIDATE_ROOT)
    parser.add_argument("--bank-directory", type=Path, default=DEFAULT_BANK_DIRECTORY)
    parser.add_argument(
        "--levels",
        nargs="+",
        choices=sorted(odd.LEVEL_OFFSETS_DECIBELS),
        default=sorted(odd.LEVEL_OFFSETS_DECIBELS),
    )
    parser.add_argument(
        "--promote",
        choices=["none", *sorted(odd.LEVEL_OFFSETS_DECIBELS)],
        default="none",
    )
    arguments = parser.parse_args()
    source_directory = arguments.source_directory.resolve()
    candidate_root = arguments.candidate_root.resolve()
    bank_directory = arguments.bank_directory.resolve()

    builder = odd.load_builder_module()
    originals: dict[str, np.ndarray] = {}
    beds: dict[str, np.ndarray] = {}
    layers: dict[str, np.ndarray] = {}
    fundamentals: dict[str, float] = {}
    generation_records = []
    for filename, definition in odd.LOOP_SOURCE_DEFINITIONS.items():
        source_path = source_directory / filename
        source_hash = odd.sha256_file(source_path)
        if source_hash != definition["sha256"]:
            raise ValueError(f"source hash mismatch for {filename}")
        original, sample_rate = odd.read_mono_pcm16(source_path)
        if sample_rate != odd.SAMPLE_RATE:
            raise ValueError(f"unexpected sample rate for {filename}: {sample_rate}")
        fundamental_hertz = odd.detect_fundamental_hertz(builder, original, sample_rate)
        expected = float(definition["expected_comb_spacing_hz"])
        if abs(fundamental_hertz - expected) / expected > odd.FUNDAMENTAL_TOLERANCE_RATIO:
            raise ValueError(
                f"fundamental mismatch for {filename}: {fundamental_hertz} != {expected}"
            )
        seed = odd.RANDOM_SEED + int(
            hashlib.sha256(filename.encode("utf-8")).hexdigest()[:8], 16
        )
        bed = build_bed(original, filename, sample_rate)
        layer, diagnostics = build_layer(
            original, bed, filename, fundamental_hertz, sample_rate, seed
        )
        originals[filename] = original
        beds[filename] = bed
        layers[filename] = layer
        fundamentals[filename] = fundamental_hertz
        diagnostics["detected_fundamental_hertz"] = fundamental_hertz
        diagnostics["source_sha256"] = source_hash
        generation_records.append(diagnostics)

    processed_by_level: dict[str, dict[str, np.ndarray]] = {
        level_name: {} for level_name in arguments.levels
    }
    level_treatments: dict[str, dict[str, dict[str, float]]] = {
        level_name: {} for level_name in arguments.levels
    }
    per_sample_records = []
    for filename, definition in odd.LOOP_SOURCE_DEFINITIONS.items():
        original = originals[filename]
        bed = beds[filename]
        record = {
            "filename": filename,
            "asset_id": definition["asset_id"],
            "source_sha256": definition["sha256"],
            "fundamental_hertz": fundamentals[filename],
            "before": odd.measure(original, odd.SAMPLE_RATE),
            "bed": odd.measure(bed, odd.SAMPLE_RATE),
            "levels": {},
        }
        for level_name in arguments.levels:
            level_offset = odd.amplitude_from_decibels(
                odd.LEVEL_OFFSETS_DECIBELS[level_name]
            )
            scaled_layer = layers[filename] * level_offset
            limited, reduction_decibels = odd.limit_layer_peak_excess(
                bed, scaled_layer, odd.SAMPLE_RATE
            )
            processed, clipped_sample_percent = odd.soft_clip_peak(limited)
            processed_by_level[level_name][filename] = processed
            level_treatments[level_name][filename] = {
                "layer_reduction_decibels": reduction_decibels,
                "hard_excess_sample_percent": clipped_sample_percent,
            }
            record["levels"][level_name] = {
                "level": level_name,
                "level_offset_decibels": odd.LEVEL_OFFSETS_DECIBELS[level_name],
                "layer_reduction_decibels": reduction_decibels,
                "hard_excess_sample_percent": clipped_sample_percent,
                "band_delta_vs_bed_decibels": odd.band_delta_decibels(
                    bed, processed, odd.SAMPLE_RATE
                ),
                "band_delta_vs_original_decibels": odd.band_delta_decibels(
                    original, processed, odd.SAMPLE_RATE
                ),
            }
        per_sample_records.append(record)

    common_gains = {}
    for level_name in arguments.levels:
        ceiling_amplitude = odd.amplitude_from_decibels(
            odd.TRUE_PEAK_CEILING_DECIBELS_FULL_SCALE
        )
        headroom = 1.0
        for processed in processed_by_level[level_name].values():
            true_peak = odd.estimate_true_peak_amplitude(processed)
            if true_peak > ceiling_amplitude:
                headroom = min(headroom, ceiling_amplitude / true_peak)
        common_gains[level_name] = headroom

    if arguments.promote == "none":
        for level_name in arguments.levels:
            for filename, processed in processed_by_level[level_name].items():
                output = processed * common_gains[level_name]
                output_path = candidate_root / "samples" / level_name / filename
                seed = odd.RANDOM_SEED + int(
                    hashlib.sha256(f"{level_name}:{filename}".encode("utf-8")).hexdigest()[:8],
                    16,
                )
                odd.write_mono_pcm16(output_path, output, odd.SAMPLE_RATE, seed)
                odd.write_level_matched_ab(
                    candidate_root / "ab" / f"{filename}.{level_name}.wav",
                    originals[filename],
                    output,
                    odd.SAMPLE_RATE,
                )
                odd.render_before_after_plot(
                    candidate_root / "plots" / f"{filename}.{level_name}.png",
                    originals[filename],
                    output,
                    odd.SAMPLE_RATE,
                    f"{filename} {level_name}",
                )
                for record in per_sample_records:
                    if record["filename"] == filename:
                        record["levels"][level_name]["output_sha256"] = odd.sha256_file(
                            output_path
                        )
                        record["levels"][level_name]["output_path"] = str(
                            output_path.relative_to(odd.REPOSITORY_ROOT)
                        ).replace("\\", "/")
                        record["levels"][level_name]["applied_common_gain"] = common_gains[
                            level_name
                        ]
                        record["levels"][level_name]["result"] = odd.measure(
                            output, odd.SAMPLE_RATE
                        )
        measurements_directory = candidate_root / "measurements"
        measurements_directory.mkdir(parents=True, exist_ok=True)
        (measurements_directory / "per_sample.json").write_text(
            json.dumps(per_sample_records, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        generation = {
            "tool": "scripts/audio/synthesize_r24_formant_engine_layer.py",
            "random_seed": odd.RANDOM_SEED,
            "component_weights": COMPONENT_WEIGHTS,
            "resonators": [
                {"center_hertz": center, "quality": quality, "weight": weight}
                for center, quality, weight in RESONATORS
            ],
            "texture_bands": [
                {"low_hertz": low, "high_hertz": high, "weight": weight}
                for low, high, weight in TEXTURE_BANDS
            ],
            "roughness_components": [
                {"frequency_hertz": frequency, "depth": depth}
                for frequency, depth in ROUGHNESS_COMPONENTS
            ],
            "saturation_drive": SATURATION_DRIVE,
            "saturation_bias": SATURATION_BIAS,
            "saturation_oversampling": SATURATION_OVERSAMPLING,
            "bed_attenuation_decibels": odd.BED_ATTENUATION_DECIBELS,
            "bed_grave_curves": {
                filename: [
                    {"frequency_hertz": frequency, "gain_decibels": gain}
                    for frequency, gain in curve
                ]
                for filename, curve in BED_GRAVE_CURVES.items()
            },
            "spectrum_target_bands": [
                {"low_hertz": low, "high_hertz": high, "increase_decibels": increase}
                for low, high, increase in SPECTRUM_TARGET_BANDS
            ],
            "true_peak_ceiling_decibels_full_scale": odd.TRUE_PEAK_CEILING_DECIBELS_FULL_SCALE,
            "level_treatments": level_treatments,
            "common_gains": common_gains,
            "recommended_profile_engine_gain": {
                level_name: round(
                    odd.amplitude_from_decibels(-odd.BED_ATTENUATION_DECIBELS)
                    / common_gains[level_name],
                    4,
                )
                for level_name in arguments.levels
            },
            "sources": generation_records,
        }
        (candidate_root / "generation.json").write_text(
            json.dumps(generation, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        print(f"candidate root: {candidate_root}")
        return 0

    if arguments.promote not in arguments.levels:
        raise ValueError("--promote requires the promoted level to be rendered")
    for filename in odd.LOOP_SOURCE_DEFINITIONS:
        output = processed_by_level[arguments.promote][filename] * common_gains[arguments.promote]
        output_path = bank_directory / filename
        seed = odd.RANDOM_SEED + int(hashlib.sha256(filename.encode("utf-8")).hexdigest()[:8], 16)
        odd.write_mono_pcm16(output_path, output, odd.SAMPLE_RATE, seed)
        print(f"promoted {arguments.promote} {filename} sha256={odd.sha256_file(output_path)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
