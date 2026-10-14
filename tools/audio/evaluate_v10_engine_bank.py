"""Evaluate the prepared V10 engine loops as a mastering-style sonic audit.

Analyzes every prepared engine loop declared by the canonical bank manifest
(powered and coast collections, exactly as the sampler plays them at unity
playback rate) and produces:

- a deterministic JSON report plus a human-readable markdown summary covering
  spectral balance per octave, narrow resonances above the spectral envelope,
  sub-fundamental rumble, direct-current offset and short-term dynamics;
- rule-derived polish settings (high-pass, shelves, bells, optional
  dynamic-range compression) computed only from measured values;
- preview renders written exclusively under ``scratch/audio/``: a level
  normalized solo loop, a roving one-sixth-octave band sweep and a level
  matched polished preview applying the derived settings.

The tool is read-only over the bank: it never rewrites runtime WAVs, the
manifest or the source inventory. Intentional polish changes go through the
offline preparation flow in a separate change.
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

import numpy as np
from scipy.signal import butter, lfilter, sosfiltfilt
from tools.audio import build_canonical_v10_engine_bank as canonical
from tools.common.output_policy import validate_output_path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BANK_DIRECTORY = ROOT / "game/sounds/banks/v10-v2-bank"
DEFAULT_OUTPUT_ROOT = ROOT / "scratch/audio/v10-polish-audit"

SAMPLE_RATE = 44100
ANALYSIS_WINDOW_FRAMES = 1 << 15
ANALYSIS_HOP_FRAMES = 1 << 13
ANALYSIS_MAXIMUM_FRAMES = 512
RESONANCE_MINIMUM_EXCESS_DB = 5.0
RESONANCE_MAXIMUM_WIDTH_HZ = 260.0
RESONANCE_MINIMUM_HZ = 40.0
RESONANCE_MAXIMUM_HZ = 11000.0
NOTCH_MINIMUM_EXCESS_DB = 6.0
NOTCH_MINIMUM_HZ = 130.0
NOTCH_MAXIMUM_HZ = 9000.0
NOTCH_MAXIMUM_GAIN_DB = 9.0
TILT_FITTING_HERTZ = (250.0, 8000.0)
TILT_REFERENCE_HERTZ = 1000.0
SUB_FUNDAMENTAL_BAND_HZ = (18.0, 32.0)
SUB_FUNDAMENTAL_LIMIT_DBFS = -48.0
PRIMARY_OCTAVE_EDGES_HZ = (31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0)
COMPRESSION_MAXIMUM_CREST_DB = 13.0
COMPRESSION_MAXIMUM_SHORT_TERM_SPREAD_DB = 9.0
SOLO_MINIMUM_SECONDS = 10.0
SWEEP_SECONDS = 12.0
SWEEP_MINIMUM_HZ = 40.0
SWEEP_MAXIMUM_HZ = 16000.0
SWEEP_WINDOW_FRAMES = 8192
SWEEP_HOP_FRAMES = 2048
SOLO_TARGET_PEAK_DBFS = -3.0
SWEEP_TARGET_PEAK_DBFS = -6.0
MUD_BELL_BAND_HZ = (180.0, 520.0)
MUD_BELL_MINIMUM_EXCESS_DB = 3.0
MUD_BELL_MAXIMUM_GAIN_DB = 4.0
HARSHNESS_BELL_BAND_HZ = (2000.0, 4800.0)
HARSHNESS_BELL_MINIMUM_EXCESS_DB = 3.0
HARSHNESS_BELL_MAXIMUM_GAIN_DB = 4.5
TREBLE_SHELF_BAND_HZ = (9000.0, 15000.0)
TREBLE_SHELF_MINIMUM_EXCESS_DB = 4.0
TREBLE_SHELF_MAXIMUM_GAIN_DB = 5.0
DIRECT_CURRENT_LIMIT = 1e-4
FREQUENCY_GRID_MINIMUM_HZ = 20.0
FREQUENCY_GRID_MAXIMUM_HZ = 20000.0
GRID_POINTS_PER_OCTAVE = 96
ENVELOPE_HALF_WIDTH_OCTAVES = 0.5
MAXIMUM_SETTINGS_COUNT = 10
COMPRESSOR_RATIO = 2.2
COMPRESSOR_ATTACK_SECONDS = 0.015
COMPRESSOR_RELEASE_SECONDS = 0.12
COMPRESSOR_GAIN_SMOOTHING_SECONDS = 0.005
MAGNITUDE_FILTER_TRANSITION_OCTAVES = 0.5
THIRD_OCTAVE_CENTERS_HZ = (
    20.0, 25.0, 31.5, 40.0, 50.0, 63.0, 80.0, 100.0, 125.0, 160.0,
    200.0, 250.0, 315.0, 400.0, 500.0, 630.0, 800.0, 1000.0, 1250.0, 1600.0,
    2000.0, 2500.0, 3150.0, 4000.0, 5000.0, 6300.0, 8000.0, 10000.0, 12500.0, 16000.0,
)
CLUSTER_MINIMUM_EXCESS_DB = 2.0
CLUSTER_MINIMUM_BANDS = 2
CLUSTER_MINIMUM_HZ = 160.0
CLUSTER_MAXIMUM_HZ = 6500.0
CLUSTER_GAIN_PROPORTION = 0.45
CLUSTER_MAXIMUM_GAIN_DB = 4.5
RINGING_MINIMUM_EXCESS_DB = 11.0
RINGING_COMB_MATCH_TOLERANCE = 0.02
RINGING_MINIMUM_HZ = 250.0
RINGING_MAXIMUM_HZ = 8000.0
RINGING_MAXIMUM_GAIN_DB = 8.0
RINGING_GAIN_PROPORTION = 0.5
MINIMUM_NOTCH_WIDTH_OCTAVES = 0.08
MAXIMUM_NOTCH_WIDTH_OCTAVES = 0.35


def log2_ratio(hz: float) -> float:
    return math.log2(max(hz, 1e-6) / TILT_REFERENCE_HERTZ)


def percentile_decibels(values: np.ndarray, percentile: float) -> float:
    return float(np.percentile(values, percentile))


def evaluate_engine_assets(bank_directory: Path) -> list[dict]:
    manifest = json.loads((bank_directory / "manifest.json").read_text(encoding="utf-8"))
    assets = []
    for collection in ("loops", "coast_loops"):
        for asset in manifest[collection]:
            assets.append(
                {
                    "asset_id": asset["id"],
                    "derived_filename": asset["derived_filename"],
                    "collection": collection,
                    "reference_revolutions_per_minute": asset["reference_revolutions_per_minute"],
                }
            )
    assets.sort(key=lambda item: (item["collection"], item["asset_id"]))
    for asset in assets:
        _, samples = canonical.read_wave_mono(bank_directory / asset["derived_filename"])
        asset["samples"] = samples
        asset["reference_hertz"] = (
            asset["reference_revolutions_per_minute"] / canonical.REVOLUTIONS_PER_MINUTE_PER_HERTZ
        )
    return assets


def windowed_starts(sample_count: int) -> list[int]:
    frame_count = 1 + max(0, (sample_count - ANALYSIS_WINDOW_FRAMES) // ANALYSIS_HOP_FRAMES)
    frame_count = min(frame_count, ANALYSIS_MAXIMUM_FRAMES)
    if frame_count <= 1:
        return [0]
    stride = max(1, (sample_count - ANALYSIS_WINDOW_FRAMES) // (frame_count - 1))
    return [index * stride for index in range(frame_count)]


def median_spectrum_decibels(samples: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    window = np.hanning(ANALYSIS_WINDOW_FRAMES)
    magnitudes = []
    for start in windowed_starts(samples.size):
        segment = samples[start : start + ANALYSIS_WINDOW_FRAMES] * window
        magnitudes.append(np.abs(np.fft.rfft(segment)))
    spectrum = np.median(np.asarray(magnitudes), axis=0)
    frequencies = np.arange(spectrum.size) * SAMPLE_RATE / ANALYSIS_WINDOW_FRAMES
    amplitudes = 2.0 * spectrum / (float(window.sum()) + 1e-12)
    decibels = 20.0 * np.log10(np.maximum(amplitudes, 1e-7))
    usable = (frequencies >= FREQUENCY_GRID_MINIMUM_HZ) & (frequencies <= FREQUENCY_GRID_MAXIMUM_HZ)
    return frequencies[usable], decibels[usable]


def logarithmic_grid() -> np.ndarray:
    octave_span = math.log2(FREQUENCY_GRID_MAXIMUM_HZ / FREQUENCY_GRID_MINIMUM_HZ)
    point_count = int(round(octave_span * GRID_POINTS_PER_OCTAVE))
    return FREQUENCY_GRID_MINIMUM_HZ * 2.0 ** (np.arange(point_count) * octave_span / point_count)


def spectral_envelope_on_grid(frequencies: np.ndarray, decibels: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    grid = logarithmic_grid()
    grid_decibels = np.interp(np.log2(grid), np.log2(frequencies), decibels)
    reach = int(round(ENVELOPE_HALF_WIDTH_OCTAVES * GRID_POINTS_PER_OCTAVE))
    grid_size = grid_decibels.size
    envelope = np.empty(grid_size)
    for index in range(grid_size):
        lower = max(0, index - reach)
        upper = min(grid_size, index + reach + 1)
        envelope[index] = float(np.mean(grid_decibels[lower:upper]))
    return grid, envelope


def detect_narrow_resonances(frequencies: np.ndarray, decibels: np.ndarray, envelope: np.ndarray) -> list[dict]:
    excess = decibels - envelope
    usable = (frequencies >= RESONANCE_MINIMUM_HZ) & (frequencies <= RESONANCE_MAXIMUM_HZ)
    detector = np.where(usable, excess, -np.inf)
    resonances = []
    index = 1
    while index < detector.size - 1:
        if detector[index] >= detector[index - 1] and detector[index] >= detector[index + 1] and detector[index] >= RESONANCE_MINIMUM_EXCESS_DB:
            crossing = detector[index] * 0.5
            lower_bound = index
            while lower_bound > 0 and detector[lower_bound - 1] >= crossing:
                lower_bound -= 1
            upper_bound = index
            while upper_bound < detector.size - 1 and detector[upper_bound + 1] >= crossing:
                upper_bound += 1
            width_hz = float(frequencies[upper_bound] - frequencies[lower_bound])
            if width_hz <= RESONANCE_MAXIMUM_WIDTH_HZ:
                resonances.append(
                    {
                        "center_hz": float(frequencies[index]),
                        "excess_db": float(detector[index]),
                        "width_hz": width_hz,
                    }
                )
                index = upper_bound + 1
                continue
        index += 1
    resonances.sort(key=lambda item: -item["excess_db"])
    return resonances[:12]


def octave_band_levels(frequencies: np.ndarray, decibels: np.ndarray) -> list[dict]:
    rows = []
    for lower_hz, upper_hz in zip(PRIMARY_OCTAVE_EDGES_HZ[:-1], PRIMARY_OCTAVE_EDGES_HZ[1:]):
        inside = (frequencies >= lower_hz) & (frequencies < upper_hz)
        if not np.any(inside):
            rows.append({"lower_hz": float(lower_hz), "upper_hz": float(upper_hz), "level_db": -120.0})
            continue
        level_amplitude = band_power_amplitude(frequencies[inside], decibels[inside])
        rows.append(
            {
                "lower_hz": float(lower_hz),
                "upper_hz": float(upper_hz),
                "level_db": float(20.0 * math.log10(max(level_amplitude, 1e-9))),
            }
        )
    return rows


def band_power_amplitude(frequencies: np.ndarray, decibels: np.ndarray) -> float:
    amplitude = 10.0 ** (decibels / 20.0)
    power = float(np.trapezoid(amplitude**2, frequencies))
    return math.sqrt(max(power, 1e-24))


def fit_tilt_reference(frequencies: np.ndarray, decibels: np.ndarray) -> tuple[float, float]:
    inside = (frequencies >= TILT_FITTING_HERTZ[0]) & (frequencies <= TILT_FITTING_HERTZ[1])
    log_frequencies = np.log2(frequencies[inside] / TILT_REFERENCE_HERTZ)
    slope_db_per_octave, intercept_decibels = np.polyfit(log_frequencies, decibels[inside], 1)
    return float(slope_db_per_octave), float(intercept_decibels)


def measure_dynamics(samples: np.ndarray) -> tuple[float, float, float, float]:
    window = np.hanning(ANALYSIS_WINDOW_FRAMES)
    normalization = float(np.mean(window))
    level_decibels = []
    for start in windowed_starts(samples.size):
        segment = samples[start : start + ANALYSIS_WINDOW_FRAMES]
        level_decibels.append(
            20.0 * math.log10(max(math.sqrt(float(np.mean(np.square(segment * window))) / normalization), 1e-7))
        )
    levels = np.asarray(level_decibels)
    peak_dbfs = float(20.0 * math.log10(max(float(np.max(np.abs(samples))), 1e-7)))
    overall_rms_dbfs = float(20.0 * math.log10(max(math.sqrt(float(np.mean(np.square(samples)))), 1e-7)))
    crest_db = peak_dbfs - overall_rms_dbfs
    spread_db = percentile_decibels(levels, 95) - percentile_decibels(levels, 10)
    return peak_dbfs, overall_rms_dbfs, crest_db, spread_db


def band_power_amplitude(frequencies: np.ndarray, decibels: np.ndarray) -> float:
    amplitude = 10.0 ** (decibels / 20.0)
    power = float(np.trapezoid(amplitude**2, frequencies))
    return math.sqrt(max(power, 1e-24))


def third_octave_band_levels(frequencies: np.ndarray, decibels: np.ndarray) -> list[dict]:
    rows = []
    for index, center_hz in enumerate(THIRD_OCTAVE_CENTERS_HZ):
        lower_hz = center_hz / 2.0 ** (1.0 / 6.0)
        upper_hz = center_hz * 2.0 ** (1.0 / 6.0)
        inside = (frequencies >= lower_hz) & (frequencies < upper_hz)
        if not np.any(inside):
            continue
        level_amplitude = band_power_amplitude(frequencies[inside], decibels[inside])
        rows.append(
            {
                "center_hz": float(center_hz),
                "lower_hz": float(lower_hz),
                "upper_hz": float(upper_hz),
                "level_db": float(20.0 * math.log10(max(level_amplitude, 1e-9))),
            }
        )
    return rows


def collect_band_humps(
    third_octave_rows: list[dict],
) -> list[dict]:
    neighbours = []
    for index, row in enumerate(third_octave_rows):
        left = third_octave_rows[index - 1] if index > 0 else None
        right = third_octave_rows[index + 1] if index + 1 < len(third_octave_rows) else None
        if left is None and right is None:
            neighbours.append(row["level_db"])
            continue
        if left is None:
            neighbours.append(right["level_db"])
        elif right is None:
            neighbours.append(left["level_db"])
        else:
            neighbours.append(0.5 * (left["level_db"] + right["level_db"]))
    excess_locals = [row["level_db"] - neighbour for row, neighbour in zip(third_octave_rows, neighbours)]
    center_zones = []
    for index, row in enumerate(third_octave_rows):
        center_zones.append(
            (CLUSTER_MINIMUM_HZ <= row["center_hz"] <= CLUSTER_MAXIMUM_HZ, excess_locals[index])
        )
    flagged = [index for index, (within, excess_local) in enumerate(center_zones) if within and excess_local > 2.5]
    humps = []
    run = []
    for index in flagged:
        if run and index != run[-1] + 1:
            humps.append(run)
            run = []
        run.append(index)
    if run:
        humps.append(run)
    derived = []
    for run_indexes in humps:
        rows = [third_octave_rows[index] for index in run_indexes]
        center_hz = math.sqrt(rows[0]["lower_hz"] * rows[-1]["upper_hz"])
        mean_excess = float(np.mean([center_zones[index][1] for index in run_indexes]))
        peak_excess = max(center_zones[index][1] for index in run_indexes)
        is_single_band_standing = len(rows) == 1 and peak_excess > 4.0
        if len(rows) < CLUSTER_MINIMUM_BANDS and not is_single_band_standing:
            continue
        span_octaves = math.log2(rows[-1]["upper_hz"] / rows[0]["lower_hz"])
        width_octaves = 0.33 * (span_octaves + 0.66)
        if len(rows) == 1:
            width_octaves = max(0.35, mean_excess * 0.12)
        width_octaves = max(0.35, min(0.9, width_octaves))
        derived.append(
            {
                "cluster_center_hz": round(center_hz, 1),
                "mean_excess_db": round(mean_excess, 2),
                "peak_excess_db": round(peak_excess, 2),
                "span_octaves": round(span_octaves, 2),
                "width_octaves": round(width_octaves, 3),
                "band_count": len(rows),
            }
        )
    derived.sort(key=lambda item: -item["peak_excess_db"])
    return derived


def detect_ringing_resonances(
    resonances: list[dict], reference_hertz: float
) -> list[dict]:
    ringing = []
    for resonance in resonances:
        center_hz = resonance["center_hz"]
        if not (
            resonance["excess_db"] >= RINGING_MINIMUM_EXCESS_DB
            and RINGING_MINIMUM_HZ <= center_hz <= RINGING_MAXIMUM_HZ
            and center_hz > 2.5 * reference_hertz
        ):
            continue
        harmonic = round(center_hz / reference_hertz)
        if harmonic >= 1:
            distance = abs(center_hz - harmonic * reference_hertz) / center_hz
            if distance <= RINGING_COMB_MATCH_TOLERANCE:
                continue
        lower_hz = max(center_hz - resonance["width_hz"] * 0.5, center_hz * 0.5)
        upper_hz = center_hz + resonance["width_hz"] * 0.5
        width_octaves = math.log2(upper_hz / lower_hz)
        ringing.append(
            {
                "center_hz": round(center_hz, 1),
                "excess_db": round(resonance["excess_db"], 2),
                "width_hz": round(resonance["width_hz"], 1),
                "nearest_comb_multiple": harmonic,
                "comb_deviation": round(float(distance), 4),
                "width_octaves": round(max(MINIMUM_NOTCH_WIDTH_OCTAVES, min(MAXIMUM_NOTCH_WIDTH_OCTAVES, width_octaves * 0.5)), 3),
            }
        )
    ringing.sort(key=lambda item: -item["excess_db"])
    return ringing[:6]


def merge_neighbouring_ringing(ringing: list[dict]) -> list[dict]:
    merged = []
    for item in ringing:
        if merged and abs(item["center_hz"] - merged[-1]["center_hz"]) < 0.02 * item["center_hz"]:
            stronger, weaker = (merged[-1], item) if merged[-1]["excess_db"] >= item["excess_db"] else (item, merged[-1])
            merged[-1] = {
                **stronger,
                "merged_count": stronger.get("merged_count", 1) + 1,
            }
            continue
        merged.append(item)
    return merged


def derive_polish_settings(
    frequencies: np.ndarray,
    decibels: np.ndarray,
    resonances: list[dict],
    reference_hertz: float,
) -> tuple[list[dict], list[dict], list[dict]]:
    slope_db_per_octave, intercept_decibels = fit_tilt_reference(frequencies, decibels)
    third_octave_rows = third_octave_band_levels(frequencies, decibels)
    clusters = collect_band_humps(third_octave_rows)
    cluster_settings = []
    for cluster in clusters[:4]:
        cluster_settings.append(
            {
                "kind": "bell",
                "purpose": "musical_band_hump",
                "center_hz": cluster["cluster_center_hz"],
                "gain_db": round(-min(CLUSTER_MAXIMUM_GAIN_DB, CLUSTER_GAIN_PROPORTION * cluster["mean_excess_db"]), 2),
                "width_octaves": cluster["width_octaves"],
                "measured": cluster,
            }
        )
    ringing = merge_neighbouring_ringing(detect_ringing_resonances(resonances, reference_hertz))
    settings: list[dict] = list(cluster_settings)
    for item in ringing:
        settings.append(
            {
                "kind": "bell",
                "purpose": "ringing_notch",
                "center_hz": item["center_hz"],
                "gain_db": round(-min(RINGING_MAXIMUM_GAIN_DB, RINGING_GAIN_PROPORTION * item["excess_db"]), 2),
                "width_octaves": item["width_octaves"],
                "measured": {
                    "excess_db": item["excess_db"],
                    "width_hz": item["width_hz"],
                    "nearest_comb_multiple": item["nearest_comb_multiple"],
                    "comb_deviation": item["comb_deviation"],
                },
            }
        )
    sub_inside = (frequencies >= SUB_FUNDAMENTAL_BAND_HZ[0]) & (frequencies <= SUB_FUNDAMENTAL_BAND_HZ[1])
    if np.any(sub_inside):
        sub_level_dbfs = float(np.median(decibels[sub_inside]))
        if sub_level_dbfs > SUB_FUNDAMENTAL_LIMIT_DBFS:
            settings.append(
                {
                    "kind": "highpass",
                    "purpose": "sub_fundamental_rumble",
                    "corner_hz": round(min(SUB_FUNDAMENTAL_BAND_HZ[1] + 2.0, 0.75 * reference_hertz), 1),
                    "slope_db_per_octave": 18.0,
                    "measured_sub_level_dbfs": round(sub_level_dbfs, 2),
                }
            )
    treble_probe = np.linspace(TREBLE_SHELF_BAND_HZ[0], TREBLE_SHELF_BAND_HZ[1], 32)
    treble_measured = float(np.mean(np.interp(np.log2(treble_probe), np.log2(frequencies), decibels)))
    treble_reference = float(
        np.mean(slope_db_per_octave * np.log2(treble_probe / TILT_REFERENCE_HERTZ) + intercept_decibels)
    )
    treble_excess = treble_measured - treble_reference
    if treble_excess > TREBLE_SHELF_MINIMUM_EXCESS_DB:
        settings.append(
            {
                "kind": "high_shelf",
                "purpose": "treble_tame",
                "center_hz": 9000.0,
                "gain_db": round(-min(TREBLE_SHELF_MAXIMUM_GAIN_DB, 0.5 * treble_excess), 2),
                "transition_octaves": MAGNITUDE_FILTER_TRANSITION_OCTAVES,
                "measured_excess_db": round(treble_excess, 2),
            }
        )
    ordering = {"highpass": 0, "high_shelf": 1, "bell": 2}
    ordered = sorted(settings, key=lambda item: (ordering[item["kind"]], -item.get("gain_db", 0.0)))
    return ordered[:MAXIMUM_SETTINGS_COUNT], clusters, ringing


def evaluate_asset(asset: dict) -> dict:
    samples = asset["samples"]
    reference_hertz = asset["reference_hertz"]
    frequencies, decibels = median_spectrum_decibels(samples)
    _, envelope = spectral_envelope_on_grid(frequencies, decibels)
    grid_envelope = np.interp(np.log2(frequencies), np.log2(logarithmic_grid()), envelope)
    resonances = detect_narrow_resonances(frequencies, decibels, grid_envelope)
    bands = octave_band_levels(frequencies, decibels)
    third_octave_rows = third_octave_band_levels(frequencies, decibels)
    slope_db_per_octave, intercept_decibels = fit_tilt_reference(frequencies, decibels)
    sub_inside = (frequencies >= SUB_FUNDAMENTAL_BAND_HZ[0]) & (frequencies <= SUB_FUNDAMENTAL_BAND_HZ[1])
    sub_level_dbfs = float(np.median(decibels[sub_inside])) if np.any(sub_inside) else -120.0
    peak_dbfs, overall_rms_dbfs, crest_db, spread_db = measure_dynamics(samples)
    direct_current = float(np.mean(samples))
    polish_settings, clusters, ringing = derive_polish_settings(
        frequencies, decibels, resonances, asset["reference_hertz"]
    )
    compression_action = None
    compression_note = (
        f"steady loop: crest {crest_db:.1f} dB and short-term spread {spread_db:.1f} dB stay below the"
        f" {COMPRESSION_MAXIMUM_CREST_DB:.0f} dB and {COMPRESSION_MAXIMUM_SHORT_TERM_SPREAD_DB:.0f} dB limits"
    )
    if crest_db > COMPRESSION_MAXIMUM_CREST_DB or spread_db > COMPRESSION_MAXIMUM_SHORT_TERM_SPREAD_DB:
        compression_action = {
            "ratio": COMPRESSOR_RATIO,
            "threshold_dbfs": round(overall_rms_dbfs, 2),
            "attack_seconds": COMPRESSOR_ATTACK_SECONDS,
            "release_seconds": COMPRESSOR_RELEASE_SECONDS,
            "reason": f"crest {crest_db:.1f} dB, short-term spread {spread_db:.1f} dB",
        }
    harmonic_comb_peaks = [
        item
        for item in resonances
        if round(item["center_hz"] / reference_hertz) >= 1
        and abs(item["center_hz"] - round(item["center_hz"] / reference_hertz) * reference_hertz)
        / item["center_hz"]
        <= RINGING_COMB_MATCH_TOLERANCE
    ]
    return {
        "asset_id": asset["asset_id"],
        "derived_filename": asset["derived_filename"],
        "collection": asset["collection"],
        "reference_revolutions_per_minute": asset["reference_revolutions_per_minute"],
        "reference_hertz": asset["reference_hertz"],
        "frames": int(samples.size),
        "duration_seconds": round(samples.size / SAMPLE_RATE, 3),
        "peak_dbfs": round(peak_dbfs, 3),
        "rms_dbfs": round(overall_rms_dbfs, 3),
        "crest_db": round(crest_db, 2),
        "short_term_spread_db": round(spread_db, 2),
        "direct_current_offset": float(f"{direct_current:.6g}"),
        "direct_current_correction": abs(direct_current) > DIRECT_CURRENT_LIMIT,
        "tilt_slope_db_per_octave": round(slope_db_per_octave, 3),
        "tilt_intercept_db_at_1khz": round(intercept_decibels, 3),
        "sub_fundamental_level_dbfs": round(sub_level_dbfs, 2),
        "octave_bands": bands,
        "third_octave_bands": third_octave_rows,
        "band_clusters": clusters,
        "harmonic_comb_peaks": [
            {
                "center_hz": round(item["center_hz"], 1),
                "excess_db": round(item["excess_db"], 2),
                "width_hz": round(item["width_hz"], 1),
                "comb_multiple": round(item["center_hz"] / reference_hertz, 2),
            }
            for item in harmonic_comb_peaks
        ],
        "ringing_resonances": ringing,
        "resonances": [
            {
                "center_hz": round(item["center_hz"], 1),
                "excess_db": round(item["excess_db"], 2),
                "width_hz": round(item["width_hz"], 1),
            }
            for item in resonances
        ],
        "polish_settings": polish_settings,
        "compression": compression_action,
        "compression_note": compression_note,
    }


def build_report(bank_directory: Path) -> dict:
    assets = evaluate_engine_assets(bank_directory)
    return {"bank_path": bank_directory.relative_to(ROOT).as_posix(), "assets": [evaluate_asset(asset) for asset in assets]}


def magnitude_response(frequencies: np.ndarray, settings: list[dict]) -> np.ndarray:
    response = np.ones(frequencies.size)
    for setting in settings:
        if setting["kind"] == "highpass":
            corner = setting["corner_hz"]
            exponent_octaves = np.log2(np.maximum(frequencies, 1e-6) / corner)
            attenuation_db = setting["slope_db_per_octave"] * np.minimum(exponent_octaves, 0.0) / (
                1.0 + np.exp(-exponent_octaves / MAGNITUDE_FILTER_TRANSITION_OCTAVES * 8.0)
            )
            response *= 10.0 ** (attenuation_db / 20.0)
        elif setting["kind"] == "high_shelf":
            transition = setting["transition_octaves"]
            excess_octaves = np.log2(np.maximum(frequencies, 1e-6) / setting["center_hz"]) / transition
            ramp = np.clip((excess_octaves + 1.0) / 2.0, 0.0, 1.0)
            ramp = ramp * ramp * (3.0 - 2.0 * ramp)
            response *= 10.0 ** (setting["gain_db"] / 20.0 * ramp)
        else:
            width_octaves = setting["width_octaves"]
            distance_octaves = np.log2(np.maximum(frequencies, 1e-6) / setting["center_hz"]) / width_octaves
            weight = np.exp(-0.5 * distance_octaves**2)
            response *= 1.0 + (10.0 ** (setting["gain_db"] / 20.0) - 1.0) * weight
    return response


def apply_polish_filters(samples: np.ndarray, settings: list[dict]) -> np.ndarray:
    spectrum = np.fft.rfft(samples)
    frequencies = np.arange(spectrum.size) * SAMPLE_RATE / samples.size
    frequencies[0] = max(frequencies[0], 0.5)
    return np.fft.irfft(spectrum * magnitude_response(frequencies, settings), n=samples.size)


def apply_compression(samples: np.ndarray, settings: dict) -> np.ndarray:
    decay_coefficient = math.exp(-1.0 / (settings["release_seconds"] * SAMPLE_RATE))
    envelope = lfilter([1.0 - decay_coefficient], [1.0, -decay_coefficient], np.abs(samples))
    threshold_linear = 10.0 ** (settings["threshold_dbfs"] / 20.0)
    ratio = settings["ratio"]
    with np.errstate(divide="ignore", invalid="ignore"):
        gain = np.where(
            envelope > threshold_linear,
            threshold_linear * (envelope / threshold_linear) ** (1.0 / ratio) / np.maximum(envelope, 1e-12),
            1.0,
        )
    smoothing_coefficient = math.exp(-1.0 / (COMPRESSOR_GAIN_SMOOTHING_SECONDS * SAMPLE_RATE))
    gain = lfilter([1.0 - smoothing_coefficient], [1.0, -smoothing_coefficient], gain)
    return samples * gain


def render_audio_payload(samples: np.ndarray) -> bytes:
    return canonical.wave_mono16_bytes(samples)


def normalize_peak(samples: np.ndarray, target_peak_dbfs: float) -> np.ndarray:
    peak = float(np.max(np.abs(samples)))
    if peak < 1e-9:
        return samples
    return samples * (10.0 ** (target_peak_dbfs / 20.0) / peak)


def tile_seamless(samples: np.ndarray, minimum_frames: int) -> np.ndarray:
    if samples.size >= minimum_frames:
        return samples
    tiled = samples
    while tiled.size < minimum_frames:
        tiled = np.concatenate((tiled, samples))
    return tiled


def render_solo(samples: np.ndarray) -> bytes:
    tiled = tile_seamless(samples, int(SOLO_MINIMUM_SECONDS * SAMPLE_RATE))
    return render_audio_payload(normalize_peak(tiled, SOLO_TARGET_PEAK_DBFS))


def render_sweep(samples: np.ndarray) -> bytes:
    tiled = tile_seamless(samples, int(SWEEP_SECONDS * SAMPLE_RATE))
    total_frames = tiled.size
    processed = np.zeros(total_frames, dtype=np.float64)
    normalization = np.zeros(total_frames, dtype=np.float64)
    window = np.hanning(SWEEP_WINDOW_FRAMES)
    window_squared = window * window
    glide_ratio = SWEEP_MAXIMUM_HZ / SWEEP_MINIMUM_HZ
    starts = list(range(0, total_frames - SWEEP_WINDOW_FRAMES, SWEEP_HOP_FRAMES))
    starts.append(total_frames - SWEEP_WINDOW_FRAMES)
    for start in starts:
        center_time = (start + SWEEP_WINDOW_FRAMES * 0.5) / SAMPLE_RATE
        center_hz = float(np.clip(SWEEP_MINIMUM_HZ * glide_ratio ** (center_time / SWEEP_SECONDS), SWEEP_MINIMUM_HZ, SWEEP_MAXIMUM_HZ))
        lower_hz = center_hz / 2.0 ** (1.0 / 12.0)
        upper_hz = center_hz * 2.0 ** (1.0 / 12.0)
        sos = butter(
            3,
            [lower_hz / (SAMPLE_RATE * 0.5), upper_hz / (SAMPLE_RATE * 0.5)],
            btype="bandpass",
            output="sos",
        )
        indices = (start + np.arange(SWEEP_WINDOW_FRAMES)) % total_frames
        filtered = sosfiltfilt(sos, tiled[indices])
        processed[start : start + SWEEP_WINDOW_FRAMES] += filtered * window_squared
        normalization[start : start + SWEEP_WINDOW_FRAMES] += window_squared
    processed /= np.maximum(normalization, 1e-12)
    return render_audio_payload(normalize_peak(processed, SWEEP_TARGET_PEAK_DBFS))


def render_polished(samples: np.ndarray, evaluation: dict) -> bytes:
    processed = samples - float(np.mean(samples))
    processed = apply_polish_filters(processed, evaluation["polish_settings"])
    if evaluation["compression"] is not None:
        processed = apply_compression(processed, evaluation["compression"])
    processed *= math.sqrt(float(np.mean(np.square(samples)))) / max(math.sqrt(float(np.mean(np.square(processed)))), 1e-12)
    peak = float(np.max(np.abs(processed)))
    if peak > 0.95:
        processed *= 0.95 / peak
    return render_audio_payload(processed)


def render_markdown(report: dict) -> str:
    lines = [
        "# V10 engine loops sonic audit",
        "",
        f"Bank: `{report['bank_path']}` (mono 44.1 kHz PCM16, evaluated at unity playback rate)",
        "",
        "Every recommendation below is derived from measurements with the fixed rules in",
        "`tools/audio/evaluate_v10_engine_bank.py`; audition renders live in `renders/`.",
        "",
    ]
    for asset in report["assets"]:
        lines.extend(
            [
                f"## {asset['asset_id']} (`{asset['derived_filename']}`, {asset['collection']})",
                "",
                f"- {asset['duration_seconds']} s, peak {asset['peak_dbfs']} dBFS, RMS {asset['rms_dbfs']} dBFS,"
                f" crest {asset['crest_db']} dB, short-term spread {asset['short_term_spread_db']} dB",
                f"- tilt {asset['tilt_slope_db_per_octave']} dB/oct; sub-fundamental (18–32 Hz) level"
                f" {asset['sub_fundamental_level_dbfs']} dBFS; DC offset {asset['direct_current_offset']}",
                f"- reference pitch {asset['reference_revolutions_per_minute']} rpm"
                f" ({asset['reference_hertz']:.2f} Hz firing comb)",
                "",
                "| octave | level dB |",
                "|---|---|",
            ]
        )
        for band in asset["octave_bands"]:
            lines.append(f"| {band['lower_hz']:.0f}–{band['upper_hz']:.0f} Hz | {band['level_db']:.1f} |")
        if asset["resonances"]:
            lines.extend(["", "Narrow resonances (excess over the half-octave envelope):"])
            for resonance in asset["resonances"]:
                lines.append(
                    f"- {resonance['center_hz']} Hz: +{resonance['excess_db']} dB, width {resonance['width_hz']} Hz"
                )
        if asset["band_clusters"]:
            lines.extend(
                ["", "Musical band humps above the neighbouring third-octave level (EQ targets):"]
            )
            for cluster in asset["band_clusters"]:
                lines.append(
                    f"- {cluster['cluster_center_hz']} Hz: mean +{cluster['mean_excess_db']} dB"
                    f" (peak +{cluster['peak_excess_db']} dB) over {cluster['band_count']}"
                    f" third-octave bands, span {cluster['span_octaves']} oct"
                )
        if asset["harmonic_comb_peaks"]:
            lines.extend(
                [
                    "",
                    "Harmonic comb peaks (part of the firing comb — retained, not EQ targets):",
                ]
            )
            for peak in asset["harmonic_comb_peaks"][:8]:
                lines.append(
                    f"- {peak['center_hz']} Hz (x{peak['comb_multiple']} of {asset['reference_hertz']:.2f} Hz):"
                    f" +{peak['excess_db']} dB over envelope"
                )
            remaining = len(asset["harmonic_comb_peaks"]) - 8
            if remaining > 0:
                lines.append(f"- … and {remaining} more comb multiples")
        if asset["ringing_resonances"]:
            lines.extend(["", "Ringing resonances off the firing comb (notch targets):"])
            for item in asset["ringing_resonances"]:
                lines.append(
                    f"- {item['center_hz']} Hz: +{item['excess_db']} dB, width {item['width_hz']} Hz,"
                    f" closest comb multiple x{item['nearest_comb_multiple']}"
                    f" ({item['comb_deviation'] * 100:.1f}% away)"
                )
        if asset["polish_settings"] or asset["compression"] or asset["direct_current_correction"]:
            lines.extend(["", "Derived polish chain:"])
            for setting in asset["polish_settings"]:
                if setting["kind"] == "highpass":
                    lines.append(
                        f"- high-pass {setting['corner_hz']} Hz at {setting['slope_db_per_octave']} dB/oct"
                        f" ({setting['purpose']})"
                    )
                elif setting["kind"] == "high_shelf":
                    lines.append(
                        f"- high shelf {setting['center_hz']} Hz {setting['gain_db']} dB ({setting['purpose']})"
                    )
                else:
                    lines.append(
                        f"- bell {setting['center_hz']} Hz {setting['gain_db']} dB,"
                        f" width {setting['width_octaves']} oct ({setting['purpose']})"
                    )
            if asset["direct_current_correction"]:
                lines.append("- remove DC offset")
            if asset["compression"]:
                lines.append(
                    f"- compressor {asset['compression']['ratio']}:1 at {asset['compression']['threshold_dbfs']} dBFS,"
                    f" attack {asset['compression']['attack_seconds'] * 1000:.0f} ms,"
                    f" release {asset['compression']['release_seconds'] * 1000:.0f} ms"
                    f" ({asset['compression']['reason']})"
                )
        else:
            lines.extend(
                ["", "Derived polish chain: only DC clean-up was needed — balance already matches the tilt envelope."]
            )
        lines.extend(
            [
                "",
                f"Compression verdict: {asset['compression_note']}.",
                "",
            ]
        )
    return "\n".join(lines) + "\n"


def write_report_outputs(report: dict, output_root: Path) -> tuple[Path, Path]:
    output_root.mkdir(parents=True, exist_ok=True)
    report_path = output_root / "report.json"
    summary_path = output_root / "report.md"
    validate_output_path(ROOT, report_path, "preview")
    validate_output_path(ROOT, summary_path, "preview")
    report_path.write_bytes(canonical.serialize_json(report))
    summary_path.write_text(render_markdown(report), encoding="utf-8")
    return report_path, summary_path


def write_render_outputs(report: dict, bank_directory: Path, output_root: Path, limited_asset_ids: list[str] | None) -> list[Path]:
    renders_directory = output_root / "renders"
    validate_output_path(ROOT, renders_directory, "preview")
    renders_directory.mkdir(parents=True, exist_ok=True)
    written = []
    for asset in report["assets"]:
        if limited_asset_ids is not None and asset["asset_id"] not in limited_asset_ids:
            continue
        _, samples = canonical.read_wave_mono(bank_directory / asset["derived_filename"])
        render_kind_payloads = (
            ("solo", render_solo(samples)),
            ("sweep", render_sweep(samples)),
            ("polished", render_polished(samples, asset)),
        )
        for kind, payload in render_kind_payloads:
            destination = renders_directory / f"{asset['asset_id']}_{kind}.wav"
            validate_output_path(ROOT, destination, "preview")
            destination.write_bytes(payload)
            written.append(destination)
    return written


def main() -> int:
    argument_parser = argparse.ArgumentParser()
    argument_parser.add_argument("--bank-directory", type=Path, default=DEFAULT_BANK_DIRECTORY)
    argument_parser.add_argument("--output-root", type=Path, default=DEFAULT_OUTPUT_ROOT)
    argument_parser.add_argument("--analyze", action="store_true")
    argument_parser.add_argument("--render", action="store_true")
    argument_parser.add_argument("--limit-asset", action="append", default=None)
    arguments = argument_parser.parse_args()
    if not arguments.analyze and not arguments.render:
        arguments.analyze = True

    report = build_report(arguments.bank_directory)
    if arguments.render:
        written = write_render_outputs(
            report, arguments.bank_directory, arguments.output_root, arguments.limit_asset
        )
        print(f"rendered {len(written)} audition files under {arguments.output_root / 'renders'}")
        return 0

    report_path, summary_path = write_report_outputs(report, arguments.output_root)
    print(f"analysis written to {report_path} and {summary_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
