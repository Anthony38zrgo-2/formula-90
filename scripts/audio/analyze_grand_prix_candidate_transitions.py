#!/usr/bin/env python3
"""Measure candidate Grand Prix layer transitions and loop seams.

Reads a candidate bank manifest, resamples each adjacent loop pair to the
shared transition RPMs through the same preparation math as the runtime bank
(gain plus playback rate), and records level, band, crossfade and overshoot
evidence. Also verifies loop wrap continuity on the stored derivatives.
"""
from __future__ import annotations

import argparse
import json
import math
import wave
from fractions import Fraction
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from scipy.signal import resample_poly, welch

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BANK_DIRECTORY = (
    REPOSITORY_ROOT / "reports/audio-v10/grand-prix-sampler/candidates/v1/bank"
)

BAND_EDGES_HZ = (
    (20.0, 80.0),
    (80.0, 150.0),
    (150.0, 300.0),
    (300.0, 600.0),
    (600.0, 1200.0),
    (1200.0, 2500.0),
    (2500.0, 5000.0),
)


def dbfs(value: float) -> float:
    return 20.0 * math.log10(max(float(value), 1e-12))


def root_mean_square(samples: np.ndarray) -> float:
    if samples.size == 0:
        return 0.0
    return float(np.sqrt(np.mean(samples.astype(np.float64) ** 2)))


def read_pcm16_mono(path: Path) -> tuple[np.ndarray, int]:
    with wave.open(str(path), "rb") as handle:
        channels = handle.getnchannels()
        rate = handle.getframerate()
        frames = handle.getnframes()
        raw = handle.readframes(frames)
    samples = np.frombuffer(raw, dtype="<i2").astype(np.float64) / 32768.0
    if channels > 1:
        samples = samples.reshape(-1, channels).mean(axis=1)
    return np.asarray(samples), rate


def apply_rate(samples: np.ndarray, rate: int, ratio: float) -> np.ndarray:
    if abs(ratio - 1.0) < 1e-9:
        return samples.copy()
    fraction = Fraction(ratio).limit_denominator(8192)
    return resample_poly(samples, fraction.numerator, fraction.denominator)


def loop_level(samples: np.ndarray) -> float:
    margin = len(samples) // 10
    core = samples[margin: len(samples) - margin] if len(samples) > 4 * margin else samples
    return root_mean_square(core)


def band_shares(samples: np.ndarray, rate: int) -> dict[str, float]:
    if samples.size < 256:
        return {f"{int(low)}_{int(high)}": 0.0 for low, high in BAND_EDGES_HZ}
    frequencies, power = welch(samples, fs=rate, nperseg=min(samples.size, 1 << 14))
    total = float(np.sum(power)) + 1e-30
    return {
        f"{int(low)}_{int(high)}": float(100.0 * np.sum(power[(frequencies >= low) & (frequencies < high)]) / total)
        for low, high in BAND_EDGES_HZ
    }


def smoothstep_weights(position: float) -> tuple[float, float]:
    control = position * position * (3.0 - 2.0 * position)
    angle = control * math.pi / 2.0
    return math.cos(angle), math.sin(angle)


def crossfade_snapshot(
    from_samples: np.ndarray,
    to_samples: np.ndarray,
    position: float,
    offset: int,
) -> tuple[float, float]:
    length = min(from_samples.size, to_samples.size)
    if length <= 0:
        return 0.0, 0.0
    from_weight, to_weight = smoothstep_weights(position)
    start = offset % max(length, 1)
    indices = (np.arange(length) + start) % length
    blended = (
        from_samples[indices] * from_weight + to_samples[: length][indices] * to_weight
    )
    return root_mean_square(blended), float(np.max(np.abs(blended)))


