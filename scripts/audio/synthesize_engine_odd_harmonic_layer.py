#!/usr/bin/env python3
"""Add a phase-coherent odd-harmonic layer to the Grand Prix engine loops.

Reads the four original steady engine loops, measures the harmonic comb spacing
of each recording, projects every odd multiple of that spacing inside the
medium-high and high band, synthesizes the square-like partials with the
measured phase, shapes their spectral envelope to a conservative per-band
target, modulates the layer with the recording's own upper-band envelope, and
writes deterministic PCM16 candidates plus measurements. The original bank is
not modified unless --promote is requested.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import math
import wave
from dataclasses import dataclass
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
import pyloudnorm
import soundfile
from scipy.ndimage import minimum_filter1d
from scipy.signal import butter, resample_poly, sosfiltfilt, welch

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE_DIRECTORY = REPOSITORY_ROOT / "source-assets/audio/legacy-f1-1998"
DEFAULT_CANDIDATE_ROOT = (
    REPOSITORY_ROOT / "reports/audio-v10/grand-prix-sampler/candidates/odd-harmonic-v1"
)
DEFAULT_BANK_DIRECTORY = REPOSITORY_ROOT / "game/sounds/banks/v10-gp3"
BUILDER_PATH = REPOSITORY_ROOT / "scripts/audio/build_grand_prix_sampler_bank.py"

SAMPLE_RATE = 44_100
TRUE_PEAK_CEILING_DECIBELS_FULL_SCALE = -1.0
RANDOM_SEED = 20_260_921
DITHER_BIT_DEPTH = 16

FUNDAMENTAL_SEARCH_LOW_HERTZ = 20.0
FUNDAMENTAL_SEARCH_HIGH_HERTZ = 180.0
FUNDAMENTAL_TOLERANCE_RATIO = 0.01

BAND_LOWER_HERTZ = 700.0
BAND_LOWER_ROLLIN_HERTZ = 1_400.0
BAND_UPPER_ROLLOFF_HERTZ = 9_500.0
BAND_UPPER_HERTZ = 11_000.0
BAND_EDGE_FREQUENCY_LIMIT_RATIO = 0.45
PARTIAL_AMPLITUDE_EXPONENT = 1.0
PHASE_DISPERSION_THRESHOLD_HERTZ = 0.0
PHASE_DISPERSION_RATIO = 0.6180339887498949

SPECTRUM_TARGET_BANDS = (
    (630.0, 1_250.0, 0.0),
    (1_250.0, 2_500.0, 0.5),
    (2_500.0, 5_000.0, 1.0),
    (5_000.0, 8_000.0, 2.5),
    (8_000.0, 12_000.0, 2.0),
)
SPECTRUM_TARGET_ITERATIONS = 3

ENVELOPE_BAND_HERTZ = (1_000.0, 8_000.0)
ENVELOPE_WINDOW_SECONDS = 0.030
ENVELOPE_HOP_SECONDS = 0.005
ENVELOPE_SMOOTHING_SECONDS = 0.050
ENVELOPE_MINIMUM_GAIN = 0.25
ENVELOPE_MAXIMUM_GAIN = 4.0

PEAK_CEILING = 0.9999
BED_ATTENUATION_DECIBELS = -3.0
PHASE_SHIFT_SEARCH_STEPS = 32
SOFT_CLIP_THRESHOLD = 0.995
OVERSAMPLING_FACTOR = 8
HEADROOM_GUARD_SECONDS = 0.0015

MEASUREMENT_BAND_EDGES_HERTZ = (
    (20.0, 160.0),
    (160.0, 315.0),
    (315.0, 630.0),
    (630.0, 1_250.0),
    (1_250.0, 2_500.0),
    (2_500.0, 5_000.0),
    (5_000.0, 8_000.0),
    (8_000.0, 12_000.0),
)

LEVEL_OFFSETS_DECIBELS = {
    "level_a": -3.5,
    "level_b": 0.0,
    "level_c": 3.5,
}

LOOP_SOURCE_DEFINITIONS = {
    "98_int_low.wav": {
        "asset_id": "engine_low_loop",
        "sha256": "2eac0ef746c818ecac50fe77ca0099894f4762d55428a3cddfeb3007a6092cc5",
        "expected_comb_spacing_hz": 62.52000000010685,
    },
    "98_int_med.wav": {
        "asset_id": "engine_medium_loop",
        "sha256": "d1a527d5ddf3ec6d4f38127d1c6a3b00d7b7771d5473f68315e4352c98db4007",
        "expected_comb_spacing_hz": 69.18000000010079,
    },
    "98_int_high_1.wav": {
        "asset_id": "engine_high_loop",
        "sha256": "fc68bc279e3f8098be009be24eb4249c4ab76ab805e595af7bc5c509b1413477",
        "expected_comb_spacing_hz": 94.28000000007796,
    },
    "98_int_max_5.wav": {
        "asset_id": "engine_maximum_loop",
        "sha256": "f6b6cd4389fee32cb5f4b41a550e7aa146d4a5d0912fda86551dabfd598aa50b",
        "expected_comb_spacing_hz": 128.9900000000464,
    },
}


@dataclass
class HarmonicPartial:
    harmonic_index: int
    frequency_hertz: float
    measured_amplitude: float
    measured_phase_radians: float
    synthesized_phase_radians: float
    base_shape_amplitude: float
    edge_gain: float
    spectrum_weight: float = 1.0


def load_builder_module():
    import sys

    specification = importlib.util.spec_from_file_location(
        "build_grand_prix_sampler_bank", BUILDER_PATH
    )
    module = importlib.util.module_from_spec(specification)
    sys.modules[specification.name] = module
    specification.loader.exec_module(module)
    return module


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
        raise ValueError(f"expected PCM16 source: {path}")
    samples = np.frombuffer(payload, dtype="<i2").astype(np.float64)
    samples = samples.reshape(-1, channel_count).mean(axis=1) / 32768.0
    return samples, sample_rate


def write_mono_pcm16(path: Path, samples: np.ndarray, sample_rate: int, seed: int) -> None:
    generator = np.random.default_rng(seed)
    dither = (generator.random(samples.size) - generator.random(samples.size)) / (
        1 << DITHER_BIT_DEPTH
    )
    quantized = np.clip(np.round((samples + dither) * 32767.0), -32768, 32767).astype("<i2")
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(sample_rate)
        handle.writeframes(quantized.tobytes())


def write_float_wav(path: Path, samples: np.ndarray, sample_rate: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    soundfile.write(str(path), np.asarray(samples, dtype=np.float32), sample_rate, subtype="FLOAT")


def amplitude_from_decibels(decibels: float) -> float:
    return 10.0 ** (decibels / 20.0)


def decibels_full_scale(amplitude: float) -> float:
    return 20.0 * math.log10(max(float(amplitude), 1e-12))


def root_mean_square(samples: np.ndarray) -> float:
    if samples.size == 0:
        return 0.0
    return float(np.sqrt(np.mean(np.asarray(samples, dtype=np.float64) ** 2)))


def bandpass_filter(
    samples: np.ndarray, sample_rate: int, low_hertz: float, high_hertz: float
) -> np.ndarray:
    nyquist = sample_rate / 2.0
    low = max(low_hertz / nyquist, 1e-6)
    high = min(high_hertz / nyquist, 0.999999)
    if low >= high:
        return np.zeros_like(samples)
    sections = butter(4, [low, high], btype="bandpass", output="sos")
    return sosfiltfilt(sections, samples)


def band_power(
    samples: np.ndarray, sample_rate: int, low_hertz: float, high_hertz: float
) -> float:
    band = bandpass_filter(samples, sample_rate, low_hertz, high_hertz)
    return float(np.sum(band * band))


def estimate_true_peak_amplitude(samples: np.ndarray) -> float:
    if samples.size == 0:
        return 0.0
    reconstructed = resample_poly(samples, 8, 1, window=("kaiser", 12.0), padtype="line")
    return float(np.max(np.abs(reconstructed)))


def integrated_loudness_lufs(samples: np.ndarray, sample_rate: int) -> float | None:
    if samples.size < sample_rate // 2:
        return None
    meter = pyloudnorm.Meter(sample_rate)
    try:
        value = meter.integrated_loudness(np.asarray(samples, dtype=np.float64))
    except ValueError:
        return None
    if not math.isfinite(value):
        return None
    return float(value)


def band_energy_percentages(samples: np.ndarray, sample_rate: int) -> dict[str, float]:
    if samples.size < 64:
        return {f"{int(low)}_{int(high)}": 0.0 for low, high in MEASUREMENT_BAND_EDGES_HERTZ}
    frequencies, power = welch(samples, fs=sample_rate, nperseg=min(samples.size, 1 << 14))
    total = float(np.sum(power)) + 1e-30
    shares = {}
    for low, high in MEASUREMENT_BAND_EDGES_HERTZ:
        selection = (frequencies >= low) & (frequencies < high)
        shares[f"{int(low)}_{int(high)}"] = float(100.0 * np.sum(power[selection]) / total)
    return shares


def spectral_centroid_hertz(samples: np.ndarray, sample_rate: int) -> float:
    if samples.size < 64:
        return 0.0
    frequencies, power = welch(samples, fs=sample_rate, nperseg=min(samples.size, 1 << 14))
    total = float(np.sum(power)) + 1e-30
    return float(np.sum(frequencies * power) / total)


def spectral_rolloff_hertz(samples: np.ndarray, sample_rate: int, fraction: float = 0.85) -> float:
    if samples.size < 64:
        return 0.0
    frequencies, power = welch(samples, fs=sample_rate, nperseg=min(samples.size, 1 << 14))
    cumulative = np.cumsum(power)
    target = fraction * float(cumulative[-1])
    index = int(np.searchsorted(cumulative, target))
    index = min(index, len(frequencies) - 1)
    return float(frequencies[index])


def measure(samples: np.ndarray, sample_rate: int) -> dict:
    true_peak = estimate_true_peak_amplitude(samples)
    return {
        "frames": int(samples.size),
        "duration_seconds": float(samples.size / sample_rate),
        "root_mean_square_decibels_full_scale": decibels_full_scale(root_mean_square(samples)),
        "sample_peak_decibels_full_scale": decibels_full_scale(
            float(np.max(np.abs(samples))) if samples.size else 0.0
        ),
        "estimated_true_peak_decibels_full_scale": decibels_full_scale(true_peak),
        "integrated_loudness_lufs": integrated_loudness_lufs(samples, sample_rate),
        "spectral_centroid_hertz": spectral_centroid_hertz(samples, sample_rate),
        "spectral_rolloff_85_percent_hertz": spectral_rolloff_hertz(samples, sample_rate),
        "band_energy_percent": band_energy_percentages(samples, sample_rate),
        "dc_offset": float(np.mean(samples)) if samples.size else 0.0,
    }


def project_harmonic(
    samples: np.ndarray, sample_rate: int, frequency_hertz: float
) -> tuple[float, float]:
    frame_count = samples.size
    time_seconds = np.arange(frame_count, dtype=np.float64) / sample_rate
    window = np.hanning(frame_count)
    angle = 2.0 * math.pi * frequency_hertz * time_seconds
    real = float(np.sum(window * samples * np.cos(angle)))
    imaginary = float(-np.sum(window * samples * np.sin(angle)))
    normalization = 0.5 * float(np.sum(window))
    amplitude = math.hypot(real, imaginary) / max(normalization, 1e-12)
    phase_radians = math.atan2(imaginary, real)
    return amplitude, phase_radians


def band_edge_gain(frequency_hertz: float) -> float:
    if frequency_hertz < BAND_LOWER_HERTZ or frequency_hertz > BAND_UPPER_HERTZ:
        return 0.0
    if frequency_hertz < BAND_LOWER_ROLLIN_HERTZ:
        position = (frequency_hertz - BAND_LOWER_HERTZ) / (
            BAND_LOWER_ROLLIN_HERTZ - BAND_LOWER_HERTZ
        )
        return 0.5 - 0.5 * math.cos(math.pi * position)
    if frequency_hertz > BAND_UPPER_ROLLOFF_HERTZ:
        position = (frequency_hertz - BAND_UPPER_ROLLOFF_HERTZ) / (
            BAND_UPPER_HERTZ - BAND_UPPER_ROLLOFF_HERTZ
        )
        return 0.5 + 0.5 * math.cos(math.pi * position)
    return 1.0


def collect_odd_harmonics(
    samples: np.ndarray, sample_rate: int, fundamental_hertz: float
) -> list[HarmonicPartial]:
    partials: list[HarmonicPartial] = []
    frequency_limit = min(BAND_UPPER_HERTZ, BAND_EDGE_FREQUENCY_LIMIT_RATIO * sample_rate)
    harmonic_index = 1
    while harmonic_index * fundamental_hertz <= frequency_limit:
        frequency_hertz = harmonic_index * fundamental_hertz
        edge_gain = band_edge_gain(frequency_hertz)
        if edge_gain > 0.0:
            amplitude, phase_radians = project_harmonic(samples, sample_rate, frequency_hertz)
            synthesized_phase = phase_radians
            if frequency_hertz > PHASE_DISPERSION_THRESHOLD_HERTZ:
                synthesized_phase += 2.0 * math.pi * (
                    (harmonic_index * PHASE_DISPERSION_RATIO) % 1.0
                )
            partials.append(
                HarmonicPartial(
                    harmonic_index=harmonic_index,
                    frequency_hertz=frequency_hertz,
                    measured_amplitude=amplitude,
                    measured_phase_radians=phase_radians,
                    synthesized_phase_radians=synthesized_phase,
                    base_shape_amplitude=edge_gain
                    / harmonic_index ** PARTIAL_AMPLITUDE_EXPONENT,
                    edge_gain=edge_gain,
                )
            )
        harmonic_index += 2
    return partials


def render_partials(
    partials: list[HarmonicPartial], frame_count: int, sample_rate: int
) -> np.ndarray:
    time_seconds = np.arange(frame_count, dtype=np.float64) / sample_rate
    layer = np.zeros(frame_count, dtype=np.float64)
    for partial in partials:
        amplitude = partial.base_shape_amplitude * partial.spectrum_weight
        if amplitude == 0.0:
            continue
        layer += amplitude * np.cos(
            2.0 * math.pi * partial.frequency_hertz * time_seconds + partial.synthesized_phase_radians
        )
    return layer


def band_correction_factors(
    layer: np.ndarray, original: np.ndarray, sample_rate: int
) -> dict[str, float]:
    corrections = {}
    for low, high, increase_decibels in SPECTRUM_TARGET_BANDS:
        layer_power = band_power(layer, sample_rate, low, high)
        original_power = band_power(original, sample_rate, low, high)
        desired_ratio = 10.0 ** (increase_decibels / 10.0) - 1.0
        target_power = desired_ratio * original_power
        key = f"{int(low)}_{int(high)}"
        if layer_power <= 1e-18:
            corrections[key] = 1.0
        else:
            corrections[key] = math.sqrt(max(target_power, 0.0) / layer_power)
    return corrections


def frequency_band_key(frequency_hertz: float) -> str | None:
    for low, high, _ in SPECTRUM_TARGET_BANDS:
        if low <= frequency_hertz < high:
            return f"{int(low)}_{int(high)}"
    return None


def assign_band_weights(partials: list[HarmonicPartial], band_weights: dict[str, float]) -> None:
    assigned = []
    for partial in partials:
        key = frequency_band_key(partial.frequency_hertz)
        assigned.append(band_weights.get(key, 0.0))
    smoothed = []
    for index in range(len(assigned)):
        low_index = max(0, index - 1)
        high_index = min(len(assigned), index + 2)
        neighbourhood = assigned[low_index:high_index]
        smoothed.append(sum(neighbourhood) / len(neighbourhood))
    for partial, weight in zip(partials, smoothed):
        partial.spectrum_weight = weight


def shape_odd_harmonic_layer(
    partials: list[HarmonicPartial], original: np.ndarray, sample_rate: int
) -> dict[str, float]:
    band_weights = {f"{int(low)}_{int(high)}": 1.0 for low, high, _ in SPECTRUM_TARGET_BANDS}
    for _ in range(SPECTRUM_TARGET_ITERATIONS):
        layer = render_partials(partials, original.size, sample_rate)
        corrections = band_correction_factors(layer, original, sample_rate)
        for key, correction in corrections.items():
            band_weights[key] = min(band_weights[key] * correction, 64.0)
        assign_band_weights(partials, band_weights)
    return {key: round(value, 6) for key, value in band_weights.items()}


def windowed_root_mean_square(
    samples: np.ndarray, window_frames: int, hop_frames: int
) -> np.ndarray:
    if samples.size < window_frames:
        return np.array([root_mean_square(samples)])
    count = (samples.size - window_frames) // hop_frames + 1
    return np.array(
        [
            root_mean_square(samples[index * hop_frames: index * hop_frames + window_frames])
            for index in range(count)
        ]
    )


def one_pole_smoothing(samples: np.ndarray, coefficient: float) -> np.ndarray:
    forward = np.empty_like(samples)
    state = samples[0] if samples.size else 0.0
    for index in range(samples.size):
        state = coefficient * state + (1.0 - coefficient) * samples[index]
        forward[index] = state
    backward = np.empty_like(samples)
    state = forward[-1] if forward.size else 0.0
    for index in range(forward.size - 1, -1, -1):
        state = coefficient * state + (1.0 - coefficient) * forward[index]
        backward[index] = state
    return backward


def upper_band_envelope(samples: np.ndarray, sample_rate: int) -> np.ndarray:
    band = bandpass_filter(samples, sample_rate, *ENVELOPE_BAND_HERTZ)
    window_frames = max(1, round(ENVELOPE_WINDOW_SECONDS * sample_rate))
    hop_frames = max(1, round(ENVELOPE_HOP_SECONDS * sample_rate))
    envelope_windows = windowed_root_mean_square(band, window_frames, hop_frames)
    centers = np.arange(envelope_windows.size) * hop_frames + window_frames / 2.0
    envelope = np.interp(
        np.arange(samples.size, dtype=np.float64), centers, envelope_windows
    )
    mean = float(np.mean(envelope)) if envelope.size else 0.0
    if mean <= 1e-12:
        return np.ones_like(samples)
    envelope = envelope / mean
    smoothing_coefficient = math.exp(-1.0 / max(ENVELOPE_SMOOTHING_SECONDS * sample_rate, 1.0))
    envelope = one_pole_smoothing(envelope, smoothing_coefficient)
    return np.clip(envelope, ENVELOPE_MINIMUM_GAIN, ENVELOPE_MAXIMUM_GAIN)


def choose_layer_shift(
    original: np.ndarray, layer: np.ndarray, sample_rate: int, fundamental_hertz: float
) -> tuple[int, float, float]:
    period_frames = max(1, round(sample_rate / fundamental_hertz))
    best_shift = 0
    best_polarity = 1.0
    best_peak = float(np.max(np.abs(original + layer)))
    for polarity in (1.0, -1.0):
        for step in range(PHASE_SHIFT_SEARCH_STEPS):
            shift = round(step * period_frames / PHASE_SHIFT_SEARCH_STEPS)
            candidate_peak = float(np.max(np.abs(original + polarity * np.roll(layer, shift))))
            if candidate_peak < best_peak:
                best_peak = candidate_peak
                best_shift = shift
                best_polarity = polarity
    return best_shift, best_polarity, best_peak


def limit_layer_peak_excess(
    original: np.ndarray, layer: np.ndarray, sample_rate: int
) -> tuple[np.ndarray, float]:
    available_headroom = np.maximum(PEAK_CEILING - np.abs(original), 0.0)
    gain = np.minimum(
        1.0, available_headroom / np.maximum(np.abs(layer), 1e-12)
    )
    guard_frames = max(3, round(HEADROOM_GUARD_SECONDS * sample_rate))
    if guard_frames % 2 == 0:
        guard_frames += 1
    gain = minimum_filter1d(gain, size=guard_frames, origin=guard_frames // 2, mode="nearest")
    limited = original + layer * gain
    reduction = float(np.min(gain))
    reduction_decibels = decibels_full_scale(reduction) if reduction < 1.0 else 0.0
    return limited, reduction_decibels


def soft_clip_peak(samples: np.ndarray) -> tuple[np.ndarray, float]:
    measured_excess_percent = float(
        100.0 * np.mean(np.abs(samples) > PEAK_CEILING)
    )
    if measured_excess_percent == 0.0:
        return samples, measured_excess_percent
    oversampled = resample_poly(
        samples, OVERSAMPLING_FACTOR, 1, window=("kaiser", 12.0)
    )
    magnitude = np.abs(oversampled)
    compression_range = PEAK_CEILING - SOFT_CLIP_THRESHOLD
    compressed = np.where(
        magnitude > SOFT_CLIP_THRESHOLD,
        SOFT_CLIP_THRESHOLD
        + compression_range
        * np.tanh(np.maximum(magnitude - SOFT_CLIP_THRESHOLD, 0.0) / compression_range),
        magnitude,
    )
    shaped = np.sign(oversampled) * compressed
    restored = resample_poly(shaped, 1, OVERSAMPLING_FACTOR, window=("kaiser", 12.0))
    return np.clip(restored, -PEAK_CEILING, PEAK_CEILING), measured_excess_percent


def render_before_after_plot(
    path: Path,
    before: np.ndarray,
    after: np.ndarray,
    sample_rate: int,
    title: str,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    figure, axes = plt.subplots(3, 1, figsize=(12, 9))
    axes[0].plot(np.linspace(0.0, before.size / sample_rate, before.size), before, linewidth=0.4)
    axes[0].plot(np.linspace(0.0, after.size / sample_rate, after.size), after, linewidth=0.4)
    axes[0].set_title(f"{title} waveform")
    before_envelope = windowed_root_mean_square(
        before, round(0.020 * sample_rate), round(0.005 * sample_rate)
    )
    after_envelope = windowed_root_mean_square(
        after, round(0.020 * sample_rate), round(0.005 * sample_rate)
    )
    axes[1].plot(20.0 * np.log10(before_envelope + 1e-9), label="before")
    axes[1].plot(20.0 * np.log10(after_envelope + 1e-9), label="after")
    axes[1].legend()
    axes[1].set_title("envelope (dBFS)")
    before_spectrum = np.abs(np.fft.rfft(before * np.hanning(before.size)))
    after_spectrum = np.abs(np.fft.rfft(after * np.hanning(after.size)))
    frequencies = np.fft.rfftfreq(before.size, 1.0 / sample_rate)
    axes[2].semilogx(
        frequencies[1:],
        20.0 * np.log10(before_spectrum[1:] + 1e-9),
        label="before",
        linewidth=0.5,
    )
    axes[2].semilogx(
        frequencies[1:],
        20.0 * np.log10(after_spectrum[1:] + 1e-9),
        label="after",
        linewidth=0.5,
    )
    axes[2].legend()
    axes[2].set_title("spectrum (dB)")
    figure.tight_layout()
    figure.savefig(path, dpi=90)
    plt.close(figure)


def write_level_matched_ab(
    path: Path,
    before: np.ndarray,
    after: np.ndarray,
    sample_rate: int,
) -> dict:
    before_rms = root_mean_square(before) or 1e-9
    after_rms = root_mean_square(after) or 1e-9
    target = min(before_rms, after_rms)
    matched_before = before * (target / before_rms)
    matched_after = after * (target / after_rms)
    silence = np.zeros(round(0.25 * sample_rate))
    joined = np.concatenate([matched_before, silence, matched_after])
    write_float_wav(path, joined, sample_rate)
    return {
        "before_gain_decibels": decibels_full_scale(target / before_rms),
        "after_gain_decibels": decibels_full_scale(target / after_rms),
        "segment_frames": int(before.size),
    }


def detect_fundamental_hertz(builder, samples: np.ndarray, sample_rate: int) -> float:
    peaks = builder.spectral_peaks(samples, sample_rate)
    spacing, _, _ = builder.harmonic_comb_spacing(
        peaks, FUNDAMENTAL_SEARCH_LOW_HERTZ, FUNDAMENTAL_SEARCH_HIGH_HERTZ
    )
    return float(spacing)


def band_delta_decibels(
    before: np.ndarray, after: np.ndarray, sample_rate: int
) -> dict[str, float]:
    deltas = {}
    for low, high in MEASUREMENT_BAND_EDGES_HERTZ:
        before_power = band_power(before, sample_rate, low, high)
        after_power = band_power(after, sample_rate, low, high)
        deltas[f"{int(low)}_{int(high)}"] = 10.0 * math.log10(
            max(after_power, 1e-30) / max(before_power, 1e-30)
        )
    return deltas


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-directory", type=Path, default=DEFAULT_SOURCE_DIRECTORY)
    parser.add_argument("--candidate-root", type=Path, default=DEFAULT_CANDIDATE_ROOT)
    parser.add_argument("--bank-directory", type=Path, default=DEFAULT_BANK_DIRECTORY)
    parser.add_argument(
        "--levels",
        nargs="+",
        choices=sorted(LEVEL_OFFSETS_DECIBELS),
        default=sorted(LEVEL_OFFSETS_DECIBELS),
    )
    parser.add_argument(
        "--promote",
        choices=["none", *sorted(LEVEL_OFFSETS_DECIBELS)],
        default="none",
    )
    arguments = parser.parse_args()
    source_directory = arguments.source_directory.resolve()
    candidate_root = arguments.candidate_root.resolve()
    bank_directory = arguments.bank_directory.resolve()

    builder = load_builder_module()
    originals: dict[str, np.ndarray] = {}
    fundamentals: dict[str, float] = {}
    beds: dict[str, np.ndarray] = {}
    processed_layers: dict[str, np.ndarray] = {}
    harmonics_by_file: dict[str, list[HarmonicPartial]] = {}
    band_weights_by_file: dict[str, dict[str, float]] = {}
    generation_records = []
    for filename, definition in LOOP_SOURCE_DEFINITIONS.items():
        source_path = source_directory / filename
        source_hash = sha256_file(source_path)
        if source_hash != definition["sha256"]:
            raise ValueError(f"source hash mismatch for {filename}")
        original, sample_rate = read_mono_pcm16(source_path)
        if sample_rate != SAMPLE_RATE:
            raise ValueError(f"unexpected sample rate for {filename}: {sample_rate}")
        fundamental_hertz = detect_fundamental_hertz(builder, original, sample_rate)
        expected = float(definition["expected_comb_spacing_hz"])
        if abs(fundamental_hertz - expected) / expected > FUNDAMENTAL_TOLERANCE_RATIO:
            raise ValueError(
                f"fundamental mismatch for {filename}: {fundamental_hertz} != {expected}"
            )
        bed = original * amplitude_from_decibels(BED_ATTENUATION_DECIBELS)
        partials = collect_odd_harmonics(original, sample_rate, fundamental_hertz)
        band_weights = shape_odd_harmonic_layer(partials, bed, sample_rate)
        shaped_layer = render_partials(partials, original.size, sample_rate)
        modulated_layer = shaped_layer * upper_band_envelope(original, sample_rate)
        shift_frames, polarity, aligned_peak = choose_layer_shift(
            bed, modulated_layer, sample_rate, fundamental_hertz
        )
        aligned_layer = polarity * np.roll(modulated_layer, shift_frames)
        originals[filename] = original
        beds[filename] = bed
        fundamentals[filename] = fundamental_hertz
        processed_layers[filename] = aligned_layer
        harmonics_by_file[filename] = partials
        band_weights_by_file[filename] = band_weights
        generation_records.append(
            {
                "filename": filename,
                "asset_id": definition["asset_id"],
                "source_sha256": source_hash,
                "detected_fundamental_hertz": fundamental_hertz,
                "expected_comb_spacing_hz": expected,
                "partial_count": len(partials),
                "band_weights": band_weights,
                "layer_shift_frames": int(shift_frames),
                "layer_polarity": float(polarity),
                "aligned_bed_sample_peak": float(aligned_peak),
                "partials": [
                    {
                        "harmonic_index": partial.harmonic_index,
                        "frequency_hertz": partial.frequency_hertz,
                        "measured_amplitude": partial.measured_amplitude,
                        "measured_phase_radians": partial.measured_phase_radians,
                        "synthesized_phase_radians": partial.synthesized_phase_radians,
                        "base_shape_amplitude": partial.base_shape_amplitude,
                        "edge_gain": partial.edge_gain,
                        "spectrum_weight": partial.spectrum_weight,
                    }
                    for partial in partials
                ],
            }
        )

    processed_by_level: dict[str, dict[str, np.ndarray]] = {
        level_name: {} for level_name in arguments.levels
    }
    layer_reductions: dict[str, dict[str, float]] = {
        level_name: {} for level_name in arguments.levels
    }
    per_sample_records = []
    for filename, definition in LOOP_SOURCE_DEFINITIONS.items():
        original = originals[filename]
        bed = beds[filename]
        record = {
            "filename": filename,
            "asset_id": definition["asset_id"],
            "source_sha256": definition["sha256"],
            "fundamental_hertz": fundamentals[filename],
            "band_weights": band_weights_by_file[filename],
            "before": measure(original, SAMPLE_RATE),
            "bed": measure(bed, SAMPLE_RATE),
            "levels": {},
        }
        for level_name in arguments.levels:
            level_offset = amplitude_from_decibels(LEVEL_OFFSETS_DECIBELS[level_name])
            scaled_layer = processed_layers[filename] * level_offset
            limited, reduction_decibels = limit_layer_peak_excess(
                bed, scaled_layer, SAMPLE_RATE
            )
            processed, clipped_sample_percent = soft_clip_peak(limited)
            processed_by_level[level_name][filename] = processed
            layer_reductions[level_name][filename] = {
                "layer_reduction_decibels": reduction_decibels,
                "hard_excess_sample_percent": clipped_sample_percent,
            }
            record["levels"][level_name] = {
                "level": level_name,
                "level_offset_decibels": LEVEL_OFFSETS_DECIBELS[level_name],
                "layer_reduction_decibels": reduction_decibels,
                "hard_excess_sample_percent": clipped_sample_percent,
                "band_delta_vs_bed_decibels": band_delta_decibels(bed, processed, SAMPLE_RATE),
                "band_delta_vs_original_decibels": band_delta_decibels(
                    original, processed, SAMPLE_RATE
                ),
            }
        per_sample_records.append(record)

    common_gains = {}
    for level_name in arguments.levels:
        ceiling_amplitude = amplitude_from_decibels(TRUE_PEAK_CEILING_DECIBELS_FULL_SCALE)
        headroom = 1.0
        for processed in processed_by_level[level_name].values():
            true_peak = estimate_true_peak_amplitude(processed)
            if true_peak > ceiling_amplitude:
                headroom = min(headroom, ceiling_amplitude / true_peak)
        common_gains[level_name] = headroom

    if arguments.promote == "none":
        for level_name in arguments.levels:
            for filename, processed in processed_by_level[level_name].items():
                output = processed * common_gains[level_name]
                output_path = candidate_root / "samples" / level_name / filename
                seed = RANDOM_SEED + int(
                    hashlib.sha256(f"{level_name}:{filename}".encode("utf-8")).hexdigest()[:8],
                    16,
                )
                write_mono_pcm16(output_path, output, SAMPLE_RATE, seed)
                write_level_matched_ab(
                    candidate_root / "ab" / f"{filename}.{level_name}.wav",
                    originals[filename],
                    output,
                    SAMPLE_RATE,
                )
                render_before_after_plot(
                    candidate_root / "plots" / f"{filename}.{level_name}.png",
                    originals[filename],
                    output,
                    SAMPLE_RATE,
                    f"{filename} {level_name}",
                )
                for record in per_sample_records:
                    if record["filename"] == filename:
                        record["levels"][level_name]["output_sha256"] = sha256_file(output_path)
                        record["levels"][level_name]["output_path"] = str(
                            output_path.relative_to(REPOSITORY_ROOT)
                        ).replace("\\", "/")
                        record["levels"][level_name]["applied_common_gain"] = common_gains[level_name]
                        record["levels"][level_name]["result"] = measure(output, SAMPLE_RATE)
        measurements_directory = candidate_root / "measurements"
        measurements_directory.mkdir(parents=True, exist_ok=True)
        (measurements_directory / "per_sample.json").write_text(
            json.dumps(per_sample_records, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        generation = {
            "tool": "scripts/audio/synthesize_engine_odd_harmonic_layer.py",
            "random_seed": RANDOM_SEED,
            "spectrum_target_bands": [
                {"low_hertz": low, "high_hertz": high, "increase_decibels": increase}
                for low, high, increase in SPECTRUM_TARGET_BANDS
            ],
            "level_offsets_decibels": LEVEL_OFFSETS_DECIBELS,
            "partial_amplitude_exponent": PARTIAL_AMPLITUDE_EXPONENT,
            "band_lower_hertz": BAND_LOWER_HERTZ,
            "band_lower_rollin_hertz": BAND_LOWER_ROLLIN_HERTZ,
            "band_upper_rolloff_hertz": BAND_UPPER_ROLLOFF_HERTZ,
            "band_upper_hertz": BAND_UPPER_HERTZ,
            "true_peak_ceiling_decibels_full_scale": TRUE_PEAK_CEILING_DECIBELS_FULL_SCALE,
            "sample_peak_ceiling": PEAK_CEILING,
            "bed_attenuation_decibels": BED_ATTENUATION_DECIBELS,
            "soft_clip_threshold": SOFT_CLIP_THRESHOLD,
            "oversampling_factor": OVERSAMPLING_FACTOR,
            "phase_shift_search_steps": PHASE_SHIFT_SEARCH_STEPS,
            "headroom_guard_seconds": HEADROOM_GUARD_SECONDS,
            "level_treatments": layer_reductions,
            "common_gains": common_gains,
            "recommended_profile_engine_gain": {
                level_name: round(
                    amplitude_from_decibels(-BED_ATTENUATION_DECIBELS)
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
    for filename in LOOP_SOURCE_DEFINITIONS:
        output = processed_by_level[arguments.promote][filename] * common_gains[arguments.promote]
        output_path = bank_directory / filename
        seed = RANDOM_SEED + int(hashlib.sha256(filename.encode("utf-8")).hexdigest()[:8], 16)
        write_mono_pcm16(output_path, output, SAMPLE_RATE, seed)
        print(f"promoted {arguments.promote} {filename} sha256={sha256_file(output_path)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
