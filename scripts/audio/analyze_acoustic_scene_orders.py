#!/usr/bin/env python3
"""Measure how AcousticScene stems construct or cancel crank orders.

The analysis is intentionally tied to a stationary render.  It reports raw and
mix-weighted order magnitude, phase relative to scene_mix, magnitude-squared
coherence, signed contribution to the final complex order, low-mid energy and
envelope pumping.  It does not modify source audio.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import warnings
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from scipy.io import wavfile
from scipy.signal import coherence, hilbert, welch


DEFAULT_GAINS = {
    "engine_air": 1.0,
    "metallic_structure": 1.00,
    "gearbox_housing": 0.82,
    "cylinder_head_covers": 0.70,
    "airbox_plenum": 0.64,
    "engine_cover": 0.42,
    "rear_exhaust": 0.48,
    "mount_monocoque": 0.12,
    "under_seat_vibration": 0.09,
    "cockpit_cavity": 0.16,
    "low_mid_parallel": 0.24,
    "load_saturation": 0.18,
}


def read_mono(path: Path) -> tuple[int, np.ndarray]:
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        sample_rate, raw = wavfile.read(path)
    if np.issubdtype(raw.dtype, np.integer):
        info = np.iinfo(raw.dtype)
        audio = raw.astype(np.float64) / float(max(abs(info.min), info.max))
    else:
        audio = raw.astype(np.float64)
    if audio.ndim == 2:
        audio = np.mean(audio, axis=1)
    return int(sample_rate), audio - np.mean(audio)


def complex_tone(audio: np.ndarray, sample_rate: int, frequency: float) -> complex:
    window = np.hanning(len(audio))
    phase = np.exp(-2j * np.pi * frequency * np.arange(len(audio)) / sample_rate)
    return complex(2.0 * np.sum(audio * window * phase) / np.sum(window))


def band_energy(audio: np.ndarray, sample_rate: int, lo: float, hi: float) -> float:
    frequency, power = welch(
        audio,
        sample_rate,
        window="hann",
        nperseg=min(32768, len(audio)),
        noverlap=min(24576, max(0, len(audio) - 1)),
        scaling="spectrum",
    )
    return float(np.sum(power[(frequency >= lo) & (frequency < hi)]))


def pumping(audio: np.ndarray, sample_rate: int, target_hz: float) -> tuple[float, float]:
    envelope = np.abs(hilbert(audio))
    envelope -= np.mean(envelope)
    frequency, power = welch(
        envelope,
        sample_rate,
        window="hann",
        nperseg=min(65536, len(envelope)),
        noverlap=min(49152, max(0, len(envelope) - 1)),
        scaling="spectrum",
    )
    search = (frequency >= target_hz - 2.0) & (frequency <= target_hz + 2.0)
    if not np.any(search):
        return 0.0, 0.0
    local = np.flatnonzero(search)[np.argmax(power[search])]
    return float(frequency[local]), float(power[local])


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("stems_dir", type=Path)
    parser.add_argument("--rpm", required=True, type=float)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--start", type=float, default=0.5)
    parser.add_argument("--end", type=float, default=None)
    parser.add_argument("--max-order", type=float, default=15.0)
    parser.add_argument("--output-gain", type=float, default=2.10)
    args = parser.parse_args()

    args.output_dir.mkdir(parents=True, exist_ok=True)
    mix_path = args.stems_dir / "scene_mix.wav"
    sample_rate, mix_full = read_mono(mix_path)
    begin = round(args.start * sample_rate)
    finish = len(mix_full) if args.end is None else round(args.end * sample_rate)
    mix = mix_full[begin:finish]
    if len(mix) < sample_rate:
        raise SystemExit("analysis window must contain at least one second")

    stems: dict[str, np.ndarray] = {}
    for name in DEFAULT_GAINS:
        path = args.stems_dir / f"{name}.wav"
        if path.exists():
            stem_rate, audio = read_mono(path)
            if stem_rate != sample_rate:
                raise SystemExit(f"sample-rate mismatch: {path}")
            stems[name] = audio[begin:finish] * DEFAULT_GAINS[name] * args.output_gain

    shaft_hz = args.rpm / 60.0
    orders = np.arange(0.5, args.max_order + 0.25, 0.5)
    rows: list[dict] = []
    for order in orders:
        frequency = shaft_hz * order
        mix_tone = complex_tone(mix, sample_rate, frequency)
        mix_power = abs(mix_tone) ** 2 + 1e-30
        for name, audio in stems.items():
            tone = complex_tone(audio, sample_rate, frequency)
            relative_phase = math.degrees(np.angle(tone * np.conj(mix_tone)))
            contribution = float(np.real(tone * np.conj(mix_tone)) / mix_power)
            f_coh, coh = coherence(
                audio,
                mix,
                fs=sample_rate,
                window="hann",
                nperseg=min(16384, len(audio)),
                noverlap=min(12288, max(0, len(audio) - 1)),
            )
            coherence_at_order = float(np.interp(frequency, f_coh, coh))
            rows.append(
                {
                    "stem": name,
                    "order": float(order),
                    "frequency_hz": frequency,
                    "weighted_magnitude": abs(tone),
                    "relative_to_mix_db": 20.0 * math.log10((abs(tone) + 1e-30) / (abs(mix_tone) + 1e-30)),
                    "phase_relative_mix_deg": relative_phase,
                    "coherence": coherence_at_order,
                    "signed_mix_contribution": contribution,
                }
            )

    with (args.output_dir / "stem_order_matrix.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)

    diagnostics = []
    engine_cycle_hz = shaft_hz / 2.0
    mix_low_mid = band_energy(mix, sample_rate, 80.0, 600.0) + 1e-30
    for name, audio in stems.items():
        pump_hz, pump_power = pumping(audio, sample_rate, engine_cycle_hz)
        diagnostics.append(
            {
                "stem": name,
                "gain": DEFAULT_GAINS[name],
                "rms_dbfs": 20.0 * math.log10(np.sqrt(np.mean(audio**2)) + 1e-30),
                "band_80_600_energy": band_energy(audio, sample_rate, 80.0, 600.0),
                "band_80_600_vs_mix_db": 10.0 * math.log10(
                    (band_energy(audio, sample_rate, 80.0, 600.0) + 1e-30) / mix_low_mid
                ),
                "engine_cycle_pump_hz": pump_hz,
                "engine_cycle_pump_power": pump_power,
            }
        )
    diagnostics.sort(key=lambda row: row["band_80_600_energy"], reverse=True)
    (args.output_dir / "stem_diagnostics.json").write_text(
        json.dumps(
            {
                "rpm": args.rpm,
                "shaft_hz": shaft_hz,
                "engine_cycle_hz": engine_cycle_hz,
                "window_seconds": [args.start, finish / sample_rate],
                "stems": diagnostics,
            },
            indent=2,
        ),
        encoding="utf-8",
    )

    selected = [1.0, 1.5, 2.0, 2.5, 3.5, 4.5, 5.0]
    names = list(stems)
    matrix = np.zeros((len(names), len(selected)))
    for i, name in enumerate(names):
        for j, order in enumerate(selected):
            row = next(r for r in rows if r["stem"] == name and r["order"] == order)
            matrix[i, j] = row["signed_mix_contribution"]
    fig, ax = plt.subplots(figsize=(11, 7), constrained_layout=True)
    limit = max(0.25, float(np.percentile(np.abs(matrix), 95)))
    image = ax.imshow(matrix, aspect="auto", cmap="RdBu_r", vmin=-limit, vmax=limit)
    ax.set_xticks(range(len(selected)), [str(order) for order in selected])
    ax.set_yticks(range(len(names)), names)
    ax.set_xlabel("Crank order")
    ax.set_title("Signed contribution to AcousticScene mix (red constructs, blue cancels)")
    fig.colorbar(image, ax=ax, label="fraction of final complex order")
    fig.savefig(args.output_dir / "stem_order_contribution.png", dpi=170)
    plt.close(fig)

    print(f"matrix={args.output_dir / 'stem_order_matrix.csv'}")
    print(f"diagnostics={args.output_dir / 'stem_diagnostics.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