def measure_loop_seam(samples: np.ndarray, rate: int) -> dict:
    wrap_step = float(abs(samples[0] - samples[-1]))
    adjacent = np.abs(np.diff(samples))
    internal_p99 = float(np.percentile(adjacent, 99.0)) if adjacent.size else 0.0
    repeated = np.concatenate([samples, samples, samples])
    repeated_steps = np.abs(np.diff(repeated))
    wrap_positions = [samples.size, 2 * samples.size]
    wrap_steps = [float(repeated_steps[position - 1]) for position in wrap_positions]
    return {
        "loop_frames": int(samples.size),
        "wrap_step": wrap_step,
        "internal_p99_adjacent_step": internal_p99,
        "wrap_step_over_internal_p99": float(wrap_step / max(internal_p99, 1e-9)),
        "repeated_wrap_steps": wrap_steps,
        "repeated_wrap_max_over_internal_p99": float(
            max(wrap_steps) / max(internal_p99, 1e-9)
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bank-directory", type=Path, default=DEFAULT_BANK_DIRECTORY)
    arguments = parser.parse_args()
    bank_directory: Path = arguments.bank_directory
    manifest = json.loads((bank_directory / "manifest.json").read_text(encoding="utf-8"))
    loops = {loop["id"]: loop for loop in manifest["loops"]}
    audio = {
        loop_id: read_pcm16_mono(bank_directory / loop["derived_filename"])
        for loop_id, loop in loops.items()
    }
    transition_reports = []
    for transition in manifest["transitions"]:
        from_id = transition["from_loop_id"]
        to_id = transition["to_loop_id"]
        from_loop = loops[from_id]
        to_loop = loops[to_id]
        from_samples, sample_rate = audio[from_id]
        to_samples, _ = audio[to_id]
        points = {
            "start": transition["start_revolutions_per_minute"],
            "center": transition["center_revolutions_per_minute"],
            "end": transition["end_revolutions_per_minute"],
        }
        point_reports = {}
        for label, revolutions_per_minute in points.items():
            from_rate = revolutions_per_minute / from_loop["reference_revolutions_per_minute"]
            to_rate = revolutions_per_minute / to_loop["reference_revolutions_per_minute"]
            from_resampled = apply_rate(from_samples, sample_rate, from_rate) * from_loop["calibrated_gain"]
            to_resampled = apply_rate(to_samples, sample_rate, to_rate) * to_loop["calibrated_gain"]
            from_level = loop_level(from_resampled)
            to_level = loop_level(to_resampled)
            from_bands = band_shares(from_resampled, sample_rate)
            to_bands = band_shares(to_resampled, sample_rate)
            point_reports[label] = {
                "revolutions_per_minute": revolutions_per_minute,
                "from_playback_rate": from_rate,
                "to_playback_rate": to_rate,
                "from_level_rms_dbfs": dbfs(from_level),
                "to_level_rms_dbfs": dbfs(to_level),
                "level_delta_db": dbfs(to_level) - dbfs(from_level),
                "band_delta_db": {
                    band: dbfs(to_bands[band] / 100.0) - dbfs(from_bands[band] / 100.0)
                    for band in from_bands
                },
            }
        position = 0.5
        from_rate = transition["center_revolutions_per_minute"] / from_loop["reference_revolutions_per_minute"]
        to_rate = transition["center_revolutions_per_minute"] / to_loop["reference_revolutions_per_minute"]
        from_resampled = apply_rate(from_samples, sample_rate, from_rate) * from_loop["calibrated_gain"]
        to_resampled = apply_rate(to_samples, sample_rate, to_rate) * to_loop["calibrated_gain"]
        from_weight, to_weight = smoothstep_weights(position)
        snapshot_rms, snapshot_peak = crossfade_snapshot(from_resampled, to_resampled, position, 0)
        overlap_length = min(from_resampled.size, to_resampled.size)
        from_weighted = from_resampled[:overlap_length] * from_weight
        to_weighted = to_resampled[:overlap_length] * to_weight
        overshoot = float(np.max(np.abs(from_weighted + to_weighted))) - float(
            max(np.max(np.abs(from_weighted)), np.max(np.abs(to_weighted)))
        )
        offset_count = 32
        offsets = [int(index * overlap_length / offset_count) for index in range(offset_count)]
        offset_peaks = []
        for offset in offsets:
            _, peak = crossfade_snapshot(from_resampled, to_resampled, position, offset)
            offset_peaks.append(peak)
        transition_reports.append(
            {
                "from_loop_id": from_id,
                "to_loop_id": to_id,
                "points": point_reports,
                "center_snapshot": {
                    "crossfade_rms_dbfs": dbfs(snapshot_rms),
                    "crossfade_peak": snapshot_peak,
                    "overshoot_over_loudest_weighted_layer": overshoot,
                    "offset_peak_min": float(min(offset_peaks)),
                    "offset_peak_max": float(max(offset_peaks)),
                    "offset_peak_spread_db": dbfs(max(offset_peaks)) - dbfs(min(offset_peaks)),
                },
            }
        )
    seam_reports = {
        loop_id: measure_loop_seam(samples, rate)
        for loop_id, (samples, rate) in audio.items()
    }
    report = {
        "bank_directory": str(bank_directory),
        "transitions": transition_reports,
        "loop_seams": seam_reports,
    }
    output_path = bank_directory.parent / "measurements" / "transitions.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    figure, axes = plt.subplots(len(transition_reports), 1, figsize=(10, 3 * len(transition_reports)))
    for axis, transition_report in zip(axes, transition_reports):
        labels = list(transition_report["points"])
        deltas = [
            transition_report["points"][label]["level_delta_db"] for label in labels
        ]
        axis.bar(labels, deltas)
        axis.set_title(
            f"{transition_report['from_loop_id']} -> {transition_report['to_loop_id']} level delta"
        )
        axis.set_ylabel("dB")
    figure.tight_layout()
    figure.savefig(bank_directory.parent / "plots" / "transition_level_deltas.png", dpi=90)
    plt.close(figure)
    print(f"transition report: {output_path}")
    for transition_report in transition_reports:
        center = transition_report["points"]["center"]
        print(
            f"  {transition_report['from_loop_id']} -> {transition_report['to_loop_id']} "
            f"center delta {center['level_delta_db']:+.2f}dB "
            f"rates {center['from_playback_rate']:.3f}/{center['to_playback_rate']:.3f}"
        )
    for loop_id, seam in seam_reports.items():
        print(
            f"  {loop_id} wrap/p99 {seam['wrap_step_over_internal_p99']:.3f} "
            f"repeated {seam['repeated_wrap_max_over_internal_p99']:.3f}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
