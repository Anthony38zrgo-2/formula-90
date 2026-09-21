#!/usr/bin/env python3
"""Per-sample V10 GP3 remaster pass.

Reads the thirteen primary sources and two immutable gearbox components in
game/sounds/banks/v10-gp3, applies only the treatment justified for each file,
and writes reviewable derivatives to
reports/audio-v10/grand-prix-sampler/candidates/<version>. No source file is
modified and no runtime bank is promoted. Floating point is used throughout;
each derivative is quantized once to PCM16 with triangular dither at export.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import shutil
import wave
from dataclasses import dataclass, field
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
import pyloudnorm
import soundfile
from scipy.signal import butter, resample_poly, sosfilt, welch

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE_DIRECTORY = REPOSITORY_ROOT / "game/sounds/banks/v10-gp3"
DEFAULT_CANDIDATE_ROOT = (
    REPOSITORY_ROOT / "reports/audio-v10/grand-prix-sampler/candidates/v1"
)

SAMPLE_RATE = 44100
LOOP_STORAGE_LOUDNESS_LUFS = -16.0
LOOP_TRUE_PEAK_CEILING_DBFS = -4.0
EVENT_TRUE_PEAK_CEILING_DBFS = -3.0
LOOP_MINIMUM_SECONDS = 1.0
LOOP_EARLIEST_START_RATIO = 0.10
SEAM_SEARCH_HOP_SECONDS = 0.001
CROSSFADE_SECONDS_CANDIDATES = (0.010, 0.015, 0.020, 0.025, 0.030)
COHERENT_CORRELATION_THRESHOLD = 0.50
SPECTRUM_CANDIDATE_COUNT = 60
ENVELOPE_WINDOW_SECONDS = 0.020
ENVELOPE_HOP_SECONDS = 0.005
SUSTAINED_WINDOW_SECONDS = 0.100
SUSTAINED_HOP_SECONDS = 0.010
DITHER_BIT_DEPTH = 16
DITHER_SEED = 20300920

SOURCE_DEFINITIONS = {
    "98_int_idle.wav": {
        "role": "engine_loop",
        "asset_id": "engine_idle_loop",
        "sha256": "f0021afba4048c59ae237325c932abef76433f03c437794d73de0e417384ddea",
    },
    "98_int_low.wav": {
        "role": "engine_loop",
        "asset_id": "engine_low_loop",
        "sha256": "2eac0ef746c818ecac50fe77ca0099894f4762d55428a3cddfeb3007a6092cc5",
    },
    "98_int_med.wav": {
        "role": "engine_loop",
        "asset_id": "engine_medium_loop",
        "sha256": "d1a527d5ddf3ec6d4f38127d1c6a3b00d7b7771d5473f68315e4352c98db4007",
    },
    "98_int_high_1.wav": {
        "role": "engine_loop",
        "asset_id": "engine_high_loop",
        "sha256": "fc68bc279e3f8098be009be24eb4249c4ab76ab805e595af7bc5c509b1413477",
    },
    "98_int_max_5.wav": {
        "role": "engine_loop",
        "asset_id": "engine_maximum_loop",
        "sha256": "f6b6cd4389fee32cb5f4b41a550e7aa146d4a5d0912fda86551dabfd598aa50b",
    },
    "gearup.wav": {
        "role": "gearbox_upshift",
        "asset_id": "upshift_event",
        "sha256": "c2d6c7c8441c10439625dc7dfbcd48d8dd8fd7d1b5004fa80da19ebda64a663a",
    },
    "geardn.wav": {
        "role": "gearbox_downshift",
        "asset_id": "downshift_event",
        "sha256": "52b5d14b33522167caff9508e48aae66bf8c7bcce3e8aac73a71831004cab279",
    },
    "500_backfire3.wav": {
        "role": "lift_backfire",
        "asset_id": "backfire_burst_3",
        "sha256": "e107717d2fe4f4fccd5d10d910b7cff71cd36d11403ef2a1277d36418986d044",
    },
    "500_backfire4.wav": {
        "role": "lift_backfire",
        "asset_id": "backfire_burst_4",
        "sha256": "e903a055cf8b5ca6a79b44862681f12bca5a11e95878f091095d613e7b63171e",
    },
    "500_backfire5.wav": {
        "role": "lift_backfire",
        "asset_id": "backfire_burst_5",
        "sha256": "a471a47bc8094a38e50894713b47b7847d81e1c518830b804e25dc87a69ceec1",
    },
    "500_backfire6.wav": {
        "role": "lift_backfire",
        "asset_id": "backfire_burst_6",
        "sha256": "6179b1460c81f58ce6ed69dd48498e0efcb3ec66eb18054cf1d97ac444426797",
    },
    "500_backfire7.wav": {
        "role": "lift_backfire",
        "asset_id": "backfire_burst_7",
        "sha256": "ed324b6935950738a6cb4eb0ba99eab74b58bd24fe8b6a9dc75973b47269c42c",
    },
    "500_limiter.wav": {
        "role": "limiter_event",
        "asset_id": "limiter_event",
        "sha256": "1bdb7a2cc3eb6d95e7fa8d56bec55fd4df8c9b307902056b93f5c30b32751758",
    },
}

AUXILIARY_GEARBOX_COMPONENT_DEFINITIONS = {
    "96_gear_change_up_1.wav": {
        "role": "gearbox_upshift_component",
        "asset_id": "upshift_event_1996_component",
        "sha256": "8332c4815bd4d26378600a5592e93bc764ec667377b912db5a43c04906470a59",
    },
    "96_gear_change_down_2.wav": {
        "role": "gearbox_downshift_component",
        "asset_id": "downshift_event_1996_component",
        "sha256": "efaf45f46722051b7f454e421657cfe72ef7ece3412ae0cb9a7a91099106c032",
    },
}

CONDITIONAL_DYNAMIC_BELLS = {
    "engine_idle_loop": {
        "center_hz": 328.0,
        "quality": 1.5,
        "maximum_reduction_db": 1.5,
        "detector_low_hz": 300.0,
        "detector_high_hz": 600.0,
        "detector_threshold_dbfs": -13.6,
        "ratio": 1.5,
        "attack_seconds": 0.040,
        "release_seconds": 0.180,
    },
    "engine_low_loop": {
        "center_hz": 124.0,
        "quality": 2.0,
        "maximum_reduction_db": 1.5,
        "detector_low_hz": 80.0,
        "detector_high_hz": 150.0,
        "detector_threshold_dbfs": -21.6,
        "ratio": 1.5,
        "attack_seconds": 0.030,
        "release_seconds": 0.120,
    },
    "engine_medium_loop": {
        "center_hz": 137.0,
        "quality": 2.0,
        "maximum_reduction_db": 1.5,
        "detector_low_hz": 80.0,
        "detector_high_hz": 150.0,
        "detector_threshold_dbfs": -19.4,
        "ratio": 1.5,
        "attack_seconds": 0.030,
        "release_seconds": 0.120,
    },
    "engine_high_loop": {
        "center_hz": 466.0,
        "quality": 1.2,
        "maximum_reduction_db": 1.0,
        "detector_low_hz": 300.0,
        "detector_high_hz": 600.0,
        "detector_threshold_dbfs": -8.1,
        "ratio": 1.3,
        "attack_seconds": 0.040,
        "release_seconds": 0.160,
    },
    "engine_maximum_loop": {
        "center_hz": 128.0,
        "quality": 2.0,
        "maximum_reduction_db": 1.5,
        "detector_low_hz": 80.0,
        "detector_high_hz": 150.0,
        "detector_threshold_dbfs": -10.3,
        "ratio": 1.5,
        "attack_seconds": 0.030,
        "release_seconds": 0.150,
    },
}

AUTHORED_REFERENCE_RPM = {
    "engine_idle_loop": 4542.534847313378,
    "engine_low_loop": 8642.704767296445,
    "engine_medium_loop": 9563.376772255555,
    "engine_high_loop": 13033.176670825176,
    "engine_maximum_loop": 17831.453741715675,
}

BAND_EDGES_HZ = (
    (0.0, 20.0),
    (20.0, 80.0),
    (80.0, 300.0),
    (300.0, 1200.0),
    (1200.0, 5000.0),
    (5000.0, 8000.0),
    (8000.0, 24000.0),
)


@dataclass
class TreatmentResult:
    derivative: np.ndarray
    recipe: str
    details: dict = field(default_factory=dict)


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def read_source_mono(path: Path) -> tuple[np.ndarray, int]:
    data, rate = soundfile.read(str(path), dtype="float64", always_2d=True)
    mono = data.mean(axis=1)
    return np.asarray(mono, dtype=np.float64), int(rate)


def dbfs(value: float) -> float:
    return 20.0 * math.log10(max(float(value), 1e-12))


def root_mean_square(samples: np.ndarray) -> float:
    if samples.size == 0:
        return 0.0
    return float(np.sqrt(np.mean(np.asarray(samples, dtype=np.float64) ** 2)))


def peak_amplitude(samples: np.ndarray) -> float:
    if samples.size == 0:
        return 0.0
    return float(np.max(np.abs(samples)))


def estimate_true_peak_amplitude(samples: np.ndarray) -> float:
    if samples.size == 0:
        return 0.0
    oversampled = resample_poly(samples, 8, 1, window=("kaiser", 12.0))
    return float(np.max(np.abs(oversampled)))


def integrated_loudness_lufs(samples: np.ndarray, rate: int) -> float | None:
    if samples.size < rate // 2:
        return None
    meter = pyloudnorm.Meter(rate)
    try:
        value = meter.integrated_loudness(np.asarray(samples, dtype=np.float64))
    except ValueError:
        return None
    if not math.isfinite(value):
        return None
    return float(value)


def windowed_rms(samples: np.ndarray, rate: int, window_seconds: float, hop_seconds: float) -> np.ndarray:
    window = max(1, round(window_seconds * rate))
    hop = max(1, round(hop_seconds * rate))
    if samples.size < window:
        return np.array([root_mean_square(samples)])
    count = (samples.size - window) // hop + 1
    return np.array(
        [root_mean_square(samples[index * hop: index * hop + window]) for index in range(count)]
    )


def envelope_dynamic_range_db(samples: np.ndarray, rate: int) -> float:
    windows = windowed_rms(samples, rate, ENVELOPE_WINDOW_SECONDS, ENVELOPE_HOP_SECONDS)
    if windows.size == 0:
        return 0.0
    low, high = np.percentile(windows, [10.0, 95.0])
    return float(dbfs(high) - dbfs(low))


def sustained_variation_db(samples: np.ndarray, rate: int) -> float:
    windows = windowed_rms(samples, rate, SUSTAINED_WINDOW_SECONDS, SUSTAINED_HOP_SECONDS)
    if windows.size == 0:
        return 0.0
    start = int(0.10 * windows.size)
    end = max(start + 1, int(0.90 * windows.size))
    core = windows[start:end]
    low, high = np.percentile(core, [10.0, 90.0])
    return float(dbfs(high) - dbfs(low))


def band_share_percentages(samples: np.ndarray, rate: int) -> dict[str, float]:
    if samples.size < 64:
        return {f"{int(low)}_{int(high)}": 0.0 for low, high in BAND_EDGES_HZ}
    frequencies, power = welch(samples, fs=rate, nperseg=min(samples.size, 1 << 14))
    total = float(np.sum(power)) + 1e-30
    shares = {}
    for low, high in BAND_EDGES_HZ:
        mask = (frequencies >= low) & (frequencies < high)
        shares[f"{int(low)}_{int(high)}"] = float(100.0 * np.sum(power[mask]) / total)
    return shares


def spectral_centroid_hz(samples: np.ndarray, rate: int) -> float:
    if samples.size < 64:
        return 0.0
    frequencies, power = welch(samples, fs=rate, nperseg=min(samples.size, 1 << 14))
    total = float(np.sum(power)) + 1e-30
    return float(np.sum(frequencies * power) / total)


def spectral_rolloff_hz(samples: np.ndarray, rate: int, fraction: float = 0.99) -> float:
    if samples.size < 64:
        return 0.0
    frequencies, power = welch(samples, fs=rate, nperseg=min(samples.size, 1 << 14))
    cumulative = np.cumsum(power)
    target = fraction * float(cumulative[-1])
    index = int(np.searchsorted(cumulative, target))
    index = min(index, len(frequencies) - 1)
    return float(frequencies[index])


def dominant_spectral_peaks_hz(samples: np.ndarray, rate: int, count: int = 6) -> list[float]:
    if samples.size < 256:
        return []
    frequencies, power = welch(samples, fs=rate, nperseg=min(samples.size, 1 << 15), nfft=1 << 16)
    band = (frequencies >= 25.0) & (frequencies <= 1500.0)
    frequencies = frequencies[band]
    power = power[band]
    local_maxima = [
        index
        for index in range(1, len(power) - 1)
        if power[index] >= power[index - 1] and power[index] >= power[index + 1]
    ]
    local_maxima.sort(key=lambda index: power[index], reverse=True)
    picked: list[float] = []
    for index in local_maxima:
        frequency = float(frequencies[index])
        if any(abs(frequency - existing) / existing < 0.03 for existing in picked):
            continue
        picked.append(frequency)
        if len(picked) >= count:
            break
    return picked


def sub_20hz_power_after_mean_removal(samples: np.ndarray, rate: int) -> float:
    centered = samples - float(np.mean(samples))
    if centered.size < 64:
        return 0.0
    frequencies, power = welch(centered, fs=rate, nperseg=min(centered.size, 1 << 14))
    mask = frequencies < 20.0
    total = float(np.sum(power)) + 1e-30
    return float(100.0 * np.sum(power[mask]) / total)


def near_clip_percentages(samples: np.ndarray) -> dict[str, float]:
    if samples.size == 0:
        return {"at_or_above_minus_0_1_dbfs": 0.0, "at_or_above_minus_1_dbfs": 0.0}
    magnitude = np.abs(samples)
    return {
        "at_or_above_minus_0_1_dbfs": float(
            100.0 * np.mean(magnitude >= 10.0 ** (-0.1 / 20.0))
        ),
        "at_or_above_minus_1_dbfs": float(
            100.0 * np.mean(magnitude >= 10.0 ** (-1.0 / 20.0))
        ),
    }


def plateau_statistics(samples: np.ndarray, threshold: float = 0.5) -> dict[str, float]:
    if samples.size == 0:
        return {"plateau_participation_percent": 0.0, "longest_plateau_samples": 0}
    rounded = np.round(samples * 32768.0).astype(np.int64)
    participation = 0
    longest = 0
    run = 1
    for index in range(1, rounded.size):
        if rounded[index] == rounded[index - 1] and abs(rounded[index]) > threshold * 32768.0:
            run += 1
            participation += 1
        else:
            longest = max(longest, run)
            run = 1
    longest = max(longest, run)
    return {
        "plateau_participation_percent": float(100.0 * participation / rounded.size),
        "longest_plateau_samples": int(longest),
    }


def event_time_metrics(samples: np.ndarray, rate: int) -> dict[str, float]:
    if samples.size == 0:
        return {"peak_time_ms": 0.0, "energy_span_5_95_ms": 0.0, "final_20ms_dbfs": 0.0}
    magnitude = np.abs(samples)
    peak_index = int(np.argmax(magnitude))
    energy = np.cumsum(samples.astype(np.float64) ** 2)
    total = float(energy[-1]) + 1e-30
    low_index = int(np.searchsorted(energy, 0.05 * total))
    high_index = int(np.searchsorted(energy, 0.95 * total))
    final_window = samples[-min(int(0.020 * rate), samples.size):]
    return {
        "peak_time_ms": float(1000.0 * peak_index / rate),
        "energy_span_5_95_ms": float(1000.0 * (high_index - low_index) / rate),
        "final_20ms_dbfs": dbfs(root_mean_square(final_window)),
    }


def measure(samples: np.ndarray, rate: int, role: str) -> dict:
    report = {
        "sample_rate_hz": rate,
        "frames": int(samples.size),
        "duration_seconds": float(samples.size / rate),
        "peak_dbfs": dbfs(peak_amplitude(samples)),
        "estimated_true_peak_dbfs": dbfs(estimate_true_peak_amplitude(samples)),
        "root_mean_square_dbfs": dbfs(root_mean_square(samples)),
        "integrated_loudness_lufs": integrated_loudness_lufs(samples, rate),
        "crest_factor_db": dbfs(peak_amplitude(samples)) - dbfs(root_mean_square(samples)),
        "direct_current_offset_fraction_of_full_scale": float(np.mean(samples)),
        "envelope_dynamic_range_db": envelope_dynamic_range_db(samples, rate),
        "sustained_variation_db": sustained_variation_db(samples, rate),
        "band_share_percent": band_share_percentages(samples, rate),
        "spectral_centroid_hz": spectral_centroid_hz(samples, rate),
        "spectral_rolloff_99hz": spectral_rolloff_hz(samples, rate),
        "dominant_spectral_peaks_hz": dominant_spectral_peaks_hz(samples, rate),
        "sub_20hz_share_after_mean_removal_percent": sub_20hz_power_after_mean_removal(samples, rate),
        "near_clip": near_clip_percentages(samples),
        "plateau": plateau_statistics(samples),
    }
    if role != "engine_loop":
        report["event_time_metrics"] = event_time_metrics(samples, rate)
    return report


def bandpass_component(samples: np.ndarray, rate: int, low_hz: float, high_hz: float) -> np.ndarray:
    nyquist = rate / 2.0
    low = max(low_hz / nyquist, 1e-6)
    high = min(high_hz / nyquist, 0.999999)
    if low >= high:
        return np.zeros_like(samples)
    sos = butter(3, [low, high], btype="bandpass", output="sos")
    return sosfilt(sos, samples)


def peaking_band_component(samples: np.ndarray, rate: int, center_hz: float, quality: float) -> np.ndarray:
    bandwidth = center_hz / max(quality, 0.1)
    low = max(center_hz - bandwidth / 2.0, 1.0)
    high = min(center_hz + bandwidth / 2.0, rate / 2.0 - 1.0)
    return bandpass_component(samples, rate, low, high)


def one_pole_envelope(magnitude: np.ndarray, rate: int, attack_seconds: float, release_seconds: float) -> np.ndarray:
    attack_coefficient = math.exp(-1.0 / max(rate * attack_seconds, 1.0))
    release_coefficient = math.exp(-1.0 / max(rate * release_seconds, 1.0))
    envelope = np.empty_like(magnitude)
    state = 0.0
    for index in range(magnitude.size):
        coefficient = attack_coefficient if magnitude[index] > state else release_coefficient
        state = coefficient * state + (1.0 - coefficient) * magnitude[index]
        envelope[index] = state
    return envelope


def apply_dynamic_bell(samples: np.ndarray, rate: int, definition: dict) -> tuple[np.ndarray, float]:
    band = peaking_band_component(samples, rate, definition["center_hz"], definition["quality"])
    detector = bandpass_component(
        samples, rate, definition["detector_low_hz"], definition["detector_high_hz"]
    )
    envelope = one_pole_envelope(
        np.abs(detector), rate, definition["attack_seconds"], definition["release_seconds"]
    )
    threshold = 10.0 ** (definition["detector_threshold_dbfs"] / 20.0)
    ratio = definition["ratio"]
    over = np.maximum(envelope, 1e-12) / threshold
    reduction = np.zeros_like(over)
    exceeding = over > 1.0
    reduction[exceeding] = (1.0 - ratio ** -1.0) * np.log10(over[exceeding])
    reduction = np.minimum(reduction, definition["maximum_reduction_db"])
    gain = 10.0 ** (-reduction / 20.0)
    conditioned = samples + (gain - 1.0) * band
    return conditioned, float(np.max(reduction))


def circular_warmup_process(samples: np.ndarray, rate: int, definition: dict) -> tuple[np.ndarray, float]:
    tiled = np.concatenate([samples, samples, samples])
    conditioned, maximum_reduction = apply_dynamic_bell(tiled, rate, definition)
    start = samples.size
    return conditioned[start:start + samples.size].copy(), maximum_reduction


def correlation(first: np.ndarray, second: np.ndarray) -> float:
    denominator = math.sqrt(float(np.sum(first * first)) * float(np.sum(second * second))) + 1e-12
    return float(np.sum(first * second) / denominator)


def spectrum_similarity(first: np.ndarray, second: np.ndarray) -> float:
    window = np.hanning(min(first.size, second.size))
    first_windowed = first[: window.size] * window
    second_windowed = second[: window.size] * window
    first_spectrum = np.abs(np.fft.rfft(first_windowed)) + 1e-12
    second_spectrum = np.abs(np.fft.rfft(second_windowed)) + 1e-12
    numerator = float(np.sum(first_spectrum * second_spectrum))
    denominator = math.sqrt(float(np.sum(first_spectrum ** 2)) * float(np.sum(second_spectrum ** 2))) + 1e-12
    return numerator / denominator


def local_rms(samples: np.ndarray) -> float:
    return root_mean_square(samples)


def construct_seamless_loop(samples: np.ndarray, rate: int) -> tuple[np.ndarray, dict]:
    total = samples.size
    minimum_loop_frames = int(LOOP_MINIMUM_SECONDS * rate)
    hop = max(1, int(SEAM_SEARCH_HOP_SECONDS * rate))
    candidates: list[dict] = []
    for fade_seconds in CROSSFADE_SECONDS_CANDIDATES:
        fade = round(fade_seconds * rate)
        earliest_start = max(fade, int(LOOP_EARLIEST_START_RATIO * rate))
        latest_start = total - minimum_loop_frames
        if latest_start <= earliest_start:
            continue
        tail = samples[total - fade:total]
        tail_energy = float(np.sum(tail * tail))
        for start in range(earliest_start, latest_start, hop):
            head = samples[start - fade:start]
            head_energy = float(np.sum(head * head))
            denominator = math.sqrt(head_energy * tail_energy) + 1e-12
            value_correlation = float(np.sum(head * tail) / denominator)
            head_slope = np.diff(head)
            tail_slope = np.diff(tail)
            slope_correlation = correlation(head_slope, tail_slope)
            rms_ratio_db = abs(dbfs(local_rms(head)) - dbfs(local_rms(tail)))
            loop_head = samples[start:start + fade]
            wrap_level_step_db = abs(dbfs(local_rms(loop_head)) - dbfs(local_rms(tail)))
            score = (
                value_correlation
                + 0.5 * slope_correlation
                - 0.1 * rms_ratio_db
                - 0.5 * wrap_level_step_db
            )
            candidates.append(
                {
                    "start": start,
                    "fade": fade,
                    "fade_seconds": fade_seconds,
                    "value_correlation": value_correlation,
                    "slope_correlation": slope_correlation,
                    "rms_ratio_db": rms_ratio_db,
                    "wrap_level_step_db": wrap_level_step_db,
                    "score": score,
                }
            )
    if not candidates:
        raise ValueError("source too short for loop construction")
    candidates.sort(key=lambda candidate: candidate["score"], reverse=True)
    for candidate in candidates[:SPECTRUM_CANDIDATE_COUNT]:
        start = candidate["start"]
        fade = candidate["fade"]
        candidate["spectrum_similarity"] = spectrum_similarity(
            samples[start - fade:start], samples[total - fade:total]
        )
    for candidate in candidates:
        candidate.setdefault("spectrum_similarity", 0.0)
    candidates.sort(
        key=lambda candidate: candidate["score"] + 0.25 * candidate["spectrum_similarity"],
        reverse=True,
    )
    selected = candidates[0]
    start = selected["start"]
    fade = selected["fade"]
    segment = samples[start:].copy()
    preceding = samples[start - fade:start]
    tail_segment = segment[segment.size - fade:].copy()
    use_coherent_law = selected["value_correlation"] >= COHERENT_CORRELATION_THRESHOLD
    ramp = np.linspace(0.0, 1.0, fade, endpoint=False)
    if use_coherent_law:
        outgoing_weight = 1.0 - ramp
        incoming_weight = ramp
        fade_law = "linear_coherent"
    else:
        outgoing_weight = np.cos(ramp * math.pi / 2.0)
        incoming_weight = np.sin(ramp * math.pi / 2.0)
        fade_law = "equal_power_decorrelated"
    segment[segment.size - fade:] = (
        tail_segment * outgoing_weight + preceding * incoming_weight
    )
    wrap_step = float(abs(segment[-1] - segment[0]))
    adjacent_steps = np.abs(np.diff(segment))
    internal_p99_step = float(np.percentile(adjacent_steps, 99.0)) if adjacent_steps.size else 0.0
    fade_region = segment[segment.size - fade:]
    loop_head_region = segment[:fade]
    envelope_disturbance_db = abs(
        dbfs(local_rms(fade_region)) - dbfs(local_rms(loop_head_region))
    )
    metrics = {
        "source_start_frame": int(start),
        "source_end_frame_exclusive": int(total),
        "loop_frames": int(segment.size),
        "crossfade_frames": int(fade),
        "crossfade_seconds": float(fade / rate),
        "crossfade_law": fade_law,
        "value_correlation": selected["value_correlation"],
        "slope_correlation": selected["slope_correlation"],
        "spectrum_similarity": selected["spectrum_similarity"],
        "rms_ratio_db": selected["rms_ratio_db"],
        "selected_wrap_level_step_db": selected["wrap_level_step_db"],
        "prepared_wrap_step": wrap_step,
        "internal_p99_adjacent_step": internal_p99_step,
        "wrap_step_over_internal_p99": float(wrap_step / max(internal_p99_step, 1e-9)),
        "envelope_disturbance_db": float(envelope_disturbance_db),
    }
    return segment, metrics


def apply_high_pass(samples: np.ndarray, rate: int, cutoff_hz: float = 20.0, order: int = 2) -> np.ndarray:
    sos = butter(order, cutoff_hz / (rate / 2.0), btype="highpass", output="sos")
    return sosfilt(sos, samples)


def scale_to_loudness(samples: np.ndarray, rate: int, target_lufs: float) -> tuple[np.ndarray, float]:
    current = integrated_loudness_lufs(samples, rate)
    if current is None:
        return samples, 0.0
    gain_db = target_lufs - current
    return samples * (10.0 ** (gain_db / 20.0)), gain_db


def clamp_true_peak(
    samples: np.ndarray, ceiling_dbfs: float
) -> tuple[np.ndarray, float]:
    current = dbfs(estimate_true_peak_amplitude(samples))
    if current <= ceiling_dbfs:
        return samples, 0.0
    reduction_db = ceiling_dbfs - current
    return samples * (10.0 ** (reduction_db / 20.0)), reduction_db


def attenuate_to_ceiling(samples: np.ndarray, ceiling_dbfs: float) -> tuple[np.ndarray, float]:
    current = dbfs(estimate_true_peak_amplitude(samples))
    gain_db = min(0.0, ceiling_dbfs - current)
    return samples * (10.0 ** (gain_db / 20.0)), gain_db


def measure_split_window_reduction(samples: np.ndarray, rate: int, definition: dict) -> float:
    detector = bandpass_component(
        samples, rate, definition["detector_low_hz"], definition["detector_high_hz"]
    )
    envelope = windowed_rms(detector, rate, 0.100, 0.010)
    if envelope.size == 0:
        return 0.0
    start = int(0.10 * envelope.size)
    end = max(start + 1, int(0.90 * envelope.size))
    core = envelope[start:end]
    low, high = np.percentile(core, [10.0, 90.0])
    return float(dbfs(high) - dbfs(low))


def quantize_pcm16_dithered(samples: np.ndarray, seed: int) -> np.ndarray:
    generator = np.random.default_rng(seed)
    first = generator.random(samples.size)
    second = generator.random(samples.size)
    dither = (first - second) / float(1 << DITHER_BIT_DEPTH)
    scaled = np.clip(samples + dither, -1.0, 32767.0 / 32768.0) * 32768.0
    return np.clip(np.round(scaled), -32768, 32767).astype(np.int16)


def write_pcm16(path: Path, samples: np.ndarray, seed: int) -> str:
    pcm = quantize_pcm16_dithered(samples, seed)
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(SAMPLE_RATE)
        handle.writeframes(pcm.tobytes())
    return sha256_file(path)


def write_float_wav(path: Path, samples: np.ndarray) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    soundfile.write(str(path), np.asarray(samples, dtype=np.float32), SAMPLE_RATE, subtype="FLOAT")


def render_before_after_plot(
    path: Path,
    before: np.ndarray,
    after: np.ndarray,
    rate: int,
    title: str,
    boundary: bool,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    figure, axes = plt.subplots(4, 1, figsize=(12, 12))
    axes[0].plot(np.linspace(0.0, before.size / rate, before.size), before, linewidth=0.4)
    axes[0].set_title(f"{title} before")
    axes[1].plot(np.linspace(0.0, after.size / rate, after.size), after, linewidth=0.4)
    axes[1].set_title(f"{title} after")
    before_envelope = windowed_rms(before, rate, 0.020, 0.005)
    after_envelope = windowed_rms(after, rate, 0.020, 0.005)
    axes[2].plot(np.linspace(0.0, 1.0, before_envelope.size), 20.0 * np.log10(before_envelope + 1e-9), label="before")
    axes[2].plot(np.linspace(0.0, 1.0, after_envelope.size), 20.0 * np.log10(after_envelope + 1e-9), label="after")
    axes[2].legend()
    axes[2].set_title("envelope (dBFS)")
    before_spectrum = np.abs(np.fft.rfft(before * np.hanning(before.size)))
    after_spectrum = np.abs(np.fft.rfft(after * np.hanning(after.size)))
    before_frequencies = np.fft.rfftfreq(before.size, 1.0 / rate)
    after_frequencies = np.fft.rfftfreq(after.size, 1.0 / rate)
    axes[3].semilogx(before_frequencies[1:], 20.0 * np.log10(before_spectrum[1:] + 1e-9), label="before", linewidth=0.5)
    axes[3].semilogx(after_frequencies[1:], 20.0 * np.log10(after_spectrum[1:] + 1e-9), label="after", linewidth=0.5)
    axes[3].legend()
    axes[3].set_title("spectrum (dB)")
    if boundary and after.size > 0:
        axes[1].axhline(after[0], color="red", linewidth=0.3)
    figure.tight_layout()
    figure.savefig(path, dpi=90)
    plt.close(figure)


def write_level_matched_ab(path: Path, before: np.ndarray, after: np.ndarray) -> dict:
    before_rms = root_mean_square(before) or 1e-9
    after_rms = root_mean_square(after) or 1e-9
    target = min(before_rms, after_rms)
    matched_before = before * (target / before_rms)
    matched_after = after * (target / after_rms)
    silence = np.zeros(int(0.25 * SAMPLE_RATE))
    joined = np.concatenate([matched_before, silence, matched_after])
    write_float_wav(path, joined)
    return {
        "before_gain_db": dbfs(target / before_rms),
        "after_gain_db": dbfs(target / after_rms),
        "segment_frames": int(before.size),
    }


def process_engine_loop(
    filename: str,
    definition: dict,
    samples: np.ndarray,
    rate: int,
) -> TreatmentResult:
    centered = samples - float(np.mean(samples))
    residual_sub_20_percent = sub_20hz_power_after_mean_removal(centered, rate)
    high_pass_applied = False
    if residual_sub_20_percent > 0.5:
        centered = apply_high_pass(centered, rate)
        high_pass_applied = True
    loop_segment, seam_metrics = construct_seamless_loop(centered, rate)
    leveled, loudness_gain_db = scale_to_loudness(loop_segment, rate, LOOP_STORAGE_LOUDNESS_LUFS)
    leveled, true_peak_reduction_db = clamp_true_peak(leveled, LOOP_TRUE_PEAK_CEILING_DBFS)
    details = {
        "direct_current_removal_applied": True,
        "high_pass_20hz_applied": high_pass_applied,
        "residual_sub_20hz_share_percent": residual_sub_20_percent,
        "loop_metrics": seam_metrics,
        "loudness_gain_db": loudness_gain_db,
        "true_peak_reduction_db": true_peak_reduction_db,
    }
    recipe = "prepared_engine_loop_dc_removal_loudness_calibration_v1"
    conditional = CONDITIONAL_DYNAMIC_BELLS.get(definition["asset_id"])
    if conditional is not None:
        conditioned, maximum_reduction_db = circular_warmup_process(loop_segment, rate, conditional)
        conditioned_loudness_gain_db = LOOP_STORAGE_LOUDNESS_LUFS - (
            integrated_loudness_lufs(conditioned, rate) or LOOP_STORAGE_LOUDNESS_LUFS
        )
        conditioned = conditioned * (10.0 ** (conditioned_loudness_gain_db / 20.0))
        conditioned, _ = clamp_true_peak(conditioned, LOOP_TRUE_PEAK_CEILING_DBFS)
        before_detector_range = measure_split_window_reduction(loop_segment, rate, conditional)
        after_detector_range = measure_split_window_reduction(conditioned, rate, conditional)
        details["conditional_dynamic_bell"] = {
            "definition": conditional,
            "maximum_reduction_db": maximum_reduction_db,
            "detector_band_range_before_db": before_detector_range,
            "detector_band_range_after_db": after_detector_range,
            "detector_band_range_reduction_db": before_detector_range - after_detector_range,
        }
        details["conditional_derivative"] = conditioned
    return TreatmentResult(derivative=leveled, recipe=recipe, details=details)


def process_event(
    filename: str,
    definition: dict,
    samples: np.ndarray,
    rate: int,
) -> TreatmentResult:
    role = definition["role"]
    if role in ("lift_backfire", "limiter_event"):
        centered = samples - float(np.mean(samples))
        attenuated, attenuation_db = attenuate_to_ceiling(centered, EVENT_TRUE_PEAK_CEILING_DBFS)
        details = {
            "direct_current_removal_applied": True,
            "attenuation_db": attenuation_db,
            "tail_fade_applied": False,
        }
        recipe = (
            "prepared_lift_backfire_attenuation_only_v1"
            if role == "lift_backfire"
            else "prepared_limiter_event_attenuation_only_v1"
        )
        if role == "limiter_event":
            fade_frames = int(0.004 * rate)
            faded = attenuated.copy()
            faded[faded.size - fade_frames:] *= np.linspace(1.0, 0.0, fade_frames)
            endpoint_before = dbfs(abs(attenuated[-1]))
            endpoint_after = dbfs(abs(faded[-1]))
            details["tail_fade_derivative"] = faded
            details["tail_fade_frames"] = fade_frames
            details["endpoint_dbfs_before_fade"] = endpoint_before
            details["endpoint_dbfs_after_fade"] = endpoint_after
        return TreatmentResult(derivative=attenuated, recipe=recipe, details=details)
    return TreatmentResult(
        derivative=samples.copy(),
        recipe="prepared_gearbox_event_native_copy_v1",
        details={"direct_current_removal_applied": False, "attenuation_db": 0.0},
    )


def process_file(
    filename: str,
    source_directory: Path,
    candidate_root: Path,
) -> dict:
    definition = SOURCE_DEFINITIONS[filename]
    source_path = source_directory / filename
    source_hash = sha256_file(source_path)
    if source_hash != definition["sha256"]:
        raise ValueError(f"source hash mismatch for {filename}")
    samples, rate = read_source_mono(source_path)
    if rate != SAMPLE_RATE:
        samples = resample_poly(samples, SAMPLE_RATE, rate)
        rate = SAMPLE_RATE
    before_measurement = measure(samples, rate, definition["role"])
    if definition["role"] == "engine_loop":
        result = process_engine_loop(filename, definition, samples, rate)
    else:
        result = process_event(filename, definition, samples, rate)
    derivative = result.derivative
    after_measurement = measure(derivative, rate, definition["role"])
    seed = DITHER_SEED + int(sha256_bytes(definition["asset_id"].encode("utf-8"))[:8], 16)
    sample_path = candidate_root / "samples" / filename
    if definition["role"] in ("gearbox_upshift", "gearbox_downshift"):
        sample_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source_path, sample_path)
        derivative_hash = sha256_file(sample_path)
    else:
        derivative_hash = write_pcm16(sample_path, derivative, seed)
    write_float_wav(candidate_root / "samples" / (filename + ".float.wav"), derivative)
    ab_details = write_level_matched_ab(
        candidate_root / "ab" / filename,
        samples,
        derivative,
    )
    boundary = definition["role"] == "engine_loop"
    render_before_after_plot(
        candidate_root / "plots" / (filename + ".png"),
        samples,
        derivative,
        rate,
        filename,
        boundary,
    )
    conditional_path = None
    conditional_measurement = None
    conditional_recommended = None
    if "conditional_derivative" in result.details:
        conditional_derivative = result.details.pop("conditional_derivative")
        conditional_path = candidate_root / "ab" / (filename + ".conditional_dynamic_bell.wav")
        write_float_wav(conditional_path, conditional_derivative)
        conditional_measurement = measure(conditional_derivative, rate, definition["role"])
        detector = result.details["conditional_dynamic_bell"]
        loudness_change = abs(
            (conditional_measurement["integrated_loudness_lufs"] or 0.0)
            - (after_measurement["integrated_loudness_lufs"] or 0.0)
        )
        conditional_recommended = bool(
            detector["detector_band_range_reduction_db"] >= 1.0 and loudness_change <= 0.3
        )
    if "tail_fade_derivative" in result.details:
        tail_fade_derivative = result.details.pop("tail_fade_derivative")
        write_float_wav(candidate_root / "ab" / (filename + ".tail_fade.wav"), tail_fade_derivative)
    record = {
        "filename": filename,
        "role": definition["role"],
        "asset_id": definition["asset_id"],
        "source_sha256": source_hash,
        "derivative_sha256": derivative_hash,
        "derivative_path": str(sample_path.relative_to(REPOSITORY_ROOT)).replace("\\", "/"),
        "channels": 1,
        "bits_per_sample": 16,
        "sample_rate_hz": rate,
        "recipe": result.recipe,
        "treatment_details": result.details,
        "before": before_measurement,
        "after": after_measurement,
        "ab": ab_details,
        "conditional_measurement": conditional_measurement,
        "conditional_recommended_for_listening": conditional_recommended,
    }
    return record


def copy_auxiliary_gearbox_component(
    filename: str,
    source_directory: Path,
    candidate_root: Path,
) -> dict:
    definition = AUXILIARY_GEARBOX_COMPONENT_DEFINITIONS[filename]
    source_path = source_directory / filename
    source_hash = sha256_file(source_path)
    if source_hash != definition["sha256"]:
        raise ValueError(
            f"sha256 mismatch for {filename}: {source_hash} != {definition['sha256']}"
        )
    destination_path = candidate_root / "samples" / filename
    destination_path.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source_path, destination_path)
    audio_information = soundfile.info(source_path)
    return {
        "filename": filename,
        "role": definition["role"],
        "asset_id": definition["asset_id"],
        "source_sha256": source_hash,
        "derivative_sha256": sha256_file(destination_path),
        "derivative_path": str(destination_path.relative_to(REPOSITORY_ROOT)).replace("\\", "/"),
        "channels": int(audio_information.channels),
        "sample_rate_hz": int(audio_information.samplerate),
        "frames": int(audio_information.frames),
        "recipe": "immutable_stereo_component_copy_v1",
    }


def build_candidate_inventory(records: list[dict], auxiliary_records: list[dict]) -> dict:
    entries = []
    for record in records:
        entry = {
            "filename": record["filename"],
            "role": record["role"],
            "asset_id": record["asset_id"],
            "sha256": record["derivative_sha256"],
            "original_source_filename": record["filename"],
            "original_source_sha256": record["source_sha256"],
            "preparation_recipe": record["recipe"],
            "already_prepared": True,
        }
        if record["role"] == "engine_loop":
            metrics = record["treatment_details"]["loop_metrics"]
            entry["loop_crossfade_frames"] = int(metrics["crossfade_frames"])
            entry["loop_crossfade_seconds"] = float(metrics["crossfade_seconds"])
            entry["loop_crossfade_law"] = metrics["crossfade_law"]
            entry["endpoint_correlation"] = float(metrics["value_correlation"])
            entry["loop_start_frame"] = 0
            entry["loop_end_frame_exclusive"] = int(record["after"]["frames"])
            entry["reference_revolutions_per_minute"] = AUTHORED_REFERENCE_RPM[record["asset_id"]]
            entry["reference_source"] = "authored_shipped_manifest_preserved"
        entries.append(entry)
    for record in auxiliary_records:
        entries.append(
            {
                "filename": record["filename"],
                "role": record["role"],
                "asset_id": record["asset_id"],
                "sha256": record["derivative_sha256"],
                "original_source_filename": record["filename"],
                "original_source_sha256": record["source_sha256"],
                "preparation_recipe": record["recipe"],
                "component_only": True,
            }
        )
    return {"schema_version": 1, "sources": entries}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-directory", type=Path, default=DEFAULT_SOURCE_DIRECTORY)
    parser.add_argument("--candidate-root", type=Path, default=DEFAULT_CANDIDATE_ROOT)
    arguments = parser.parse_args()
    source_directory = arguments.source_directory.resolve()
    candidate_root = arguments.candidate_root.resolve()
    records = []
    for filename in SOURCE_DEFINITIONS:
        records.append(process_file(filename, source_directory, candidate_root))
        print(f"processed {filename}")
    auxiliary_records = []
    for filename in AUXILIARY_GEARBOX_COMPONENT_DEFINITIONS:
        auxiliary_records.append(
            copy_auxiliary_gearbox_component(filename, source_directory, candidate_root)
        )
        print(f"copied {filename}")
    (candidate_root / "measurements").mkdir(parents=True, exist_ok=True)
    (candidate_root / "measurements" / "per_sample.json").write_text(
        json.dumps(records, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (candidate_root / "measurements" / "auxiliary_components.json").write_text(
        json.dumps(auxiliary_records, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    inventory = build_candidate_inventory(records, auxiliary_records)
    (candidate_root / "recipes").mkdir(parents=True, exist_ok=True)
    (candidate_root / "recipes" / "candidate_source_inventory.json").write_text(
        json.dumps(inventory, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"candidate root: {candidate_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
