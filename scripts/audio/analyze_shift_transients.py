"""Shift-transient evidence for V10 renders.

Compares a shift render against its no-shift control (same seed/args) and reports
the sample-aligned difference energy in configurable bands inside a window
around each shift event. Intended for A/B evidence of procedural shift
transients, not as an audio quality verdict.
"""

from __future__ import annotations

import argparse
import math
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import butter, sosfiltfilt

DEFAULT_BANDS = [(100, 300), (300, 1000), (1000, 4000), (4000, 9000)]


def parse_bands(spec: str) -> list[tuple[float, float]]:
    bands: list[tuple[float, float]] = []
    for chunk in spec.split(","):
        low, high = chunk.split("-")
        bands.append((float(low), float(high)))
    return bands


def parse_times(spec: str) -> list[float]:
    return [float(chunk) for chunk in spec.split(",")]


def load_mono(path: Path) -> tuple[np.ndarray, int]:
    data, sample_rate = sf.read(path, dtype="float64", always_2d=True)
    return data.mean(axis=1), sample_rate


def band_signal(signal: np.ndarray, sample_rate: int, low: float, high: float) -> np.ndarray:
    nyquist = sample_rate / 2.0
    sos = butter(4, [max(low, 1.0) / nyquist, min(high, nyquist * 0.999) / nyquist], btype="band", output="sos")
    return sosfiltfilt(sos, signal)


def rms_db(signal: np.ndarray) -> float:
    value = float(np.sqrt(np.mean(signal**2)))
    return 20.0 * math.log10(value) if value > 0.0 else -300.0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("control", type=Path)
    parser.add_argument("--events", type=parse_times, required=True, help="shift times in seconds")
    parser.add_argument("--bands", type=parse_bands, default=DEFAULT_BANDS)
    parser.add_argument("--pre-ms", type=float, default=50.0)
    parser.add_argument("--post-ms", type=float, default=400.0)
    args = parser.parse_args()

    candidate, sample_rate = load_mono(args.candidate)
    control, control_rate = load_mono(args.control)
    if sample_rate != control_rate:
        raise SystemExit("candidate and control sample rates differ")
    frames = min(candidate.size, control.size)
    candidate = candidate[:frames]
    control = control[:frames]
    delta = candidate - control

    labels = [f"{low:g}-{high:g}" for low, high in args.bands]
    print(f"file={args.candidate}")
    print(f"control={args.control} rate={sample_rate} frames={frames}")
    print(f"window pre={args.pre_ms:g} ms post={args.post_ms:g} ms")
    header = f"{'event_s':>8s} " + " ".join(f"{label:>10s}" for label in labels) + f" {'peak_ms':>8s} {'peak_band':>10s}"
    print(header)
    for event in args.events:
        start = max(0, int((event - args.pre_ms / 1000.0) * sample_rate))
        end = min(frames, int((event + args.post_ms / 1000.0) * sample_rate))
        if end <= start:
            print(f"{event:8.3f} outside render")
            continue
        window_delta = delta[start:end]
        window_control = control[start:end]
        values = []
        peak_value = -300.0
        peak_band = "-"
        for label, (low, high) in zip(labels, args.bands):
            banded = band_signal(window_delta, sample_rate, low, high)
            level = rms_db(banded)
            reference = max(rms_db(band_signal(window_control, sample_rate, low, high)), -120.0)
            values.append(level - reference)
            if level - reference > peak_value:
                peak_value = level - reference
                peak_band = label
        peak_index = int(np.argmax(np.abs(window_delta)))
        peak_ms = (start + peak_index) / sample_rate * 1000.0 - event * 1000.0
        print(f"{event:8.3f} " + " ".join(f"{value:10.2f}" for value in values) + f" {peak_ms:8.1f} {peak_band:>10s}")
    print("\nvalues are delta-vs-control band RMS in dB")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
